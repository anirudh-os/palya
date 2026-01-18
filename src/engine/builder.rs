use crate::cmd::Args;
use crate::domain::models::{BuildCache, Config, ContentItem, GlobalHash};
use crate::io::fs::{copy_static_files, load_templates};
use anyhow::{Context, Result};
use minijinja::{Environment, Value, context};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use walkdir::WalkDir;

pub struct Site {
    env: Environment<'static>,
    input_dir: PathBuf,
    output_dir: PathBuf,
    static_dir: Option<PathBuf>,
    consider_drafts: bool,
    cache: Option<BuildCache>,
    fresh_build: bool,
    config_hash: [u8; 32],
    template_hash: HashMap<PathBuf, [u8; 32]>,
}

impl Site {
    pub fn new(args: Args) -> Result<Self> {
        let input_dir = match args.input {
            Some(path) => path,
            None => PathBuf::from("."),
        };

        let output_dir = match args.output {
            Some(path) => path,
            None => PathBuf::from(&input_dir).join("dist"),
        };

        let templates_dir = match args.templates {
            Some(path) => path,
            None => PathBuf::from(&input_dir).join("templates"),
        };

        let mut env = Environment::new();

        let cache = BuildCache::new(&output_dir).ok();

        let (config, config_hash) = Config::load(&input_dir, args.config.as_ref())?;
        env.add_global("site", Value::from_serialize(&config));

        let template_hash = load_templates(&mut env, &templates_dir)?;

        let mut fresh_build = false;

        match &cache {
            Some(c) => {
                if c.global_hash.config_hash != config_hash {
                    fresh_build = true;
                } else if c.global_hash.templates_hash != template_hash {
                    fresh_build = true;
                }
            }
            None => fresh_build = true,
        }

        Ok(Site {
            env,
            input_dir,
            output_dir,
            static_dir: args.static_dir,
            consider_drafts: args.drafts,
            cache,
            fresh_build,
            config_hash,
            template_hash,
        })
    }

    pub fn build(self) -> Result<()> {
        fs::create_dir_all(&self.output_dir).context("Could not create output directory")?;
        copy_static_files(&self.input_dir, self.static_dir.as_ref(), &self.output_dir)?;

        let content_path = PathBuf::from(&self.input_dir).join("content");

        let paths: Vec<PathBuf> = WalkDir::new(&content_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.path().to_owned())
            .filter(|p| p.extension().map_or(false, |e| e == "md"))
            .collect();

        // Parse content
        let content_files: Vec<(PathBuf, ContentItem, [u8; 32])> = paths
            .par_iter()
            .filter_map(|p| {
                // Pass content_path as base to calculate collection
                match ContentItem::from_file(&content_path, p.clone(), &self.consider_drafts) {
                    Ok((Some(post), hash)) => Some((p.clone(), post, hash)),
                    Ok((None, _)) => None,
                    Err(e) => {
                        eprintln!("Warning: Skipping {:?} due to error: {}", p, e);
                        None
                    }
                }
            })
            .collect();

        let mut post_cache = HashMap::new();
        let mut content_items: Vec<ContentItem> = Vec::new();
        let mut index_item: Option<ContentItem> = None;

        // Aggregation for "Collections"
        let mut collections_map: HashMap<String, Vec<&ContentItem>> = HashMap::new();

        for (path, item, hash) in content_files.into_iter() {
            post_cache.insert(path, hash);

            // To check if its the index page
            if item.collection == "pages" && item.source.file_stem().unwrap() == "index" {
                index_item = Some(item);
            } else {
                content_items.push(item);
            }
        }

        // Sort items by date descending
        content_items.sort_by(|a, b| {
            let date_a = a.frontmatter.as_ref().and_then(|f| f.date.as_deref());
            let date_b = b.frontmatter.as_ref().and_then(|f| f.date.as_deref());
            date_b.cmp(&date_a)
        });

        // Populate collections map for the Index page
        for item in &content_items {
            collections_map
                .entry(item.collection.clone())
                .or_default()
                .push(item);
        }

        self.render_content(&content_items, &post_cache)?;
        self.render_index(&content_items, &collections_map, index_item.as_ref())?; // Pass map here
        self.render_tags(&content_items)?;

        let new_cache = BuildCache {
            file_cache: post_cache,
            global_hash: GlobalHash {
                config_hash: self.config_hash,
                templates_hash: self.template_hash,
            },
        };
        new_cache.save(&self.output_dir)?;

        Ok(())
    }

    fn render_index(
        &self,
        all_items: &[ContentItem],
        collections: &HashMap<String, Vec<&ContentItem>>,
        home_content: Option<&ContentItem>
    ) -> Result<()> {
        // We pass posts and collections
        let ctx = context! {
            posts => all_items,
            collections => collections,
            page => home_content
        };

        match self.env.get_template("index.j2") {
            Ok(tmpl) => {
                let out = tmpl.render(ctx)?;
                let mut f = File::create(self.output_dir.join("index.html"))?;
                f.write_all(out.as_bytes())?;
            }
            Err(_) => eprintln!("Skipping index.html (index.j2 not found)"),
        }
        Ok(())
    }

    fn render_content(
        &self,
        items: &[ContentItem],
        item_cache: &HashMap<PathBuf, [u8; 32]>,
    ) -> Result<()> {
        fs::create_dir_all(self.output_dir.join("posts"))?;
        let mut items_mod = HashSet::new();

        if let Some(cache) = &self.cache {
            for (path, hash) in item_cache {
                if cache.file_cache.get(path).unwrap_or(&[0; 32]) != hash {
                    items_mod.insert(path.clone());
                }
            }
        }

        items.par_iter().try_for_each(|item| -> Result<()> {
            if !self.fresh_build && !items_mod.contains(&item.source) {
                return Ok(());
            }

            let tmpl_name = item.template_name();

            // DYNAMIC CONTEXT INJECTION
            // This matches your templates: project.j2 uses "project", about.j2 uses "page"
            let ctx = match item.collection.as_str() {
                "projects" => context! { project => item },
                "pages" => context! { page => item },
                "blog" | _ => context! { post => item },
            };

            let tmpl = match self.env.get_template(&tmpl_name) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!(
                        "Template {} not found for {:?}: {}",
                        tmpl_name, item.source, e
                    );
                    return Ok(());
                }
            };

            let rendered = tmpl.render(ctx)?;
            let out_path = item.output_path(&self.output_dir);

            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let file = File::create(out_path)?;
            let mut writer = BufWriter::new(file);
            writer.write_all(rendered.as_bytes())?;
            Ok(())
        })?;
        Ok(())
    }

    fn render_tags(&self, items: &[ContentItem]) -> Result<()> {
        let mut tag_map: HashMap<String, Vec<&ContentItem>> = HashMap::new();
        fs::create_dir_all(&self.output_dir.join("tags"))
            .context("Could not create tag directory")?;

        for item in items {
            if let Some(fm) = &item.frontmatter {
                if let Some(tags) = fm.get_tags() {
                    for tag in tags {
                        tag_map.entry(tag.clone()).or_default().push(item);
                    }
                }
            }
        }

        for (tag, list) in tag_map {
            let ctx = context! {
                tag => tag.clone(),
                posts => list, // Keeping 'posts' key for tag.j2 compatibility
            };
            match self.env.get_template("tag.j2") {
                Ok(tmpl) => {
                    let out = tmpl.render(ctx)?;
                    let tag_slug = tag.replace(' ', "-").to_lowercase();
                    let path = self
                        .output_dir
                        .join("tags")
                        .join(format!("{}.html", tag_slug));
                    let mut f = File::create(path)?;
                    f.write_all(out.as_bytes())?;
                }
                Err(_) => {
                    eprintln!("Tag template tag.j2 not found");
                    break;
                }
            }
        }

        Ok(())
    }
}

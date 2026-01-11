use crate::cmd::Args;
use crate::domain::models::{BuildCache, Config, GlobalHash, Post};
use crate::io::fs::{copy_static_files, load_templates};
use anyhow::{Context, Ok, Result};
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

        let cache = match BuildCache::new(&output_dir) {
            Result::Ok(c) => Some(c),
            Result::Err(_) => None,
        };

        let (config, config_hash) = Config::load(&input_dir, args.config.as_ref())?;
        env.add_global("site", Value::from_serialize(&config));

        let template_hash = load_templates(&mut env, &templates_dir)?;

        let mut fresh_build = false;

        match &cache {
            Some(cache) => {
                if cache.global_hash.config_hash != config_hash {
                    fresh_build = true;
                } else {
                    if cache.global_hash.templates_hash != template_hash {
                        fresh_build = true;
                    }
                }
            }
            None => fresh_build = true,
        }

        Ok(Site {
            env,
            input_dir,
            output_dir,
            static_dir: args.static_dir.clone(),
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
        let paths: Vec<PathBuf> = WalkDir::new(content_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.path().to_owned())
            .filter(|p| p.extension().map_or(false, |e| e == "md"))
            .collect();

        let content_files: Vec<(PathBuf, Post, [u8; 32])> = paths
            .par_iter()
            .filter_map(
                |p| match Post::from_file(p.clone(), &self.consider_drafts) {
                    Result::Ok((Some(post), hash)) => Some((p.clone(), post, hash)),
                    Result::Ok((None, _)) => None,
                    Result::Err(e) => {
                        eprintln!("Warning: Skipping post {:#?} due to error: {}", p, e);
                        None
                    }
                },
            )
            .collect();

        let mut post_cache = HashMap::new();
        let mut posts: Vec<Post> = content_files
            .into_iter()
            .map(|(path, post, hash)| {
                post_cache.insert(path, hash);
                post
            })
            .collect();

        posts.sort_by(|a, b| {
            let date_a = a.frontmatter.as_ref().and_then(|f| f.date.as_deref());
            let date_b = b.frontmatter.as_ref().and_then(|f| f.date.as_deref());
            date_b.cmp(&date_a)
        });

        self.render_posts(&posts, &post_cache)?;

        self.render_index(&posts)?;

        self.render_tags(&posts)?;

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

    fn render_index(&self, posts: &[Post]) -> Result<()> {
        let ctx = context! { posts => posts };
        match self.env.get_template("index.j2") {
            Result::Ok(tmpl) => {
                let out = tmpl.render(ctx)?;
                let mut f = File::create(self.output_dir.join("index.html"))?;
                f.write_all(out.as_bytes())?;
            }
            Err(_) => eprintln!("Skipping index.html (index.j2 not found)"),
        }
        Ok(())
    }

    fn render_posts(&self, posts: &[Post], post_cache: &HashMap<PathBuf, [u8; 32]>) -> Result<()> {
        fs::create_dir_all(self.output_dir.join("posts"))?;
        let mut posts_mod = HashSet::new();

        match &self.cache {
            Some(cache) => {
                for (path, hash) in post_cache {
                    if cache.file_cache.get(path).unwrap_or(&[0; 32]) != hash {
                        posts_mod.insert(path.clone());
                    }
                }
            }
            None => {}
        }

        posts
            .par_iter()
            .try_for_each(|post| -> Result<()> {
                if !posts_mod.contains(&post.source) && !self.fresh_build && !self.consider_drafts {
                    ()
                }
                let tmpl_name = post.template_name();
                let ctx = context! { post => post };
                let tmpl = match self.env.get_template(tmpl_name) {
                    Result::Ok(t) => t,
                    Result::Err(e) => {
                        eprintln!(
                            "Template {} not found for post {:?}: {}",
                            tmpl_name, post.source, e
                        );
                        return Ok(());
                    }
                };

                let rendered = tmpl.render(ctx)?;
                let out_path = post.output_path(&self.output_dir);

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

    fn render_tags(&self, posts: &[Post]) -> Result<()> {
        let mut tag_map: HashMap<String, Vec<&Post>> = HashMap::new();
        fs::create_dir_all(&self.output_dir.join("tags"))
            .context("Could not create tag directory")?;

        for post in posts {
            let Some(fm) = post.frontmatter.as_ref() else {
                continue;
            };
            let Some(tags) = fm.get_tags() else { continue };

            for tag in tags {
                tag_map.entry(tag.clone()).or_default().push(post);
            }
        }

        for (tag, post_list) in tag_map {
            let ctx = context! {
                tag => tag.clone(),
                posts => post_list,
            };
            match self.env.get_template("tag.j2") {
                Result::Ok(tmpl) => {
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
                    eprintln!("Couldn't render the tag pages as tag.j2 is not found!");
                    break;
                }
            }
        }

        Ok(())
    }
}

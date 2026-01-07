use crate::cmd::Args;
use crate::domain::models::{Config, Post};
use crate::io::fs::{copy_static_files, load_templates};
use anyhow::{Context, Ok, Result};
use minijinja::{Environment, Value, context};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use walkdir::WalkDir;

pub struct Site {
    env: Environment<'static>,
    input_dir: PathBuf,
    output_dir: PathBuf,
    // templates_dir: PathBuf,
    static_dir: Option<PathBuf>,
    consider_drafts: bool,
}

impl Site {
    pub fn new(args: Args) -> Result<Self> {
        let mut env = Environment::new();

        let config = Config::load(args.input.as_ref(), args.config.as_ref())?;
        env.add_global("site", Value::from_serialize(&config));

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

        load_templates(&mut env, &input_dir, &templates_dir)?;

        Ok(Site {
            env,
            input_dir,
            output_dir,
            static_dir: args.static_dir.clone(),
            consider_drafts: args.drafts,
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

        let mut posts: Vec<Post> = paths
            .par_iter()
            .filter_map(
                |p| match Post::from_file(p.clone(), &self.consider_drafts) {
                    Err(e) => {
                        eprintln!("Warning: Skipping post {:#?} due to error: {}", p, e);
                        None
                    }
                    Result::Ok(Some(post)) => Some(post),
                    Result::Ok(None) => None,
                },
            )
            .collect();

        posts.sort_by(|a, b| {
            let date_a = a.frontmatter.as_ref().and_then(|f| f.date.as_deref());
            let date_b = b.frontmatter.as_ref().and_then(|f| f.date.as_deref());
            date_b.cmp(&date_a)
        });

        self.render_posts(&posts)?;

        self.render_index(&posts)?;

        self.render_tags(&posts)?;

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

    fn render_posts(&self, posts: &[Post]) -> Result<()> {
        posts.par_iter().try_for_each(|post| -> Result<()> {
            let tmpl_name = post.template_name();
            let ctx = context! { post => post };
            let rendered = self
                .env
                .get_template(tmpl_name)
                .with_context(|| format!("Template {} not found", tmpl_name))?
                .render(ctx)?;

            let out_path = post.output_path(&self.output_dir);
            if let Some(p) = out_path.parent() {
                fs::create_dir_all(p)?;
            }

            let mut f = File::create(out_path)?;
            f.write_all(rendered.as_bytes())?;
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
                    let mut f = File::create(self.output_dir.join("tags").join(format!("{}.html", tag)))?;
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

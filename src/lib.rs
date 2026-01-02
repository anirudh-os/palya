use anyhow::{Context, Ok, Result};
use minijinja::{Environment};
use pulldown_cmark::{Parser, html};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::from_str;
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FrontMatter {
    pub title: Option<String>,
    pub date: Option<String>,
    pub slug: Option<String>,
    pub template: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Post {
    pub frontmatter: Option<FrontMatter>,
    pub body: String,
    pub url: String, // This is the given or calculated from slug
}

#[derive(Serialize)]
pub struct PostContext<'a> {
    pub title: Option<&'a str>,
    pub date: Option<&'a str>,
    pub content: &'a str,
    pub url: String,
}

impl Post {
    pub fn from_file(input_path: PathBuf) -> Result<Post> {
        let content = fs::read_to_string(input_path.clone())
            .with_context(|| format!("Couldn't read the contents of {}", input_path.display()))?;

        let (frontmatter, body) = if let Some(rest) = content.strip_prefix("---") {
            if let Some((fm, body)) = rest.split_once("---") {
                let fm = Some(
                    from_str::<FrontMatter>(fm)
                        .with_context(|| format!("Invalid frontmatter in {:?}", input_path))?,
                );
                (fm, body)
            } else {
                (None, content.as_str())
            }
        } else {
            (None, content.as_str())
        };

        let parser = Parser::new(body);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        let slug = frontmatter
            .as_ref()
            .and_then(|fm| fm.slug.as_deref())
            .or_else(|| input_path.file_stem().and_then(|s| s.to_str()))
            .unwrap_or("post1")
            .to_string();

        let url = format!("/{}/", slug);

        Ok(Post {
            frontmatter,
            body: html_output,
            url,
        })
    }

    pub fn template_name(&self) -> &str {
        self.frontmatter
            .as_ref()
            .and_then(|fm| fm.template.as_deref())
            .unwrap_or("post.jinja")
    }

    pub fn output_path(&self, output: &PathBuf) -> PathBuf {
        let slug_path = self
            .url
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<PathBuf>();

        PathBuf::from(output).join(slug_path).join("index.html")
    }

    pub fn as_context(&self) -> PostContext<'_> {
        PostContext {
            title: self.frontmatter.as_ref().and_then(|f| f.title.as_deref()),
            date: self.frontmatter.as_ref().and_then(|f| f.date.as_deref()),
            content: &self.body,
            url: self.url.clone(),
        }
    }
}

pub fn load_templates(env: &mut Environment, dir: &Path) -> Result<()> {
    for entry in WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let name = path
                .strip_prefix(dir)
                .with_context(|| {
                    format!(
                        "Couldn't strip the prefix of {}!",
                        path.to_str().unwrap_or("unknown")
                    )
                })?
                .to_string_lossy()
                .into_owned();

            let content = fs::read_to_string(path)?;

            env.add_template_owned(name.clone(), content)
                .with_context(|| format!("Couldn't add the template {}", name))?;
        }
    }
    Ok(())
}

pub fn copy_static_files(src: &Path, dest: &Path) -> Result<()>{
    if !src.exists() {
        return Ok(())
    }

    for entry in WalkDir::new(src) {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let relative_path = path.strip_prefix(src)?;

            let target_path = dest.join(relative_path);

            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).with_context(|| format!("A directory specified in {:#?} does not exist and could not be created", target_path))?;
            }

            fs::copy(path, target_path)?;
        }
    }

    Ok(())
}
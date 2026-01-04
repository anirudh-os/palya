use anyhow::{Context, Result};
use pulldown_cmark::{Parser, html};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::from_str;
use std::{
    fs::{self, read_to_string},
    path::{Path, PathBuf},
};

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
    pub content: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub title: String,
    pub description: Option<String>,
    pub base_url: Option<String>,
}

impl Post {
    pub fn from_file(input_path: PathBuf) -> Result<Post> {
        let content = fs::read_to_string(&input_path)
            .with_context(|| format!("Couldn't read contents of {}", input_path.display()))?;
        Self::parse(content, input_path)
    }

    pub fn parse(content: String, input_path: PathBuf) -> Result<Post> {
        let (frontmatter, content) = if let Some(rest) = content.strip_prefix("---") {
            if let Some((fm, content)) = rest.split_once("---") {
                let fm = Some(
                    from_str::<FrontMatter>(fm)
                        .with_context(|| format!("Failed to parse frontmatter: {:?}", input_path))?
                );
                (fm, content)
            } else {
                (None, content.as_str())
            }
        } else {
            (None, content.as_str())
        };

        let parser = Parser::new(content);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        let slug = frontmatter
            .as_ref()
            .and_then(|fm| fm.slug.as_deref())
            .or_else(|| input_path.file_stem().and_then(|s| s.to_str()))
            .unwrap_or("post1")
            .to_string();

        let url = format!("/{}/", slug);

        Ok(Post { frontmatter, content: html_output, url })
    }

    pub fn template_name(&self) -> &str {
        self.frontmatter.as_ref()
            .and_then(|fm| fm.template.as_deref())
            .unwrap_or("post.j2")
    }

    pub fn output_path(&self, output_dir: &Path) -> PathBuf {
        let slug_path: PathBuf = self.url.split('/').filter(|s| !s.is_empty()).collect();
        output_dir.join(slug_path).join("index.html")
    }
}

impl Config {
    pub fn load(input_dir: &Path, config_path: Option<&PathBuf>) -> Result<Config> {
        let path_to_read = match config_path {
            Some(p) => p.clone(),
            None => {
                let default = input_dir.join("palya.toml");
                if !default.exists() {
                    return Ok(Config {
                        title: "My Blog".to_string(),
                        description: None,
                        base_url: None,
                    });
                }
                default
            }
        };

        let config_str = read_to_string(&path_to_read)
            .with_context(|| format!("Found config at {:?} but couldn't read it", path_to_read))?;

        toml::from_str::<Config>(&config_str)
            .with_context(|| format!("Failed to parse TOML in {:?}", path_to_read))
    }
}
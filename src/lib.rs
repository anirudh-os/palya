use anyhow::{Context, Ok, Result};
use minijinja::Environment;
use pulldown_cmark::{Parser, html};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::from_str;
use std::{
    fs::{self, read_to_string},
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub title: String,
    pub description: Option<String>,
    pub base_url: Option<String>,
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
}

impl Config {
    pub fn get_config(input: &Path, config_path: Option<PathBuf>) -> Result<Config> {
        let path_to_read = match config_path {
            Some(p) => p,
            None => {
                let parent = input.parent().unwrap_or(Path::new("."));
                let default_config = parent.join("palya.toml");

                if !default_config.exists() {
                    return Ok(Config {
                        title: "My Blog".to_string(),
                        description: None,
                        base_url: None,
                    });
                }
                default_config
            }
        };

        let config_str = read_to_string(&path_to_read)
            .with_context(|| format!("Found config at {:?} but couldn't read it", path_to_read))?;

        toml::from_str::<Config>(&config_str)
            .with_context(|| format!("Failed to parse TOML in {:?}", path_to_read))
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

pub fn copy_static_files(input: &Path, src: Option<PathBuf>, dest: &Path) -> Result<()> {
    let source_dir = match src {
        Some(path) => path,
        None => {
            let parent = input.parent().unwrap_or(Path::new("."));
            let default_static = parent.join("static");

            if !default_static.exists() {
                return Ok(());
            }
            default_static
        }
    };

    for entry in WalkDir::new(&source_dir) {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let relative_path = path.strip_prefix(&source_dir)?;
            let target_path = dest.join(relative_path);

            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Could not create directory {:#?}", parent))?;
            }

            fs::copy(path, &target_path)?;
        }
    }

    Ok(())
}

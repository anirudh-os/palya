use anyhow::{Context, Result};
use minijinja::Environment;
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
}

#[derive(Serialize)]
pub struct PostContext<'a> {
    pub title: Option<&'a str>,
    pub date: Option<&'a str>,
    pub content: &'a str,
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

        Ok(Post {
            frontmatter,
            body: html_output,
        })
    }

    pub fn template_name(&self) -> &str {
        self.frontmatter
            .as_ref()
            .and_then(|fm| fm.template.as_deref())
            .unwrap_or("post.html")
    }

    pub fn output_path(&self, input_path: PathBuf) -> PathBuf {
        let slug = self
            .frontmatter
            .as_ref()
            .and_then(|fm| fm.slug.as_deref())
            .or_else(|| input_path.file_stem().and_then(|s| s.to_str()))
            .unwrap_or("post1");

        PathBuf::from("dist").join(slug).join("index.html")
    }

    pub fn as_context(&self) -> PostContext<'_> {
        PostContext {
            title: self.frontmatter.as_ref().and_then(|f| f.title.as_deref()),
            date: self.frontmatter.as_ref().and_then(|f| f.date.as_deref()),
            content: &self.body,
        }
    }
}

pub fn load_templates(env: &mut Environment, dir: &Path) -> Result<()> {
    for entry in WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let name = path.file_name().unwrap().to_string_lossy().into_owned(); 

            let content = fs::read_to_string(path)?;

            env.add_template_owned(name, content)?;
        }
    }
    Ok(())
}

use anyhow::{Context, Result};
use pulldown_cmark::{Parser, html};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::from_str;
use std::{fs, path::PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FrontMatter {
    pub title: Option<String>,
    pub date: Option<String>,
    pub slug: Option<String>,
    pub template: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Post {
    pub frontmatter: Option<FrontMatter>,
    pub body: String,
    pub output_path: PathBuf,
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
            .unwrap_or("post1");

        let output_path = PathBuf::from("dist").join(slug).join("index.html");

        Ok(Post {
            frontmatter,
            body: html_output,
            output_path: PathBuf::from(output_path),
        })
    }

    pub fn template_name(&self) -> &str {
        self.frontmatter
            .as_ref()
            .and_then(|fm| fm.template.as_deref())
            .unwrap_or("post.html")
    }
}

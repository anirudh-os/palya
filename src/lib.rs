use pulldown_cmark::{Parser, html};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::from_str;
use std::{error::Error, fs, path::PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FrontMatter {
    pub title: Option<String>,
    pub date: Option<String>,
    pub slug: Option<String>,
    pub template: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Post {
    pub frontmatter: Option<FrontMatter>,
    pub body: String,
    pub output_path: PathBuf,
}

impl Post {
    pub fn from_file(input_path: PathBuf) -> Result<Post, Box<dyn Error>> {
        let content = fs::read_to_string(input_path.clone())?;

        let (frontmatter, body) = if let Some(rest) = content.strip_prefix("---") {
            if let Some((fm, body)) = rest.split_once("---") {
                let fm = Some(from_str::<FrontMatter>(fm)?);
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

        let mut output_path = PathBuf::new();

        if let Some(fm) = frontmatter.as_ref() {
            if let Some(slug) = fm.slug.as_ref() {
                output_path = PathBuf::from("dist").join(slug).join("index.html");
            }
        } else {
            let name = input_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("post1");

            output_path = PathBuf::from("dist").join(name).join("index.html");
        }

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

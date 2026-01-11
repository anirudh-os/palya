use anyhow::{Context, Result};
use blake3::hash;
use pulldown_cmark::{Parser, html};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::from_str;
use std::{
    collections::HashMap,
    fs::{self, File, read_to_string},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Tags {
    Single(String),
    Multiple(Vec<String>),
    Null,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FrontMatter {
    pub title: Option<String>,
    pub date: Option<String>,
    pub slug: Option<String>,
    pub template: Option<String>,

    #[serde(default)]
    pub draft: bool,
    pub tags: Option<Tags>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Post {
    pub frontmatter: Option<FrontMatter>,
    pub content: String,
    pub url: String,
    pub tags: Option<Vec<String>>,
    pub source: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub title: String,
    pub description: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GlobalHash {
    pub config_hash: [u8; 32],
    pub templates_hash: HashMap<PathBuf, [u8; 32]>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BuildCache {
    pub file_cache: HashMap<PathBuf, [u8; 32]>,
    pub global_hash: GlobalHash,
}

impl Post {
    pub fn from_file(input_path: PathBuf, drafts: &bool) -> Result<(Option<Post>, [u8; 32])> {
        let content = fs::read_to_string(&input_path)
            .with_context(|| format!("Couldn't read contents of {}", input_path.display()))?;
        let content_hash = *hash(content.as_bytes()).as_bytes();
        let post = Self::parse(content, input_path, drafts)?;
        Ok((post, content_hash))
    }

    pub fn parse(content: String, input_path: PathBuf, drafts: &bool) -> Result<Option<Post>> {
        let (frontmatter, content) = if let Some(rest) = content.strip_prefix("---") {
            if let Some((fm, content)) = rest.split_once("---") {
                let fm =
                    Some(from_str::<FrontMatter>(fm).with_context(|| {
                        format!("Failed to parse frontmatter: {:?}", input_path)
                    })?);
                (fm, content)
            } else {
                (None, content.as_str())
            }
        } else {
            (None, content.as_str())
        };

        let mut tags: Option<Vec<String>> = None;

        if let Some(fm) = &frontmatter {
            if fm.draft && !*drafts {
                return Ok(None);
            }
            tags = fm.get_tags();
        }

        let parser = Parser::new(content);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        let slug = frontmatter
            .as_ref()
            .and_then(|fm| fm.slug.as_deref())
            .or_else(|| input_path.file_stem().and_then(|s| s.to_str()))
            .unwrap_or("post1")
            .to_string();

        let url = format!("/posts/{}/", slug);

        Ok(Some(Post {
            frontmatter,
            content: html_output,
            url,
            tags,
            source: input_path,
        }))
    }

    pub fn template_name(&self) -> &str {
        self.frontmatter
            .as_ref()
            .and_then(|fm| fm.template.as_deref())
            .unwrap_or("post.j2")
    }

    pub fn output_path(&self, output_dir: &Path) -> PathBuf {
        let slug_path: PathBuf = self.url.split('/').filter(|s| !s.is_empty()).collect();
        output_dir.join(slug_path).join("index.html")
    }
}

impl Config {
    pub fn load(input_dir: &Path, config_path: Option<&PathBuf>) -> Result<(Config, [u8; 32])> {
        let path_to_read = match config_path {
            Some(p) => p.clone(),
            None => {
                let default = input_dir.join("palya.toml");
                if !default.exists() {
                    return Ok((
                        Config {
                            title: "My Blog".to_string(),
                            description: None,
                            base_url: Some("".to_string()),
                        },
                        [0; 32],
                    ));
                }
                default
            }
        };

        let config_str = read_to_string(&path_to_read)
            .with_context(|| format!("Found config at {:?} but couldn't read it", path_to_read))?;

        let config_hash = *hash(config_str.as_bytes()).as_bytes();

        let mut config = toml::from_str::<Config>(&config_str)
            .with_context(|| format!("Failed to parse TOML in {:?}", path_to_read))?;

        if config.base_url == None {
            config.base_url = Some("".to_string());
        }

        Ok((config, config_hash))
    }
}

impl FrontMatter {
    pub fn get_tags(&self) -> Option<Vec<String>> {
        match &self.tags {
            Some(Tags::Single(tag)) => Some(vec![tag.clone()]),
            Some(Tags::Multiple(tags)) => Some(tags.clone()),
            Some(Tags::Null) | None => {
                println!(
                    "Couldn't find the tags in the frontmatter of {:?}!",
                    self.title
                );
                None
            }
        }
    }
}

impl BuildCache {
    pub fn new(output_dir: &Path) -> Result<Self> {
        let cache_path = output_dir.join(".palya_cache.json");

        let cache_str = read_to_string(&cache_path)?;
        let cache = serde_json::from_str::<BuildCache>(&cache_str)
            .with_context(|| format!("Failed to parse JSON in {:?}", cache_path))?;

        Ok(cache)
    }

    pub fn save(&self, output_dir: &Path) -> Result<()> {
        let cache_path = output_dir.join(".palya_cache.json");
        let json = serde_json::to_string_pretty(self)?;
        let mut f = File::create(cache_path)?;
        f.write_all(json.as_bytes())?;
        Ok(())
    }
}

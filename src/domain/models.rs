use anyhow::{anyhow, Context, Result};
use blake3::hash;
use once_cell::sync::Lazy;
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Parser, Tag, TagEnd, html};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::from_str;
use std::{
    collections::HashMap,
    fs::{self, File, read_to_string},
    io::Write,
    path::{Path, PathBuf},
};
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| SyntaxSet::load_defaults_newlines());
static THEME_SET: Lazy<ThemeSet> = Lazy::new(|| ThemeSet::load_defaults());

/// Represents the possible formats for `tags` in YAML frontmatter.
///
/// This enum allows tags to be written in several convenient forms in
/// frontmatter while still deserializing into a single Rust type.
///
/// If the `tags` field is omitted entirely, it will deserialize as `None`
/// when used as `Option<Tags>` in `FrontMatter`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Tags {
    /// A single tag string.
    ///
    /// ```yaml
    /// tags: "rust"
    /// ```
    Single(String),

    /// Multiple tags.
    ///
    /// ```yaml
    /// tags: ["rust", "web"]
    /// ```
    Multiple(Vec<String>),

    /// Explicitly no tags.
    ///
    /// ```yaml
    /// tags: null
    /// ```
    Null,
}

/// Metadata parsed from the YAML frontmatter of Markdown files.
///
/// Frontmatter is extracted from the top of content files (such as blog posts
/// or projects) and provides configuration for rendering, including titles,
/// templates, and feature flags. All fields are optional unless otherwise noted,
/// allowing flexible per-file customization.
///
/// # Example
/// ```yaml
/// title: "My Blog Post"
/// description: "An example post"
/// subtitle: "A subtitle"
/// stack: ["Rust", "WebAssembly"]
/// links:
///   source: "https://github.com/example"
/// date: "2023-10-01"
/// slug: "my-post"
/// template: "post.j2"
/// draft: false
/// theme: "base16-ocean.dark"
/// ```
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FrontMatter {
    /// The main title of the page, post, or project.
    pub title: Option<String>,

    /// A brief description or summary of the content.
    pub description: Option<String>,

    /// An optional subtitle rendered below the title.
    pub subtitle: Option<String>,

    /// A list of technologies or tools used (for example in project pages).
    pub stack: Option<Vec<String>>,

    /// A map of named links, such as links to source code or live demos.
    pub links: Option<HashMap<String, String>>,

    /// The publication or creation date.
    pub date: Option<String>,

    /// Custom URL slug. If absent, it may be derived from the filename.
    pub slug: Option<String>,

    /// Name of the template used for rendering.
    pub template: Option<String>,

    /// If `true`, the file is treated as a draft and skipped during rendering
    /// unless draft rendering is enabled.
    #[serde(default)]
    pub draft: bool,

    /// Optional tags associated with the content.
    pub tags: Option<Tags>,

    /// Name of the syntax highlighting theme.
    pub theme: Option<String>,
}

/// Represents a processed content item (e.g., a page, post, or project) after parsing.
///
/// This struct holds all relevant data for a single piece of content, including its metadata,
/// rendered HTML, URL, and search-friendly text. It's created during the build process by
/// parsing Markdown files, extracting frontmatter, and converting content to HTML. The
/// resulting items are used for rendering templates and generating the site.
///
/// # Example
/// An item might represent a blog post with frontmatter, HTML body, and metadata
/// for template rendering and site navigation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentItem {
    /// Parsed metadata from the file's YAML frontmatter.
    pub frontmatter: Option<FrontMatter>,

    /// The rendered HTML content after processing Markdown.
    pub content: String,

    /// The final URL path for the item (e.g., "/blog/my-post/").
    pub url: String,

    /// Normalized list of tags as strings (derived from frontmatter).
    pub tags: Option<Vec<String>>,

    /// The original file path of the input Markdown file.
    pub source: PathBuf,

    /// The category or directory-based group (e.g., "blog", "projects").
    pub collection: String,

    /// Plain text version of the content, used for search indexing.
    pub text_content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub author: Option<String>,
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
    pub parsed_items: HashMap<PathBuf, ContentItem>,
}

impl ContentItem {
    pub fn from_file(input_path: PathBuf) -> Result<(String, [u8; 32])> {
        let content = fs::read_to_string(&input_path)
            .with_context(|| format!("Couldn't read contents of {}", input_path.display()))?;
        let content_hash = *hash(content.as_bytes()).as_bytes();
        // let item = Self::parse(content, base_path, input_path, parse_drafts)?;
        Ok((content, content_hash))
    }

    pub fn parse(
        content: &str,
        base_path: &Path,
        input_path: &PathBuf,
        parse_drafts: &bool,
    ) -> Result<ContentItem> {
        let (frontmatter, content_body) = if let Some(rest) = content.strip_prefix("---") {
            if let Some((fm, content)) = rest.split_once("---") {
                let fm = Some(from_str::<FrontMatter>(fm.trim()).with_context(|| {
                    format!("Failed to parse frontmatter: {:?}", input_path.clone())
                })?);
                (fm, content)
            } else {
                (None, content)
            }
        } else {
            (None, content)
        };

        let relative_path = input_path.strip_prefix(base_path).unwrap_or(&input_path);

        let collection = match relative_path.parent() {
            Some(parent) if parent.as_os_str().is_empty() => "pages".to_string(),
            Some(parent) => parent.to_string_lossy().to_string(),
            None => "pages".to_string(),
        };

        let mut tags: Option<Vec<String>> = None;

        if let Some(fm) = &frontmatter {
            if fm.draft && !*parse_drafts {
                return ContentItem::get_item(input_path, Ok(None));
            }
            tags = fm.get_tags();
        }

        let parser = Parser::new(content_body);
        let mut new_events = Vec::new();
        let mut text_content = String::new();

        let mut code_buffer = String::new();
        let mut in_code_block = false;
        let mut code_lang: Option<String> = None;

        let default_theme_name = "base16-ocean.dark";

        let theme_name = frontmatter
            .as_ref()
            .and_then(|fm| fm.theme.as_deref())
            .unwrap_or(default_theme_name);

        let theme = THEME_SET
            .themes
            .get(theme_name)
            .or_else(|| {
                if theme_name != default_theme_name {
                    eprintln!(
                        "Warning: Theme '{}' not found in {:?}. Falling back to default.",
                        theme_name, input_path
                    );
                }
                THEME_SET.themes.get(default_theme_name)
            })
            .expect("Default theme must exist!");

        for event in parser {
            match event {
                Event::Start(Tag::CodeBlock(kind)) => {
                    text_content.push(' ');
                    in_code_block = true;
                    code_lang = match kind {
                        CodeBlockKind::Fenced(lang) => Some(lang.to_string()),
                        _ => None,
                    };
                    code_buffer.clear();
                }
                Event::End(TagEnd::CodeBlock) => {
                    text_content.push(' ');
                    in_code_block = false;

                    let syntax = code_lang
                        .as_ref()
                        .and_then(|l| SYNTAX_SET.find_syntax_by_token(l))
                        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

                    let html =
                        highlighted_html_for_string(&code_buffer, &SYNTAX_SET, syntax, theme)?;

                    new_events.push(Event::Html(CowStr::from(html)));
                }
                Event::Text(t) => {
                    text_content.push_str(&t);
                    if in_code_block {
                        code_buffer.push_str(&t);
                    } else {
                        new_events.push(Event::Text(t));
                    }
                }
                _ => {
                    if !in_code_block {
                        new_events.push(event);
                    }
                }
            }
        }

        let mut html_output = String::new();
        html::push_html(&mut html_output, new_events.into_iter());

        let slug = frontmatter
            .as_ref()
            .and_then(|fm| fm.slug.as_deref())
            .or_else(|| input_path.file_stem().and_then(|s| s.to_str()))
            .unwrap_or("post1")
            .to_string();

        let url = if collection == "pages" {
            format!("/{}/", slug)
        } else {
            format!("/{}/{}/", collection, slug)
        };

        // let text_content = ContentItem::extract_text(content_body);

        let item_option = Ok(Some(ContentItem {
            frontmatter,
            content: html_output,
            url,
            tags,
            source: input_path.clone(),
            collection,
            text_content,
        }));

        ContentItem::get_item(input_path, item_option)
    }

    pub fn template_name(&self) -> String {
        if let Some(fm) = &self.frontmatter {
            if let Some(t) = &fm.template {
                return t.clone();
            }
        }

        // This could be made more dynamic as the `templates` directory might have the
        // required template; we could also just get the names of all templates in the template dir
        // while walking the dir
        match self.collection.as_str() {
            "blog" => "post.j2".to_string(),
            "projects" => "project.j2".to_string(),
            "pages" => "page.j2".to_string(),
            _ => "post.j2".to_string(),
        }
    }

    pub fn output_path(&self, output_dir: &Path) -> PathBuf {
        let slug_path: PathBuf = self.url.split('/').filter(|s| !s.is_empty()).collect();
        output_dir.join(slug_path).join("index.html")
    }

    // fn extract_text(markdown: &str) -> String {
    //     let parser = Parser::new(markdown);
    //     let mut text = String::with_capacity(markdown.len());

    //     for event in parser {
    //         match event {
    //             Event::Text(t) => text.push_str(&t),
    //             Event::Code(c) => {
    //                 text.push(' ');
    //                 text.push_str(&c);
    //                 text.push(' ');
    //             }
    //             Event::SoftBreak | Event::HardBreak => text.push(' '),
    //             Event::End(TagEnd::Paragraph | TagEnd::Item | TagEnd::Heading(_)) => text.push(' '),
    //             _ => {}
    //         }
    //     }

    //     text
    // }

    fn get_item(input_path: &Path, item_option: Result<Option<ContentItem>>) -> Result<ContentItem> {
        match item_option {
            Ok(Some(item)) => Ok(item),
            Ok(None) => Err(anyhow!(
                "Skipping {:?} due to error: ContentItem could not be parsed!",
                input_path
            )),
            Err(e) => Err(anyhow!("Skipping {:?} due to error: {}", input_path, e)),
        }
    }
}

impl Config {
    pub fn load(input_dir: &Path, config_path: Option<&PathBuf>) -> Result<(Config, [u8; 32])> {
        let default_config = Config {
            author: Some("Shadow".to_string()),
            title: "My Site".to_string(),
            description: None,
            base_url: Some("".to_string()),
        };

        let path_to_read = match config_path {
            Some(p) => p.clone(),
            None => {
                let default = input_dir.join("palya.toml");
                if !default.exists() {
                    return Ok((default_config, [0; 32]));
                }
                default
            }
        };

        let config_str = read_to_string(&path_to_read)
            .with_context(|| format!("Found config at {:?} but couldn't read it", path_to_read))?;

        let config_hash = *hash(config_str.as_bytes()).as_bytes();

        let mut config = toml::from_str::<Config>(&config_str)
            .with_context(|| format!("Failed to parse TOML in {:?}", path_to_read))?;

        if config.base_url.is_none() {
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
            Some(Tags::Null) | None => None,
        }
    }
}

impl BuildCache {
    pub fn new(output_dir: &Path) -> Result<Self> {
        let cache_path = output_dir.join(".palya_cache.json");
        let cache_str = read_to_string(&cache_path)?;
        let mut cache: serde_json::Value = serde_json::from_str(&cache_str)?;

        // Handle migration: if parsed_items doesn't exist, add empty map
        if !cache.as_object().unwrap().contains_key("parsed_items") {
            cache.as_object_mut().unwrap().insert("parsed_items".to_string(), serde_json::json!({}));
        }

        let cache: BuildCache = serde_json::from_value(cache)?;
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

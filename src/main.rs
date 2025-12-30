use std::{error::Error, fs::{self, File}, io::Write};

use pulldown_cmark::{Parser, html};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::from_str;
use tera::{Context, Tera};

#[derive(Debug, Serialize, Deserialize)]
struct FrontMatter {
    title: Option<String>,
    date: Option<String>,
    slug: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string("test_bench/content/hello.md")?;

    let (frontmatter, body) = if let Some(rest) = content.strip_prefix("---") {
        if let Some((fm, body)) = rest.split_once("---") {
            let fm = from_str::<FrontMatter>(fm).ok();
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

    let tera = Tera::new("test_bench/templates/**/*.html")?;

    let mut context = Context::new();
    if let Some(ref fm) = frontmatter {
        context.insert("title", &fm.title);
        context.insert("date", &fm.date);
    }
    context.insert("content", &html_output);

    let output = tera.render("post.html", &context)?;

    let mut output_file = File::create("output.html")?;
    output_file.write_all(output.as_bytes())?;

    println!("{:?}", frontmatter);
    println!("{}", html_output);

    Ok(())
}

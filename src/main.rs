use anyhow::Result;
use my_ssg::Post;
use std::{fs::File, io::Write};
use tera::{Context, Tera};
use walkdir::WalkDir;

fn main() -> Result<()> {
    let mut posts: Vec<Post> = Vec::new();

    for entry in WalkDir::new("test_bench/content") {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Walkdir error: {e}");
                continue;
            }
        };

        let path = entry.path();

        if path.extension() != Some(std::ffi::OsStr::new("md")) {
            continue;
        }

        match Post::from_file(path.to_path_buf()) {
            Ok(post) => posts.push(post),
            Err(err) => {
                eprintln!("Failed to parse {:?}: {err}", path);
                continue;
            }
        }
    }

    let tera = Tera::new("test_bench/templates/**/*.html")?;
    for post in posts {
        let mut context = Context::new();
        if let Some(fm) = post.frontmatter.as_ref() {
            context.insert("title", &fm.title);
            context.insert("date", &fm.date);
        }
        context.insert("content", &post.body);

        let output = tera.render(post.template_name(), &context)?;

        if let Some(parent) = post.output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = File::create(&post.output_path)?;
        file.write_all(output.as_bytes())?;
    }

    Ok(())
}

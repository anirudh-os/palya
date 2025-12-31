use my_ssg::Post;
use std::{error::Error, fs::File, io::Write, path::PathBuf};
use tera::{Context, Tera};
use walkdir::WalkDir;

fn main() -> Result<(), Box<dyn Error>> {
    let mut posts: Vec<Post> = Vec::new();

    for entry in WalkDir::new("test_bench/content") {
        if let Ok(entry) = entry {
            let path = PathBuf::from(entry.path());
            if let Some(ext) = path.extension() {
                if ext == "md" {
                    let post = Post::from_file(path)?;

                    posts.push(post);
                } else {
                    continue;
                }
            } else {
                continue;
            }
        } else {
            continue;
        }
    }

    for post in posts {
        let tera = Tera::new("test_bench/templates/**/*.html")?;
        let mut context = Context::new();
        if let Some(fm) = post.frontmatter.clone() {
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

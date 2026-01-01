use anyhow::{Context, Result};
use minijinja::{Environment};
use my_ssg::{Post, load_templates};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

fn main() -> Result<()> {
    let mut posts: Vec<(Post, PathBuf)> = Vec::new();

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
            Ok(post) => posts.push((post.clone(), post.output_path(path.to_path_buf()))),
            Err(err) => {
                eprintln!("Failed to parse {:?}: {err}", path);
                continue;
            }
        };
    }

    let mut env = Environment::new();
    load_templates(&mut env, Path::new("test_bench/templates"))?;

    for (post, output_path) in &posts {
        let template_name = post.template_name();

        let output = env
            .get_template(template_name)
            .with_context(|| format!("Template {} is not found!", template_name))?
            .render(post.as_context())
            .with_context(|| format!("Couldn't render the template {}!", template_name))?;

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).context("Couldn't create the dorectory")?;
        }

        let mut file = File::create(output_path).context("Couldn't create the output file!")?;
        file.write_all(output.as_bytes())
            .context("Couldn't write to the output file!")?;
    }

    Ok(())
}

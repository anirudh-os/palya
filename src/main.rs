use anyhow::{Context, Result};
use minijinja::{Environment, context};
use palya::{Post, copy_static_files, load_templates};
use rayon::prelude::*;
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

fn main() -> Result<()> {
    let static_path = Path::new("test_bench/static");
    let dist_path = Path::new("dist");

    fs::create_dir_all(dist_path).context("Couldn't create the directory")?;
    copy_static_files(static_path, dist_path)?;

    let file_paths: Vec<PathBuf> = WalkDir::new("test_bench/content")
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_owned())
        .filter(|p| p.extension().map_or(false, |e| e == "md"))
        .collect();

    let mut posts: Vec<Post> = file_paths
        .par_iter()
        .map(|path| Post::from_file(path.clone()))
        .filter_map(|result| match result {
            Ok(post) => Some(post),
            Err(e) => {
                eprintln!("Error: {}", e);
                None
            }
        })
        .collect();

    let mut env = Environment::new();
    load_templates(&mut env, Path::new("test_bench/templates"))?;

    posts.sort_by(|a, b| {
        let date_a = a.frontmatter.as_ref().and_then(|fm| fm.date.as_deref());
        let date_b = b.frontmatter.as_ref().and_then(|fm| fm.date.as_deref());

        date_b.cmp(&date_a)
    });

    posts.par_iter().try_for_each(|post| -> Result<()> {
        let template_name = post.template_name();

        let output = env
            .get_template(template_name)
            .with_context(|| format!("Template {} is not found!", template_name))?
            .render(post.as_context())
            .with_context(|| format!("Couldn't render the template {}!", template_name))?;

        if let Some(parent) = post.output_path().parent() {
            fs::create_dir_all(parent).context("Couldn't create the directory")?;
        }

        let mut file =
            File::create(post.output_path()).context("Couldn't create the output file!")?;
        file.write_all(output.as_bytes())
            .context("Couldn't write to the output file!")?;

        Ok(())
    })?;

    let index_context = context! {
        posts => posts
    };

    let index_output = env
        .get_template("index.jinja")
        .context("Failed to load index.html template")?
        .render(index_context)
        .context("Failed to render index.html")?;

    let index_path = PathBuf::from("dist/index.html");
    let mut file = File::create(index_path)?;
    file.write_all(index_output.as_bytes())?;

    Ok(())
}

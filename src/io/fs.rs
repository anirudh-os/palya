use anyhow::{Context, Result};
use blake3::hash;
use minijinja::Environment;
use std::{
    collections::HashMap, fs, path::{Path, PathBuf}
};
use walkdir::WalkDir;

pub fn copy_static_files(
    input_dir: &Path,
    static_arg: Option<&PathBuf>,
    dest: &Path,
) -> Result<()> {
    let static_dir = match static_arg {
        Some(path) => path.clone(),
        None => {
            let default = input_dir.join("static");

            if !default.exists() {
                println!("Couldn't find the `static` directory!");
                return Ok(());
            }
            default
        }
    };

    for entry in WalkDir::new(&static_dir) {
        let entry = entry?;
        let path = entry.path();
        let target = dest.join("static");
        fs::create_dir_all(&target).context("Failed to create static dir")?;
        if path.is_file() {
            let target_file =
                target.join(path.strip_prefix(path.parent().unwrap_or(Path::new("./")))?);
            fs::copy(path, &target_file)
                .with_context(|| format!("Couldn't copy the file {:#?}", &path))?;
        }
    }
    Ok(())
}

pub fn load_templates(env: &mut Environment, templates_dir: &Path) -> Result<HashMap<PathBuf, [u8; 32]>> {
    let mut template_hash = HashMap::new();
    for entry in WalkDir::new(templates_dir) {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let name = path
                .strip_prefix(templates_dir)
                .with_context(|| {
                    format!(
                        "Couldn't strip the prefix of {}!",
                        path.to_str().unwrap_or("unknown")
                    )
                })?
                .to_string_lossy()
                .into_owned();

            let content = fs::read_to_string(path)?;
            let content_hash = *hash(content.as_bytes()).as_bytes();

            template_hash.insert(PathBuf::from(path), content_hash);

            env.add_template_owned(name.clone(), content)
                .with_context(|| format!("Couldn't add the template {}", name))?;
        }
    }
    Ok(template_hash)
}

use anyhow::{Context, Result};
use minijinja::Environment;
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub fn copy_static_files(
    input_dir: &Path,
    static_arg: Option<&PathBuf>,
    dest: &Path,
) -> Result<()> {
    let source_dir = match static_arg {
        Some(path) => path.clone(),
        None => {
            let default = input_dir.join("static");
            if !default.exists() {
                return Ok(());
            }
            default
        }
    };

    for entry in WalkDir::new(&source_dir) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let relative = path.strip_prefix(&source_dir)?;
            let target = dest.join(relative);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).context("Failed to create static dir")?;
            }
            fs::copy(path, &target)?;
        }
    }
    Ok(())
}

pub fn load_templates(env: &mut Environment, dir: &Path) -> Result<()> {
    for entry in WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let name = path
                .strip_prefix(dir)
                .with_context(|| {
                    format!(
                        "Couldn't strip the prefix of {}!",
                        path.to_str().unwrap_or("unknown")
                    )
                })?
                .to_string_lossy()
                .into_owned();

            let content = fs::read_to_string(path)?;

            env.add_template_owned(name.clone(), content)
                .with_context(|| format!("Couldn't add the template {}", name))?;
        }
    }
    Ok(())
}

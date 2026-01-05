use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub input: PathBuf,

    #[arg(short, long)]
    pub output: PathBuf,

    #[arg(short, long)]
    pub templates: PathBuf,

    #[arg(long)]
    pub static_dir: Option<PathBuf>,

    #[arg(short, long)]
    pub config: Option<PathBuf>,

    #[arg(long, default_value_t = false)]
    pub drafts: bool,
}

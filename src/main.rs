use anyhow::Result;
use clap::Parser;
use palya::{cmd::Args, Site};

fn main() -> Result<()> {
    let args = Args::parse();
    let site = Site::new(args)?;
    site.build()?;
    Ok(())
}
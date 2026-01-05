use anyhow::Result;
use clap::Parser;
use palya::{Site, cmd::Args};

fn main() -> Result<()> {
    let args = Args::parse();
    let site = Site::new(args)?;
    site.build()?;
    Ok(())
}

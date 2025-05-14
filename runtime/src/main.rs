/// MacPack Runtime
/// Licensed under BSD 3-Clause license.

use std::{path::PathBuf, process::Stdio};
use clap::Parser;
use serde::Deserialize;
use anyhow::{Result, Context};

#[derive(Parser)]
#[command(name = "MacPack Runtime", version = "v0.1", about = "MacPack Runtime")]
struct Cli {
    /// Path to .mpb bundle
    #[arg(value_name = "BUNDLE_PATH", help = "Path to bundle (.mpb)")]
    bundle: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Metadata {
    name: String,
    version: String,
    author: String,
    exec: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Step 1: Load metadata.json
    let metadata_path = cli.bundle.join("app.json");
    let metadata_content = std::fs::read_to_string(&metadata_path)
        .with_context(|| format!("Failed to read {}", metadata_path.display()))?;
    
    let metadata: Metadata = serde_json::from_str(&metadata_content)
        .context("Failed to parse metadata.json")?;

    let exec_path = cli.bundle.join("exec").join(&metadata.exec);

    let process = std::process::Command::new(exec_path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to launch app")?;

    Ok(())
}

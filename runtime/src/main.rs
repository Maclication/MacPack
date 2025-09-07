use std::{fs::{create_dir_all, File}, path::PathBuf, process::Stdio};
use std::os::unix::fs::PermissionsExt;
use anyhow::{Result, Context};
use clap::Parser;
use serde::Deserialize;
use tempfile::Builder;
use zip::ZipArchive;

#[derive(Parser)]
#[command(name = "MacPack Runtime", version = "v0.2", about = "MacPack Runtime")]
struct Cli {
    // Bundle path, required
    #[arg(value_name = "BUNDLE_PATH", help = "Path to .mpb bundle")]
    bundle: PathBuf,
}

/// Maps the `[package]` table in macpack.toml
#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    description: Option<String>,
    version: String,
    author: String,
    exec: String,
}

#[derive(Debug, Deserialize)]
struct MacPackToml {
    package: Package,
}

/// Extracts a ZIP (.mpb) into a pre-existing folder
fn extract_mpb_to_folder(mpb_path: &PathBuf, dest_folder: &PathBuf) -> Result<()> {
    let file = File::open(mpb_path)
        .with_context(|| format!("Failed to open {}", mpb_path.display()))?;
    let mut archive = ZipArchive::new(file)
        .context("Failed to read ZIP archive")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let out_path = dest_folder.join(file.mangled_name());

        if file.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = File::create(&out_path)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

/// Reads macpack.toml and returns the executable path inside the folder
fn get_executable(extracted_folder: &PathBuf, bundle_file: &PathBuf) -> Result<PathBuf> {
    // Step 1: initial macpack.toml path
    let mut toml_path = extracted_folder.join("macpack.toml");

    if !toml_path.exists() {
        // Step 2: fallback to folder named after zip file (without extension)
        let zip_stem = bundle_file
            .file_stem()
            .ok_or_else(|| anyhow::anyhow!("Failed to get zip file stem"))?;
        let fallback_folder = extracted_folder.join(zip_stem);

        toml_path = fallback_folder.join("macpack.toml");
        if !toml_path.exists() {
            return Err(anyhow::anyhow!(
                "macpack.toml not found in either extracted folder or fallback folder: {:?}",
                toml_path
            ));
        }
    }

    // Step 3: parse TOML
    let content = std::fs::read_to_string(&toml_path)
        .context("Failed to read macpack.toml")?;
    let metadata: MacPackToml = toml::from_str(&content)
        .context("Developer Error: Failed to parse macpack.toml. Developer errors are issues that need a developer. If you're not one of them, contact the developer of the app.")?;

    // Step 4: get executable path inside the folder containing macpack.toml
    let exec_folder = toml_path.parent().unwrap(); // folder containing the TOML
    let exec_path = exec_folder.join("bin").join(&metadata.package.exec);

    if !exec_path.exists() {
        return Err(anyhow::anyhow!(
            "Developer Error: Executable specified in macpack.toml not found: {:?}",
            exec_path
        ));
    }

    Ok(exec_path)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // The bundle file (.mpb, required by get_executable)
    let bundle_path: PathBuf = cli.bundle;

    // Step 1: Create temporary folder
    let temp_dir = Builder::new()
        .prefix("mcpak_")
        .tempdir()?;

    let _ = create_dir_all(&temp_dir);

    let temp_path = temp_dir.keep();

    // Step 2: Extract .mpb into temp folder
    extract_mpb_to_folder(&bundle_path, &temp_path)?;

    // Step 3: Read executable from macpack.toml inside extracted folder
    let exec_path = get_executable(&temp_path, &bundle_path)?;
    if !exec_path.exists() {
        return Err(anyhow::anyhow!("Executable not found: {:?}", exec_path));
    }

    let mut perms = std::fs::metadata(&exec_path)?.permissions();
    perms.set_mode(0o755); // rwxr-xr-x
    std::fs::set_permissions(&exec_path, perms)?;

    // Step 4: Launch executable
    let mut child = std::process::Command::new(exec_path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to launch executable")?;

    // Step 5: Wait for executable to finish
    let status = child.wait().context("Failed to wait for process")?;
    if !status.success() {
        return Err(anyhow::anyhow!("Executable exited with {:?}", status.code()));
    }

    // Temporary folder cleaned automatically when temp_dir is dropped
    Ok(())
}

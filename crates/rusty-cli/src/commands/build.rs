use std::path::PathBuf;

use clap::Parser;
use rusty_core::manifest::PluginManifest;

#[derive(Parser)]
pub struct Args {
    /// Path to plugin directory (defaults to current directory)
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Build in debug mode instead of release
    #[arg(long)]
    debug: bool,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let dir = args.path.canonicalize().unwrap_or(args.path);

    // Read manifest to get plugin ID
    let manifest_path = dir.join("rusty-plugin.toml");
    if !manifest_path.exists() {
        anyhow::bail!(
            "no rusty-plugin.toml found in {}\n  run `rusty init <name>` to create a plugin project",
            dir.display()
        );
    }

    let manifest_str = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest = PluginManifest::from_toml(&manifest_str)?;
    let plugin_id = &manifest.plugin.id;

    use owo_colors::OwoColorize;

    println!(
        "{} {} {}",
        "Building".cyan().bold(),
        manifest.plugin.name.bold(),
        format!("v{}", manifest.plugin.version).dimmed()
    );

    // Run cargo build
    let profile = if args.debug { "dev" } else { "release" };
    let profile_dir = if args.debug { "debug" } else { "release" };
    let mut cmd = tokio::process::Command::new("cargo");
    cmd.current_dir(&dir)
        .arg("build")
        .arg("--target")
        .arg("wasm32-wasip2")
        .arg("--profile")
        .arg(profile);

    let status = cmd.status().await?;
    if !status.success() {
        anyhow::bail!("cargo build failed");
    }

    // Find the output .wasm file
    let target_dir = dir.join("target/wasm32-wasip2").join(profile_dir);
    let wasm_src = find_wasm_in_dir(&target_dir).await?;
    let wasm_dest = dir.join(format!("{plugin_id}.wasm"));

    tokio::fs::copy(&wasm_src, &wasm_dest).await?;

    let size = tokio::fs::metadata(&wasm_dest).await?.len();
    let size_kb = size / 1024;

    println!(
        "{} {} {}",
        "done".green().bold(),
        wasm_dest.display(),
        format!("({size_kb} KB)").dimmed()
    );
    println!();
    println!(
        "  {} install {}",
        "rusty".dimmed(),
        dir.display()
    );

    Ok(())
}

async fn find_wasm_in_dir(dir: &std::path::Path) -> anyhow::Result<PathBuf> {
    let mut entries = tokio::fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "wasm") {
            return Ok(path);
        }
    }
    anyhow::bail!(
        "no .wasm file found in {}\n  make sure your Cargo.toml has `crate-type = [\"cdylib\"]`",
        dir.display()
    );
}

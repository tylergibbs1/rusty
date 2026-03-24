use std::path::PathBuf;

use clap::Parser;
use owo_colors::OwoColorize;
use rusty_engine::invoke::build_linker;
use rusty_engine::registry::PluginRegistry;
use rusty_engine::runtime;

use super::rusty_home;

#[derive(Parser)]
pub struct Args {
    /// Path to plugin directory containing rusty-plugin.toml and .wasm
    path: PathBuf,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let home = rusty_home();
    tokio::fs::create_dir_all(&home).await?;

    let engine = runtime::build_engine()?;
    let linker = build_linker(&engine)?;

    let mut registry = PluginRegistry::new(&home);
    let plugin_id = registry.install(&args.path, &engine, &linker).await?;

    println!(
        "{} Installed plugin: {}",
        "done".green().bold(),
        plugin_id.cyan().bold()
    );
    println!(
        "  {} {}",
        "location:".dimmed(),
        registry.plugins_dir().join(&plugin_id).display()
    );
    Ok(())
}

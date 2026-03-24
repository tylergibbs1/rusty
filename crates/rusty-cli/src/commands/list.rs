use clap::Parser;

use super::setup;
use crate::output;

#[derive(Parser)]
pub struct Args;

pub async fn run(_args: Args) -> anyhow::Result<()> {
    let (_engine, registry, _policy) = setup().await?;

    let plugins = registry.list();
    if plugins.is_empty() {
        println!("No plugins installed.");
        println!("Install one with: rusty install <path>");
        return Ok(());
    }

    println!("{} plugin(s) installed:\n", plugins.len());
    for plugin in plugins {
        output::print_plugin_summary(&plugin.manifest);
        println!();
    }
    Ok(())
}

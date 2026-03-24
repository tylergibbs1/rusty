use clap::Parser;
use owo_colors::OwoColorize;

use super::setup;
use crate::output;

#[derive(Parser)]
pub struct Args;

pub async fn run(_args: Args) -> anyhow::Result<()> {
    let (_engine, registry, _policy) = setup().await?;

    let plugins = registry.list();
    if plugins.is_empty() {
        println!("{}", "No plugins installed.".dimmed());
        println!(
            "  {} init <name>    scaffold a new plugin",
            "rusty".dimmed()
        );
        println!(
            "  {} install <path>  install an existing plugin",
            "rusty".dimmed()
        );
        return Ok(());
    }

    println!(
        "{} plugin(s) installed:\n",
        plugins.len().to_string().bold()
    );
    for plugin in plugins {
        output::print_plugin_summary(&plugin.manifest);
        println!();
    }
    Ok(())
}

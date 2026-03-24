use clap::Parser;

use super::setup;
use crate::output;

#[derive(Parser)]
pub struct Args {
    /// Plugin ID to inspect
    plugin_id: String,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let (_engine, registry, _policy) = setup().await?;

    let plugin = registry
        .get(&args.plugin_id)
        .ok_or_else(|| anyhow::anyhow!("plugin not found: {}", args.plugin_id))?;

    let m = &plugin.manifest;
    println!("Plugin: {} ({})", m.plugin.name, m.plugin.id);
    println!("  version: {}", m.plugin.version);
    println!("  author: {}", m.plugin.author);
    println!("  description: {}", m.plugin.description);
    println!("  runtime-compat: {}", m.plugin.runtime_compat);

    if !m.capabilities.is_empty() {
        println!(
            "  capabilities: {}",
            m.capabilities
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    } else {
        println!("  capabilities: none");
    }

    println!("\nActions ({}):", m.actions.len());
    for action in &m.actions {
        println!();
        output::print_action_detail(action);
    }

    Ok(())
}

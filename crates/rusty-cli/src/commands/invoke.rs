use clap::Parser;
use tokio_util::sync::CancellationToken;

use super::setup;
use crate::output;

#[derive(Parser)]
pub struct Args {
    /// Plugin ID
    plugin_id: String,
    /// Action ID
    action_id: String,
    /// JSON input for the action
    #[arg(long, default_value = "{}")]
    input: String,
    /// Show full trace after execution
    #[arg(long)]
    trace: bool,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let (engine, registry, policy) = setup().await?;

    let plugin = registry
        .get(&args.plugin_id)
        .ok_or_else(|| anyhow::anyhow!("plugin not found: {}", args.plugin_id))?;

    let input: serde_json::Value = serde_json::from_str(&args.input)
        .map_err(|e| anyhow::anyhow!("invalid JSON input: {e}"))?;

    let cancel = CancellationToken::new();

    let invocation = engine
        .invoke(plugin, &args.action_id, input, &policy, cancel)
        .await?;

    output::print_invocation_result(&invocation);

    if args.trace {
        println!();
        output::print_trace(&invocation);
    }

    // Save trace to disk
    let home = super::rusty_home();
    let traces_dir = home.join("traces");
    tokio::fs::create_dir_all(&traces_dir).await?;
    let trace_path = traces_dir.join(format!("{}.json", invocation.id));
    let trace_json = serde_json::to_string_pretty(&invocation)?;
    tokio::fs::write(&trace_path, trace_json).await?;

    Ok(())
}

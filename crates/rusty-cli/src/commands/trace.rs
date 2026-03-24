use clap::Parser;

use super::rusty_home;
use crate::output;

#[derive(Parser)]
pub struct Args {
    /// Run ID (UUID) to show trace for
    run_id: String,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let home = rusty_home();
    let trace_path = home.join("traces").join(format!("{}.json", args.run_id));

    if !trace_path.exists() {
        anyhow::bail!("trace not found: {}", args.run_id);
    }

    let content = tokio::fs::read_to_string(&trace_path).await?;
    let invocation: rusty_core::invocation::Invocation = serde_json::from_str(&content)?;

    output::print_trace(&invocation);
    Ok(())
}

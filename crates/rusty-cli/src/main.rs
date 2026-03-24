mod commands;
mod output;

use clap::Parser;

#[derive(Parser)]
#[command(name = "rusty", about = "WASM plugin host platform", version)]
enum Cli {
    /// Install a plugin from a local path
    Install(commands::install::Args),
    /// List installed plugins and their actions
    List(commands::list::Args),
    /// Inspect a plugin's manifest, capabilities, and actions
    Inspect(commands::inspect::Args),
    /// Invoke a plugin action
    Invoke(commands::invoke::Args),
    /// Show execution trace for a run
    Trace(commands::trace::Args),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();
    match cli {
        Cli::Install(args) => commands::install::run(args).await,
        Cli::List(args) => commands::list::run(args).await,
        Cli::Inspect(args) => commands::inspect::run(args).await,
        Cli::Invoke(args) => commands::invoke::run(args).await,
        Cli::Trace(args) => commands::trace::run(args).await,
    }
}

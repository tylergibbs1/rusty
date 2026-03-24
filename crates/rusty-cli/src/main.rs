mod commands;
mod output;

use clap::Parser;

#[derive(Parser)]
#[command(name = "rusty", about = "WASM plugin host platform", version)]
enum Cli {
    /// Create a new plugin project
    Init(commands::init::Args),
    /// Build a plugin to WASM (compile + copy)
    Build(commands::build::Args),
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
    // Respect NO_COLOR (https://no-color.org)
    if std::env::var_os("NO_COLOR").is_some() {
        owo_colors::set_override(false);
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();
    match cli {
        Cli::Init(args) => commands::init::run(args).await,
        Cli::Build(args) => commands::build::run(args).await,
        Cli::Install(args) => commands::install::run(args).await,
        Cli::List(args) => commands::list::run(args).await,
        Cli::Inspect(args) => commands::inspect::run(args).await,
        Cli::Invoke(args) => commands::invoke::run(args).await,
        Cli::Trace(args) => commands::trace::run(args).await,
    }
}

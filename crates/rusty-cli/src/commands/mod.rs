pub mod inspect;
pub mod install;
pub mod invoke;
pub mod list;
pub mod trace;

use std::path::PathBuf;

use rusty_core::policy::PolicyConfig;
use rusty_engine::invoke::{build_linker, InvocationEngine};
use rusty_engine::registry::PluginRegistry;
use rusty_engine::runtime::{self, RuntimeConfig};
use rusty_policy::PolicyEngine;

pub fn rusty_home() -> PathBuf {
    std::env::var("RUSTY_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs().join(".rusty")
        })
}

fn dirs() -> PathBuf {
    home::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

pub async fn setup() -> anyhow::Result<(InvocationEngine, PluginRegistry, PolicyEngine)> {
    let home = rusty_home();
    tokio::fs::create_dir_all(&home).await?;

    let engine = runtime::build_engine()?;
    let linker = build_linker(&engine)?;
    let config = RuntimeConfig::default();

    let mut registry = PluginRegistry::new(&home);
    registry.scan(&engine, &linker).await?;

    let policy = load_policy(&home).await;

    let invocation_engine = InvocationEngine::new(engine, linker, config);
    Ok((invocation_engine, registry, policy))
}

async fn load_policy(home: &std::path::Path) -> PolicyEngine {
    let policy_path = home.join("policy.toml");
    let config = if policy_path.exists() {
        let content = tokio::fs::read_to_string(&policy_path).await.unwrap_or_default();
        PolicyConfig::from_toml(&content).unwrap_or_default()
    } else {
        PolicyConfig::default()
    };
    PolicyEngine::new(config)
}

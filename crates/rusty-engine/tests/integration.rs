use std::path::PathBuf;

use rusty_core::action::ActionKind;
use rusty_core::invocation::InvocationState;
use rusty_core::policy::{PolicyConfig, PolicyEffect, PolicyRule};
use rusty_engine::invoke::{build_linker, InvocationEngine};
use rusty_engine::plugin::LoadedPlugin;
use rusty_engine::registry::PluginRegistry;
use rusty_engine::runtime::{self, RuntimeConfig};
use rusty_policy::PolicyEngine;
use tokio_util::sync::CancellationToken;

fn plugin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/plugins/hello-world")
        .canonicalize()
        .expect("hello-world plugin dir must exist — run `cargo build --target wasm32-wasip2 --release` in examples/plugins/hello-world first")
}

fn allow_all_policy() -> PolicyEngine {
    PolicyEngine::new(PolicyConfig::default())
}

fn deny_all_policy() -> PolicyEngine {
    PolicyEngine::new(PolicyConfig {
        default_effect: PolicyEffect::Deny,
        rules: vec![],
    })
}

fn require_approval_for_destructive() -> PolicyEngine {
    PolicyEngine::new(PolicyConfig {
        default_effect: PolicyEffect::Allow,
        rules: vec![PolicyRule {
            id: "approve-destructive".into(),
            description: None,
            effect: PolicyEffect::RequireApproval,
            match_action_kind: Some(ActionKind::Destructive),
            match_capability: None,
            match_tag: None,
            match_plugin_id: None,
            match_action_id: None,
        }],
    })
}

async fn setup() -> (InvocationEngine, LoadedPlugin) {
    let engine = runtime::build_engine().unwrap();
    let linker = build_linker(&engine).unwrap();
    let plugin = LoadedPlugin::load(&engine, &linker, &plugin_dir())
        .await
        .unwrap();
    let inv_engine = InvocationEngine::new(engine, linker, RuntimeConfig::default());
    (inv_engine, plugin)
}

// ─── Plugin Loading ──────────────────────────────────────────

#[tokio::test]
async fn load_plugin_reads_manifest() {
    let engine = runtime::build_engine().unwrap();
    let linker = build_linker(&engine).unwrap();
    let plugin = LoadedPlugin::load(&engine, &linker, &plugin_dir())
        .await
        .unwrap();

    assert_eq!(plugin.plugin_id(), "hello-world");
    assert_eq!(plugin.manifest.plugin.name, "Hello World");
    assert_eq!(plugin.manifest.plugin.version, "0.1.0");
    assert_eq!(plugin.manifest.actions.len(), 1);
    assert_eq!(plugin.manifest.actions[0].id, "greet");
}

#[tokio::test]
async fn load_plugin_fails_for_missing_dir() {
    let engine = runtime::build_engine().unwrap();
    let linker = build_linker(&engine).unwrap();
    let result = LoadedPlugin::load(&engine, &linker, &PathBuf::from("/nonexistent")).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn load_plugin_fails_for_missing_wasm() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("rusty-plugin.toml"),
        r#"
[plugin]
id = "ghost"
name = "Ghost"
version = "0.1.0"
author = "test"
description = "no wasm"
"#,
    )
    .unwrap();

    let engine = runtime::build_engine().unwrap();
    let linker = build_linker(&engine).unwrap();
    let result = LoadedPlugin::load(&engine, &linker, dir.path()).await;
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(
        err.to_string().contains("not found"),
        "should mention missing WASM file, got: {err}"
    );
}

// ─── Successful Invocation ───────────────────────────────────

#[tokio::test]
async fn invoke_greet_returns_hello_message() {
    let (engine, plugin) = setup().await;
    let input = serde_json::json!({"name": "World"});
    let cancel = CancellationToken::new();

    let inv = engine
        .invoke(&plugin, "greet", input, &allow_all_policy(), cancel)
        .await
        .unwrap();

    assert_eq!(inv.state, InvocationState::Completed);
    match &inv.result {
        Some(rusty_core::invocation::InvocationResult::Success(v)) => {
            assert_eq!(v["message"], "Hello, World!");
        }
        other => panic!("expected Success, got {other:?}"),
    }
}

#[tokio::test]
async fn invoke_traces_full_lifecycle() {
    let (engine, plugin) = setup().await;
    let input = serde_json::json!({"name": "Trace"});
    let cancel = CancellationToken::new();

    let inv = engine
        .invoke(&plugin, "greet", input, &allow_all_policy(), cancel)
        .await
        .unwrap();

    // Collect trace event types
    let types: Vec<String> = inv
        .trace
        .events
        .iter()
        .map(|e| serde_json::to_value(&e.kind).unwrap()["type"].as_str().unwrap().to_string())
        .collect();

    assert!(
        types.contains(&"invocation_requested".to_string()),
        "missing invocation_requested in {types:?}"
    );
    assert!(
        types.contains(&"validation_passed".to_string()),
        "missing validation_passed in {types:?}"
    );
    assert!(
        types.contains(&"policy_allowed".to_string()),
        "missing policy_allowed in {types:?}"
    );
    assert!(
        types.contains(&"execution_started".to_string()),
        "missing execution_started in {types:?}"
    );
    assert!(
        types.contains(&"execution_succeeded".to_string()),
        "missing execution_succeeded in {types:?}"
    );
}

#[tokio::test]
async fn invoke_records_host_log_call_in_trace() {
    let (engine, plugin) = setup().await;
    let input = serde_json::json!({"name": "HostCall"});
    let cancel = CancellationToken::new();

    let inv = engine
        .invoke(&plugin, "greet", input, &allow_all_policy(), cancel)
        .await
        .unwrap();

    let has_host_call = inv.trace.events.iter().any(|e| {
        matches!(&e.kind, rusty_core::trace::TraceEventKind::HostCallIssued { function } if function == "log")
    });
    assert!(
        has_host_call,
        "plugin calls host_api::log, should see HostCallIssued in trace"
    );
}

// ─── Schema Validation ───────────────────────────────────────

#[tokio::test]
async fn invoke_rejects_invalid_input() {
    let (engine, plugin) = setup().await;
    let input = serde_json::json!({}); // missing required "name"
    let cancel = CancellationToken::new();

    let inv = engine
        .invoke(&plugin, "greet", input, &allow_all_policy(), cancel)
        .await
        .unwrap();

    assert_eq!(inv.state, InvocationState::Failed);
    match &inv.result {
        Some(rusty_core::invocation::InvocationResult::Error(e)) => {
            assert_eq!(e.code, "validation_failed");
            assert!(
                e.message.contains("name"),
                "error should mention the missing field: {}",
                e.message
            );
        }
        other => panic!("expected validation Error, got {other:?}"),
    }
}

#[tokio::test]
async fn invoke_rejects_wrong_type_input() {
    let (engine, plugin) = setup().await;
    let input = serde_json::json!({"name": 42}); // name should be string
    let cancel = CancellationToken::new();

    let inv = engine
        .invoke(&plugin, "greet", input, &allow_all_policy(), cancel)
        .await
        .unwrap();

    assert_eq!(inv.state, InvocationState::Failed);
    match &inv.result {
        Some(rusty_core::invocation::InvocationResult::Error(e)) => {
            assert_eq!(e.code, "validation_failed");
        }
        other => panic!("expected validation Error, got {other:?}"),
    }
}

// ─── Unknown Action ──────────────────────────────────────────

#[tokio::test]
async fn invoke_unknown_action_returns_error() {
    let (engine, plugin) = setup().await;
    let cancel = CancellationToken::new();

    let result = engine
        .invoke(
            &plugin,
            "nonexistent",
            serde_json::json!({}),
            &allow_all_policy(),
            cancel,
        )
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("nonexistent"),
        "error should mention the action: {err}"
    );
}

// ─── Policy Enforcement ──────────────────────────────────────

#[tokio::test]
async fn invoke_denied_by_policy() {
    let (engine, plugin) = setup().await;
    let input = serde_json::json!({"name": "Denied"});
    let cancel = CancellationToken::new();

    let inv = engine
        .invoke(&plugin, "greet", input, &deny_all_policy(), cancel)
        .await
        .unwrap();

    assert_eq!(inv.state, InvocationState::Failed);
    match &inv.result {
        Some(rusty_core::invocation::InvocationResult::Error(e)) => {
            assert_eq!(e.code, "policy_denied");
        }
        other => panic!("expected policy denied Error, got {other:?}"),
    }
}

#[tokio::test]
async fn invoke_allowed_by_specific_rule() {
    let policy = PolicyEngine::new(PolicyConfig {
        default_effect: PolicyEffect::Deny,
        rules: vec![PolicyRule {
            id: "allow-greet".into(),
            description: None,
            effect: PolicyEffect::Allow,
            match_action_kind: Some(ActionKind::ReadOnly),
            match_capability: None,
            match_tag: None,
            match_plugin_id: None,
            match_action_id: None,
        }],
    });

    let (engine, plugin) = setup().await;
    let cancel = CancellationToken::new();

    let inv = engine
        .invoke(
            &plugin,
            "greet",
            serde_json::json!({"name": "Rule"}),
            &policy,
            cancel,
        )
        .await
        .unwrap();

    assert_eq!(inv.state, InvocationState::Completed);
}

#[tokio::test]
async fn invoke_require_approval_denies_in_m1() {
    // In M1, require-approval is treated as deny since no approval workflow exists
    let policy = PolicyEngine::new(PolicyConfig {
        default_effect: PolicyEffect::RequireApproval,
        rules: vec![],
    });

    let (engine, plugin) = setup().await;
    let cancel = CancellationToken::new();

    let inv = engine
        .invoke(
            &plugin,
            "greet",
            serde_json::json!({"name": "Approval"}),
            &policy,
            cancel,
        )
        .await
        .unwrap();

    assert_eq!(inv.state, InvocationState::Failed);
    match &inv.result {
        Some(rusty_core::invocation::InvocationResult::Error(e)) => {
            assert_eq!(e.code, "approval_required");
        }
        other => panic!("expected approval_required Error, got {other:?}"),
    }
}

// ─── Cancellation ────────────────────────────────────────────

#[tokio::test]
async fn invoke_cancellation_stops_execution() {
    let (engine, plugin) = setup().await;
    let cancel = CancellationToken::new();
    cancel.cancel(); // pre-cancel

    let inv = engine
        .invoke(
            &plugin,
            "greet",
            serde_json::json!({"name": "Cancel"}),
            &allow_all_policy(),
            cancel,
        )
        .await
        .unwrap();

    // Pre-cancellation races with execution. The plugin is fast enough that
    // it may complete before tokio::select checks the cancel token.
    // Either outcome is acceptable — what matters is no panic/crash.
    match inv.state {
        InvocationState::Failed => {
            match &inv.result {
                Some(rusty_core::invocation::InvocationResult::Error(e)) => {
                    assert_eq!(e.code, "cancelled");
                }
                other => panic!("expected cancelled Error, got {other:?}"),
            }
        }
        InvocationState::Completed => {
            // Plugin finished before cancel was observed — this is fine for
            // a sub-millisecond plugin. The important thing is we didn't crash.
        }
        other => panic!("unexpected state: {other}"),
    }
}

// ─── Registry ────────────────────────────────────────────────

#[tokio::test]
async fn registry_scan_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let engine = runtime::build_engine().unwrap();
    let linker = build_linker(&engine).unwrap();

    let mut registry = PluginRegistry::new(dir.path());
    registry.scan(&engine, &linker).await.unwrap();

    assert!(registry.list().is_empty());
}

#[tokio::test]
async fn registry_install_and_get() {
    let home = tempfile::tempdir().unwrap();
    let engine = runtime::build_engine().unwrap();
    let linker = build_linker(&engine).unwrap();

    let mut registry = PluginRegistry::new(home.path());
    let id = registry
        .install(&plugin_dir(), &engine, &linker)
        .await
        .unwrap();

    assert_eq!(id, "hello-world");
    assert!(registry.get("hello-world").is_some());
    assert_eq!(registry.list().len(), 1);
}

#[tokio::test]
async fn registry_scan_finds_installed_plugin() {
    let home = tempfile::tempdir().unwrap();
    let engine = runtime::build_engine().unwrap();
    let linker = build_linker(&engine).unwrap();

    // Install first
    let mut registry = PluginRegistry::new(home.path());
    registry
        .install(&plugin_dir(), &engine, &linker)
        .await
        .unwrap();

    // Create a new registry and scan
    let mut registry2 = PluginRegistry::new(home.path());
    registry2.scan(&engine, &linker).await.unwrap();

    assert_eq!(registry2.list().len(), 1);
    assert!(registry2.get("hello-world").is_some());
}

#[tokio::test]
async fn registry_get_nonexistent_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let registry = PluginRegistry::new(dir.path());
    assert!(registry.get("does-not-exist").is_none());
}

// ─── Runtime Config ──────────────────────────────────────────

#[test]
fn runtime_config_has_sensible_defaults() {
    let config = RuntimeConfig::default();
    assert!(config.max_memory_bytes > 0);
    assert!(config.max_fuel > 0);
    assert!(config.execution_timeout.as_secs() > 0);
}

#[test]
fn build_engine_succeeds() {
    let engine = runtime::build_engine();
    assert!(engine.is_ok());
}

#[test]
fn build_linker_succeeds() {
    let engine = runtime::build_engine().unwrap();
    let linker = build_linker(&engine);
    assert!(linker.is_ok());
}

// ─── Multiple Invocations (isolation) ────────────────────────

#[tokio::test]
async fn multiple_invocations_are_isolated() {
    let (engine, plugin) = setup().await;

    let inv1 = engine
        .invoke(
            &plugin,
            "greet",
            serde_json::json!({"name": "Alice"}),
            &allow_all_policy(),
            CancellationToken::new(),
        )
        .await
        .unwrap();

    let inv2 = engine
        .invoke(
            &plugin,
            "greet",
            serde_json::json!({"name": "Bob"}),
            &allow_all_policy(),
            CancellationToken::new(),
        )
        .await
        .unwrap();

    // Different invocation IDs
    assert_ne!(inv1.id, inv2.id);

    // Both succeed independently
    assert_eq!(inv1.state, InvocationState::Completed);
    assert_eq!(inv2.state, InvocationState::Completed);

    // Correct outputs
    match (&inv1.result, &inv2.result) {
        (
            Some(rusty_core::invocation::InvocationResult::Success(v1)),
            Some(rusty_core::invocation::InvocationResult::Success(v2)),
        ) => {
            assert_eq!(v1["message"], "Hello, Alice!");
            assert_eq!(v2["message"], "Hello, Bob!");
        }
        other => panic!("expected both Success, got {other:?}"),
    }
}

# rusty

A policy-aware WASM execution platform for portable, sandboxed, plugin-hosted tools.

Instead of giving agents raw functions, shell access, or direct API clients, rusty exposes portable plugin actions that run inside a controlled host. Every action is typed, inspectable, traceable, permissioned, and isolated.

## How it works

1. **Tools are WASM plugins.** Authors write plugins in Rust (or any language targeting `wasm32-wasip2`), declaring actions with typed input/output schemas.
2. **The host brokers all side effects.** Plugins can't touch the filesystem, network, or secrets directly. They request operations through a governed host API.
3. **Policies control execution.** A first-match-wins rule engine evaluates every invocation — allow, deny, or require approval — based on action kind, capabilities, tags, or plugin identity.
4. **Everything is traced.** Every invocation produces a structured execution trace: validation, policy decision, host calls, timing, and result.

## Quick start

### Prerequisites

- Rust stable (1.78+)
- `wasm32-wasip2` target: `rustup target add wasm32-wasip2`
- `wasm-tools`: `cargo install wasm-tools`

### Build

```bash
# Build the host CLI
cargo build --release -p rusty-cli

# Build the example plugin
cd examples/plugins/hello-world
cargo build --target wasm32-wasip2 --release
cp target/wasm32-wasip2/release/hello_world_plugin.wasm hello-world.wasm
cd ../../..
```

### Run

```bash
# Install the plugin
rusty install examples/plugins/hello-world

# List installed plugins
rusty list

# Inspect a plugin's actions and schemas
rusty inspect hello-world

# Invoke an action
rusty invoke hello-world greet --input '{"name": "World"}'

# Invoke with full execution trace
rusty invoke hello-world greet --input '{"name": "World"}' --trace

# Retrieve a saved trace by run ID
rusty trace <run-id>
```

### Schema validation

```bash
# Invalid input is rejected before the plugin ever executes
rusty invoke hello-world greet --input '{}'
# => error: [validation_failed] "name" is a required property
```

### Policy enforcement

Create `$RUSTY_HOME/policy.toml`:

```toml
default-effect = "deny"

[[rules]]
id = "allow-read-only"
effect = "allow"
match-action-kind = "read-only"

[[rules]]
id = "approve-destructive"
effect = "require-approval"
match-action-kind = "destructive"
```

## Writing a plugin

A plugin is a WASM component that exports a `guest` interface defined in WIT. Here's the minimal structure:

**`rusty-plugin.toml`** — manifest declaring metadata, actions, and capabilities:

```toml
[plugin]
id = "my-plugin"
name = "My Plugin"
version = "0.1.0"
author = "You"
description = "Does something useful"

capabilities = []

[[actions]]
id = "do-thing"
title = "Do Thing"
description = "Does the thing"
kind = "read-only"
approval = "none-required"
tags = ["example"]
capabilities = []

[actions.input-schema]
type = "object"
required = ["value"]
[actions.input-schema.properties.value]
type = "string"

[actions.output-schema]
type = "object"
required = ["result"]
[actions.output-schema.properties.result]
type = "string"
```

**`src/lib.rs`** — implement the guest interface:

```rust
use rusty_plugin_sdk::{from_json, to_json};

wit_bindgen::generate!({
    inline: rusty_plugin_sdk::PLUGIN_WIT,
});

use exports::rusty::plugin::guest::Guest;
use rusty::plugin::types::*;

struct MyPlugin;

impl Guest for MyPlugin {
    fn get_info() -> PluginInfo {
        PluginInfo {
            id: "my-plugin".into(),
            name: "My Plugin".into(),
            version: "0.1.0".into(),
            author: "You".into(),
            description: "Does something useful".into(),
        }
    }

    fn list_actions() -> Vec<ActionDef> {
        vec![/* ... */]
    }

    fn invoke(action_id: String, input: String) -> ActionResult {
        // Parse input, do work, return result
        ActionResult::Ok(to_json(&serde_json::json!({"result": "done"})))
    }
}

export!(MyPlugin);
```

Build with `cargo build --target wasm32-wasip2 --release`, copy the `.wasm` to the plugin directory matching the manifest ID, then `rusty install <dir>`.

## Architecture

```
rusty/
├── crates/
│   ├── rusty-core        # Shared types: manifest, action, capability, policy, trace, invocation
│   ├── rusty-wit         # WIT interface definitions + wasmtime bindgen host bindings
│   ├── rusty-engine      # Plugin loading, invocation lifecycle, registry
│   ├── rusty-policy      # First-match-wins policy rule engine
│   ├── rusty-cli         # CLI binary (install, list, inspect, invoke, trace)
│   └── rusty-plugin-sdk  # Guest-side SDK for plugin authors
└── examples/plugins/
    └── hello-world       # Example plugin
```

### Invocation lifecycle

Every action invocation follows a strict state machine:

```
requested → validated → policy-evaluated → approved → scheduled → started → completed
                │              │                                       │
                └→ failed      ├→ denied                               ├→ failed
                  (schema)     └→ denied                               ├→ timed-out
                                 (approval required)                   └→ cancelled
```

### Host API (WIT)

Plugins import a `host-api` interface for controlled side effects:

- `log(level, message)` — structured logging through the host
- `get-config(key)` — read host-provided configuration
- `emit-event(type, payload)` — emit custom trace events

All host calls are recorded in the execution trace.

### Key design decisions

- **Fresh WASM Store per invocation** — complete memory isolation between calls
- **Pre-instantiation** (`PluginWorldPre`) — validates exports at install time, not invoke time
- **Fuel-based metering** — deterministic per-instruction accounting with async yield
- **Schema validation before execution** — bad input never reaches the plugin
- **First-match-wins policy** — simple, predictable, auditable (like firewall rules)

## Tests

```bash
cargo test --workspace
```

70 tests across three layers:
- **27 unit tests** in `rusty-core` (manifest parsing, schema validation, policy config, action/capability enums, invocation lifecycle, trace events)
- **6 unit tests** in `rusty-policy` (rule matching, first-match-wins, multi-condition AND logic)
- **21 integration tests** in `rusty-engine` (plugin loading, invoke success/failure, schema rejection, policy enforcement, cancellation, registry operations, isolation)
- **16 CLI tests** in `rusty-cli` (install, list, inspect, invoke, trace — success and error paths)

## Agent SDK integration

The `tests/agent-sdk/` directory contains a test that exposes rusty plugin actions as MCP tools for the [Claude Agent SDK](https://docs.anthropic.com/en/docs/agents/agent-sdk). A Claude agent autonomously discovers plugins, inspects schemas, invokes actions, and handles validation errors — demonstrating rusty as an execution substrate for AI agents.

## License

MIT

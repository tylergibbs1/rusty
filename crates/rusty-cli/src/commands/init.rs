use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
pub struct Args {
    /// Plugin name (e.g. "my-plugin")
    name: String,
    /// Directory to create the plugin in (defaults to ./<name>)
    #[arg(long)]
    path: Option<PathBuf>,
}

const LIB_RS_TEMPLATE: &str = r##"use rusty_plugin_sdk::{from_json, to_json};

wit_bindgen::generate!({
    inline: rusty_plugin_sdk::PLUGIN_WIT,
});

use exports::rusty::plugin::guest::Guest;
use rusty::plugin::host_api;
use rusty::plugin::types::*;

struct Plugin;

impl Guest for Plugin {
    fn get_info() -> PluginInfo {
        PluginInfo {
            id: "$$PLUGIN_NAME$$".into(),
            name: "$$PLUGIN_NAME$$".into(),
            version: "0.1.0".into(),
            author: "".into(),
            description: "".into(),
        }
    }

    fn list_actions() -> Vec<ActionDef> {
        vec![ActionDef {
            id: "hello".into(),
            title: "Hello".into(),
            description: "A starter action".into(),
            input_schema: r#"{"type":"object","required":["name"],"properties":{"name":{"type":"string"}}}"#.into(),
            output_schema: r#"{"type":"object","required":["message"],"properties":{"message":{"type":"string"}}}"#.into(),
            kind: ActionKind::ReadOnly,
            approval: ApprovalClass::NoneRequired,
            tags: vec![],
        }]
    }

    fn invoke(action_id: String, input: String) -> ActionResult {
        host_api::log(LogLevel::Info, &format!("invoking {action_id}"));

        match action_id.as_str() {
            "hello" => {
                let parsed: serde_json::Value = match from_json(&input) {
                    Ok(v) => v,
                    Err(e) => return ActionResult::Err(ActionError {
                        code: "parse_error".into(),
                        message: e,
                        details: None,
                    }),
                };
                let name = parsed["name"].as_str().unwrap_or("world");
                ActionResult::Ok(to_json(&serde_json::json!({ "message": format!("Hello, {name}!") })))
            }
            _ => ActionResult::Err(ActionError {
                code: "unknown_action".into(),
                message: format!("unknown action: {action_id}"),
                details: None,
            }),
        }
    }
}

export!(Plugin);
"##;

pub async fn run(args: Args) -> anyhow::Result<()> {
    let dir = args.path.unwrap_or_else(|| PathBuf::from(&args.name));

    if dir.exists() {
        anyhow::bail!("directory already exists: {}", dir.display());
    }

    let name = &args.name;
    let crate_name = name.replace('-', "_");

    tokio::fs::create_dir_all(dir.join("src")).await?;

    let cargo_toml = format!(
        r#"[package]
name = "{crate_name}"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
rusty-plugin-sdk = {{ git = "https://github.com/tylergibbs1/rusty.git", path = "crates/rusty-plugin-sdk" }}
wit-bindgen = "0.54"
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
"#
    );

    let manifest = format!(
        r#"[plugin]
id = "{name}"
name = "{name}"
version = "0.1.0"
author = ""
description = ""

capabilities = []

[[actions]]
id = "hello"
title = "Hello"
description = "A starter action"
kind = "read-only"
approval = "none-required"
tags = []
capabilities = []

[actions.input-schema]
type = "object"
required = ["name"]

[actions.input-schema.properties.name]
type = "string"

[actions.output-schema]
type = "object"
required = ["message"]

[actions.output-schema.properties.message]
type = "string"
"#
    );

    let lib_rs = LIB_RS_TEMPLATE.replace("$$PLUGIN_NAME$$", name);

    tokio::fs::write(dir.join("Cargo.toml"), cargo_toml).await?;
    tokio::fs::write(dir.join("rusty-plugin.toml"), manifest).await?;
    tokio::fs::write(dir.join("src/lib.rs"), lib_rs).await?;

    use owo_colors::OwoColorize;

    println!(
        "{} Created plugin: {}",
        "done".green().bold(),
        name.cyan().bold()
    );
    println!();
    println!("  {} {name}", "cd".dimmed());
    println!("  {} build", "rusty".dimmed());
    println!("  {} install .", "rusty".dimmed());

    Ok(())
}

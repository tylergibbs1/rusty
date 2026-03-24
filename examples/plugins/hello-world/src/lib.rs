use rusty_plugin_sdk::{from_json, to_json};

wit_bindgen::generate!({
    inline: r#"
        package rusty:plugin@0.1.0;

        interface types {
            enum log-level { trace, debug, info, warn, error }
            enum action-kind { read-only, mutating, destructive }
            enum approval-class { none-required, auto-approve, require-human }

            record action-def {
                id: string,
                title: string,
                description: string,
                input-schema: string,
                output-schema: string,
                kind: action-kind,
                approval: approval-class,
                tags: list<string>,
            }

            record plugin-info {
                id: string,
                name: string,
                version: string,
                author: string,
                description: string,
            }

            record action-error {
                code: string,
                message: string,
                details: option<string>,
            }

            variant action-result {
                ok(string),
                err(action-error),
            }
        }

        interface host-api {
            use types.{log-level};
            log: func(level: log-level, message: string);
            get-config: func(key: string) -> option<string>;
            emit-event: func(event-type: string, payload: string);
        }

        interface guest {
            use types.{plugin-info, action-def, action-result};
            get-info: func() -> plugin-info;
            list-actions: func() -> list<action-def>;
            invoke: func(action-id: string, input: string) -> action-result;
        }

        world plugin-world {
            import host-api;
            export guest;
        }
    "#,
});

use exports::rusty::plugin::guest::Guest;
use rusty::plugin::host_api;
use rusty::plugin::types::*;

struct HelloWorld;

impl Guest for HelloWorld {
    fn get_info() -> PluginInfo {
        PluginInfo {
            id: "hello-world".into(),
            name: "Hello World".into(),
            version: "0.1.0".into(),
            author: "Tyler Gibbs".into(),
            description: "A simple greeting plugin".into(),
        }
    }

    fn list_actions() -> Vec<ActionDef> {
        vec![ActionDef {
            id: "greet".into(),
            title: "Greet".into(),
            description: "Returns a personalized greeting".into(),
            input_schema: r#"{"type":"object","required":["name"],"properties":{"name":{"type":"string"}}}"#.into(),
            output_schema: r#"{"type":"object","required":["message"],"properties":{"message":{"type":"string"}}}"#.into(),
            kind: ActionKind::ReadOnly,
            approval: ApprovalClass::NoneRequired,
            tags: vec!["demo".into(), "greeting".into()],
        }]
    }

    fn invoke(action_id: String, input: String) -> ActionResult {
        host_api::log(LogLevel::Info, &format!("invoking action: {action_id}"));

        match action_id.as_str() {
            "greet" => {
                let parsed: serde_json::Value = match from_json(&input) {
                    Ok(v) => v,
                    Err(e) => {
                        return ActionResult::Err(ActionError {
                            code: "parse_error".into(),
                            message: e,
                            details: None,
                        });
                    }
                };
                let name = parsed["name"].as_str().unwrap_or("world");
                let output = serde_json::json!({ "message": format!("Hello, {name}!") });
                ActionResult::Ok(to_json(&output))
            }
            _ => ActionResult::Err(ActionError {
                code: "unknown_action".into(),
                message: format!("no action with id: {action_id}"),
                details: None,
            }),
        }
    }
}

export!(HelloWorld);

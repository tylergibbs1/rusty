use rusty_core::action::ActionDef;
use rusty_core::invocation::Invocation;
use rusty_core::manifest::PluginManifest;

pub fn print_plugin_summary(manifest: &PluginManifest) {
    println!(
        "  {} v{} — {}",
        manifest.plugin.name, manifest.plugin.version, manifest.plugin.description
    );
    println!("  id: {}", manifest.plugin.id);
    if !manifest.capabilities.is_empty() {
        println!(
            "  capabilities: {}",
            manifest
                .capabilities
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!("  actions: {}", manifest.actions.len());
    for action in &manifest.actions {
        println!("    - {} [{}] {}", action.id, action.kind, action.title);
    }
}

pub fn print_action_detail(action: &ActionDef) {
    println!("Action: {} ({})", action.title, action.id);
    println!("  description: {}", action.description);
    println!("  kind: {}", action.kind);
    println!("  approval: {}", action.approval);
    if !action.tags.is_empty() {
        println!("  tags: {}", action.tags.join(", "));
    }
    if !action.capabilities.is_empty() {
        println!(
            "  capabilities: {}",
            action
                .capabilities
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!(
        "  input schema: {}",
        serde_json::to_string_pretty(&action.input_schema).unwrap_or_default()
    );
    println!(
        "  output schema: {}",
        serde_json::to_string_pretty(&action.output_schema).unwrap_or_default()
    );
}

pub fn print_invocation_result(invocation: &Invocation) {
    println!("Run: {}", invocation.id);
    println!("  plugin: {}", invocation.plugin_id);
    println!("  action: {}", invocation.action_id);
    println!("  state: {}", invocation.state);

    match &invocation.result {
        Some(rusty_core::invocation::InvocationResult::Success(output)) => {
            println!(
                "  output: {}",
                serde_json::to_string_pretty(output).unwrap_or_default()
            );
        }
        Some(rusty_core::invocation::InvocationResult::Error(err)) => {
            println!("  error: [{}] {}", err.code, err.message);
            if let Some(details) = &err.details {
                println!("  details: {details}");
            }
        }
        None => {}
    }
}

pub fn print_trace(invocation: &Invocation) {
    println!("Trace for run {}", invocation.id);
    println!(
        "  plugin: {} | action: {}",
        invocation.plugin_id, invocation.action_id
    );
    println!("  state: {}", invocation.state);
    println!();
    for event in &invocation.trace.events {
        println!(
            "  [{}] {}",
            event.timestamp.format("%H:%M:%S%.3f"),
            serde_json::to_string(&event.kind).unwrap_or_else(|_| format!("{:?}", event.kind))
        );
    }
}

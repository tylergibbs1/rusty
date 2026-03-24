use owo_colors::OwoColorize;
use rusty_core::action::ActionDef;
use rusty_core::invocation::{Invocation, InvocationState};
use rusty_core::manifest::PluginManifest;
use rusty_core::trace::TraceEventKind;

pub fn print_plugin_summary(manifest: &PluginManifest) {
    println!(
        "  {} {} — {}",
        manifest.plugin.name.bold(),
        format!("v{}", manifest.plugin.version).dimmed(),
        manifest.plugin.description
    );
    println!("  {} {}", "id:".dimmed(), manifest.plugin.id.cyan());
    if !manifest.capabilities.is_empty() {
        println!(
            "  {} {}",
            "capabilities:".dimmed(),
            manifest
                .capabilities
                .iter()
                .map(|c| c.to_string().yellow().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!("  {} {}", "actions:".dimmed(), manifest.actions.len());
    for action in &manifest.actions {
        let kind_colored = match action.kind.to_string().as_str() {
            "read-only" => action.kind.to_string().green().to_string(),
            "mutating" => action.kind.to_string().yellow().to_string(),
            "destructive" => action.kind.to_string().red().to_string(),
            _ => action.kind.to_string(),
        };
        println!(
            "    {} {} {} {}",
            "-".dimmed(),
            action.id.cyan(),
            format!("[{kind_colored}]"),
            action.title
        );
    }
}

pub fn print_action_detail(action: &ActionDef) {
    let kind_colored = match action.kind.to_string().as_str() {
        "read-only" => action.kind.to_string().green().to_string(),
        "mutating" => action.kind.to_string().yellow().to_string(),
        "destructive" => action.kind.to_string().red().to_string(),
        _ => action.kind.to_string(),
    };

    println!(
        "{} {} {}",
        "Action:".bold(),
        action.title.bold(),
        format!("({})", action.id).dimmed()
    );
    println!("  {} {}", "description:".dimmed(), action.description);
    println!("  {} {}", "kind:".dimmed(), kind_colored);
    println!("  {} {}", "approval:".dimmed(), action.approval);
    if !action.tags.is_empty() {
        println!(
            "  {} {}",
            "tags:".dimmed(),
            action
                .tags
                .iter()
                .map(|t| t.cyan().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if !action.capabilities.is_empty() {
        println!(
            "  {} {}",
            "capabilities:".dimmed(),
            action
                .capabilities
                .iter()
                .map(|c| c.to_string().yellow().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!(
        "  {} {}",
        "input schema:".dimmed(),
        serde_json::to_string_pretty(&action.input_schema).unwrap_or_default()
    );
    println!(
        "  {} {}",
        "output schema:".dimmed(),
        serde_json::to_string_pretty(&action.output_schema).unwrap_or_default()
    );
}

pub fn print_invocation_result(invocation: &Invocation) {
    println!("{} {}", "Run:".bold(), invocation.id.dimmed());
    println!(
        "  {} {}",
        "plugin:".dimmed(),
        invocation.plugin_id.cyan()
    );
    println!(
        "  {} {}",
        "action:".dimmed(),
        invocation.action_id.cyan()
    );

    let state_str = match invocation.state {
        InvocationState::Completed => "completed".green().bold().to_string(),
        InvocationState::Failed => "failed".red().bold().to_string(),
        InvocationState::Denied => "denied".red().bold().to_string(),
        InvocationState::TimedOut => "timed-out".yellow().bold().to_string(),
        InvocationState::Cancelled => "cancelled".yellow().bold().to_string(),
        other => other.to_string(),
    };
    println!("  {} {}", "state:".dimmed(), state_str);

    match &invocation.result {
        Some(rusty_core::invocation::InvocationResult::Success(output)) => {
            println!(
                "  {} {}",
                "output:".dimmed(),
                serde_json::to_string_pretty(output)
                    .unwrap_or_default()
                    .green()
            );
        }
        Some(rusty_core::invocation::InvocationResult::Error(err)) => {
            println!(
                "  {} {} {}",
                "error:".red().bold(),
                format!("[{}]", err.code).red(),
                err.message
            );
            if let Some(details) = &err.details {
                println!("  {} {details}", "details:".dimmed());
            }
        }
        None => {}
    }
}

pub fn print_trace(invocation: &Invocation) {
    println!(
        "{} {}",
        "Trace for run".bold(),
        invocation.id.to_string().dimmed()
    );
    println!(
        "  {} {} {} {}",
        "plugin:".dimmed(),
        invocation.plugin_id.cyan(),
        "action:".dimmed(),
        invocation.action_id.cyan()
    );

    let state_str = match invocation.state {
        InvocationState::Completed => "completed".green().to_string(),
        InvocationState::Failed => "failed".red().to_string(),
        InvocationState::Denied => "denied".red().to_string(),
        other => other.to_string(),
    };
    println!("  {} {}", "state:".dimmed(), state_str);
    println!();

    for event in &invocation.trace.events {
        let ts = event.timestamp.format("%H:%M:%S%.3f").to_string();
        let timestamp = ts.dimmed();
        let kind_str = format_trace_event(&event.kind);
        println!("  [{timestamp}] {kind_str}");
    }
}

fn format_trace_event(kind: &TraceEventKind) -> String {
    match kind {
        TraceEventKind::InvocationRequested { action_id } => {
            format!("{} {}", "invoke".cyan(), action_id.bold())
        }
        TraceEventKind::ValidationPassed => "validation passed".green().to_string(),
        TraceEventKind::ValidationFailed { reason } => {
            format!("{} {reason}", "validation failed:".red())
        }
        TraceEventKind::PolicyAllowed { rule } => {
            format!("{} ({})", "policy: allowed".green(), rule.dimmed())
        }
        TraceEventKind::PolicyDenied { rule, reason } => {
            format!(
                "{} ({}) {}",
                "policy: denied".red(),
                rule.dimmed(),
                reason.dimmed()
            )
        }
        TraceEventKind::PolicyRequiresApproval { rule } => {
            format!(
                "{} ({})",
                "policy: requires approval".yellow(),
                rule.dimmed()
            )
        }
        TraceEventKind::ExecutionStarted => "execution started".dimmed().to_string(),
        TraceEventKind::ExecutionSucceeded { duration_ms } => {
            format!("{} {}", "done".green().bold(), format!("({duration_ms}ms)").dimmed())
        }
        TraceEventKind::ExecutionFailed {
            error_code,
            message,
        } => {
            format!("{} [{error_code}] {message}", "failed".red().bold())
        }
        TraceEventKind::ExecutionTimedOut { timeout_ms } => {
            format!("{} after {timeout_ms}ms", "timed out".yellow().bold())
        }
        TraceEventKind::ExecutionCancelled => "cancelled".yellow().bold().to_string(),
        TraceEventKind::HostCallIssued { function } => {
            format!("{} {}", "host call:".dimmed(), function.cyan())
        }
        TraceEventKind::CustomEvent {
            event_type,
            payload,
        } => {
            format!(
                "{} {} {}",
                "event:".dimmed(),
                event_type.cyan(),
                serde_json::to_string(payload).unwrap_or_default().dimmed()
            )
        }
        TraceEventKind::PluginLoaded { plugin_id } => {
            format!("{} {}", "loaded".green(), plugin_id.cyan())
        }
        TraceEventKind::ActionDiscovered {
            plugin_id,
            action_id,
        } => {
            format!(
                "{} {}/{}",
                "discovered".dimmed(),
                plugin_id.cyan(),
                action_id
            )
        }
    }
}

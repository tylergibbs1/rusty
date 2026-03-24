use rusty_core::trace::TraceEventKind;
use rusty_wit::host_api;
use rusty_wit::types::LogLevel;

use crate::store::HostState;

impl rusty_wit::types::Host for HostState {}

impl host_api::Host for HostState {
    async fn log(&mut self, level: LogLevel, message: String) {
        match level {
            LogLevel::Trace => tracing::trace!(plugin = %self.plugin_id, "{message}"),
            LogLevel::Debug => tracing::debug!(plugin = %self.plugin_id, "{message}"),
            LogLevel::Info => tracing::info!(plugin = %self.plugin_id, "{message}"),
            LogLevel::Warn => tracing::warn!(plugin = %self.plugin_id, "{message}"),
            LogLevel::Error => tracing::error!(plugin = %self.plugin_id, "{message}"),
        }
        self.record_trace(TraceEventKind::HostCallIssued {
            function: "log".into(),
        });
    }

    async fn get_config(&mut self, key: String) -> Option<String> {
        self.record_trace(TraceEventKind::HostCallIssued {
            function: "get_config".into(),
        });
        self.config_values.get(&key).cloned()
    }

    async fn emit_event(&mut self, event_type: String, payload: String) {
        let value = serde_json::from_str(&payload).unwrap_or(serde_json::Value::Null);
        self.record_trace(TraceEventKind::CustomEvent {
            event_type,
            payload: value,
        });
    }
}

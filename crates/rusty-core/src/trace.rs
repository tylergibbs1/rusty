use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum TraceEventKind {
    PluginLoaded {
        plugin_id: String,
    },
    ActionDiscovered {
        plugin_id: String,
        action_id: String,
    },
    InvocationRequested {
        action_id: String,
    },
    ValidationPassed,
    ValidationFailed {
        reason: String,
    },
    PolicyAllowed {
        rule: String,
    },
    PolicyDenied {
        rule: String,
        reason: String,
    },
    PolicyRequiresApproval {
        rule: String,
    },
    ExecutionStarted,
    ExecutionSucceeded {
        duration_ms: u64,
    },
    ExecutionFailed {
        error_code: String,
        message: String,
    },
    ExecutionTimedOut {
        timeout_ms: u64,
    },
    ExecutionCancelled,
    HostCallIssued {
        function: String,
    },
    CustomEvent {
        event_type: String,
        payload: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub timestamp: DateTime<Utc>,
    pub invocation_id: Option<Uuid>,
    pub kind: TraceEventKind,
}

impl TraceEvent {
    pub fn new(invocation_id: Option<Uuid>, kind: TraceEventKind) -> Self {
        Self {
            timestamp: Utc::now(),
            invocation_id,
            kind,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub invocation_id: Uuid,
    pub plugin_id: String,
    pub action_id: String,
    pub events: Vec<TraceEvent>,
}

impl ExecutionTrace {
    pub fn new(invocation_id: Uuid, plugin_id: String, action_id: String) -> Self {
        Self {
            invocation_id,
            plugin_id,
            action_id,
            events: Vec::new(),
        }
    }

    pub fn record(&mut self, kind: TraceEventKind) {
        self.events
            .push(TraceEvent::new(Some(self.invocation_id), kind));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_event_has_timestamp_and_invocation_id() {
        let id = Uuid::new_v4();
        let event = TraceEvent::new(Some(id), TraceEventKind::ExecutionStarted);
        assert_eq!(event.invocation_id, Some(id));
        // Timestamp should be recent (within last second)
        let elapsed = Utc::now() - event.timestamp;
        assert!(elapsed.num_seconds() < 1);
    }

    #[test]
    fn execution_trace_records_events_in_order() {
        let id = Uuid::new_v4();
        let mut trace = ExecutionTrace::new(id, "plug".into(), "act".into());
        assert!(trace.events.is_empty());

        trace.record(TraceEventKind::ValidationPassed);
        trace.record(TraceEventKind::PolicyAllowed {
            rule: "default".into(),
        });
        trace.record(TraceEventKind::ExecutionStarted);
        trace.record(TraceEventKind::ExecutionSucceeded { duration_ms: 42 });

        assert_eq!(trace.events.len(), 4);
        // Verify ordering by type
        assert!(matches!(
            &trace.events[0].kind,
            TraceEventKind::ValidationPassed
        ));
        assert!(matches!(
            &trace.events[3].kind,
            TraceEventKind::ExecutionSucceeded { .. }
        ));
        // All events should reference the same invocation
        for event in &trace.events {
            assert_eq!(event.invocation_id, Some(id));
        }
    }

    #[test]
    fn trace_event_kind_serializes_with_type_tag() {
        let kind = TraceEventKind::PolicyDenied {
            rule: "deny-all".into(),
            reason: "blocked".into(),
        };
        let json = serde_json::to_value(&kind).unwrap();
        assert_eq!(json["type"], "policy_denied");
        assert_eq!(json["rule"], "deny-all");
        assert_eq!(json["reason"], "blocked");
    }

    #[test]
    fn trace_event_kind_roundtrips_all_variants() {
        let variants: Vec<TraceEventKind> = vec![
            TraceEventKind::PluginLoaded {
                plugin_id: "p".into(),
            },
            TraceEventKind::ActionDiscovered {
                plugin_id: "p".into(),
                action_id: "a".into(),
            },
            TraceEventKind::InvocationRequested {
                action_id: "a".into(),
            },
            TraceEventKind::ValidationPassed,
            TraceEventKind::ValidationFailed {
                reason: "bad".into(),
            },
            TraceEventKind::PolicyAllowed {
                rule: "r".into(),
            },
            TraceEventKind::PolicyDenied {
                rule: "r".into(),
                reason: "no".into(),
            },
            TraceEventKind::PolicyRequiresApproval {
                rule: "r".into(),
            },
            TraceEventKind::ExecutionStarted,
            TraceEventKind::ExecutionSucceeded { duration_ms: 1 },
            TraceEventKind::ExecutionFailed {
                error_code: "e".into(),
                message: "m".into(),
            },
            TraceEventKind::ExecutionTimedOut { timeout_ms: 5000 },
            TraceEventKind::ExecutionCancelled,
            TraceEventKind::HostCallIssued {
                function: "log".into(),
            },
            TraceEventKind::CustomEvent {
                event_type: "custom".into(),
                payload: serde_json::json!({"k": "v"}),
            },
        ];
        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let restored: TraceEventKind = serde_json::from_str(&json).unwrap();
            // Roundtrip: re-serialize and compare
            assert_eq!(json, serde_json::to_string(&restored).unwrap());
        }
    }
}

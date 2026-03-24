use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::trace::{ExecutionTrace, TraceEventKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InvocationState {
    Requested,
    Validated,
    PolicyEvaluated,
    Approved,
    Denied,
    Scheduled,
    Started,
    Completed,
    Failed,
    TimedOut,
    Cancelled,
}

impl std::fmt::Display for InvocationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Requested => write!(f, "requested"),
            Self::Validated => write!(f, "validated"),
            Self::PolicyEvaluated => write!(f, "policy-evaluated"),
            Self::Approved => write!(f, "approved"),
            Self::Denied => write!(f, "denied"),
            Self::Scheduled => write!(f, "scheduled"),
            Self::Started => write!(f, "started"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::TimedOut => write!(f, "timed-out"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InvocationResult {
    Success(serde_json::Value),
    Error(ActionError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invocation {
    pub id: Uuid,
    pub plugin_id: String,
    pub action_id: String,
    pub input: serde_json::Value,
    pub state: InvocationState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub result: Option<InvocationResult>,
    pub trace: ExecutionTrace,
}

impl Invocation {
    pub fn new(plugin_id: String, action_id: String, input: serde_json::Value) -> Self {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let trace = ExecutionTrace::new(id, plugin_id.clone(), action_id.clone());
        Self {
            id,
            plugin_id,
            action_id,
            input,
            state: InvocationState::Requested,
            created_at: now,
            updated_at: now,
            result: None,
            trace,
        }
    }

    pub fn transition(&mut self, state: InvocationState) {
        self.state = state;
        self.updated_at = Utc::now();
    }

    pub fn record_trace(&mut self, kind: TraceEventKind) {
        self.trace.record(kind);
    }

    pub fn finalize_success(&mut self, output: serde_json::Value, duration_ms: u64) {
        self.result = Some(InvocationResult::Success(output));
        self.transition(InvocationState::Completed);
        self.record_trace(TraceEventKind::ExecutionSucceeded { duration_ms });
    }

    pub fn finalize_error(&mut self, error: ActionError) {
        let code = error.code.clone();
        let message = error.message.clone();
        self.result = Some(InvocationResult::Error(error));
        self.transition(InvocationState::Failed);
        self.record_trace(TraceEventKind::ExecutionFailed {
            error_code: code,
            message,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn new_invocation_starts_in_requested_state() {
        let inv = Invocation::new("p".into(), "a".into(), json!({}));
        assert_eq!(inv.state, InvocationState::Requested);
        assert!(inv.result.is_none());
        assert_eq!(inv.plugin_id, "p");
        assert_eq!(inv.action_id, "a");
        assert!(inv.trace.events.is_empty());
    }

    #[test]
    fn transition_updates_state_and_timestamp() {
        let mut inv = Invocation::new("p".into(), "a".into(), json!({}));
        let before = inv.updated_at;
        // Tiny sleep to ensure timestamp changes (chrono has microsecond precision)
        std::thread::sleep(std::time::Duration::from_millis(1));
        inv.transition(InvocationState::Validated);
        assert_eq!(inv.state, InvocationState::Validated);
        assert!(inv.updated_at >= before);
    }

    #[test]
    fn record_trace_appends_event() {
        let mut inv = Invocation::new("p".into(), "a".into(), json!({}));
        assert_eq!(inv.trace.events.len(), 0);
        inv.record_trace(TraceEventKind::ValidationPassed);
        assert_eq!(inv.trace.events.len(), 1);
        inv.record_trace(TraceEventKind::ExecutionStarted);
        assert_eq!(inv.trace.events.len(), 2);
    }

    #[test]
    fn finalize_success_sets_completed_with_output() {
        let mut inv = Invocation::new("p".into(), "a".into(), json!({}));
        inv.finalize_success(json!({"result": 42}), 100);
        assert_eq!(inv.state, InvocationState::Completed);
        match &inv.result {
            Some(InvocationResult::Success(v)) => assert_eq!(v["result"], 42),
            other => panic!("expected Success, got {other:?}"),
        }
        // Should have recorded ExecutionSucceeded trace
        assert!(inv.trace.events.iter().any(|e| matches!(
            &e.kind,
            TraceEventKind::ExecutionSucceeded { duration_ms: 100 }
        )));
    }

    #[test]
    fn finalize_error_sets_failed_with_error() {
        let mut inv = Invocation::new("p".into(), "a".into(), json!({}));
        inv.finalize_error(ActionError {
            code: "boom".into(),
            message: "it broke".into(),
            details: Some(json!({"hint": "fix it"})),
        });
        assert_eq!(inv.state, InvocationState::Failed);
        match &inv.result {
            Some(InvocationResult::Error(e)) => {
                assert_eq!(e.code, "boom");
                assert_eq!(e.message, "it broke");
                assert!(e.details.is_some());
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn invocation_roundtrips_through_json() {
        let mut inv = Invocation::new("test".into(), "act".into(), json!({"x": 1}));
        inv.finalize_success(json!({"y": 2}), 50);
        let json = serde_json::to_string(&inv).unwrap();
        let restored: Invocation = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.plugin_id, "test");
        assert_eq!(restored.state, InvocationState::Completed);
        assert_eq!(restored.id, inv.id);
    }
}

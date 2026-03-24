use std::collections::{HashMap, HashSet};
use std::time::Instant;

use rusty_core::error::EngineError;
use rusty_core::invocation::{ActionError, Invocation, InvocationState};
use rusty_core::policy::PolicyEffect;
use rusty_core::trace::TraceEventKind;
use rusty_policy::{PolicyContext, PolicyEngine};
use tokio_util::sync::CancellationToken;
use wasmtime::component::Linker;
use wasmtime::component::HasSelf;
use wasmtime::{Engine, Store};

use crate::plugin::LoadedPlugin;
use crate::runtime::RuntimeConfig;
use crate::store::HostState;

pub struct InvocationEngine {
    engine: Engine,
    linker: Linker<HostState>,
    config: RuntimeConfig,
}

impl InvocationEngine {
    pub fn new(engine: Engine, linker: Linker<HostState>, config: RuntimeConfig) -> Self {
        Self {
            engine,
            linker,
            config,
        }
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn linker(&self) -> &Linker<HostState> {
        &self.linker
    }

    pub async fn invoke(
        &self,
        plugin: &LoadedPlugin,
        action_id: &str,
        input: serde_json::Value,
        policy_engine: &PolicyEngine,
        cancel: CancellationToken,
    ) -> Result<Invocation, EngineError> {
        let mut invocation = Invocation::new(
            plugin.plugin_id().to_string(),
            action_id.to_string(),
            input.clone(),
        );

        invocation.record_trace(TraceEventKind::InvocationRequested {
            action_id: action_id.to_string(),
        });

        // 1. Find the action in the manifest
        let action = plugin.manifest.find_action(action_id).ok_or_else(|| {
            EngineError::ActionNotFound {
                plugin_id: plugin.plugin_id().to_string(),
                action_id: action_id.to_string(),
            }
        })?;

        // 2. Validate input against schema
        if let Err(e) = rusty_core::schema::validate(&action.input_schema, &input) {
            invocation.record_trace(TraceEventKind::ValidationFailed {
                reason: e.to_string(),
            });
            invocation.finalize_error(ActionError {
                code: "validation_failed".into(),
                message: e.to_string(),
                details: None,
            });
            return Ok(invocation);
        }
        invocation.transition(InvocationState::Validated);
        invocation.record_trace(TraceEventKind::ValidationPassed);

        // 3. Policy evaluation
        let ctx = PolicyContext {
            plugin_id: plugin.plugin_id().to_string(),
            action_id: action_id.to_string(),
            action_kind: action.kind,
            capabilities: action.capabilities.iter().copied().collect::<HashSet<_>>(),
            tags: action.tags.iter().cloned().collect::<HashSet<_>>(),
        };

        let decision = policy_engine.evaluate(&ctx);
        invocation.transition(InvocationState::PolicyEvaluated);

        match decision.effect {
            PolicyEffect::Allow => {
                invocation.record_trace(TraceEventKind::PolicyAllowed {
                    rule: decision
                        .matched_rule
                        .clone()
                        .unwrap_or_else(|| "default".into()),
                });
                invocation.transition(InvocationState::Approved);
            }
            PolicyEffect::Deny => {
                invocation.record_trace(TraceEventKind::PolicyDenied {
                    rule: decision
                        .matched_rule
                        .clone()
                        .unwrap_or_else(|| "default".into()),
                    reason: decision.reason.clone(),
                });
                invocation.transition(InvocationState::Denied);
                invocation.finalize_error(ActionError {
                    code: "policy_denied".into(),
                    message: decision.reason,
                    details: None,
                });
                return Ok(invocation);
            }
            PolicyEffect::RequireApproval => {
                invocation.record_trace(TraceEventKind::PolicyRequiresApproval {
                    rule: decision
                        .matched_rule
                        .clone()
                        .unwrap_or_else(|| "default".into()),
                });
                invocation.transition(InvocationState::Denied);
                invocation.finalize_error(ActionError {
                    code: "approval_required".into(),
                    message: format!(
                        "action requires approval (not yet implemented): {}",
                        decision.reason
                    ),
                    details: None,
                });
                return Ok(invocation);
            }
        }

        // 4. Execute
        invocation.transition(InvocationState::Scheduled);
        let result = self
            .execute(plugin, &mut invocation, action_id, &input, cancel)
            .await;

        match result {
            Ok((output, duration_ms)) => {
                invocation.finalize_success(output, duration_ms);
            }
            Err(e) => {
                let (code, msg) = match &e {
                    EngineError::Timeout(ms) => {
                        invocation.record_trace(TraceEventKind::ExecutionTimedOut {
                            timeout_ms: *ms,
                        });
                        ("timeout".into(), e.to_string())
                    }
                    EngineError::Cancelled => {
                        invocation.record_trace(TraceEventKind::ExecutionCancelled);
                        ("cancelled".into(), e.to_string())
                    }
                    _ => ("execution_error".into(), e.to_string()),
                };
                invocation.finalize_error(ActionError {
                    code,
                    message: msg,
                    details: None,
                });
            }
        }

        Ok(invocation)
    }

    async fn execute(
        &self,
        plugin: &LoadedPlugin,
        invocation: &mut Invocation,
        action_id: &str,
        input: &serde_json::Value,
        cancel: CancellationToken,
    ) -> Result<(serde_json::Value, u64), EngineError> {
        let host_state = HostState::new(
            invocation.id,
            plugin.plugin_id().to_string(),
            action_id.to_string(),
            HashMap::new(),
            self.config.max_memory_bytes,
        );

        invocation.transition(InvocationState::Started);
        invocation.record_trace(TraceEventKind::ExecutionStarted);

        let mut store = Store::new(&self.engine, host_state);
        store.limiter(|state| &mut state.limits);
        store
            .set_fuel(self.config.max_fuel)
            .map_err(|e| EngineError::Other(format!("failed to set fuel: {e}")))?;
        let _ = store.fuel_async_yield_interval(Some(self.config.async_yield_interval));

        let instance = plugin
            .pre
            .instantiate_async(&mut store)
            .await
            .map_err(|e| EngineError::Trap(format!("instantiation failed: {e}")))?;

        let input_json = serde_json::to_string(input)
            .map_err(|e| EngineError::Other(format!("input serialization failed: {e}")))?;

        let start = Instant::now();

        let result = tokio::select! {
            r = tokio::time::timeout(
                self.config.execution_timeout,
                instance.rusty_plugin_guest().call_invoke(&mut store, action_id, &input_json),
            ) => {
                match r {
                    Ok(Ok(action_result)) => Ok(action_result),
                    Ok(Err(e)) => Err(EngineError::Trap(e.to_string())),
                    Err(_) => Err(EngineError::Timeout(self.config.execution_timeout.as_millis() as u64)),
                }
            }
            _ = cancel.cancelled() => {
                Err(EngineError::Cancelled)
            }
        }?;

        let duration_ms = start.elapsed().as_millis() as u64;

        // Merge trace events from the store's host state
        let host_state = store.into_data();
        for event in host_state.trace.events {
            invocation.trace.events.push(event);
        }

        match result {
            rusty_wit::types::ActionResult::Ok(json_str) => {
                let output: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
                    EngineError::Other(format!("output deserialization failed: {e}"))
                })?;
                Ok((output, duration_ms))
            }
            rusty_wit::types::ActionResult::Err(e) => Err(EngineError::ActionError {
                code: e.code,
                message: e.message,
            }),
        }
    }
}

pub fn build_linker(engine: &Engine) -> anyhow::Result<Linker<HostState>> {
    let mut linker = Linker::new(engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    rusty_wit::PluginWorld::add_to_linker::<HostState, HasSelf<HostState>>(&mut linker, |state| state)?;
    Ok(linker)
}

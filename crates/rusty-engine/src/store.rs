use std::collections::HashMap;

use rusty_core::trace::{ExecutionTrace, TraceEventKind};
use uuid::Uuid;
use wasmtime::component::ResourceTable;
use wasmtime::{StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

pub struct HostState {
    pub wasi_ctx: WasiCtx,
    pub table: ResourceTable,
    pub invocation_id: Uuid,
    pub plugin_id: String,
    pub trace: ExecutionTrace,
    pub config_values: HashMap<String, String>,
    pub limits: StoreLimits,
}

impl HostState {
    pub fn new(
        invocation_id: Uuid,
        plugin_id: String,
        action_id: String,
        config_values: HashMap<String, String>,
        max_memory: usize,
    ) -> Self {
        let wasi_ctx = WasiCtx::builder().build();
        let trace = ExecutionTrace::new(invocation_id, plugin_id.clone(), action_id);
        Self {
            wasi_ctx,
            table: ResourceTable::new(),
            invocation_id,
            plugin_id,
            trace,
            config_values,
            limits: StoreLimitsBuilder::new().memory_size(max_memory).build(),
        }
    }

    pub fn record_trace(&mut self, kind: TraceEventKind) {
        self.trace.record(kind);
    }
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.table,
        }
    }
}

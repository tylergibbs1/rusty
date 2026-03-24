use std::time::Duration;

use wasmtime::{Config, Engine};

pub struct RuntimeConfig {
    pub max_memory_bytes: usize,
    pub max_fuel: u64,
    pub async_yield_interval: u64,
    pub execution_timeout: Duration,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 64 * 1024 * 1024, // 64MB
            max_fuel: 1_000_000_000,
            async_yield_interval: 10_000,
            execution_timeout: Duration::from_secs(30),
        }
    }
}

pub fn build_engine() -> anyhow::Result<Engine> {
    let mut config = Config::new();
    config.consume_fuel(true);
    config.wasm_component_model(true);
    Ok(Engine::new(&config)?)
}

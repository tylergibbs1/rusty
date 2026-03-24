use std::path::Path;

use rusty_core::error::EngineError;
use rusty_core::manifest::PluginManifest;
use wasmtime::component::{Component, Linker};
use wasmtime::Engine;

use crate::store::HostState;

pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub pre: rusty_wit::PluginWorldPre<HostState>,
}

impl LoadedPlugin {
    pub async fn load(
        engine: &Engine,
        linker: &Linker<HostState>,
        package_path: &Path,
    ) -> Result<Self, EngineError> {
        let manifest_path = package_path.join("rusty-plugin.toml");
        let manifest_str = tokio::fs::read_to_string(&manifest_path)
            .await
            .map_err(|e| EngineError::LoadFailed(format!("cannot read manifest: {e}")))?;

        let manifest = PluginManifest::from_toml(&manifest_str)
            .map_err(|e| EngineError::LoadFailed(format!("invalid manifest: {e}")))?;

        let wasm_path = package_path.join(format!("{}.wasm", manifest.plugin.id));
        if !wasm_path.exists() {
            return Err(EngineError::LoadFailed(format!(
                "WASM file not found: {}",
                wasm_path.display()
            )));
        }

        let component = Component::from_file(engine, &wasm_path)
            .map_err(|e| EngineError::LoadFailed(format!("failed to load WASM component: {e}")))?;

        let instance_pre = linker
            .instantiate_pre(&component)
            .map_err(|e| EngineError::LoadFailed(format!("pre-instantiation failed: {e}")))?;

        let pre = rusty_wit::PluginWorldPre::new(instance_pre)
            .map_err(|e| EngineError::LoadFailed(format!("plugin world binding failed: {e}")))?;

        Ok(Self { manifest, pre })
    }

    pub fn plugin_id(&self) -> &str {
        &self.manifest.plugin.id
    }
}

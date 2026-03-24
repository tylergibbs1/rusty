use std::path::{Path, PathBuf};

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

        let wasm_path = find_wasm(package_path, &manifest.plugin.id).await?;

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

/// Find the .wasm file for a plugin. Tries `{id}.wasm` first, then falls back
/// to any .wasm file in the directory.
async fn find_wasm(dir: &Path, plugin_id: &str) -> Result<PathBuf, EngineError> {
    // Prefer exact match
    let exact = dir.join(format!("{plugin_id}.wasm"));
    if exact.exists() {
        return Ok(exact);
    }

    // Fall back to any .wasm file
    let mut entries = tokio::fs::read_dir(dir)
        .await
        .map_err(|e| EngineError::LoadFailed(format!("cannot read plugin dir: {e}")))?;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "wasm") {
            return Ok(path);
        }
    }

    Err(EngineError::LoadFailed(format!(
        "no .wasm file found in {}\n  run `rusty build` in your plugin directory first",
        dir.display()
    )))
}

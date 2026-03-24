use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusty_core::error::EngineError;
use wasmtime::component::Linker;
use wasmtime::Engine;

use crate::plugin::LoadedPlugin;
use crate::store::HostState;

pub struct PluginRegistry {
    plugins_dir: PathBuf,
    plugins: HashMap<String, LoadedPlugin>,
}

impl PluginRegistry {
    pub fn new(home_dir: &Path) -> Self {
        let plugins_dir = home_dir.join("plugins");
        Self {
            plugins_dir,
            plugins: HashMap::new(),
        }
    }

    pub async fn scan(
        &mut self,
        engine: &Engine,
        linker: &Linker<HostState>,
    ) -> anyhow::Result<()> {
        if !self.plugins_dir.exists() {
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(&self.plugins_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if !entry.file_type().await?.is_dir() {
                continue;
            }
            let path = entry.path();
            match LoadedPlugin::load(engine, linker, &path).await {
                Ok(plugin) => {
                    let id = plugin.plugin_id().to_string();
                    tracing::info!(plugin_id = %id, "loaded plugin");
                    self.plugins.insert(id, plugin);
                }
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "skipping invalid plugin");
                }
            }
        }
        Ok(())
    }

    pub async fn install(
        &mut self,
        source: &Path,
        engine: &Engine,
        linker: &Linker<HostState>,
    ) -> Result<String, EngineError> {
        // Load and validate first
        let plugin = LoadedPlugin::load(engine, linker, source).await?;
        let plugin_id = plugin.plugin_id().to_string();

        // Copy to plugins dir
        let dest = self.plugins_dir.join(&plugin_id);
        tokio::fs::create_dir_all(&dest)
            .await
            .map_err(|e| EngineError::Other(format!("cannot create plugin dir: {e}")))?;

        copy_dir(source, &dest)
            .await
            .map_err(|e| EngineError::Other(format!("cannot copy plugin files: {e}")))?;

        // Reload from installed location
        let installed = LoadedPlugin::load(engine, linker, &dest).await?;
        self.plugins.insert(plugin_id.clone(), installed);
        Ok(plugin_id)
    }

    pub fn get(&self, plugin_id: &str) -> Option<&LoadedPlugin> {
        self.plugins.get(plugin_id)
    }

    pub fn list(&self) -> Vec<&LoadedPlugin> {
        self.plugins.values().collect()
    }

    pub fn plugins_dir(&self) -> &Path {
        &self.plugins_dir
    }
}

async fn copy_dir(src: &Path, dest: &Path) -> std::io::Result<()> {
    let mut entries = tokio::fs::read_dir(src).await?;
    while let Some(entry) = entries.next_entry().await? {
        let file_type = entry.file_type().await?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if file_type.is_file() {
            tokio::fs::copy(&src_path, &dest_path).await?;
        } else if file_type.is_dir() {
            tokio::fs::create_dir_all(&dest_path).await?;
            Box::pin(copy_dir(&src_path, &dest_path)).await?;
        }
    }
    Ok(())
}

use serde::{Deserialize, Serialize};

use crate::action::ActionDef;
use crate::capability::Capability;
use crate::error::ManifestError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginMeta,
    #[serde(default)]
    pub actions: Vec<ActionDef>,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    #[serde(rename = "runtime-compat", default = "default_runtime_compat")]
    pub runtime_compat: String,
}

fn default_runtime_compat() -> String {
    "0.1".to_string()
}

impl PluginManifest {
    pub fn from_toml(s: &str) -> Result<Self, ManifestError> {
        let manifest: Self = toml::from_str(s).map_err(ManifestError::Parse)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.plugin.id.is_empty() {
            return Err(ManifestError::Validation("plugin.id is required".into()));
        }
        if self.plugin.name.is_empty() {
            return Err(ManifestError::Validation("plugin.name is required".into()));
        }
        if self.plugin.version.is_empty() {
            return Err(ManifestError::Validation(
                "plugin.version is required".into(),
            ));
        }

        for action in &self.actions {
            if action.id.is_empty() {
                return Err(ManifestError::Validation(
                    "action.id is required for all actions".into(),
                ));
            }
        }

        // Every action-level capability must be declared at the plugin level
        for action in &self.actions {
            for cap in &action.capabilities {
                if !self.capabilities.contains(cap) {
                    return Err(ManifestError::Validation(format!(
                        "action '{}' requests capability '{}' not declared at plugin level",
                        action.id, cap,
                    )));
                }
            }
        }

        Ok(())
    }

    pub fn find_action(&self, action_id: &str) -> Option<&ActionDef> {
        self.actions.iter().find(|a| a.id == action_id)
    }

    pub fn all_capabilities(&self) -> &[Capability] {
        &self.capabilities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_manifest() {
        let toml = r#"
[plugin]
id = "test"
name = "Test Plugin"
version = "0.1.0"
author = "test"
description = "A test plugin"
"#;
        let manifest = PluginManifest::from_toml(toml).unwrap();
        assert_eq!(manifest.plugin.id, "test");
        assert!(manifest.actions.is_empty());
    }

    #[test]
    fn parse_full_manifest() {
        let toml = r#"
[plugin]
id = "hello-world"
name = "Hello World"
version = "0.1.0"
author = "Tyler Gibbs"
description = "A greeting plugin"

capabilities = ["clock"]

[[actions]]
id = "greet"
title = "Greet"
description = "Returns a personalized greeting"
kind = "read-only"
approval = "none-required"
tags = ["demo"]
capabilities = []

[actions.input-schema]
type = "object"
required = ["name"]

[actions.input-schema.properties.name]
type = "string"

[actions.output-schema]
type = "object"
required = ["message"]

[actions.output-schema.properties.message]
type = "string"
"#;
        let manifest = PluginManifest::from_toml(toml).unwrap();
        assert_eq!(manifest.plugin.id, "hello-world");
        assert_eq!(manifest.actions.len(), 1);
        assert_eq!(manifest.actions[0].id, "greet");
    }

    #[test]
    fn reject_empty_plugin_id() {
        let toml = r#"
[plugin]
id = ""
name = "Test"
version = "0.1.0"
author = "test"
description = "test"
"#;
        assert!(PluginManifest::from_toml(toml).is_err());
    }

    #[test]
    fn reject_undeclared_action_capability() {
        let toml = r#"
[plugin]
id = "test"
name = "Test"
version = "0.1.0"
author = "test"
description = "test"

capabilities = []

[[actions]]
id = "do-stuff"
title = "Do Stuff"
description = "Does stuff"
kind = "mutating"
approval = "none-required"
capabilities = ["network-fetch"]

[actions.input-schema]
type = "object"

[actions.output-schema]
type = "object"
"#;
        let err = PluginManifest::from_toml(toml).unwrap_err();
        assert!(
            err.to_string().contains("network-fetch"),
            "error should mention the undeclared capability: {err}"
        );
    }
}

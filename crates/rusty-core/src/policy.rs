use serde::{Deserialize, Serialize};

use crate::action::ActionKind;
use crate::capability::Capability;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyEffect {
    Allow,
    Deny,
    RequireApproval,
}

impl Default for PolicyEffect {
    fn default() -> Self {
        Self::Allow
    }
}

impl std::fmt::Display for PolicyEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Allow => write!(f, "allow"),
            Self::Deny => write!(f, "deny"),
            Self::RequireApproval => write!(f, "require-approval"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    pub effect: PolicyEffect,
    #[serde(rename = "match-action-kind")]
    pub match_action_kind: Option<ActionKind>,
    #[serde(rename = "match-capability")]
    pub match_capability: Option<Capability>,
    #[serde(rename = "match-tag")]
    pub match_tag: Option<String>,
    #[serde(rename = "match-plugin-id")]
    pub match_plugin_id: Option<String>,
    #[serde(rename = "match-action-id")]
    pub match_action_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    #[serde(rename = "default-effect", default)]
    pub default_effect: PolicyEffect,
    #[serde(default)]
    pub rules: Vec<PolicyRule>,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            default_effect: PolicyEffect::Allow,
            rules: Vec::new(),
        }
    }
}

impl PolicyConfig {
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_policy_config() {
        let toml = r#"
default-effect = "deny"

[[rules]]
id = "allow-read-only"
description = "Allow all read-only actions"
effect = "allow"
match-action-kind = "read-only"

[[rules]]
id = "approve-destructive"
effect = "require-approval"
match-action-kind = "destructive"
"#;
        let config = PolicyConfig::from_toml(toml).unwrap();
        assert_eq!(config.default_effect, PolicyEffect::Deny);
        assert_eq!(config.rules.len(), 2);
        assert_eq!(config.rules[0].id, "allow-read-only");
        assert_eq!(config.rules[1].effect, PolicyEffect::RequireApproval);
    }

    #[test]
    fn default_policy_allows_all() {
        let config = PolicyConfig::default();
        assert_eq!(config.default_effect, PolicyEffect::Allow);
        assert!(config.rules.is_empty());
    }
}

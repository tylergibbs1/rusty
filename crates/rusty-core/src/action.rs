use serde::{Deserialize, Serialize};

use crate::capability::Capability;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionKind {
    ReadOnly,
    Mutating,
    Destructive,
}

impl std::fmt::Display for ActionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadOnly => write!(f, "read-only"),
            Self::Mutating => write!(f, "mutating"),
            Self::Destructive => write!(f, "destructive"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalClass {
    NoneRequired,
    AutoApprove,
    RequireHuman,
}

impl std::fmt::Display for ApprovalClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoneRequired => write!(f, "none-required"),
            Self::AutoApprove => write!(f, "auto-approve"),
            Self::RequireHuman => write!(f, "require-human"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDef {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "input-schema")]
    pub input_schema: serde_json::Value,
    #[serde(rename = "output-schema")]
    pub output_schema: serde_json::Value,
    pub kind: ActionKind,
    pub approval: ApprovalClass,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_kind_serde_roundtrip() {
        for (kind, expected) in [
            (ActionKind::ReadOnly, "\"read-only\""),
            (ActionKind::Mutating, "\"mutating\""),
            (ActionKind::Destructive, "\"destructive\""),
        ] {
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, expected);
            let restored: ActionKind = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, kind);
        }
    }

    #[test]
    fn approval_class_serde_roundtrip() {
        for (class, expected) in [
            (ApprovalClass::NoneRequired, "\"none-required\""),
            (ApprovalClass::AutoApprove, "\"auto-approve\""),
            (ApprovalClass::RequireHuman, "\"require-human\""),
        ] {
            let json = serde_json::to_string(&class).unwrap();
            assert_eq!(json, expected);
            let restored: ApprovalClass = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, class);
        }
    }

    #[test]
    fn action_kind_display() {
        assert_eq!(ActionKind::ReadOnly.to_string(), "read-only");
        assert_eq!(ActionKind::Mutating.to_string(), "mutating");
        assert_eq!(ActionKind::Destructive.to_string(), "destructive");
    }

    #[test]
    fn approval_class_display() {
        assert_eq!(ApprovalClass::NoneRequired.to_string(), "none-required");
        assert_eq!(ApprovalClass::AutoApprove.to_string(), "auto-approve");
        assert_eq!(ApprovalClass::RequireHuman.to_string(), "require-human");
    }
}

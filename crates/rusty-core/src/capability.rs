use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    FilesystemRead,
    FilesystemWrite,
    NetworkFetch,
    Sqlite,
    KeyValue,
    Secrets,
    EnvVars,
    Clock,
    Artifacts,
    Events,
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FilesystemRead => write!(f, "filesystem-read"),
            Self::FilesystemWrite => write!(f, "filesystem-write"),
            Self::NetworkFetch => write!(f, "network-fetch"),
            Self::Sqlite => write!(f, "sqlite"),
            Self::KeyValue => write!(f, "key-value"),
            Self::Secrets => write!(f, "secrets"),
            Self::EnvVars => write!(f, "env-vars"),
            Self::Clock => write!(f, "clock"),
            Self::Artifacts => write!(f, "artifacts"),
            Self::Events => write!(f, "events"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn serde_roundtrip_all_variants() {
        let all = [
            Capability::FilesystemRead,
            Capability::FilesystemWrite,
            Capability::NetworkFetch,
            Capability::Sqlite,
            Capability::KeyValue,
            Capability::Secrets,
            Capability::EnvVars,
            Capability::Clock,
            Capability::Artifacts,
            Capability::Events,
        ];
        for cap in all {
            let json = serde_json::to_string(&cap).unwrap();
            let restored: Capability = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, cap);
        }
    }

    #[test]
    fn display_matches_serde() {
        let cap = Capability::FilesystemRead;
        let display = cap.to_string();
        let serde = serde_json::to_string(&cap).unwrap();
        // serde includes quotes
        assert_eq!(format!("\"{display}\""), serde);
    }

    #[test]
    fn hashable_for_sets() {
        let mut set = HashSet::new();
        set.insert(Capability::NetworkFetch);
        set.insert(Capability::NetworkFetch);
        set.insert(Capability::Clock);
        assert_eq!(set.len(), 2);
    }
}

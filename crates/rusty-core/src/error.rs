use thiserror::Error;

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("failed to parse manifest: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("manifest validation failed: {0}")]
    Validation(String),
}

#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("invalid JSON schema: {0}")]
    InvalidSchema(String),
    #[error("validation failed: {}", .0.join("; "))]
    ValidationFailed(Vec<String>),
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("plugin not found: {0}")]
    PluginNotFound(String),
    #[error("action not found: {plugin_id}/{action_id}")]
    ActionNotFound {
        plugin_id: String,
        action_id: String,
    },
    #[error("plugin load failed: {0}")]
    LoadFailed(String),
    #[error("plugin init failed: {0}")]
    InitFailed(String),
    #[error("execution timed out after {0}ms")]
    Timeout(u64),
    #[error("execution cancelled")]
    Cancelled,
    #[error("wasm trap: {0}")]
    Trap(String),
    #[error("action returned error: [{code}] {message}")]
    ActionError { code: String, message: String },
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("policy denied: {0}")]
    Denied(String),
    #[error("approval required: {0}")]
    ApprovalRequired(String),
}

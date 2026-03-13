use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing env var: {key}")]
    MissingVar { key: &'static str },

    #[error("invalid env var {key}={value:?}: {reason}")]
    InvalidVar {
        key: &'static str,
        value: String,
        reason: String,
    },
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("App is not found")]
    AppNotFound,
    #[error("registry error: {0}")]
    Message(String),
}

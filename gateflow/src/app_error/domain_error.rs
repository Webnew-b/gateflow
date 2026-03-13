use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("domain error: {0}")]
    Message(String),
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("state error: {0}")]
    Message(String),
}

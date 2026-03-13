use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("db error: {0}")]
    Message(String),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

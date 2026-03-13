use thiserror::Error;
use crate::app_error::DbError;

#[derive(Debug, Error)]
pub enum AdminRpcError {
    #[error("invalid username or password")]
    InvalidCredential,

    #[error("admin user is disabled")]
    UserDisabled,

    #[error("app already exists")]
    AppAlreadyExists,

    #[error(transparent)]
    Db(#[from] DbError),

    #[error("admin_rpc error: {0}")]
    Message(String),
}

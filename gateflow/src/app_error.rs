use thiserror::Error;

pub mod admin_rpc_error;

pub mod config_error;

pub mod dataplane_error;

pub mod db_error;

pub mod domain_error;

pub mod health_udp_error;

pub mod registry_error;

pub mod state_error;

pub use admin_rpc_error::AdminRpcError;
pub use config_error::ConfigError;
pub use dataplane_error::DataplaneError;
pub use db_error::DbError;
pub use domain_error::DomainError;
pub use health_udp_error::HealthUdpError;
pub use registry_error::RegistryError;
pub use state_error::StateError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    AdminRpc(#[from] AdminRpcError),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Dataplane(#[from] DataplaneError),
    #[error(transparent)]
    Db(#[from] DbError),
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    HealthUdp(#[from] HealthUdpError),
    #[error(transparent)]
    Registry(#[from] RegistryError),
    #[error(transparent)]
    State(#[from] StateError),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum HealthUdpError {
    #[error("health_udp error: {0}")]
    Message(String),
}

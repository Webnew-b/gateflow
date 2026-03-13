use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpRule {
    Single(IpAddr),
    Cidr { network: IpAddr, prefix_len: u8 },
}

#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub database_url: String,

    /// Browser -> Gateway (HTTP dataplane)
    pub http_listen_addr: SocketAddr,

    /// CLI -> Gateway (Admin gRPC)
    pub admin_grpc_listen_addr: SocketAddr,

    /// healthd -> Gateway (UDP health reports)
    pub udp_listen_addr: SocketAddr,

    /// Global salt/pepper used for deriving signing key material.
    /// v0: plain string; you can store hex/base64 later if you want.
    pub hmac_global_salt: String,

    /// Static bearer token expected on dataplane requests.
    pub gateway_token: String,

    /// Server-side pepper used when verifying admin password hashes.
    pub admin_password_pepper: String,

    /// Optional global allowlist for dataplane client IPs.
    pub allowed_source_ips: Vec<IpRule>,

    /// Optional global denylist for dataplane client IPs.
    pub blocked_source_ips: Vec<IpRule>,

    // Optional tunables (give defaults if not set)
    pub db_max_connections: u32,
    pub db_connect_timeout: Duration,
    pub proxy_timeout: Duration,
    pub default_app_rate_limit_rps: u32,
    pub run_migrations_on_boot: bool,
    pub persist_health_to_db: bool,
    pub health_status_ttl_secs: u64,
    pub health_cleanup_interval_secs: u64,
    pub signature_replay_window_secs: u64,
}

impl GatewayConfig {
    pub const DEFAULT_DB_MAX_CONNECTIONS: u32 = 10;
    pub const DEFAULT_DB_CONNECT_TIMEOUT_SECS: u64 = 5;
    pub const DEFAULT_PROXY_TIMEOUT_SECS: u64 = 30;
    pub const DEFAULT_APP_RATE_LIMIT_RPS: u32 = 100;
    pub const DEFAULT_HEALTH_STATUS_TTL_SECS: u64 = 120;
    pub const DEFAULT_HEALTH_CLEANUP_INTERVAL_SECS: u64 = 30;
    pub const DEFAULT_SIGNATURE_REPLAY_WINDOW_SECS: u64 = 60;
}

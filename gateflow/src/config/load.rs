// src/config/load.rs
use crate::app_error::ConfigError;
use crate::config::model::{GatewayConfig, IpRule};

use std::{
    env,
    net::{IpAddr, SocketAddr},
    num::ParseIntError,
    time::Duration,
};

pub fn load_config() -> Result<GatewayConfig, ConfigError> {
    let _ = dotenvy::dotenv();

    let database_url = must_get("DATABASE_URL")?;

    let http_listen_addr = must_parse_socket_addr("HTTP_LISTEN_ADDR")?;
    let admin_grpc_listen_addr = must_parse_socket_addr("ADMIN_GRPC_LISTEN_ADDR")?;
    if !admin_grpc_listen_addr.ip().is_loopback() {
        return Err(ConfigError::InvalidVar {
            key: "ADMIN_GRPC_LISTEN_ADDR",
            value: admin_grpc_listen_addr.to_string(),
            reason: "must be a loopback address (127.0.0.1 or ::1)".into(),
        });
    }
    let udp_listen_addr = must_parse_socket_addr("UDP_LISTEN_ADDR")?;

    let hmac_global_salt = must_get("HMAC_GLOBAL_SALT")?;
    if hmac_global_salt.trim().is_empty() {
        return Err(ConfigError::InvalidVar {
            key: "HMAC_GLOBAL_SALT",
            value: hmac_global_salt,
            reason: "must not be empty".into(),
        });
    }

    let gateway_token = must_get("GW_TOKEN")?;
    if gateway_token.trim().is_empty() {
        return Err(ConfigError::InvalidVar {
            key: "GW_TOKEN",
            value: gateway_token,
            reason: "must not be empty".into(),
        });
    }

    let admin_password_pepper = env::var("ADMIN_PASSWORD_PEPPER").unwrap_or_default();
    let allowed_source_ips = parse_ip_rules("ALLOWED_SOURCE_IPS")?;
    let blocked_source_ips = parse_ip_rules("BLOCKED_SOURCE_IPS")?;

    // Optional tunables (defaults apply if missing)
    let db_max_connections =
        get_u32("DB_MAX_CONNECTIONS")?.unwrap_or(GatewayConfig::DEFAULT_DB_MAX_CONNECTIONS);

    let db_connect_timeout_secs = get_u64("DB_CONNECT_TIMEOUT_SECS")?
        .unwrap_or(GatewayConfig::DEFAULT_DB_CONNECT_TIMEOUT_SECS);

    let proxy_timeout_secs =
        get_u64("PROXY_TIMEOUT_SECS")?.unwrap_or(GatewayConfig::DEFAULT_PROXY_TIMEOUT_SECS);
    let default_app_rate_limit_rps = get_u32("DEFAULT_APP_RATE_LIMIT_RPS")?
        .unwrap_or(GatewayConfig::DEFAULT_APP_RATE_LIMIT_RPS);
    let run_migrations_on_boot = get_bool("RUN_MIGRATIONS_ON_BOOT")?.unwrap_or(false);
    let persist_health_to_db = get_bool("PERSIST_HEALTH_TO_DB")?.unwrap_or(true);
    let health_status_ttl_secs = get_u64("HEALTH_STATUS_TTL_SECS")?
        .unwrap_or(GatewayConfig::DEFAULT_HEALTH_STATUS_TTL_SECS);
    let health_cleanup_interval_secs = get_u64("HEALTH_CLEANUP_INTERVAL_SECS")?
        .unwrap_or(GatewayConfig::DEFAULT_HEALTH_CLEANUP_INTERVAL_SECS);
    let signature_replay_window_secs = get_u64("SIGNATURE_REPLAY_WINDOW_SECS")?
        .unwrap_or(GatewayConfig::DEFAULT_SIGNATURE_REPLAY_WINDOW_SECS);

    Ok(GatewayConfig {
        database_url,
        http_listen_addr,
        admin_grpc_listen_addr,
        udp_listen_addr,
        hmac_global_salt,
        gateway_token,
        admin_password_pepper,
        allowed_source_ips,
        blocked_source_ips,
        db_max_connections,
        db_connect_timeout: Duration::from_secs(db_connect_timeout_secs),
        proxy_timeout: Duration::from_secs(proxy_timeout_secs),
        default_app_rate_limit_rps,
        run_migrations_on_boot,
        persist_health_to_db,
        health_status_ttl_secs,
        health_cleanup_interval_secs,
        signature_replay_window_secs,
    })
}

fn must_get(key: &'static str) -> Result<String, ConfigError> {
    match env::var(key) {
        Ok(v) => Ok(v),
        Err(_) => Err(ConfigError::MissingVar { key }),
    }
}

fn must_parse_socket_addr(key: &'static str) -> Result<SocketAddr, ConfigError> {
    let raw = must_get(key)?;
    raw.parse::<SocketAddr>()
        .map_err(|e| ConfigError::InvalidVar {
            key,
            value: raw,
            reason: format!("expected SocketAddr like 0.0.0.0:8080 ({e})"),
        })
}

fn get_u32(key: &'static str) -> Result<Option<u32>, ConfigError> {
    match env::var(key) {
        Ok(raw) => {
            raw.parse::<u32>()
                .map(Some)
                .map_err(|e: ParseIntError| ConfigError::InvalidVar {
                    key,
                    value: raw,
                    reason: format!("expected u32 ({e})"),
                })
        }
        Err(_) => Ok(None),
    }
}

fn get_u64(key: &'static str) -> Result<Option<u64>, ConfigError> {
    match env::var(key) {
        Ok(raw) => {
            raw.parse::<u64>()
                .map(Some)
                .map_err(|e: ParseIntError| ConfigError::InvalidVar {
                    key,
                    value: raw,
                    reason: format!("expected u64 ({e})"),
                })
        }
        Err(_) => Ok(None),
    }
}

fn get_bool(key: &'static str) -> Result<Option<bool>, ConfigError> {
    match env::var(key) {
        Ok(raw) => match raw.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(Some(true)),
            "0" | "false" | "no" | "off" => Ok(Some(false)),
            _ => Err(ConfigError::InvalidVar {
                key,
                value: raw,
                reason: "expected bool (true/false/1/0/yes/no/on/off)".into(),
            }),
        },
        Err(_) => Ok(None),
    }
}

fn parse_ip_rules(key: &'static str) -> Result<Vec<IpRule>, ConfigError> {
    let raw = match env::var(key) {
        Ok(raw) => raw,
        Err(_) => return Ok(Vec::new()),
    };

    raw.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| parse_ip_rule(key, entry))
        .collect()
}

fn parse_ip_rule(key: &'static str, value: &str) -> Result<IpRule, ConfigError> {
    if let Some((network, prefix_len)) = value.split_once('/') {
        let network = network
            .parse::<IpAddr>()
            .map_err(|e| ConfigError::InvalidVar {
                key,
                value: value.to_string(),
                reason: format!("invalid IP network ({e})"),
            })?;
        let prefix_len =
            prefix_len
                .parse::<u8>()
                .map_err(|e: ParseIntError| ConfigError::InvalidVar {
                    key,
                    value: value.to_string(),
                    reason: format!("invalid prefix length ({e})"),
                })?;
        let max_prefix = match network {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        if prefix_len > max_prefix {
            return Err(ConfigError::InvalidVar {
                key,
                value: value.to_string(),
                reason: format!("prefix length must be <= {max_prefix}"),
            });
        }
        return Ok(IpRule::Cidr {
            network,
            prefix_len,
        });
    }

    let ip = value
        .parse::<IpAddr>()
        .map_err(|e| ConfigError::InvalidVar {
            key,
            value: value.to_string(),
            reason: format!("invalid IP address ({e})"),
        })?;
    Ok(IpRule::Single(ip))
}

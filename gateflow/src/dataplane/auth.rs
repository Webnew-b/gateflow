use axum::body::Body;
use axum::http::{HeaderName, Request};

use crate::app_error::DataplaneError;
use crate::config::{GatewayConfig, IpRule};
use crate::db::AppRow;
use dashmap::DashMap;
use governor::{
    Quota, RateLimiter, clock::DefaultClock, state::InMemoryState, state::direct::NotKeyed,
};
use std::num::NonZeroU32;
use std::sync::Arc;

#[derive(Clone)]
struct LimiterEntry {
    rps: u32,
    limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

// simple in-memory rate limiter per app
#[derive(Clone)]
pub struct RateLimiters(Arc<DashMap<String, LimiterEntry>>);

impl Default for RateLimiters {
    fn default() -> Self {
        Self(Arc::new(DashMap::new()))
    }
}

pub const GATEWAY_AUTH_HEADER: HeaderName = HeaderName::from_static("x-authorization");

pub fn validate_gw_token(req: &Request<Body>, cfg: &GatewayConfig) -> Result<(), DataplaneError> {
    let auth = req
        .headers()
        .get(&GATEWAY_AUTH_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if let Some(token) = auth.strip_prefix("Bearer ") {
        if constant_time_eq(token.trim().as_bytes(), cfg.gateway_token.as_bytes()) {
            return Ok(());
        }
    }
    Err(DataplaneError::Unauthorized(
        "missing or invalid gw token".into(),
    ))
}

/// Per-app rate limit with global fallback (`DEFAULT_APP_RATE_LIMIT_RPS`).
pub fn check_rate_limit(
    limiters: &RateLimiters,
    app: &AppRow,
    cfg: &GatewayConfig,
) -> Result<(), DataplaneError> {
    let app_key = app.app_uuid.to_string();
    let rps = effective_rate_limit(app, cfg);

    let limiter = if let Some(existing) = limiters.0.get(&app_key) {
        if existing.rps == rps {
            existing.limiter.clone()
        } else {
            drop(existing);
            let limiter = new_limiter(rps);
            limiters.0.insert(
                app_key,
                LimiterEntry {
                    rps,
                    limiter: limiter.clone(),
                },
            );
            limiter
        }
    } else {
        let limiter = new_limiter(rps);
        limiters.0.insert(
            app_key,
            LimiterEntry {
                rps,
                limiter: limiter.clone(),
            },
        );
        limiter
    };

    if limiter.check().is_ok() {
        Ok(())
    } else {
        Err(DataplaneError::RateLimited)
    }
}

/// Per-app IP policy with global fallback for allowlist.
pub fn check_ip_policy(
    client_ip: std::net::IpAddr,
    app: &AppRow,
    cfg: &GatewayConfig,
) -> Result<(), DataplaneError> {
    let app_blocked = parse_ip_rules_from_text(&app.blocked_source_ips)?;
    let app_allowed = parse_ip_rules_from_text(&app.allowed_source_ips)?;

    let mut blocked_rules = cfg.blocked_source_ips.clone();
    blocked_rules.extend(app_blocked);
    if matches_any_rule(client_ip, &blocked_rules) {
        return Err(DataplaneError::IpForbidden);
    }

    let allowed_rules = if app_allowed.is_empty() {
        cfg.allowed_source_ips.clone()
    } else {
        app_allowed
    };
    if !allowed_rules.is_empty() && !matches_any_rule(client_ip, &allowed_rules) {
        return Err(DataplaneError::IpForbidden);
    }

    Ok(())
}

fn effective_rate_limit(app: &AppRow, cfg: &GatewayConfig) -> u32 {
    match app.rate_limit_rps {
        Some(v) if v > 0 => v as u32,
        _ => cfg.default_app_rate_limit_rps,
    }
}

fn new_limiter(rps: u32) -> Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>> {
    let quota = Quota::per_second(
        NonZeroU32::new(rps).unwrap_or(NonZeroU32::new(1).expect("1 must be non-zero")),
    );
    Arc::new(RateLimiter::direct(quota))
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut diff = 0u8;
    for (&l, &r) in left.iter().zip(right.iter()) {
        diff |= l ^ r;
    }
    diff == 0
}

fn matches_any_rule(client_ip: std::net::IpAddr, rules: &[IpRule]) -> bool {
    rules.iter().any(|rule| match_ip_rule(client_ip, rule))
}

fn parse_ip_rules_from_text(values: &[String]) -> Result<Vec<IpRule>, DataplaneError> {
    values
        .iter()
        .map(|value| parse_ip_rule(value))
        .collect::<Result<Vec<_>, _>>()
}

fn parse_ip_rule(value: &str) -> Result<IpRule, DataplaneError> {
    if let Some((network, prefix_len)) = value.split_once('/') {
        let network =
            network
                .parse::<std::net::IpAddr>()
                .map_err(|e| DataplaneError::Internal(format!("invalid app ip rule '{value}': {e}")))?;
        let prefix_len = prefix_len.parse::<u8>().map_err(|e| {
            DataplaneError::Internal(format!("invalid app ip rule '{value}' prefix: {e}"))
        })?;
        let max_prefix = match network {
            std::net::IpAddr::V4(_) => 32,
            std::net::IpAddr::V6(_) => 128,
        };
        if prefix_len > max_prefix {
            return Err(DataplaneError::Internal(format!(
                "invalid app ip rule '{value}': prefix must be <= {max_prefix}"
            )));
        }
        return Ok(IpRule::Cidr {
            network,
            prefix_len,
        });
    }

    let ip = value
        .parse::<std::net::IpAddr>()
        .map_err(|e| DataplaneError::Internal(format!("invalid app ip rule '{value}': {e}")))?;
    Ok(IpRule::Single(ip))
}

fn match_ip_rule(client_ip: std::net::IpAddr, rule: &IpRule) -> bool {
    match rule {
        IpRule::Single(ip) => client_ip == *ip,
        IpRule::Cidr {
            network,
            prefix_len,
        } => match (client_ip, *network) {
            (std::net::IpAddr::V4(ip), std::net::IpAddr::V4(network)) => {
                ipv4_matches(ip.octets(), network.octets(), *prefix_len)
            }
            (std::net::IpAddr::V6(ip), std::net::IpAddr::V6(network)) => {
                ipv6_matches(ip.octets(), network.octets(), *prefix_len)
            }
            _ => false,
        },
    }
}

fn ipv4_matches(ip: [u8; 4], network: [u8; 4], prefix_len: u8) -> bool {
    if prefix_len == 0 {
        return true;
    }
    let host_bits = 32 - prefix_len;
    let mask = if host_bits == 0 {
        u32::MAX
    } else {
        !((1u32 << host_bits) - 1)
    };
    (u32::from_be_bytes(ip) & mask) == (u32::from_be_bytes(network) & mask)
}

fn ipv6_matches(ip: [u8; 16], network: [u8; 16], prefix_len: u8) -> bool {
    if prefix_len == 0 {
        return true;
    }
    let host_bits = 128 - prefix_len;
    let mask = if host_bits == 0 {
        u128::MAX
    } else {
        !((1u128 << host_bits) - 1)
    };
    (u128::from_be_bytes(ip) & mask) == (u128::from_be_bytes(network) & mask)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GatewayConfig;
    use chrono::Utc;
    use axum::http::Request;
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        time::Duration,
    };
    use uuid::Uuid;

    fn config() -> GatewayConfig {
        GatewayConfig {
            database_url: "postgres://localhost/test".into(),
            http_listen_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080),
            admin_grpc_listen_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 50051),
            udp_listen_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8123),
            hmac_global_salt: "salt".into(),
            gateway_token: "token-123".into(),
            admin_password_pepper: String::new(),
            allowed_source_ips: Vec::new(),
            blocked_source_ips: Vec::new(),
            db_max_connections: 10,
            db_connect_timeout: Duration::from_secs(5),
            proxy_timeout: Duration::from_secs(30),
            default_app_rate_limit_rps: 100,
            run_migrations_on_boot: false,
            persist_health_to_db: true,
            health_status_ttl_secs: 120,
            health_cleanup_interval_secs: 30,
            signature_replay_window_secs: 60,
        }
    }

    fn app(name: &str) -> AppRow {
        AppRow {
            app_uuid: Uuid::new_v4(),
            name: name.to_string(),
            target_url: "http://demo.internal".into(),
            status: "Active".into(),
            mount_path: "/demo".into(),
            upstream_path: "/".into(),
            app_secret: "secret".into(),
            rate_limit_rps: None,
            allowed_source_ips: Vec::new(),
            blocked_source_ips: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn validate_gw_token_accepts_x_authorization_bearer_token() {
        let req = Request::builder()
            .header("x-authorization", "Bearer token-123")
            .body(Body::empty())
            .unwrap();

        let res = validate_gw_token(&req, &config());

        assert!(res.is_ok());
    }

    #[test]
    fn validate_gw_token_rejects_missing_token() {
        let req = Request::builder().body(Body::empty()).unwrap();

        let res = validate_gw_token(&req, &config());

        assert!(matches!(res, Err(DataplaneError::Unauthorized(_))));
    }

    #[test]
    fn validate_gw_token_rejects_wrong_token() {
        let req = Request::builder()
            .header("x-authorization", "Bearer wrong-token")
            .body(Body::empty())
            .unwrap();

        let res = validate_gw_token(&req, &config());

        assert!(matches!(res, Err(DataplaneError::Unauthorized(_))));
    }

    #[test]
    fn rate_limiters_cache_per_app() {
        let limiters = RateLimiters::default();
        let app = app("demo");

        check_rate_limit(&limiters, &app, &config()).unwrap();
        check_rate_limit(&limiters, &app, &config()).unwrap();

        assert_eq!(limiters.0.len(), 1);
    }

    #[test]
    fn ip_policy_rejects_blocked_ip() {
        let mut cfg = config();
        cfg.blocked_source_ips = vec![IpRule::Single("127.0.0.1".parse().unwrap())];
        let app = app("demo");

        let res = check_ip_policy("127.0.0.1".parse().unwrap(), &app, &cfg);

        assert!(matches!(res, Err(DataplaneError::IpForbidden)));
    }

    #[test]
    fn ip_policy_enforces_allowlist() {
        let mut cfg = config();
        cfg.allowed_source_ips = vec![IpRule::Cidr {
            network: "10.0.0.0".parse().unwrap(),
            prefix_len: 8,
        }];
        let app = app("demo");

        assert!(check_ip_policy("10.1.2.3".parse().unwrap(), &app, &cfg).is_ok());
        assert!(matches!(
            check_ip_policy("192.168.1.10".parse().unwrap(), &app, &cfg),
            Err(DataplaneError::IpForbidden)
        ));
    }

    #[test]
    fn ip_policy_prefers_app_allowlist_over_global() {
        let mut cfg = config();
        cfg.allowed_source_ips = vec![IpRule::Single("10.0.0.1".parse().unwrap())];
        let mut app = app("demo");
        app.allowed_source_ips = vec!["192.168.1.0/24".into()];

        assert!(check_ip_policy("192.168.1.23".parse().unwrap(), &app, &cfg).is_ok());
        assert!(matches!(
            check_ip_policy("10.0.0.1".parse().unwrap(), &app, &cfg),
            Err(DataplaneError::IpForbidden)
        ));
    }
}

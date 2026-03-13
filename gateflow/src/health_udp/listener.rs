use std::net::SocketAddr;

use tokio::net::UdpSocket;
use tokio::time::{Duration, interval};

use crate::db::{health_repo::HealthRepo, health_rows::AppHealthRow};
use crate::health_udp::{parse, store};
use crate::{app_error::AppError, state::AppState};

/// UDP health report listener with periodic stale-health cleanup.
pub async fn serve(state: AppState) -> Result<(), AppError> {
    let addr: SocketAddr = state.config.udp_listen_addr;
    let socket = UdpSocket::bind(addr).await.map_err(|e| {
        AppError::HealthUdp(crate::app_error::HealthUdpError::Message(e.to_string()))
    })?;

    tracing::info!("health UDP listening on {addr}");

    let mut cleanup_tick = interval(Duration::from_secs(
        state.config.health_cleanup_interval_secs,
    ));
    let ttl = Duration::from_secs(state.config.health_status_ttl_secs);
    let mut buf = vec![0u8; 1500];
    loop {
        tokio::select! {
            recv = socket.recv_from(&mut buf) => {
                let (len, peer) = recv.map_err(|e| {
                    AppError::HealthUdp(crate::app_error::HealthUdpError::Message(e.to_string()))
                })?;
                tracing::debug!(%peer, len, "received health datagram");
                if let Ok(reports) = parse::parse_datagram(&buf[..len]) {
                    store::upsert_many(&state.health_store, &reports).await;
                    if state.config.persist_health_to_db {
                        let rows: Vec<AppHealthRow> = reports
                            .iter()
                            .map(|report| AppHealthRow {
                                app_uuid: report.app_uuid,
                                last_checked_at: report.checked_at,
                                ok: report.ok,
                                status_code: report.status_code as i32,
                                latency_ms: report.latency_ms as i32,
                            })
                            .collect();
                        let repo = HealthRepo::new(state.db.clone());
                        if let Err(err) = repo.upsert_many_latest(&rows).await {
                            tracing::warn!(error = %err, "failed to persist health report batch");
                        }
                    }
                } else {
                    tracing::warn!(%peer, "failed to parse health datagram");
                }
            }
            _ = cleanup_tick.tick() => {
                let removed = store::prune_stale(&state.health_store, ttl).await;
                if removed > 0 {
                    tracing::info!(removed, "pruned stale health records");
                }
            }
        }
    }
}

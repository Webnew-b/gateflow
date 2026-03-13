use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::health::{AppHealth, HealthReport};

pub type HealthStore = Arc<RwLock<HashMap<Uuid, AppHealth>>>;

pub async fn upsert(store: &HealthStore, report: HealthReport) {
    let mut guard = store.write().await;
    guard.insert(
        report.app_uuid,
        AppHealth {
            last_checked_at: report.checked_at,
            ok: report.ok,
            status_code: report.status_code,
            latency_ms: report.latency_ms,
        },
    );
}

pub async fn upsert_many(store: &HealthStore, reports: &[HealthReport]) {
    if reports.is_empty() {
        return;
    }
    let mut guard = store.write().await;
    for report in reports {
        guard.insert(
            report.app_uuid,
            AppHealth {
                last_checked_at: report.checked_at,
                ok: report.ok,
                status_code: report.status_code,
                latency_ms: report.latency_ms,
            },
        );
    }
}

pub async fn prune_stale(store: &HealthStore, ttl: std::time::Duration) -> usize {
    let now = chrono::Utc::now();
    let mut guard = store.write().await;
    let before = guard.len();
    guard.retain(|_, health| !is_stale_at(health, ttl, now));
    before.saturating_sub(guard.len())
}

pub fn is_stale(health: &AppHealth, ttl: std::time::Duration) -> bool {
    is_stale_at(health, ttl, chrono::Utc::now())
}

fn is_stale_at(
    health: &AppHealth,
    ttl: std::time::Duration,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    now.signed_duration_since(health.last_checked_at)
        > chrono::Duration::from_std(ttl).unwrap_or_else(|_| chrono::Duration::seconds(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration as ChronoDuration, Utc};

    #[tokio::test]
    async fn prune_stale_removes_old_entries() {
        let store: HealthStore = std::sync::Arc::new(RwLock::new(std::collections::HashMap::new()));
        let old_uuid = Uuid::new_v4();
        let fresh_uuid = Uuid::new_v4();
        {
            let mut guard = store.write().await;
            guard.insert(
                old_uuid,
                AppHealth {
                    last_checked_at: Utc::now() - ChronoDuration::seconds(300),
                    ok: true,
                    status_code: 200,
                    latency_ms: 10,
                },
            );
            guard.insert(
                fresh_uuid,
                AppHealth {
                    last_checked_at: Utc::now(),
                    ok: true,
                    status_code: 200,
                    latency_ms: 10,
                },
            );
        }

        let removed = prune_stale(&store, std::time::Duration::from_secs(120)).await;
        let guard = store.read().await;
        assert_eq!(removed, 1);
        assert!(guard.contains_key(&fresh_uuid));
        assert!(!guard.contains_key(&old_uuid));
    }
}

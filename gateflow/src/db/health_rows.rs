use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AppHealthRow {
    pub app_uuid: Uuid,
    pub last_checked_at: DateTime<Utc>,
    pub ok: bool,
    pub status_code: i32,
    pub latency_ms: i32,
}

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Row mapping for table `apps`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AppRow {
    pub app_uuid: Uuid,
    pub name: String,
    pub target_url: String,
    pub status: String,

    pub mount_path: String,
    pub upstream_path: String,

    pub app_secret: String,
    pub rate_limit_rps: Option<i32>,
    pub allowed_source_ips: Vec<String>,
    pub blocked_source_ips: Vec<String>,

    // TIMESTAMPTZ (v0: keep as String; you can later map to time/chrono)
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

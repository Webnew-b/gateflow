/// Row mapping for table `cli_sessions`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CliSessionRow {
    pub session_id: i64, // BIGSERIAL
    pub user_id: i64,    // BIGINT FK -> admin_users.user_id

    pub session_token: String,

    // TIMESTAMPTZ
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}
use chrono::{DateTime, Utc};

/// Row mapping for table `admin_users`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AdminUserRow {
    pub user_id: i64, // BIGSERIAL

    pub username: String,
    pub password_hash: String,

    pub is_active: bool,

    // TIMESTAMPTZ
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
use chrono::{DateTime, Utc};

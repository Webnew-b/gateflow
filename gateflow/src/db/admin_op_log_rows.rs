use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

/// Row mapping for table `admin_op_logs`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AdminOpLogRow {
    pub op_id: i64,   // BIGSERIAL
    pub user_id: i64, // BIGINT FK -> admin_users.user_id

    pub op_type: String,
    pub app_uuid: Option<Uuid>, // NULL -> ON DELETE SET NULL

    pub op_params: Value, // JSONB

    // TIMESTAMPTZ
    pub created_at: DateTime<Utc>,
}

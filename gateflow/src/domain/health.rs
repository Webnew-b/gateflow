#[derive(Debug, Clone)]
pub struct HealthReport {
    pub app_uuid: uuid::Uuid,
    pub name: String,
    pub checked_at: chrono::DateTime<chrono::Utc>,
    pub ok: bool,
    pub status_code: u16,
    pub latency_ms: u32,
}

#[derive(Debug, Clone)]
pub struct AppHealth {
    pub last_checked_at: chrono::DateTime<chrono::Utc>,
    pub ok: bool,
    pub status_code: u16,
    pub latency_ms: u32,
}

#[derive(Debug, Clone)]
pub struct App {
    pub app_uuid: uuid::Uuid,
    pub name: String,
    pub target_url: String,
    pub status: String,
    pub mount_path: String,
    pub upstream_path: String,
    pub app_secret: String,
}

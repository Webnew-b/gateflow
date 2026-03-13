use crate::{
    app_error::AppError,
    config::GatewayConfig,
    dataplane::auth::RateLimiters,
    db::{apps_repo::AppsRepo, pool::DbPool},
    domain::health::AppHealth,
    registry::store::AppRegistry,
    state::metrics::AppMetrics,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub config: GatewayConfig,
    pub db: DbPool,
    pub registry: Arc<RwLock<AppRegistry>>,
    pub rate_limiters: RateLimiters,
    pub http_client: reqwest::Client,
    pub health_store: Arc<RwLock<HashMap<Uuid, AppHealth>>>,
    pub metrics: AppMetrics,
}

impl AppState {
    pub async fn refresh_registry(&self) -> Result<(), AppError> {
        let repo = AppsRepo::new(self.db.clone());
        let apps = repo.fetch_all().await?;

        let mut registry = self.registry.write().await;
        registry.refresh(apps)?;

        Ok(())
    }
}

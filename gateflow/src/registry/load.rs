use crate::{
    app_error::AppError,
    db::{apps_repo::AppsRepo, pool::DbPool},
    registry::store::AppRegistry,
};

pub async fn load_registry(pool: &DbPool) -> Result<AppRegistry, AppError> {
    let repo = AppsRepo::new(pool.clone());
    let apps = repo.fetch_all().await?;

    let mut registry = AppRegistry::new();
    registry.refresh(apps)?;

    Ok(registry)
}

use gateflow::{
    admin_rpc, app_error::AppError, config::load_config, dataplane, db::pool::DbPool, health_udp,
    registry::load::load_registry, state::AppState,
};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Init logging early
    tracing_subscriber::fmt::init();

    // 1) Load env/config
    let cfg = load_config()?;
    tracing::info!("config loaded");

    // 2) Connect Postgres
    let db = DbPool::connect(
        &cfg.database_url,
        cfg.db_max_connections,
        cfg.db_connect_timeout,
    )
    .await?;
    tracing::info!("db pool ready");

    if cfg.run_migrations_on_boot {
        tracing::info!("running sqlx migrations on boot");
        sqlx::migrate!("./migrations")
            .run(db.inner())
            .await
            .map_err(|e| gateflow::app_error::DbError::Message(e.to_string()))?;
        tracing::info!("sqlx migrations finished");
    }

    // 3) Load registry from DB
    let registry = load_registry(&db).await?;
    tracing::info!("registry loaded");

    // 4) Build shared state
    let http_client = reqwest::Client::builder()
        .build()
        .map_err(anyhow::Error::from)
        .map_err(AppError::from)?;

    let health_store =
        std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

    if cfg.persist_health_to_db {
        let rows = gateflow::db::health_repo::HealthRepo::new(db.clone())
            .fetch_all_latest()
            .await?;
        let mut guard = health_store.write().await;
        for row in rows {
            guard.insert(
                row.app_uuid,
                gateflow::domain::health::AppHealth {
                    last_checked_at: row.last_checked_at,
                    ok: row.ok,
                    status_code: row.status_code.max(0) as u16,
                    latency_ms: row.latency_ms.max(0) as u32,
                },
            );
        }
        tracing::info!(count = guard.len(), "restored health state from db");
    }

    let state = AppState {
        config: cfg.clone(),
        db: db.clone(),
        registry: std::sync::Arc::new(tokio::sync::RwLock::new(registry)),
        rate_limiters: dataplane::auth::RateLimiters::default(),
        http_client,
        health_store,
        metrics: gateflow::state::metrics::AppMetrics::default(),
    };

    // 5) Start subsystems concurrently
    let http = dataplane::handler::serve(state.clone());
    let admin = admin_rpc::server::serve(state.clone());
    let health = health_udp::listener::serve(state);

    if let Err(e) = tokio::try_join!(http, admin, health) {
        tracing::error!("service crashed: {e}");
        return Err(e);
    }

    Ok(())
}

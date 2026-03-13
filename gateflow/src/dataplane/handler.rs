use axum::{
    body::Body,
    Json, Router,
    extract::{ConnectInfo, State},
    response::IntoResponse,
    routing::any,
};
use serde::Serialize;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

use super::auth::{GATEWAY_AUTH_HEADER, check_ip_policy, check_rate_limit, validate_gw_token};
use super::errors::DataplaneError;
use super::proxy::forward;
use super::route_match::match_app;
use crate::{app_error::AppError, state::AppState};
use std::net::SocketAddr;

/// Minimal HTTP server for dataplane; currently only exposes `/healthz`.
pub async fn serve(state: AppState) -> Result<(), AppError> {
    let addr = state.config.http_listen_addr;

    let app = Router::new()
        .route("/healthz", axum::routing::get(healthz))
        .route("/metrics", axum::routing::get(metrics))
        .route("/*path", any(handle))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    tracing::info!("http dataplane listening on {addr}");
    let listener = TcpListener::bind(addr).await.map_err(|e| {
        AppError::Dataplane(crate::dataplane::errors::DataplaneError::Internal(
            e.to_string(),
        ))
    })?;

    let svc = app.into_make_service_with_connect_info::<SocketAddr>();

    axum::serve(listener, svc).await.map_err(|e| {
        AppError::Dataplane(crate::dataplane::errors::DataplaneError::Internal(
            e.to_string(),
        ))
    })
}

async fn handle(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    mut req: axum::http::Request<axum::body::Body>,
) -> Result<axum::response::Response, DataplaneError> {
    state.metrics.inc_requests_total();
    let path = req.uri().path().to_string();

    // attach client ip for downstream checks
    req.extensions_mut().insert(addr);
    // 1) route
    let app = {
        let registry = state.registry.read().await;
        match match_app(req.uri().path(), &registry) {
            Ok(app) => app,
            Err(err) => {
                state.metrics.inc_requests_error_total();
                return Err(err);
            }
        }
    };

    // 2) status check
    if app.status.to_lowercase() != "active" {
        state.metrics.inc_requests_error_total();
        return Err(DataplaneError::AppDisabled);
    }

    // 3) auth / rate / ip
    if let Err(err) = validate_gw_token(&req, &state.config) {
        state.metrics.inc_requests_error_total();
        return Err(err);
    }
    // X-Authorization is gateway-only and must not be forwarded upstream.
    req.headers_mut().remove(&GATEWAY_AUTH_HEADER);
    if let Err(err) = check_rate_limit(&state.rate_limiters, &app, &state.config) {
        if matches!(err, DataplaneError::RateLimited) {
            state.metrics.inc_rate_limited_total();
        }
        state.metrics.inc_requests_error_total();
        return Err(err);
    }
    let client_ip = addr.ip();
    if let Err(err) = check_ip_policy(client_ip, &app, &state.config) {
        state.metrics.inc_requests_error_total();
        return Err(err);
    }

    // 4) proxy
    let resp = match forward(req, app.clone(), state.http_client.clone(), &state.config, &state.metrics).await {
        Ok(resp) => resp,
        Err(err) => {
            state.metrics.inc_requests_error_total();
            return Err(err);
        }
    };
    tracing::info!(
        app_uuid = %app.app_uuid,
        app_name = %app.name,
        path = %path,
        client_ip = %client_ip,
        status = %resp.status(),
        "dataplane request handled"
    );
    Ok(resp)
}

#[derive(Serialize)]
struct HealthzBody {
    status: &'static str,
}

async fn healthz() -> Json<HealthzBody> {
    Json(HealthzBody { status: "ok" })
}

async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    let snapshot = state.metrics.snapshot();
    let body = format!(
        "gateflow_requests_total {}\n\
gateflow_requests_error_total {}\n\
gateflow_rate_limited_total {}\n\
gateflow_upstream_requests_total {}\n\
gateflow_upstream_error_total {}\n\
gateflow_upstream_latency_ms_total {}\n",
        snapshot.requests_total,
        snapshot.requests_error_total,
        snapshot.rate_limited_total,
        snapshot.upstream_requests_total,
        snapshot.upstream_error_total,
        snapshot.upstream_latency_ms_total
    );
    (
        [(axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        Body::from(body),
    )
}

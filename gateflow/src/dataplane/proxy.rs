use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, HeaderName, Method, Request, Response};
use reqwest::Client;
use std::collections::HashSet;
use std::time::Instant;

use crate::{app_error::DataplaneError, config::GatewayConfig, db::AppRow};
use crate::state::metrics::AppMetrics;

use super::{auth::GATEWAY_AUTH_HEADER, rewrite, signing};

pub async fn forward(
    req: Request<Body>,
    app: AppRow,
    client: Client,
    cfg: &GatewayConfig,
    metrics: &AppMetrics,
) -> Result<Response<Body>, DataplaneError> {
    let (parts, body) = req.into_parts();
    let method = parts.method.clone();
    let uri = parts.uri.clone();
    let body_bytes = to_bytes(body, 8 * 1024 * 1024)
        .await
        .map_err(|e| DataplaneError::Internal(format!("read body failed: {e}")))?;

    let target_path = rewrite::rewrite_path(uri.path(), &app.mount_path, &app.upstream_path);
    let full_path = match uri.query() {
        Some(q) => format!("{target_path}?{q}"),
        None => target_path.clone(),
    };

    let upstream_url = format!("{}{}", app.target_url.trim_end_matches('/'), full_path);
    tracing::debug!(
        app_uuid = %app.app_uuid,
        app_name = %app.name,
        upstream_url = %upstream_url,
        "forwarding request to upstream"
    );

    let builder = client
        .request(method_reqwest(&method), upstream_url)
        .timeout(cfg.proxy_timeout)
        .body(body_bytes.clone());

    let mut builder = copy_headers(&parts.headers, builder)?;

    let (x_app_id, x_ts, x_sig, x_nonce, x_ttl) = signing::sign_request(
        &app.app_uuid,
        &app.app_secret,
        &cfg.hmac_global_salt,
        cfg.signature_replay_window_secs,
        method.as_str(),
        &full_path,
        &body_bytes,
    )?;

    builder = builder
        .header("X-Gw-App-Id", x_app_id)
        .header("X-Gw-Ts", x_ts)
        .header("X-Gw-Sig", x_sig)
        .header("X-Gw-Nonce", x_nonce)
        .header("X-Gw-Ttl", x_ttl);

    let started = Instant::now();
    metrics.inc_upstream_requests_total();
    let upstream_resp = builder
        .send()
        .await
        .map_err(|e| {
            metrics.inc_upstream_error_total();
            DataplaneError::Proxy(e.to_string())
        })?;
    let latency_ms = started.elapsed().as_millis() as u64;
    metrics.add_upstream_latency_ms(latency_ms);
    tracing::info!(
        app_uuid = %app.app_uuid,
        app_name = %app.name,
        upstream_status = %upstream_resp.status(),
        upstream_latency_ms = latency_ms,
        "upstream response received"
    );

    let status = upstream_resp.status();
    let mut resp_builder = Response::builder().status(status);
    for (k, v) in upstream_resp.headers().iter() {
        resp_builder = resp_builder.header(k, v);
    }
    let bytes = upstream_resp
        .bytes()
        .await
        .map_err(|e| DataplaneError::Proxy(e.to_string()))?;
    let resp = resp_builder
        .body(Body::from(bytes))
        .map_err(|e| DataplaneError::Internal(e.to_string()))?;
    Ok(resp)
}

fn method_reqwest(method: &Method) -> reqwest::Method {
    match *method {
        Method::GET => reqwest::Method::GET,
        Method::POST => reqwest::Method::POST,
        Method::PUT => reqwest::Method::PUT,
        Method::DELETE => reqwest::Method::DELETE,
        Method::PATCH => reqwest::Method::PATCH,
        Method::HEAD => reqwest::Method::HEAD,
        Method::OPTIONS => reqwest::Method::OPTIONS,
        _ => {
            reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap_or(reqwest::Method::GET)
        }
    }
}

fn copy_headers(
    src: &HeaderMap,
    mut builder: reqwest::RequestBuilder,
) -> Result<reqwest::RequestBuilder, DataplaneError> {
    let connection_headers = connection_managed_headers(src);
    for (name, value) in src.iter() {
        // Drop hop-by-hop and gateway-owned headers from client input.
        if should_drop_header(name, &connection_headers) {
            continue;
        }
        builder = builder.header(name, value);
    }
    Ok(builder)
}

fn should_drop_header(name: &HeaderName, connection_headers: &HashSet<HeaderName>) -> bool {
    let keep_alive = HeaderName::from_static("keep-alive");
    let proxy_connection = HeaderName::from_static("proxy-connection");
    name == axum::http::header::HOST
        || name == axum::http::header::CONTENT_LENGTH
        || name == axum::http::header::CONNECTION
        || name == &keep_alive
        || name == axum::http::header::TRANSFER_ENCODING
        || name == axum::http::header::TE
        || name == axum::http::header::TRAILER
        || name == axum::http::header::UPGRADE
        || name == &proxy_connection
        || name == &GATEWAY_AUTH_HEADER
        || name.as_str().starts_with("x-gw-")
        || connection_headers.contains(name)
}

fn connection_managed_headers(src: &HeaderMap) -> HashSet<HeaderName> {
    let mut out = HashSet::new();
    for value in src.get_all(axum::http::header::CONNECTION).iter() {
        let Ok(raw) = value.to_str() else {
            continue;
        };
        for token in raw.split(',').map(str::trim).filter(|v| !v.is_empty()) {
            if let Ok(name) = HeaderName::from_bytes(token.as_bytes()) {
                out.insert(name);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue, header};

    #[test]
    fn copy_headers_strips_hop_by_hop_and_gateway_headers() {
        let mut src = HeaderMap::new();
        src.insert(header::HOST, HeaderValue::from_static("client.example"));
        src.insert(header::CONTENT_LENGTH, HeaderValue::from_static("123"));
        src.insert(header::CONNECTION, HeaderValue::from_static("keep-alive, x-temp-hop"));
        src.insert("keep-alive", HeaderValue::from_static("timeout=5"));
        src.insert(header::TRANSFER_ENCODING, HeaderValue::from_static("chunked"));
        src.insert(header::UPGRADE, HeaderValue::from_static("h2c"));
        src.insert("proxy-connection", HeaderValue::from_static("keep-alive"));
        src.insert("x-authorization", HeaderValue::from_static("Bearer gw-token"));
        src.insert("x-gw-ts", HeaderValue::from_static("123"));
        src.insert("x-temp-hop", HeaderValue::from_static("to-be-removed"));
        src.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer svc-token"));
        src.insert(header::USER_AGENT, HeaderValue::from_static("gateflow-test"));

        let client = reqwest::Client::new();
        let builder = client.get("http://example.com");
        let req = copy_headers(&src, builder).unwrap().build().unwrap();
        let headers = req.headers();

        assert!(!headers.contains_key(header::HOST));
        assert!(!headers.contains_key(header::CONTENT_LENGTH));
        assert!(!headers.contains_key(header::CONNECTION));
        assert!(!headers.contains_key("keep-alive"));
        assert!(!headers.contains_key(header::TRANSFER_ENCODING));
        assert!(!headers.contains_key(header::UPGRADE));
        assert!(!headers.contains_key("proxy-connection"));
        assert!(!headers.contains_key("x-authorization"));
        assert!(!headers.contains_key("x-gw-ts"));
        assert!(!headers.contains_key("x-temp-hop"));
        assert!(headers.contains_key(header::AUTHORIZATION));
        assert!(headers.contains_key(header::USER_AGENT));
    }
}

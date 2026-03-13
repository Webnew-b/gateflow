#!/usr/bin/env bash
set -euo pipefail

ROOT="gateflow-gateway"

mkdir -p "$ROOT"/{proto,db/migrations,src}
mkdir -p "$ROOT"/src/{config,db,domain,registry,dataplane,admin_rpc,health_udp,state}

# ---------- top-level ----------
cat > "$ROOT/Cargo.toml" <<'TOML'
[package]
name = "gateflow-gateway"
version = "0.1.0"
edition = "2021"

[dependencies]
# 先留空：你后面按选型补（axum/hyper/reqwest/tonic/sqlx等）

TOML

cat > "$ROOT/build.rs" <<'RS'
fn main() {
    // v0：先不做 proto 生成也能编译（你后续接 tonic-build 再补）
    println!("cargo:rerun-if-changed=proto/admin.proto");
}
RS

touch "$ROOT/proto/admin.proto"
touch "$ROOT/db/migrations/0001_init.sql"
touch "$ROOT/db/migrations/0002_admin_op_logs.sql"

# ---------- src/main.rs ----------
cat > "$ROOT/src/main.rs" <<'RS'
fn main() {
    // v0 skeleton
    println!("gateflow-gateway skeleton");
}
RS

# ---------- module roots (NO mod.rs pattern) ----------
cat > "$ROOT/src/config.rs" <<'RS'
pub mod load;
pub mod model;

pub use model::*;
RS

cat > "$ROOT/src/db.rs" <<'RS'
pub mod pool;
pub mod apps_repo;
pub mod admin_repo;
RS

cat > "$ROOT/src/domain.rs" <<'RS'
pub mod app;
pub mod health;

pub use app::*;
pub use health::*;
RS

cat > "$ROOT/src/registry.rs" <<'RS'
pub mod load;
pub mod store;
RS

cat > "$ROOT/src/dataplane.rs" <<'RS'
pub mod handler;
pub mod route_match;
pub mod auth;
pub mod rewrite;
pub mod signing;
pub mod proxy;
pub mod errors;
RS

cat > "$ROOT/src/admin_rpc.rs" <<'RS'
pub mod server;
pub mod auth;
pub mod apps;
pub mod types;
RS

cat > "$ROOT/src/health_udp.rs" <<'RS'
pub mod listener;
pub mod parse;
pub mod store;
RS

cat > "$ROOT/src/state.rs" <<'RS'
pub mod app_state;

pub use app_state::*;
RS

# ---------- leaf files (empty stubs) ----------
# config/*
cat > "$ROOT/src/config/model.rs" <<'RS'
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    // v0：按你的 env/config 字段补齐
}
RS

cat > "$ROOT/src/config/load.rs" <<'RS'
use crate::config::GatewayConfig;

pub fn load_config() -> GatewayConfig {
    // v0 skeleton
    GatewayConfig {}
}
RS

# db/*
cat > "$ROOT/src/db/pool.rs" <<'RS'
#[derive(Clone)]
pub struct DbPool;
// v0：这里后续放 sqlx::PgPool 或 deadpool-postgres 等
RS

cat > "$ROOT/src/db/apps_repo.rs" <<'RS'
use crate::db::pool::DbPool;

pub struct AppsRepo {
    pub pool: DbPool,
}
RS

cat > "$ROOT/src/db/admin_repo.rs" <<'RS'
use crate::db::pool::DbPool;

pub struct AdminRepo {
    pub pool: DbPool,
}
RS

# domain/*
cat > "$ROOT/src/domain/app.rs" <<'RS'
#[derive(Debug, Clone)]
pub struct App {
    // v0：app_uuid/name/target_url/status/route/app_secret...
}
RS

cat > "$ROOT/src/domain/health.rs" <<'RS'
#[derive(Debug, Clone)]
pub struct HealthReport {
    // v0：app_uuid/name/checked_at/ok/status_code/latency_ms
}

#[derive(Debug, Clone)]
pub struct AppHealth {
    // v0：last_checked_at/ok/status_code/latency_ms
}
RS

# registry/*
cat > "$ROOT/src/registry/store.rs" <<'RS'
pub struct AppRegistry {
    // v0：apps_by_id/apps_by_name/paths 索引
}
RS

cat > "$ROOT/src/registry/load.rs" <<'RS'
use crate::db::pool::DbPool;
use crate::registry::store::AppRegistry;

pub fn load_registry(_pool: &DbPool) -> AppRegistry {
    // v0 skeleton
    AppRegistry {}
}
RS

# dataplane/*
touch "$ROOT/src/dataplane/handler.rs"
touch "$ROOT/src/dataplane/route_match.rs"
touch "$ROOT/src/dataplane/auth.rs"
touch "$ROOT/src/dataplane/rewrite.rs"
touch "$ROOT/src/dataplane/signing.rs"
touch "$ROOT/src/dataplane/proxy.rs"
touch "$ROOT/src/dataplane/errors.rs"

# admin_rpc/*
touch "$ROOT/src/admin_rpc/server.rs"
touch "$ROOT/src/admin_rpc/auth.rs"
touch "$ROOT/src/admin_rpc/apps.rs"
touch "$ROOT/src/admin_rpc/types.rs"

# health_udp/*
touch "$ROOT/src/health_udp/listener.rs"
touch "$ROOT/src/health_udp/parse.rs"
touch "$ROOT/src/health_udp/store.rs"

# state/*
cat > "$ROOT/src/state/app_state.rs" <<'RS'
use crate::{config::GatewayConfig, db::pool::DbPool, registry::store::AppRegistry};

#[derive(Clone)]
pub struct AppState {
    pub config: GatewayConfig,
    pub db: DbPool,
    pub registry: AppRegistry,
    // v0：health_store 之后补（HashMap<AppUuid, AppHealth>）
}
RS

echo "✅ Created $ROOT (module-root file pattern, no mod.rs)"


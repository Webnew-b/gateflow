use tonic::{Request, Response, Status};
use uuid::Uuid;
use std::collections::HashMap;
use std::time::Duration;

use crate::{
    app_error::{AdminRpcError, DbError},
    admin_rpc::types::*,
    admin_rpc::{auth, types::proto::gateflow_service_server::GateflowService},
    db::{
        admin_op_log_rows::AdminOpLogRow, admin_repo::AdminRepo, app_rows::AppRow,
        apps_repo::AppsRepo,
    },
    domain::health::AppHealth,
    state::AppState,
};

#[derive(Clone)]
pub struct AdminService {
    pub state: AppState,
}

#[tonic::async_trait]
impl GateflowService for AdminService {
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        enforce_loopback_peer(&request)?;
        let req = request.into_inner();
        let (token, expire_at) = auth::login(&self.state, &req.username, &req.password)
            .await
            .map_err(admin_rpc_status)?;
        let resp = LoginResponse {
            msg: "ok".into(),
            session_token: token,
            expire_at,
        };
        Ok(Response::new(resp))
    }

    async fn add_app(
        &self,
        request: Request<AddAppRequest>,
    ) -> Result<Response<AddAppResponse>, Status> {
        enforce_loopback_peer(&request)?;
        let session = require_session(&self.state, &request).await?;
        let req = request.into_inner();
        let repo = AppsRepo::new(self.state.db.clone());
        validate_new_app_request(&req)?;

        if repo
            .find_by_name(&req.app_name)
            .await
            .map_err(internal_status)?
            .is_some()
        {
            return Err(Status::already_exists("app name already exists"));
        }

        let now = chrono::Utc::now();
        let app = AppRow {
            app_uuid: Uuid::new_v4(),
            name: req.app_name.clone(),
            target_url: req.target_url.clone(),
            status: "Registered".into(),
            mount_path: req.mount_path.clone(),
            upstream_path: req.upstream_path.clone(),
            app_secret: req
                .secret
                .clone()
                .unwrap_or_else(|| Uuid::new_v4().simple().to_string()),
            rate_limit_rps: None,
            allowed_source_ips: Vec::new(),
            blocked_source_ips: Vec::new(),
            created_at: now,
            updated_at: now,
        };

        repo.insert(&app)
            .await
            .map_err(map_db_write_error)
            .map_err(admin_rpc_status)?;
        self.log_admin_op(
            session.user_id,
            "add_app",
            Some(app.app_uuid),
            serde_json::json!({
                "app_name": app.name,
                "target_url": app.target_url,
                "mount_path": app.mount_path,
                "upstream_path": app.upstream_path,
                "status": app.status,
            }),
        )
        .await?;
        self.state
            .refresh_registry()
            .await
            .map_err(internal_status)?;

        let resp = AddAppResponse {
            app_name: app.name,
            target_url: app.target_url,
            secret: app.app_secret,
        };
        Ok(Response::new(resp))
    }

    async fn approve_app(
        &self,
        request: Request<ApproveAppRequest>,
    ) -> Result<Response<ApproveAppResponse>, Status> {
        enforce_loopback_peer(&request)?;
        let session = require_session(&self.state, &request).await?;
        let req = request.into_inner();
        let repo = AppsRepo::new(self.state.db.clone());
        let app = find_app_by_identify(&repo, &req.app_identify, &req.identify_type).await?;
        ensure_status_transition(&app.status, "Registered", "Active")?;

        repo.update_status(&app.name, "Active")
            .await
            .map_err(internal_status)?;
        self.log_admin_op(
            session.user_id,
            "approve_app",
            Some(app.app_uuid),
            serde_json::json!({ "identify": req.app_identify, "identify_type": req.identify_type }),
        )
        .await?;
        self.state
            .refresh_registry()
            .await
            .map_err(internal_status)?;

        let resp = ApproveAppResponse {
            app_id: app.app_uuid.to_string(),
            app_name: app.name,
            status: "Active".into(),
        };
        Ok(Response::new(resp))
    }

    async fn disable_app(
        &self,
        request: Request<DisableAppRequest>,
    ) -> Result<Response<DisableAppResponse>, Status> {
        enforce_loopback_peer(&request)?;
        let session = require_session(&self.state, &request).await?;
        let req = request.into_inner();
        let repo = AppsRepo::new(self.state.db.clone());
        let app = find_app_by_identify(&repo, &req.app_identify, &req.identify_type).await?;
        ensure_status_transition(&app.status, "Active", "Disabled")?;

        repo.update_status(&app.name, "Disabled")
            .await
            .map_err(internal_status)?;
        self.log_admin_op(
            session.user_id,
            "disable_app",
            Some(app.app_uuid),
            serde_json::json!({ "identify": req.app_identify, "identify_type": req.identify_type }),
        )
        .await?;
        self.state
            .refresh_registry()
            .await
            .map_err(internal_status)?;

        let resp = DisableAppResponse {
            app_id: app.app_uuid.to_string(),
            app_name: app.name,
            status: "Disabled".into(),
        };
        Ok(Response::new(resp))
    }

    async fn route_update(
        &self,
        request: Request<RouteUpdateRequest>,
    ) -> Result<Response<RouteUpdateResponse>, Status> {
        enforce_loopback_peer(&request)?;
        let session = require_session(&self.state, &request).await?;
        let req = request.into_inner();
        let repo = AppsRepo::new(self.state.db.clone());
        let existing = repo
            .find_by_name(&req.app_name)
            .await
            .map_err(internal_status)?
            .ok_or_else(|| Status::not_found("app not found"))?;
        validate_route_paths(&req.mount_path, &req.upstream_path)?;

        repo.update_route(&req.app_name, &req.mount_path, &req.upstream_path)
            .await
            .map_err(map_db_write_error)
            .map_err(admin_rpc_status)?;
        self.log_admin_op(
            session.user_id,
            "route_update",
            Some(existing.app_uuid),
            serde_json::json!({
                "app_name": req.app_name,
                "mount_path": req.mount_path,
                "upstream_path": req.upstream_path,
            }),
        )
        .await?;
        self.state
            .refresh_registry()
            .await
            .map_err(internal_status)?;

        let resp = RouteUpdateResponse {
            app_id: existing.app_uuid.to_string(),
            app_name: req.app_name,
            status: "Updated".into(),
            mount_path: req.mount_path,
            upstream_path: req.upstream_path,
        };
        Ok(Response::new(resp))
    }

    async fn list(&self, request: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        enforce_loopback_peer(&request)?;
        let _session = require_session(&self.state, &request).await?;
        let _ = request.into_inner();
        let repo = AppsRepo::new(self.state.db.clone());
        let list = repo
            .fetch_all()
            .await
            .map_err(internal_status)?
            .into_iter()
            .map(app_row_to_proto)
            .collect();
        let resp = ListResponse { list };
        Ok(Response::new(resp))
    }

    async fn show(&self, request: Request<ShowRequest>) -> Result<Response<ShowResponse>, Status> {
        enforce_loopback_peer(&request)?;
        let _session = require_session(&self.state, &request).await?;
        let req = request.into_inner();
        let repo = AppsRepo::new(self.state.db.clone());
        let app = find_app_by_identify(&repo, &req.app_identify, &req.identify_type).await?;
        let resp = ShowResponse {
            app: Some(app_row_to_proto(app)),
        };
        Ok(Response::new(resp))
    }

    async fn node_list(
        &self,
        request: Request<NodeRequest>,
    ) -> Result<Response<NodeResponse>, Status> {
        enforce_loopback_peer(&request)?;
        let _session = require_session(&self.state, &request).await?;
        let _ = request.into_inner();

        let apps = AppsRepo::new(self.state.db.clone())
            .fetch_all()
            .await
            .map_err(internal_status)?;
        let health = self.state.health_store.read().await;
        let list = build_node_list_entries(
            &apps,
            &health,
            Duration::from_secs(self.state.config.health_status_ttl_secs),
        );

        Ok(Response::new(NodeResponse { list }))
    }
}

impl From<AppState> for AdminService {
    fn from(state: AppState) -> Self {
        Self { state }
    }
}

impl AdminService {
    async fn log_admin_op(
        &self,
        user_id: i64,
        op_type: &str,
        app_uuid: Option<Uuid>,
        op_params: serde_json::Value,
    ) -> Result<(), Status> {
        let repo = AdminRepo::new(self.state.db.clone());
        let log = AdminOpLogRow {
            op_id: 0,
            user_id,
            op_type: op_type.to_string(),
            app_uuid,
            op_params,
            created_at: chrono::Utc::now(),
        };
        repo.insert_op_log(&log).await.map_err(internal_status)
    }
}

async fn require_session<T>(
    state: &AppState,
    request: &Request<T>,
) -> Result<crate::db::CliSessionRow, Status> {
    let token = auth::extract_bearer_token(request)?;
    auth::validate_session(state, &token).await
}

async fn find_app_by_identify(
    repo: &AppsRepo,
    identify: &str,
    identify_type: &str,
) -> Result<AppRow, Status> {
    match normalize_identify_type(identify_type) {
        "id" => {
            let app_uuid = Uuid::parse_str(identify)
                .map_err(|_| Status::invalid_argument("invalid app uuid"))?;
            repo.find_by_uuid(app_uuid)
                .await
                .map_err(internal_status)?
                .ok_or_else(|| Status::not_found("app not found"))
        }
        _ => repo
            .find_by_name(identify)
            .await
            .map_err(internal_status)?
            .ok_or_else(|| Status::not_found("app not found")),
    }
}

fn normalize_identify_type(value: &str) -> &str {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "id" | "uuid" | "app_id" => "id",
        _ => "name",
    }
}

fn app_row_to_proto(app: AppRow) -> AppList {
    AppList {
        app_id: app.app_uuid.to_string(),
        app_name: app.name,
        target_url: app.target_url,
        mount_path: app.mount_path,
        upstream_path: app.upstream_path,
        status: app.status,
    }
}

fn build_node_list_entries(
    apps: &[AppRow],
    health: &HashMap<Uuid, AppHealth>,
    health_ttl: Duration,
) -> Vec<NodeList> {
    apps.iter()
        .map(|app| NodeList {
            app_id: app.app_uuid.to_string(),
            app_name: app.name.clone(),
            health_url: format!("{}{}", app.target_url, app.upstream_path),
            status: health
                .get(&app.app_uuid)
                .map(|h| {
                    if crate::health_udp::store::is_stale(h, health_ttl) {
                        "unknown"
                    } else if h.ok {
                        "ok"
                    } else {
                        "fail"
                    }
                })
                .unwrap_or("unknown")
                .into(),
            // healthd compares this as expected HTTP status code.
            expect_status: "200".into(),
        })
        .collect()
}

fn internal_status(err: impl std::fmt::Display) -> Status {
    Status::internal(err.to_string())
}

fn enforce_loopback_peer<T>(request: &Request<T>) -> Result<(), Status> {
    match request.remote_addr() {
        Some(peer) if peer.ip().is_loopback() => Ok(()),
        Some(_) => Err(Status::permission_denied(
            "admin gRPC only accepts loopback clients",
        )),
        None => Err(Status::permission_denied(
            "admin gRPC peer address is unavailable",
        )),
    }
}

fn admin_rpc_status(err: AdminRpcError) -> Status {
    match err {
        AdminRpcError::InvalidCredential | AdminRpcError::UserDisabled => {
            Status::unauthenticated(err.to_string())
        }
        AdminRpcError::AppAlreadyExists => Status::already_exists(err.to_string()),
        AdminRpcError::Db(_) | AdminRpcError::Message(_) => Status::internal(err.to_string()),
    }
}

fn map_db_write_error(err: DbError) -> AdminRpcError {
    if is_unique_violation(&err) {
        AdminRpcError::AppAlreadyExists
    } else {
        AdminRpcError::Db(err)
    }
}

fn is_unique_violation(err: &DbError) -> bool {
    match err {
        DbError::Sqlx(sqlx::Error::Database(db_err)) => db_err.code().as_deref() == Some("23505"),
        _ => false,
    }
}

fn validate_new_app_request(req: &AddAppRequest) -> Result<(), Status> {
    if req.app_name.trim().is_empty() {
        return Err(Status::invalid_argument("app_name must not be empty"));
    }

    let url = reqwest::Url::parse(&req.target_url)
        .map_err(|_| Status::invalid_argument("target_url must be a valid absolute URL"))?;
    match url.scheme() {
        "http" | "https" => {}
        _ => {
            return Err(Status::invalid_argument(
                "target_url must use http or https",
            ));
        }
    }

    validate_route_paths(&req.mount_path, &req.upstream_path)?;
    Ok(())
}

fn validate_route_paths(mount_path: &str, upstream_path: &str) -> Result<(), Status> {
    validate_route_path("mount_path", mount_path, false)?;
    validate_route_path("upstream_path", upstream_path, true)?;
    Ok(())
}

fn validate_route_path(field: &str, value: &str, allow_root: bool) -> Result<(), Status> {
    if value.is_empty() {
        return Err(Status::invalid_argument(format!(
            "{field} must not be empty"
        )));
    }
    if !value.starts_with('/') {
        return Err(Status::invalid_argument(format!(
            "{field} must start with '/'"
        )));
    }
    if !allow_root && value == "/" {
        return Err(Status::invalid_argument(format!("{field} must not be '/'")));
    }
    if value.len() > 1 && value.ends_with('/') {
        return Err(Status::invalid_argument(format!(
            "{field} must not have a trailing '/'"
        )));
    }
    if value.contains("//") {
        return Err(Status::invalid_argument(format!(
            "{field} must not contain '//'"
        )));
    }
    Ok(())
}

fn ensure_status_transition(current: &str, expected: &str, next: &str) -> Result<(), Status> {
    if current == expected {
        Ok(())
    } else {
        Err(Status::failed_precondition(format!(
            "cannot change app status from {current} to {next}; expected {expected}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn add_app_request() -> AddAppRequest {
        AddAppRequest {
            app_name: "demo".into(),
            target_url: "http://demo.internal".into(),
            mount_path: "/demo".into(),
            upstream_path: "/api".into(),
            secret: None,
        }
    }

    #[test]
    fn validate_new_app_request_accepts_valid_fields() {
        assert!(validate_new_app_request(&add_app_request()).is_ok());
    }

    #[test]
    fn validate_new_app_request_rejects_bad_target_url() {
        let mut req = add_app_request();
        req.target_url = "demo.internal".into();

        let err = validate_new_app_request(&req).unwrap_err();

        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }

    #[test]
    fn validate_route_paths_rejects_root_mount() {
        let err = validate_route_paths("/", "/api").unwrap_err();

        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }

    #[test]
    fn ensure_status_transition_rejects_wrong_current_status() {
        let err = ensure_status_transition("Disabled", "Registered", "Active").unwrap_err();

        assert_eq!(err.code(), tonic::Code::FailedPrecondition);
    }

    #[test]
    fn build_node_list_entries_uses_health_store_status() {
        let app_uuid = Uuid::new_v4();
        let app = AppRow {
            app_uuid,
            name: "demo".into(),
            target_url: "http://demo.internal".into(),
            status: "Active".into(),
            mount_path: "/demo".into(),
            upstream_path: "/api".into(),
            app_secret: "secret".into(),
            rate_limit_rps: None,
            allowed_source_ips: Vec::new(),
            blocked_source_ips: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let mut health = HashMap::new();
        health.insert(
            app_uuid,
            AppHealth {
                last_checked_at: Utc::now(),
                ok: false,
                status_code: 500,
                latency_ms: 120,
            },
        );

        let result = build_node_list_entries(&[app], &health, Duration::from_secs(120));

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, "fail");
    }

    #[test]
    fn build_node_list_entries_marks_stale_as_unknown() {
        let app_uuid = Uuid::new_v4();
        let app = AppRow {
            app_uuid,
            name: "demo".into(),
            target_url: "http://demo.internal".into(),
            status: "Active".into(),
            mount_path: "/demo".into(),
            upstream_path: "/api".into(),
            app_secret: "secret".into(),
            rate_limit_rps: None,
            allowed_source_ips: Vec::new(),
            blocked_source_ips: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let mut health = HashMap::new();
        health.insert(
            app_uuid,
            AppHealth {
                last_checked_at: Utc::now() - chrono::Duration::seconds(600),
                ok: true,
                status_code: 200,
                latency_ms: 1,
            },
        );

        let result = build_node_list_entries(&[app], &health, Duration::from_secs(120));

        assert_eq!(result[0].status, "unknown");
    }

    #[test]
    fn admin_rpc_status_maps_typed_errors() {
        assert_eq!(
            admin_rpc_status(AdminRpcError::InvalidCredential).code(),
            tonic::Code::Unauthenticated
        );
        assert_eq!(
            admin_rpc_status(AdminRpcError::UserDisabled).code(),
            tonic::Code::Unauthenticated
        );
        assert_eq!(
            admin_rpc_status(AdminRpcError::AppAlreadyExists).code(),
            tonic::Code::AlreadyExists
        );
        assert_eq!(
            admin_rpc_status(AdminRpcError::Message("boom".into())).code(),
            tonic::Code::Internal
        );
    }
}

use crate::admin_rpc::apps::AdminService;
use crate::admin_rpc::types::proto::gateflow_service_server::GateflowServiceServer;
use crate::{
    app_error::{AdminRpcError, AppError},
    state::AppState,
};
use tonic::transport::Server;

pub async fn serve(state: AppState) -> Result<(), AppError> {
    let addr = state.config.admin_grpc_listen_addr;
    let svc = GateflowServiceServer::new(AdminService::from(state));

    tracing::info!("admin gRPC listening on {addr}");
    Server::builder()
        .add_service(svc)
        .serve(addr)
        .await
        .map_err(|e| AppError::AdminRpc(AdminRpcError::Message(e.to_string())))
}

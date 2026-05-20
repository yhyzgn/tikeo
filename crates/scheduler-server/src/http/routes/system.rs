use axum::Json;

use crate::http::dto::{
    ApiResponse, ClusterApiResponse, ClusterResponse, SystemInfoApiResponse, SystemInfoResponse,
};

/// Return scheduler server build and API metadata.
#[utoipa::path(
    get,
    path = "/api/v1/system/info",
    tag = "system",
    responses((status = 200, description = "System info", body = SystemInfoApiResponse))
)]
pub async fn system_info() -> Json<SystemInfoApiResponse> {
    Json(ApiResponse::success(SystemInfoResponse {
        name: "scheduler",
        version: env!("CARGO_PKG_VERSION"),
        target: std::env::consts::OS,
    }))
}

/// Return the current cluster status placeholder.
#[utoipa::path(
    get,
    path = "/api/v1/cluster",
    tag = "system",
    responses((status = 200, description = "Cluster status", body = ClusterApiResponse))
)]
pub async fn cluster_status() -> Json<ClusterApiResponse> {
    Json(ApiResponse::success(ClusterResponse {
        mode: "standalone",
        role: "leader",
        nodes: 1,
    }))
}

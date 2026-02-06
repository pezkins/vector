//! Git remote sync API endpoints
//!
//! Provides endpoints for:
//! - Remote repository configuration
//! - Push/pull synchronization
//! - Branch management

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, error};

use crate::AppState;

// =============================================================================
// Request/Response Types
// =============================================================================

/// Request to configure a remote
#[derive(Debug, Deserialize)]
pub struct ConfigureRemoteRequest {
    pub name: String,
    pub url: String,
}

/// Request to push/pull/sync
#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    pub branch: Option<String>,
}

/// Request to create a branch
#[derive(Debug, Deserialize)]
pub struct CreateBranchRequest {
    pub name: String,
}

/// Response for remote operations
#[derive(Debug, Serialize)]
pub struct RemoteResponse {
    pub success: bool,
    pub message: String,
}

// =============================================================================
// API Endpoints
// =============================================================================

/// List all configured remotes
pub async fn list_remotes(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.git_store.list_remotes() {
        Ok(remotes) => {
            (StatusCode::OK, Json(serde_json::json!({
                "remotes": remotes
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to list remotes: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": e.to_string()
            }))).into_response()
        }
    }
}

/// Configure a remote
pub async fn configure_remote(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ConfigureRemoteRequest>,
) -> impl IntoResponse {
    match state.git_store.configure_remote(&request.name, &request.url) {
        Ok(_) => {
            info!("Configured remote '{}' -> {}", request.name, request.url);
            (StatusCode::OK, Json(RemoteResponse {
                success: true,
                message: format!("Remote '{}' configured", request.name),
            })).into_response()
        }
        Err(e) => {
            error!("Failed to configure remote: {}", e);
            (StatusCode::BAD_REQUEST, Json(RemoteResponse {
                success: false,
                message: e.to_string(),
            })).into_response()
        }
    }
}

/// Remove a remote
pub async fn delete_remote(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.git_store.remove_remote(&name) {
        Ok(_) => {
            info!("Removed remote '{}'", name);
            (StatusCode::OK, Json(RemoteResponse {
                success: true,
                message: format!("Remote '{}' removed", name),
            })).into_response()
        }
        Err(e) => {
            error!("Failed to remove remote: {}", e);
            (StatusCode::BAD_REQUEST, Json(RemoteResponse {
                success: false,
                message: e.to_string(),
            })).into_response()
        }
    }
}

/// Push to a remote
pub async fn push_to_remote(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(request): Json<SyncRequest>,
) -> impl IntoResponse {
    match state.git_store.push(&name, request.branch.as_deref()) {
        Ok(result) => {
            info!("Pushed to remote '{}' ({})", name, result.branch);
            (StatusCode::OK, Json(result)).into_response()
        }
        Err(e) => {
            error!("Failed to push: {}", e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            }))).into_response()
        }
    }
}

/// Pull from a remote
pub async fn pull_from_remote(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(request): Json<SyncRequest>,
) -> impl IntoResponse {
    match state.git_store.pull(&name, request.branch.as_deref()) {
        Ok(result) => {
            info!("Pulled from remote '{}' ({})", name, result.branch);
            (StatusCode::OK, Json(result)).into_response()
        }
        Err(e) => {
            error!("Failed to pull: {}", e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            }))).into_response()
        }
    }
}

/// Sync with a remote (pull then push)
pub async fn sync_with_remote(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(request): Json<SyncRequest>,
) -> impl IntoResponse {
    match state.git_store.sync(&name, request.branch.as_deref()) {
        Ok(result) => {
            info!("Synced with remote '{}'", name);
            (StatusCode::OK, Json(result)).into_response()
        }
        Err(e) => {
            error!("Failed to sync: {}", e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            }))).into_response()
        }
    }
}

/// Get sync status with a remote
pub async fn get_sync_status(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.git_store.sync_status(&name, None) {
        Ok(status) => {
            (StatusCode::OK, Json(status)).into_response()
        }
        Err(e) => {
            error!("Failed to get sync status: {}", e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": e.to_string()
            }))).into_response()
        }
    }
}

/// List branches
pub async fn list_branches(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.git_store.list_branches() {
        Ok(branches) => {
            let current = state.git_store.current_branch().unwrap_or_default();
            (StatusCode::OK, Json(serde_json::json!({
                "current": current,
                "branches": branches
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to list branches: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": e.to_string()
            }))).into_response()
        }
    }
}

/// Create a new branch
pub async fn create_branch(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateBranchRequest>,
) -> impl IntoResponse {
    match state.git_store.create_branch(&request.name) {
        Ok(_) => {
            info!("Created branch '{}'", request.name);
            (StatusCode::CREATED, Json(RemoteResponse {
                success: true,
                message: format!("Branch '{}' created", request.name),
            })).into_response()
        }
        Err(e) => {
            error!("Failed to create branch: {}", e);
            (StatusCode::BAD_REQUEST, Json(RemoteResponse {
                success: false,
                message: e.to_string(),
            })).into_response()
        }
    }
}

/// Checkout a branch
pub async fn checkout_branch(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.git_store.checkout_branch(&name) {
        Ok(_) => {
            info!("Checked out branch '{}'", name);
            (StatusCode::OK, Json(RemoteResponse {
                success: true,
                message: format!("Checked out branch '{}'", name),
            })).into_response()
        }
        Err(e) => {
            error!("Failed to checkout branch: {}", e);
            (StatusCode::BAD_REQUEST, Json(RemoteResponse {
                success: false,
                message: e.to_string(),
            })).into_response()
        }
    }
}

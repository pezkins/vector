//! Deployment management API endpoints
//!
//! Provides endpoints for:
//! - Creating and managing deployments
//! - Deployment strategies (rolling, canary)
//! - Approval workflows
//! - Deployment history and status

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, error};

use crate::AppState;
use crate::db::repository::{DeploymentRepository, WorkerGroupRepository};
use crate::deployment::{
    DeploymentExecutor, DeploymentOptions, RollingOptions, CanaryOptions,
    check_version_consistency,
};

/// Request to create a new deployment
#[derive(Debug, Deserialize)]
pub struct CreateDeploymentRequest {
    /// Config version to deploy (defaults to current)
    pub config_version: Option<String>,
    /// Override deployment strategy for this deployment
    pub strategy: Option<String>,
    /// Rolling deployment options
    pub rolling_options: Option<RollingOptions>,
    /// Canary deployment options
    pub canary_options: Option<CanaryOptions>,
    /// Force deployment even if version mismatch
    #[serde(default)]
    pub force: bool,
}

/// Response for deployment creation
#[derive(Debug, Serialize)]
pub struct CreateDeploymentResponse {
    pub deployment_id: String,
    pub status: String,
    pub message: String,
    pub requires_approval: bool,
    pub queued: bool,
}

/// Response for deployment status
#[derive(Debug, Serialize)]
pub struct DeploymentStatusResponse {
    pub id: String,
    pub group_id: String,
    pub config_version: String,
    pub strategy: String,
    pub status: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_by: Option<String>,
    pub approved_by: Option<String>,
    pub error: Option<String>,
    pub stats: DeploymentStatsResponse,
    pub agents: Vec<DeploymentAgentResponse>,
}

/// Deployment statistics
#[derive(Debug, Serialize)]
pub struct DeploymentStatsResponse {
    pub total: u32,
    pub completed: u32,
    pub failed: u32,
    pub in_progress: u32,
    pub pending: u32,
}

/// Deployment agent status
#[derive(Debug, Serialize)]
pub struct DeploymentAgentResponse {
    pub agent_id: String,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error: Option<String>,
}

/// Query parameters for deployment list
#[derive(Debug, Deserialize)]
pub struct DeploymentListQuery {
    pub limit: Option<i64>,
}

/// Response for version check
#[derive(Debug, Serialize)]
pub struct VersionCheckResponse {
    pub consistent: bool,
    pub versions: Vec<VersionInfoResponse>,
    pub message: String,
    pub can_deploy: bool,
}

/// Version info
#[derive(Debug, Serialize)]
pub struct VersionInfoResponse {
    pub version: String,
    pub agents: Vec<String>,
}

/// Request to approve deployment
#[derive(Debug, Deserialize)]
pub struct ApproveDeploymentRequest {
    pub approved_by: String,
}

/// Request to reject deployment
#[derive(Debug, Deserialize)]
pub struct RejectDeploymentRequest {
    pub rejected_by: String,
    pub reason: Option<String>,
}

// =============================================================================
// API Endpoints
// =============================================================================

/// Create a new deployment for a group
pub async fn create_deployment(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
    Json(request): Json<CreateDeploymentRequest>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Get the group
    let group = match WorkerGroupRepository::get_by_id(pool, &group_id).await {
        Ok(Some(group)) => group,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Group not found"
            }))).into_response();
        }
        Err(e) => {
            error!("Failed to get group: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get group"
            }))).into_response();
        }
    };
    
    // Get config version
    let config_version = match &request.config_version {
        Some(v) => v.clone(),
        None => {
            match group.current_config_version {
                Some(v) => v,
                None => {
                    return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                        "error": "No configuration set for this group"
                    }))).into_response();
                }
            }
        }
    };
    
    // Build options
    let options = DeploymentOptions {
        rolling: request.rolling_options.clone(),
        canary: request.canary_options.clone(),
    };
    
    // Create executor
    let executor = DeploymentExecutor::new(state.db.clone(), state.git_store.clone());
    
    // Start deployment
    match executor.start_deployment(
        &group_id,
        &config_version,
        Some(options),
        None, // TODO: get from auth context
        request.force,
    ).await {
        Ok(result) => {
            (StatusCode::CREATED, Json(CreateDeploymentResponse {
                deployment_id: result.deployment_id,
                status: result.status,
                message: result.message,
                requires_approval: result.requires_approval,
                queued: result.queued,
            })).into_response()
        }
        Err(e) => {
            error!("Failed to create deployment: {}", e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": e
            }))).into_response()
        }
    }
}

/// Get deployment status
pub async fn get_deployment(
    State(state): State<Arc<AppState>>,
    Path(deployment_id): Path<String>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    let deployment = match DeploymentRepository::get_by_id(pool, &deployment_id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Deployment not found"
            }))).into_response();
        }
        Err(e) => {
            error!("Failed to get deployment: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get deployment"
            }))).into_response();
        }
    };
    
    let stats = match DeploymentRepository::get_stats(pool, &deployment_id).await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to get stats: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get deployment stats"
            }))).into_response();
        }
    };
    
    let agents = match DeploymentRepository::get_agents(pool, &deployment_id).await {
        Ok(a) => a,
        Err(e) => {
            error!("Failed to get agents: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get deployment agents"
            }))).into_response();
        }
    };
    
    let response = DeploymentStatusResponse {
        id: deployment.id,
        group_id: deployment.group_id,
        config_version: deployment.config_version,
        strategy: deployment.strategy,
        status: deployment.status,
        created_at: deployment.created_at,
        started_at: deployment.started_at,
        completed_at: deployment.completed_at,
        created_by: deployment.created_by,
        approved_by: deployment.approved_by,
        error: deployment.error,
        stats: DeploymentStatsResponse {
            total: stats.total,
            completed: stats.completed,
            failed: stats.failed,
            in_progress: stats.in_progress,
            pending: stats.pending,
        },
        agents: agents.into_iter().map(|a| DeploymentAgentResponse {
            agent_id: a.agent_id,
            status: a.status,
            started_at: a.started_at,
            completed_at: a.completed_at,
            error: a.error,
        }).collect(),
    };
    
    (StatusCode::OK, Json(response)).into_response()
}

/// List deployments for a group
pub async fn list_deployments(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
    Query(params): Query<DeploymentListQuery>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    let limit = params.limit.unwrap_or(50);
    
    match DeploymentRepository::list_by_group(pool, &group_id, limit).await {
        Ok(deployments) => {
            let responses: Vec<serde_json::Value> = deployments.into_iter().map(|d| {
                serde_json::json!({
                    "id": d.id,
                    "config_version": d.config_version,
                    "strategy": d.strategy,
                    "status": d.status,
                    "created_at": d.created_at,
                    "completed_at": d.completed_at,
                    "error": d.error,
                })
            }).collect();
            
            (StatusCode::OK, Json(serde_json::json!({
                "deployments": responses
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to list deployments: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to list deployments"
            }))).into_response()
        }
    }
}

/// Check version consistency for a group
pub async fn check_versions(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
) -> impl IntoResponse {
    match check_version_consistency(&state.db, &group_id).await {
        Ok(result) => {
            let response = VersionCheckResponse {
                consistent: result.consistent,
                versions: result.versions.into_iter().map(|v| VersionInfoResponse {
                    version: v.version,
                    agents: v.agents,
                }).collect(),
                message: result.message.clone(),
                can_deploy: result.consistent,
            };
            
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            error!("Failed to check versions: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": e
            }))).into_response()
        }
    }
}

/// Approve a pending deployment
pub async fn approve_deployment(
    State(state): State<Arc<AppState>>,
    Path(deployment_id): Path<String>,
    Json(request): Json<ApproveDeploymentRequest>,
) -> impl IntoResponse {
    let executor = DeploymentExecutor::new(state.db.clone(), state.git_store.clone());
    
    match executor.approve_deployment(&deployment_id, &request.approved_by).await {
        Ok(_) => {
            info!("Deployment {} approved", deployment_id);
            (StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": "Deployment approved and started"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to approve deployment: {}", e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": e
            }))).into_response()
        }
    }
}

/// Reject a pending deployment
pub async fn reject_deployment(
    State(state): State<Arc<AppState>>,
    Path(deployment_id): Path<String>,
    Json(request): Json<RejectDeploymentRequest>,
) -> impl IntoResponse {
    let executor = DeploymentExecutor::new(state.db.clone(), state.git_store.clone());
    
    match executor.reject_deployment(&deployment_id, &request.rejected_by, request.reason.as_deref()).await {
        Ok(_) => {
            info!("Deployment {} rejected", deployment_id);
            (StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": "Deployment rejected"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to reject deployment: {}", e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": e
            }))).into_response()
        }
    }
}

/// Cancel a deployment
pub async fn cancel_deployment(
    State(state): State<Arc<AppState>>,
    Path(deployment_id): Path<String>,
) -> impl IntoResponse {
    let executor = DeploymentExecutor::new(state.db.clone(), state.git_store.clone());
    
    match executor.cancel_deployment(&deployment_id).await {
        Ok(_) => {
            info!("Deployment {} cancelled", deployment_id);
            (StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": "Deployment cancelled"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to cancel deployment: {}", e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": e
            }))).into_response()
        }
    }
}

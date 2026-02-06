//! Worker Group management API endpoints
//!
//! Provides endpoints for:
//! - Creating and managing worker groups
//! - Assigning agents to groups
//! - Managing group configurations
//! - Version history and rollback

use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn, error};

use crate::AppState;
use crate::db::models::{WorkerGroupResponse, AgentResponse};
use crate::db::repository::{WorkerGroupRepository, AgentRepository};

/// Request to create a new worker group
#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    /// Name of the worker group (must be unique)
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Deployment strategy: basic, rolling, canary
    #[serde(default = "default_strategy")]
    pub deployment_strategy: String,
    /// Whether deployments require approval
    #[serde(default)]
    pub requires_approval: bool,
    /// List of user IDs/emails who can approve deployments
    pub approvers: Option<Vec<String>>,
}

fn default_strategy() -> String {
    "rolling".to_string()
}

/// Request to update a worker group
#[derive(Debug, Deserialize)]
pub struct UpdateGroupRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub deployment_strategy: Option<String>,
    pub requires_approval: Option<bool>,
    pub approvers: Option<Vec<String>>,
}

/// Request to update group configuration
#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    /// TOML configuration content
    pub config: String,
    /// Optional commit message (reserved for future use)
    #[allow(dead_code)]
    pub message: Option<String>,
}

/// Response for configuration update
#[derive(Debug, Serialize)]
pub struct UpdateConfigResponse {
    pub success: bool,
    pub version: Option<String>,
    pub message: String,
}

/// Request to rollback configuration
#[derive(Debug, Deserialize)]
pub struct RollbackRequest {
    /// Commit hash to rollback to
    pub version: String,
}

/// Query parameters for history
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<usize>,
}

/// Query parameters for diff
#[derive(Debug, Deserialize)]
pub struct DiffQuery {
    /// From version (commit hash or "current" for working tree)
    pub from: String,
    /// To version (commit hash or "current" for working tree)
    pub to: String,
}

/// Response for config at version
#[derive(Debug, Serialize)]
pub struct ConfigAtVersionResponse {
    pub config: Option<String>,
    pub version: String,
    pub group_name: String,
}

/// Response for diff
#[derive(Debug, Serialize)]
pub struct DiffResponse {
    pub from_version: String,
    pub to_version: String,
    pub diff: String,
    pub has_changes: bool,
}

/// Request to deploy configuration to agents
#[derive(Debug, Deserialize)]
pub struct DeployRequest {
    /// Optional specific version to deploy (defaults to current)
    pub version: Option<String>,
    /// Optional list of specific agent IDs to deploy to (defaults to all in group)
    pub agent_ids: Option<Vec<String>>,
}

/// Response for deployment
#[derive(Debug, Serialize)]
pub struct DeployResponse {
    pub success: bool,
    pub message: String,
    pub deployed_to: Vec<AgentDeployResult>,
    pub version: String,
}

/// Result for individual agent deployment
#[derive(Debug, Serialize)]
pub struct AgentDeployResult {
    pub agent_id: String,
    pub agent_name: String,
    pub success: bool,
    pub message: String,
}

/// List all worker groups
pub async fn list_groups(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    match WorkerGroupRepository::list(pool).await {
        Ok(groups) => {
            // Convert to response format with agent counts and health status
            let mut responses: Vec<WorkerGroupResponse> = Vec::new();
            
            for group in groups {
                // Get real agent health counts from the database
                let (total, healthy, unhealthy) = WorkerGroupRepository::get_agent_health_counts(pool, &group.id)
                    .await
                    .unwrap_or((0, 0, 0));
                
                let mut response = WorkerGroupResponse::from(group);
                response.agent_count = Some(total);
                response.healthy_count = Some(healthy);
                response.unhealthy_count = Some(unhealthy);
                responses.push(response);
            }
            
            (StatusCode::OK, Json(responses)).into_response()
        }
        Err(e) => {
            error!("Failed to list worker groups: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to list worker groups"
            }))).into_response()
        }
    }
}

/// Create a new worker group
pub async fn create_group(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateGroupRequest>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Check if group with this name already exists
    if let Ok(Some(_)) = WorkerGroupRepository::get_by_name(pool, &request.name).await {
        return (StatusCode::CONFLICT, Json(serde_json::json!({
            "error": "A worker group with this name already exists"
        }))).into_response();
    }
    
    // Validate deployment strategy
    if !["basic", "rolling", "canary"].contains(&request.deployment_strategy.as_str()) {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "Invalid deployment strategy. Must be: basic, rolling, or canary"
        }))).into_response();
    }
    
    // Create the group in database
    match WorkerGroupRepository::create(
        pool,
        &request.name,
        request.description.as_deref(),
        None, // TODO: get from auth context
    ).await {
        Ok(group) => {
            // Update with additional fields
            let approvers_json = request.approvers
                .map(|a| serde_json::to_string(&a).unwrap_or_default());
            
            if let Err(e) = WorkerGroupRepository::update(
                pool,
                &group.id,
                None,
                None,
                Some(&request.deployment_strategy),
                Some(request.requires_approval),
                approvers_json.as_deref(),
            ).await {
                warn!("Failed to update group settings: {}", e);
            }
            
            // Create the group directory in git store
            if let Err(e) = state.git_store.create_group(&request.name) {
                warn!("Failed to create git directory for group: {}", e);
            }
            
            info!("Created worker group: {}", request.name);
            
            // Fetch the updated group
            match WorkerGroupRepository::get_by_id(pool, &group.id).await {
                Ok(Some(updated)) => {
                    (StatusCode::CREATED, Json(WorkerGroupResponse::from(updated))).into_response()
                }
                _ => {
                    (StatusCode::CREATED, Json(WorkerGroupResponse::from(group))).into_response()
                }
            }
        }
        Err(e) => {
            error!("Failed to create worker group: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("Failed to create worker group: {}", e)
            }))).into_response()
        }
    }
}

/// Get a specific worker group
pub async fn get_group(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    match WorkerGroupRepository::get_by_id(pool, &id).await {
        Ok(Some(group)) => {
            // Get real agent health counts from the database
            let (total, healthy, unhealthy) = WorkerGroupRepository::get_agent_health_counts(pool, &id)
                .await
                .unwrap_or((0, 0, 0));
            
            let mut response = WorkerGroupResponse::from(group);
            response.agent_count = Some(total);
            response.healthy_count = Some(healthy);
            response.unhealthy_count = Some(unhealthy);
            
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(None) => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Worker group not found"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to get worker group: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get worker group"
            }))).into_response()
        }
    }
}

/// Update a worker group
pub async fn update_group(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<UpdateGroupRequest>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Validate deployment strategy if provided
    if let Some(ref strategy) = request.deployment_strategy {
        if !["basic", "rolling", "canary"].contains(&strategy.as_str()) {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid deployment strategy. Must be: basic, rolling, or canary"
            }))).into_response();
        }
    }
    
    let approvers_json = request.approvers
        .map(|a| serde_json::to_string(&a).unwrap_or_default());
    
    match WorkerGroupRepository::update(
        pool,
        &id,
        request.name.as_deref(),
        request.description.as_ref().map(|d| d.as_deref()),
        request.deployment_strategy.as_deref(),
        request.requires_approval,
        approvers_json.as_deref(),
    ).await {
        Ok(Some(group)) => {
            info!("Updated worker group: {}", id);
            (StatusCode::OK, Json(WorkerGroupResponse::from(group))).into_response()
        }
        Ok(None) => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Worker group not found"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to update worker group: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to update worker group"
            }))).into_response()
        }
    }
}

/// Delete a worker group
pub async fn delete_group(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Get the group first to get its name
    let group_name = match WorkerGroupRepository::get_by_id(pool, &id).await {
        Ok(Some(group)) => group.name,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Worker group not found"
            }))).into_response();
        }
        Err(e) => {
            error!("Failed to get worker group: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get worker group"
            }))).into_response();
        }
    };
    
    // Check if group has agents assigned
    let agent_count = WorkerGroupRepository::get_agent_count(pool, &id)
        .await
        .unwrap_or(0);
    
    if agent_count > 0 {
        return (StatusCode::CONFLICT, Json(serde_json::json!({
            "error": format!("Cannot delete group with {} assigned agents. Unassign agents first.", agent_count)
        }))).into_response();
    }
    
    // Delete from database
    match WorkerGroupRepository::delete(pool, &id).await {
        Ok(true) => {
            // Delete from git store
            if let Err(e) = state.git_store.delete_group(&group_name) {
                warn!("Failed to delete git directory for group: {}", e);
            }
            
            info!("Deleted worker group: {} ({})", group_name, id);
            (StatusCode::NO_CONTENT, "").into_response()
        }
        Ok(false) => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Worker group not found"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to delete worker group: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to delete worker group"
            }))).into_response()
        }
    }
}

/// List agents in a worker group
pub async fn list_group_agents(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Verify group exists
    if let Ok(None) = WorkerGroupRepository::get_by_id(pool, &id).await {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "error": "Worker group not found"
        }))).into_response();
    }
    
    match AgentRepository::list_by_group(pool, &id).await {
        Ok(agents) => {
            let responses: Vec<AgentResponse> = agents.into_iter().map(AgentResponse::from).collect();
            (StatusCode::OK, Json(responses)).into_response()
        }
        Err(e) => {
            error!("Failed to list group agents: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to list group agents"
            }))).into_response()
        }
    }
}

/// Get current configuration for a worker group
pub async fn get_group_config(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Get the group
    let group = match WorkerGroupRepository::get_by_id(pool, &id).await {
        Ok(Some(group)) => group,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Worker group not found"
            }))).into_response();
        }
        Err(e) => {
            error!("Failed to get worker group: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get worker group"
            }))).into_response();
        }
    };
    
    // Read config from git store
    match state.git_store.read_config(&group.name) {
        Ok(Some(config)) => {
            (StatusCode::OK, Json(serde_json::json!({
                "config": config,
                "version": group.current_config_version,
                "group_name": group.name
            }))).into_response()
        }
        Ok(None) => {
            (StatusCode::OK, Json(serde_json::json!({
                "config": null,
                "version": null,
                "group_name": group.name,
                "message": "No configuration set for this group"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to read group config: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to read group configuration"
            }))).into_response()
        }
    }
}

/// Update configuration for a worker group
pub async fn update_group_config(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<UpdateConfigRequest>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Get the group
    let group = match WorkerGroupRepository::get_by_id(pool, &id).await {
        Ok(Some(group)) => group,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(UpdateConfigResponse {
                success: false,
                version: None,
                message: "Worker group not found".to_string(),
            })).into_response();
        }
        Err(e) => {
            error!("Failed to get worker group: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(UpdateConfigResponse {
                success: false,
                version: None,
                message: format!("Failed to get worker group: {}", e),
            })).into_response();
        }
    };
    
    // Validate configuration using full validator
    let vector_bin = state.vector_process.get_binary_path();
    let validator = crate::validation::ConfigValidator::new(vector_bin);
    let validation = validator.validate(&request.config);
    
    if !validation.valid {
        let error_msg = validation.errors.first()
            .map(|e| e.message.clone())
            .unwrap_or_else(|| "Configuration validation failed".to_string());
        
        return (StatusCode::BAD_REQUEST, Json(UpdateConfigResponse {
            success: false,
            version: None,
            message: format!("Validation failed: {}", error_msg),
        })).into_response();
    }
    
    // Log warnings if any
    for warning in &validation.warnings {
        warn!("Config validation warning: {}", warning.message);
    }
    
    // Write config to git store (auto-commits)
    match state.git_store.write_config(&group.name, &request.config) {
        Ok(commit_hash) => {
            // Update the group's current config version
            if let Err(e) = WorkerGroupRepository::update_config_version(pool, &id, &commit_hash).await {
                warn!("Failed to update group config version: {}", e);
            }
            
            info!("Updated config for group {} (version: {})", group.name, &commit_hash[..8]);
            
            (StatusCode::OK, Json(UpdateConfigResponse {
                success: true,
                version: Some(commit_hash),
                message: "Configuration updated successfully".to_string(),
            })).into_response()
        }
        Err(e) => {
            error!("Failed to write group config: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(UpdateConfigResponse {
                success: false,
                version: None,
                message: format!("Failed to write configuration: {}", e),
            })).into_response()
        }
    }
}

/// Get configuration history for a worker group
pub async fn get_group_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<HistoryQuery>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    let limit = params.limit.unwrap_or(50);
    
    // Get the group
    let group = match WorkerGroupRepository::get_by_id(pool, &id).await {
        Ok(Some(group)) => group,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Worker group not found"
            }))).into_response();
        }
        Err(e) => {
            error!("Failed to get worker group: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get worker group"
            }))).into_response();
        }
    };
    
    // Get history from git store
    match state.git_store.get_history(Some(&group.name), limit) {
        Ok(history) => {
            (StatusCode::OK, Json(history)).into_response()
        }
        Err(e) => {
            error!("Failed to get group history: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get configuration history"
            }))).into_response()
        }
    }
}

/// Rollback configuration for a worker group to a previous version
pub async fn rollback_group_config(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<RollbackRequest>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Get the group
    let group = match WorkerGroupRepository::get_by_id(pool, &id).await {
        Ok(Some(group)) => group,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(UpdateConfigResponse {
                success: false,
                version: None,
                message: "Worker group not found".to_string(),
            })).into_response();
        }
        Err(e) => {
            error!("Failed to get worker group: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(UpdateConfigResponse {
                success: false,
                version: None,
                message: format!("Failed to get worker group: {}", e),
            })).into_response();
        }
    };
    
    // Perform rollback in git store
    match state.git_store.rollback(&group.name, &request.version) {
        Ok(new_hash) => {
            // Update the group's current config version
            if let Err(e) = WorkerGroupRepository::update_config_version(pool, &id, &new_hash).await {
                warn!("Failed to update group config version: {}", e);
            }
            
            info!("Rolled back config for group {} to version {} (new: {})", 
                group.name, &request.version[..8.min(request.version.len())], &new_hash[..8]);
            
            (StatusCode::OK, Json(UpdateConfigResponse {
                success: true,
                version: Some(new_hash),
                message: format!("Configuration rolled back to version {}", &request.version[..8.min(request.version.len())]),
            })).into_response()
        }
        Err(e) => {
            error!("Failed to rollback group config: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(UpdateConfigResponse {
                success: false,
                version: None,
                message: format!("Failed to rollback configuration: {}", e),
            })).into_response()
        }
    }
}

/// Get configuration at a specific version
pub async fn get_group_config_at_version(
    State(state): State<Arc<AppState>>,
    Path((id, version)): Path<(String, String)>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Get the group
    let group = match WorkerGroupRepository::get_by_id(pool, &id).await {
        Ok(Some(group)) => group,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Worker group not found"
            }))).into_response();
        }
        Err(e) => {
            error!("Failed to get worker group: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get worker group"
            }))).into_response();
        }
    };
    
    // Get config at version
    match state.git_store.get_config_at_version(&group.name, &version) {
        Ok(config) => {
            (StatusCode::OK, Json(ConfigAtVersionResponse {
                config,
                version,
                group_name: group.name,
            })).into_response()
        }
        Err(e) => {
            error!("Failed to get config at version: {}", e);
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": format!("Failed to get config at version: {}", e)
            }))).into_response()
        }
    }
}

/// Get diff between two versions
pub async fn get_group_diff(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<DiffQuery>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Get the group (verify it exists)
    let _group = match WorkerGroupRepository::get_by_id(pool, &id).await {
        Ok(Some(group)) => group,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Worker group not found"
            }))).into_response();
        }
        Err(e) => {
            error!("Failed to get worker group: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get worker group"
            }))).into_response();
        }
    };
    
    // Handle "current" as HEAD
    let from_hash = if params.from == "current" {
        match state.git_store.head_hash() {
            Ok(hash) => hash,
            Err(e) => {
                error!("Failed to get HEAD: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                    "error": "Failed to get current version"
                }))).into_response();
            }
        }
    } else {
        params.from.clone()
    };
    
    let to_hash = if params.to == "current" {
        match state.git_store.head_hash() {
            Ok(hash) => hash,
            Err(e) => {
                error!("Failed to get HEAD: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                    "error": "Failed to get current version"
                }))).into_response();
            }
        }
    } else {
        params.to.clone()
    };
    
    // Get diff
    match state.git_store.diff(&from_hash, &to_hash) {
        Ok(diff) => {
            let has_changes = !diff.trim().is_empty();
            (StatusCode::OK, Json(DiffResponse {
                from_version: from_hash,
                to_version: to_hash,
                diff,
                has_changes,
            })).into_response()
        }
        Err(e) => {
            error!("Failed to get diff: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("Failed to get diff: {}", e)
            }))).into_response()
        }
    }
}

/// Deploy configuration to agents in a worker group
pub async fn deploy_to_group(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<DeployRequest>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Get the group
    let group = match WorkerGroupRepository::get_by_id(pool, &id).await {
        Ok(Some(group)) => group,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(DeployResponse {
                success: false,
                message: "Worker group not found".to_string(),
                deployed_to: vec![],
                version: String::new(),
            })).into_response();
        }
        Err(e) => {
            error!("Failed to get worker group: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(DeployResponse {
                success: false,
                message: format!("Failed to get worker group: {}", e),
                deployed_to: vec![],
                version: String::new(),
            })).into_response();
        }
    };
    
    // Get the config to deploy (config is verified to exist but agents pull it themselves)
    let (_config, version) = if let Some(ref ver) = request.version {
        // Deploy specific version
        match state.git_store.get_config_at_version(&group.name, ver) {
            Ok(Some(cfg)) => (cfg, ver.clone()),
            Ok(None) => {
                return (StatusCode::NOT_FOUND, Json(DeployResponse {
                    success: false,
                    message: format!("Config not found at version {}", ver),
                    deployed_to: vec![],
                    version: String::new(),
                })).into_response();
            }
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(DeployResponse {
                    success: false,
                    message: format!("Failed to get config: {}", e),
                    deployed_to: vec![],
                    version: String::new(),
                })).into_response();
            }
        }
    } else {
        // Deploy current config
        match state.git_store.read_config(&group.name) {
            Ok(Some(cfg)) => {
                let ver = group.current_config_version.clone().unwrap_or_else(|| {
                    state.git_store.head_hash().unwrap_or_default()
                });
                (cfg, ver)
            }
            Ok(None) => {
                return (StatusCode::NOT_FOUND, Json(DeployResponse {
                    success: false,
                    message: "No configuration set for this group".to_string(),
                    deployed_to: vec![],
                    version: String::new(),
                })).into_response();
            }
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(DeployResponse {
                    success: false,
                    message: format!("Failed to read config: {}", e),
                    deployed_to: vec![],
                    version: String::new(),
                })).into_response();
            }
        }
    };
    
    // Get agents in the group
    let agents = match AgentRepository::list_by_group(pool, &id).await {
        Ok(agents) => agents,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(DeployResponse {
                success: false,
                message: format!("Failed to list agents: {}", e),
                deployed_to: vec![],
                version: version.clone(),
            })).into_response();
        }
    };
    
    if agents.is_empty() {
        return (StatusCode::OK, Json(DeployResponse {
            success: true,
            message: "No agents in group to deploy to".to_string(),
            deployed_to: vec![],
            version,
        })).into_response();
    }
    
    // Filter agents if specific IDs provided
    let target_agents: Vec<_> = if let Some(ref agent_ids) = request.agent_ids {
        agents.into_iter().filter(|a| agent_ids.contains(&a.id)).collect()
    } else {
        agents
    };
    
    // Deploy to each agent
    let mut results = Vec::new();
    let mut success_count = 0;
    
    for agent in target_agents {
        // For now, agents pull configs via the agent sidecar
        // In a full implementation, we could push directly if the agent exposes an API
        // This marks the deployment as "pending" - agents will pick up the new version
        
        // Record that we're deploying to this agent
        info!("Marking deployment for agent {} ({})", agent.name, agent.id);
        
        results.push(AgentDeployResult {
            agent_id: agent.id.clone(),
            agent_name: agent.name.clone(),
            success: true,
            message: "Deployment queued - agent will pull on next sync".to_string(),
        });
        success_count += 1;
    }
    
    let all_success = success_count == results.len();
    
    info!("Deployment to group {} complete: {}/{} agents", group.name, success_count, results.len());
    
    (StatusCode::OK, Json(DeployResponse {
        success: all_success,
        message: format!("Deployment initiated for {} agents", success_count),
        deployed_to: results,
        version,
    })).into_response()
}

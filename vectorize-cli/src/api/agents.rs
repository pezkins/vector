//! Agent management API endpoints
//!
//! Provides endpoints for:
//! - Registering new agents (self-registration or manual)
//! - Listing and filtering agents
//! - Updating agent information
//! - Checking agent health

use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx;
use std::sync::Arc;
use tracing::{info, error};

use crate::AppState;
use crate::db::models::AgentResponse;
use crate::db::repository::AgentRepository;

/// Request to register a new agent
#[derive(Debug, Deserialize)]
pub struct RegisterAgentRequest {
    /// Human-readable name for the agent
    pub name: String,
    /// URL to the agent's GraphQL API (e.g., http://localhost:8686)
    pub url: String,
    /// Optional worker group to assign the agent to
    pub group_id: Option<String>,
    /// Optional metadata (reserved for future use)
    #[allow(dead_code)]
    pub metadata: Option<serde_json::Value>,
}

/// Response when registering an agent
#[derive(Debug, Serialize)]
pub struct RegisterAgentResponse {
    pub success: bool,
    pub agent: Option<AgentResponse>,
    pub message: String,
}

/// Query parameters for listing agents
#[derive(Debug, Deserialize)]
pub struct ListAgentsQuery {
    /// Filter by worker group
    pub group_id: Option<String>,
    /// Filter by status
    pub status: Option<String>,
}

/// Request to update an agent
#[derive(Debug, Deserialize)]
pub struct UpdateAgentRequest {
    /// New name (optional)
    pub name: Option<String>,
    /// New group assignment (optional, use null to unassign)
    pub group_id: Option<Option<String>>,
}

/// List all agents
pub async fn list_agents(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListAgentsQuery>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    let agents = if let Some(group_id) = query.group_id {
        match AgentRepository::list_by_group(pool, &group_id).await {
            Ok(agents) => agents,
            Err(e) => {
                error!("Failed to list agents by group: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                    "error": "Failed to list agents"
                }))).into_response();
            }
        }
    } else {
        match AgentRepository::list(pool).await {
            Ok(agents) => agents,
            Err(e) => {
                error!("Failed to list agents: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                    "error": "Failed to list agents"
                }))).into_response();
            }
        }
    };
    
    // Convert to response format and optionally filter by status
    let agents: Vec<AgentResponse> = agents
        .into_iter()
        .map(AgentResponse::from)
        .filter(|a| {
            if let Some(ref status_filter) = query.status {
                a.status.to_string() == *status_filter
            } else {
                true
            }
        })
        .collect();
    
    (StatusCode::OK, Json(agents)).into_response()
}

/// List unassigned agents (agents without a group)
pub async fn list_unassigned_agents(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    match AgentRepository::list_unassigned(pool).await {
        Ok(agents) => {
            let agents: Vec<AgentResponse> = agents.into_iter().map(AgentResponse::from).collect();
            (StatusCode::OK, Json(agents)).into_response()
        }
        Err(e) => {
            error!("Failed to list unassigned agents: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to list unassigned agents"
            }))).into_response()
        }
    }
}

/// Assign an agent to a group
#[derive(Debug, Deserialize)]
pub struct AssignAgentRequest {
    pub group_id: Option<String>,  // None to unassign
}

pub async fn assign_agent_to_group(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<AssignAgentRequest>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Verify agent exists
    let agent = match AgentRepository::get_by_id(pool, &id).await {
        Ok(Some(agent)) => agent,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Agent not found"
            }))).into_response();
        }
        Err(e) => {
            error!("Failed to get agent: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get agent"
            }))).into_response();
        }
    };
    
    // If group_id is provided, verify the group exists
    if let Some(ref group_id) = request.group_id {
        match crate::db::repository::WorkerGroupRepository::get_by_id(pool, group_id).await {
            Ok(Some(_)) => {}
            Ok(None) => {
                return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                    "error": "Group not found"
                }))).into_response();
            }
            Err(e) => {
                error!("Failed to verify group: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                    "error": "Failed to verify group"
                }))).into_response();
            }
        }
    }
    
    // Update the agent's group
    match AgentRepository::update(
        pool,
        &id,
        Some(&agent.name),
        Some(request.group_id.as_deref()),
    ).await {
        Ok(Some(updated)) => {
            let action = if request.group_id.is_some() { "assigned to group" } else { "unassigned from group" };
            info!("Agent '{}' {}", updated.name, action);
            (StatusCode::OK, Json(AgentResponse::from(updated))).into_response()
        }
        Ok(None) => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Agent not found"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to assign agent: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to assign agent"
            }))).into_response()
        }
    }
}

/// Register a new agent
pub async fn register_agent(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RegisterAgentRequest>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Check if an agent with this name already exists
    if let Ok(Some(existing)) = AgentRepository::get_by_name(pool, &request.name).await {
        // Agent exists - update its URL and group if changed, and return it
        // This allows agents to re-register after restart with potentially different URLs
        let _ = AgentRepository::update(
            pool,
            &existing.id,
            Some(&request.name),
            Some(request.group_id.as_deref()),
        ).await;
        
        // Update URL if changed
        if existing.url != request.url {
            let _ = sqlx::query("UPDATE agents SET url = ? WHERE id = ?")
                .bind(&request.url)
                .bind(&existing.id)
                .execute(pool)
                .await;
        }
        
        // Re-fetch and return the updated agent
        if let Ok(Some(agent)) = AgentRepository::get_by_id(pool, &existing.id).await {
            info!("Agent '{}' re-registered (updated URL/group)", request.name);
            return (StatusCode::OK, Json(RegisterAgentResponse {
                success: true,
                agent: Some(AgentResponse::from(agent)),
                message: "Agent re-registered successfully".to_string(),
            })).into_response();
        }
    }
    
    // Verify the agent is reachable by checking its health endpoint
    let health_url = format!("{}/health", request.url.trim_end_matches('/'));
    let health_check = state.http_client.get(&health_url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;
    
    let initial_status = match health_check {
        Ok(response) if response.status().is_success() => "healthy",
        Ok(_) => "unhealthy",
        Err(_) => "unreachable",
    };
    
    // Try to get Vector version
    let version = get_vector_version(&state.http_client, &request.url).await;
    
    // Create the agent in the database
    match AgentRepository::create(
        pool,
        &request.name,
        &request.url,
        request.group_id.as_deref(),
    ).await {
        Ok(agent) => {
            // Update status and version
            let _ = AgentRepository::update_status(
                pool,
                &agent.id,
                initial_status,
                version.as_deref(),
            ).await;
            
            info!("Registered new agent: {} at {} (status: {})", agent.name, agent.url, initial_status);
            
            // Re-fetch to get updated status
            let updated = AgentRepository::get_by_id(pool, &agent.id).await
                .ok()
                .flatten()
                .map(AgentResponse::from);
            
            (StatusCode::CREATED, Json(RegisterAgentResponse {
                success: true,
                agent: updated,
                message: format!("Agent registered successfully (status: {})", initial_status),
            })).into_response()
        }
        Err(e) => {
            error!("Failed to register agent: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(RegisterAgentResponse {
                success: false,
                agent: None,
                message: format!("Failed to register agent: {}", e),
            })).into_response()
        }
    }
}

/// Get a specific agent by ID
pub async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    match AgentRepository::get_by_id(pool, &id).await {
        Ok(Some(agent)) => {
            (StatusCode::OK, Json(AgentResponse::from(agent))).into_response()
        }
        Ok(None) => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Agent not found"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to get agent: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get agent"
            }))).into_response()
        }
    }
}

/// Update an agent
pub async fn update_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<UpdateAgentRequest>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    match AgentRepository::update(
        pool,
        &id,
        request.name.as_deref(),
        request.group_id.as_ref().map(|g| g.as_deref()),
    ).await {
        Ok(Some(agent)) => {
            info!("Updated agent: {}", id);
            (StatusCode::OK, Json(AgentResponse::from(agent))).into_response()
        }
        Ok(None) => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Agent not found"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to update agent: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to update agent"
            }))).into_response()
        }
    }
}

/// Delete an agent
pub async fn delete_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    match AgentRepository::delete(pool, &id).await {
        Ok(true) => {
            info!("Deleted agent: {}", id);
            (StatusCode::NO_CONTENT, "").into_response()
        }
        Ok(false) => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Agent not found"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to delete agent: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to delete agent"
            }))).into_response()
        }
    }
}

/// Get agent health history
pub async fn get_agent_health(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<HealthQuery>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    let limit = params.limit.unwrap_or(100);
    
    // Verify agent exists
    if let Ok(None) = AgentRepository::get_by_id(pool, &id).await {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "error": "Agent not found"
        }))).into_response();
    }
    
    match AgentRepository::get_health_checks(pool, &id, limit).await {
        Ok(checks) => {
            (StatusCode::OK, Json(checks)).into_response()
        }
        Err(e) => {
            error!("Failed to get health checks: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get health checks"
            }))).into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct HealthQuery {
    pub limit: Option<i64>,
}

/// Get Vector version from an agent
async fn get_vector_version(client: &reqwest::Client, base_url: &str) -> Option<String> {
    let graphql_url = format!("{}/graphql", base_url.trim_end_matches('/'));
    
    let query = serde_json::json!({
        "query": "{ meta { versionString } }"
    });
    
    let response = client.post(&graphql_url)
        .json(&query)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .ok()?;
    
    let json: serde_json::Value = response.json().await.ok()?;
    
    json.get("data")
        .and_then(|d| d.get("meta"))
        .and_then(|m| m.get("versionString"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

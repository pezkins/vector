//! Tap/Sample API endpoints
//!
//! Provides endpoints for:
//! - Sampling live data from agents
//! - Rate limit status
//! - Agent connection info for WebSocket streaming

use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, error};

use crate::AppState;
use crate::db::repository::AgentRepository;
use crate::tap::{TapService, SampleRequest, RateLimitConfig};

// =============================================================================
// Request/Response Types
// =============================================================================

/// Query parameters for sample request
#[derive(Debug, Deserialize)]
pub struct SampleQuery {
    /// Component patterns (comma-separated)
    pub patterns: Option<String>,
    /// Maximum events to return
    pub limit: Option<u32>,
    /// Timeout in seconds
    pub timeout: Option<u64>,
}

/// Response for sample endpoint
#[derive(Debug, Serialize)]
pub struct SampleApiResponse {
    pub agent_id: String,
    pub agent_name: String,
    pub events: Vec<serde_json::Value>,
    pub count: usize,
    pub duration_ms: u64,
    pub message: String,
}

/// Response for rate limit status
#[derive(Debug, Serialize)]
pub struct RateLimitStatusResponse {
    pub can_sample: bool,
    pub message: String,
    pub config: RateLimitConfigResponse,
}

/// Rate limit configuration response
#[derive(Debug, Serialize)]
pub struct RateLimitConfigResponse {
    pub max_requests_per_minute: u32,
    pub max_concurrent_per_agent: u32,
    pub global_max_concurrent: u32,
}

// =============================================================================
// API Endpoints
// =============================================================================

/// Sample live data from an agent
pub async fn sample_agent(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
    Query(params): Query<SampleQuery>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Get the agent
    let agent = match AgentRepository::get_by_id(pool, &agent_id).await {
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
    
    // Build sample request
    let patterns: Vec<String> = params.patterns
        .map(|p| p.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_else(|| vec!["*".to_string()]);
    
    let request = SampleRequest {
        agent_id: agent_id.clone(),
        patterns,
        limit: params.limit.unwrap_or(10),
        timeout_secs: params.timeout.unwrap_or(5),
    };
    
    // Create tap service
    let tap_service = TapService::new(RateLimitConfig::default());
    
    // Sample from agent
    match tap_service.sample(&agent.url, &request).await {
        Ok(response) => {
            (StatusCode::OK, Json(SampleApiResponse {
                agent_id: agent.id,
                agent_name: agent.name,
                events: response.events.into_iter().map(|e| e.event).collect(),
                count: response.count,
                duration_ms: response.duration_ms,
                message: "Sample complete. Use WebSocket for real-time streaming.".to_string(),
            })).into_response()
        }
        Err(e) => {
            error!("Failed to sample from agent: {}", e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": e.to_string()
            }))).into_response()
        }
    }
}

/// Check rate limit status for an agent
pub async fn check_rate_limit(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Verify agent exists
    if let Ok(None) = AgentRepository::get_by_id(pool, &agent_id).await {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "error": "Agent not found"
        }))).into_response();
    }
    
    let tap_service = TapService::new(RateLimitConfig::default());
    let config = RateLimitConfig::default();
    
    match tap_service.can_sample(&agent_id).await {
        Ok(()) => {
            (StatusCode::OK, Json(RateLimitStatusResponse {
                can_sample: true,
                message: "Sampling is allowed".to_string(),
                config: RateLimitConfigResponse {
                    max_requests_per_minute: config.max_requests_per_minute,
                    max_concurrent_per_agent: config.max_concurrent_per_agent,
                    global_max_concurrent: config.global_max_concurrent,
                },
            })).into_response()
        }
        Err(e) => {
            (StatusCode::TOO_MANY_REQUESTS, Json(RateLimitStatusResponse {
                can_sample: false,
                message: e.to_string(),
                config: RateLimitConfigResponse {
                    max_requests_per_minute: config.max_requests_per_minute,
                    max_concurrent_per_agent: config.max_concurrent_per_agent,
                    global_max_concurrent: config.global_max_concurrent,
                },
            })).into_response()
        }
    }
}

/// Get WebSocket connection info for an agent
/// 
/// Returns the WebSocket URL for direct connection to Vector's GraphQL API.
/// The UI uses this to establish real-time event streaming.
pub async fn get_websocket_info(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Get the agent
    let agent = match AgentRepository::get_by_id(pool, &agent_id).await {
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
    
    info!("WebSocket info requested for agent {} at {}", agent_id, agent.url);
    
    // Convert agent URL to WebSocket URL
    let ws_url = agent.url
        .replace("http://", "ws://")
        .replace("https://", "wss://");
    let graphql_ws_url = format!("{}/graphql", ws_url.trim_end_matches('/'));
    
    (StatusCode::OK, Json(serde_json::json!({
        "agent_id": agent.id,
        "agent_name": agent.name,
        "websocket_url": graphql_ws_url,
        "protocol": "graphql-transport-ws",
        "message": "Connect directly to this WebSocket URL for real-time streaming"
    }))).into_response()
}

/// Get tap configuration
pub async fn get_tap_config() -> impl IntoResponse {
    let config = RateLimitConfig::default();
    
    (StatusCode::OK, Json(serde_json::json!({
        "rate_limiting": {
            "max_requests_per_minute": config.max_requests_per_minute,
            "max_concurrent_per_agent": config.max_concurrent_per_agent,
            "global_max_concurrent": config.global_max_concurrent,
        },
        "default_sample_limit": 10,
        "default_timeout_secs": 5,
        "websocket_enabled": true,
    }))).into_response()
}

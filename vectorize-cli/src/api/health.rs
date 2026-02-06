//! Health monitoring API endpoints
//!
//! Provides endpoints for monitoring agent health status, metrics, and topology.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn, error};

use crate::AppState;
use crate::db::models::AgentStatus;
use crate::db::repository::AgentRepository;
use crate::health::{fetch_agent_metrics, AgentMetrics};

/// Health check result for a single agent
#[derive(Debug, Serialize)]
pub struct AgentHealthResult {
    pub agent_id: String,
    pub agent_name: String,
    pub url: String,
    pub status: AgentStatus,
    pub latency_ms: Option<u64>,
    pub vector_version: Option<String>,
    pub error: Option<String>,
}

/// Summary of all agent health
#[derive(Debug, Serialize)]
pub struct HealthSummary {
    pub total: usize,
    pub healthy: usize,
    pub unhealthy: usize,
    pub unreachable: usize,
    pub agents: Vec<AgentHealthResult>,
}

/// Fleet health summary (for dashboard UI)
#[derive(Debug, Serialize)]
pub struct FleetHealth {
    pub total_agents: u32,
    pub healthy: u32,
    pub unhealthy: u32,
    pub unknown: u32,
    pub version_distribution: Vec<VersionCount>,
}

/// Version count for fleet health
#[derive(Debug, Serialize)]
pub struct VersionCount {
    pub version: String,
    pub count: u32,
}

/// Get fleet health summary for dashboard
pub async fn get_fleet_health(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Get all agents
    let agents = match AgentRepository::list(pool).await {
        Ok(agents) => agents,
        Err(e) => {
            error!("Failed to list agents for fleet health: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to list agents"
            }))).into_response();
        }
    };
    
    let mut healthy_count: u32 = 0;
    let mut unhealthy_count: u32 = 0;
    let mut unknown_count: u32 = 0;
    let mut version_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    
    for agent in &agents {
        match agent.status.as_str() {
            "healthy" => healthy_count += 1,
            "unhealthy" => unhealthy_count += 1,
            _ => unknown_count += 1,
        }
        
        if let Some(ref version) = agent.vector_version {
            *version_counts.entry(version.clone()).or_insert(0) += 1;
        }
    }
    
    let version_distribution: Vec<VersionCount> = version_counts
        .into_iter()
        .map(|(version, count)| VersionCount { version, count })
        .collect();
    
    let fleet_health = FleetHealth {
        total_agents: agents.len() as u32,
        healthy: healthy_count,
        unhealthy: unhealthy_count,
        unknown: unknown_count,
        version_distribution,
    };
    
    (StatusCode::OK, Json(fleet_health)).into_response()
}

/// Check health of all registered agents
pub async fn check_all_agents(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Get all agents
    let agents = match AgentRepository::list(pool).await {
        Ok(agents) => agents,
        Err(e) => {
            error!("Failed to list agents for health check: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to list agents"
            }))).into_response();
        }
    };
    
    let mut results = Vec::new();
    let mut healthy_count = 0;
    let mut unhealthy_count = 0;
    let mut unreachable_count = 0;
    
    // Check each agent
    for agent in agents {
        let result = check_agent_health(&state.http_client, &agent.id, &agent.name, &agent.url).await;
        
        // Update agent status in database
        let status_str = result.status.to_string();
        if let Err(e) = AgentRepository::update_status(
            pool,
            &agent.id,
            &status_str,
            result.vector_version.as_deref(),
        ).await {
            warn!("Failed to update agent status: {}", e);
        }
        
        // Record health check
        if let Err(e) = AgentRepository::record_health_check(
            pool,
            &agent.id,
            matches!(result.status, AgentStatus::Healthy),
            result.latency_ms.map(|l| l as i64),
            result.error.as_deref(),
        ).await {
            warn!("Failed to record health check: {}", e);
        }
        
        match result.status {
            AgentStatus::Healthy => healthy_count += 1,
            AgentStatus::Unhealthy => unhealthy_count += 1,
            AgentStatus::Unreachable | AgentStatus::Unknown => unreachable_count += 1,
        }
        
        results.push(result);
    }
    
    let summary = HealthSummary {
        total: results.len(),
        healthy: healthy_count,
        unhealthy: unhealthy_count,
        unreachable: unreachable_count,
        agents: results,
    };
    
    info!(
        "Health check complete: {}/{} healthy, {} unhealthy, {} unreachable",
        summary.healthy, summary.total, summary.unhealthy, summary.unreachable
    );
    
    (StatusCode::OK, Json(summary)).into_response()
}

/// Check health of a single agent
async fn check_agent_health(
    client: &reqwest::Client,
    agent_id: &str,
    agent_name: &str,
    agent_url: &str,
) -> AgentHealthResult {
    let health_url = format!("{}/health", agent_url.trim_end_matches('/'));
    
    let start = std::time::Instant::now();
    
    let health_response = client.get(&health_url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;
    
    let latency = start.elapsed().as_millis() as u64;
    
    match health_response {
        Ok(response) if response.status().is_success() => {
            // Agent is healthy, try to get version
            let version = get_vector_version(client, agent_url).await;
            
            AgentHealthResult {
                agent_id: agent_id.to_string(),
                agent_name: agent_name.to_string(),
                url: agent_url.to_string(),
                status: AgentStatus::Healthy,
                latency_ms: Some(latency),
                vector_version: version,
                error: None,
            }
        }
        Ok(response) => {
            // Agent responded but with an error status
            AgentHealthResult {
                agent_id: agent_id.to_string(),
                agent_name: agent_name.to_string(),
                url: agent_url.to_string(),
                status: AgentStatus::Unhealthy,
                latency_ms: Some(latency),
                vector_version: None,
                error: Some(format!("Health check returned status: {}", response.status())),
            }
        }
        Err(e) => {
            // Agent is unreachable
            AgentHealthResult {
                agent_id: agent_id.to_string(),
                agent_name: agent_name.to_string(),
                url: agent_url.to_string(),
                status: AgentStatus::Unreachable,
                latency_ms: None,
                vector_version: None,
                error: Some(format!("Connection failed: {}", e)),
            }
        }
    }
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

// =============================================================================
// Metrics Endpoints
// =============================================================================

/// Aggregated metrics response
#[derive(Debug, Serialize)]
pub struct AggregatedMetrics {
    pub total_agents: usize,
    pub total_events_processed: u64,
    pub total_bytes_processed: u64,
    pub agents: Vec<AgentMetrics>,
    pub collected_at: String,
}

/// Get metrics for all agents
pub async fn get_all_metrics(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    let agents = match AgentRepository::list(pool).await {
        Ok(agents) => agents,
        Err(e) => {
            error!("Failed to list agents for metrics: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to list agents"
            }))).into_response();
        }
    };
    
    let mut metrics = Vec::new();
    let mut total_events: u64 = 0;
    let mut total_bytes: u64 = 0;
    
    // Fetch metrics from each agent in parallel
    let mut handles = Vec::new();
    
    for agent in &agents {
        let client = state.http_client.clone();
        let agent_id = agent.id.clone();
        let agent_url = agent.url.clone();
        
        handles.push(tokio::spawn(async move {
            fetch_agent_metrics(&client, &agent_id, &agent_url).await
        }));
    }
    
    for handle in handles {
        if let Ok(m) = handle.await {
            total_events += m.events_processed_total.unwrap_or(0);
            total_bytes += m.bytes_processed_total.unwrap_or(0);
            metrics.push(m);
        }
    }
    
    let response = AggregatedMetrics {
        total_agents: agents.len(),
        total_events_processed: total_events,
        total_bytes_processed: total_bytes,
        agents: metrics,
        collected_at: chrono::Utc::now().to_rfc3339(),
    };
    
    (StatusCode::OK, Json(response)).into_response()
}

/// Get metrics for a specific agent
pub async fn get_agent_metrics(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
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
    
    let metrics = fetch_agent_metrics(&state.http_client, &agent.id, &agent.url).await;
    
    (StatusCode::OK, Json(metrics)).into_response()
}

// =============================================================================
// Topology Endpoints
// =============================================================================

/// Component info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInfo {
    pub component_id: String,
    pub component_type: String,  // source, transform, sink
    pub component_kind: String,  // e.g., demo_logs, filter, console
}

/// Agent topology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTopology {
    pub agent_id: String,
    pub agent_name: String,
    pub components: Vec<ComponentInfo>,
}

/// Aggregated topology from all agents
#[derive(Debug, Serialize)]
pub struct AggregatedTopology {
    pub total_agents: usize,
    pub total_components: usize,
    pub sources_count: usize,
    pub transforms_count: usize,
    pub sinks_count: usize,
    pub agents: Vec<AgentTopology>,
}

/// Get aggregated topology from all agents
pub async fn get_aggregated_topology(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    let agents = match AgentRepository::list(pool).await {
        Ok(agents) => agents,
        Err(e) => {
            error!("Failed to list agents for topology: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to list agents"
            }))).into_response();
        }
    };
    
    let mut topologies = Vec::new();
    let mut total_components = 0;
    let mut sources_count = 0;
    let mut transforms_count = 0;
    let mut sinks_count = 0;
    
    // Fetch topology from each agent
    for agent in &agents {
        if let Some(topology) = fetch_agent_topology(&state.http_client, &agent.id, &agent.name, &agent.url).await {
            for comp in &topology.components {
                total_components += 1;
                match comp.component_type.as_str() {
                    "source" => sources_count += 1,
                    "transform" => transforms_count += 1,
                    "sink" => sinks_count += 1,
                    _ => {}
                }
            }
            topologies.push(topology);
        }
    }
    
    let response = AggregatedTopology {
        total_agents: agents.len(),
        total_components,
        sources_count,
        transforms_count,
        sinks_count,
        agents: topologies,
    };
    
    (StatusCode::OK, Json(response)).into_response()
}

/// Fetch topology from an agent
async fn fetch_agent_topology(
    client: &reqwest::Client,
    agent_id: &str,
    agent_name: &str,
    agent_url: &str,
) -> Option<AgentTopology> {
    let graphql_url = format!("{}/graphql", agent_url.trim_end_matches('/'));
    
    let query = r#"
        query {
            components {
                edges {
                    node {
                        componentId
                        componentType
                        componentKind
                    }
                }
            }
        }
    "#;
    
    let body = serde_json::json!({ "query": query });
    
    let response = client.post(&graphql_url)
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .ok()?;
    
    let json: serde_json::Value = response.json().await.ok()?;
    
    let mut components = Vec::new();
    
    if let Some(edges) = json["data"]["components"]["edges"].as_array() {
        for edge in edges {
            if let (Some(id), Some(ctype), Some(kind)) = (
                edge["node"]["componentId"].as_str(),
                edge["node"]["componentType"].as_str(),
                edge["node"]["componentKind"].as_str(),
            ) {
                components.push(ComponentInfo {
                    component_id: id.to_string(),
                    component_type: ctype.to_lowercase(),
                    component_kind: kind.to_string(),
                });
            }
        }
    }
    
    Some(AgentTopology {
        agent_id: agent_id.to_string(),
        agent_name: agent_name.to_string(),
        components,
    })
}

// =============================================================================
// Health History Endpoints
// =============================================================================

/// Health check history entry
#[derive(Debug, Serialize)]
pub struct HealthHistoryEntry {
    pub checked_at: String,
    pub healthy: bool,
    pub latency_ms: Option<i64>,
    pub error: Option<String>,
}

/// Get health check history for an agent
pub async fn get_agent_health_history(
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
    
    let history = match AgentRepository::get_health_checks(pool, &agent_id, 100).await {
        Ok(checks) => checks,
        Err(e) => {
            error!("Failed to get health history: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get health history"
            }))).into_response();
        }
    };
    
    let entries: Vec<HealthHistoryEntry> = history.into_iter().map(|h| {
        HealthHistoryEntry {
            checked_at: h.checked_at,
            healthy: h.healthy,
            latency_ms: h.latency_ms,
            error: h.error,
        }
    }).collect();
    
    (StatusCode::OK, Json(serde_json::json!({
        "agent_id": agent_id,
        "history": entries
    }))).into_response()
}

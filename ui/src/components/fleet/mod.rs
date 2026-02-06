//! Fleet Management Components
//!
//! Components for managing Vector agents and viewing topology.

use serde::{Deserialize, Serialize};

mod agents_list;
mod agent_detail;
mod topology;

pub use agents_list::AgentsList;
pub use agent_detail::AgentDetail;
pub use topology::TopologyView;

/// Agent info for display
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub url: String,
    pub status: AgentStatus,
    pub version: Option<String>,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub last_seen: Option<String>,
    pub latency_ms: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

impl AgentStatus {
    /// CSS class for status dot
    pub fn dot_class(&self) -> &'static str {
        match self {
            AgentStatus::Healthy => "healthy",
            AgentStatus::Unhealthy => "unhealthy",
            AgentStatus::Unknown => "unknown",
        }
    }
    
    /// CSS class for text color
    pub fn text_class(&self) -> &'static str {
        match self {
            AgentStatus::Healthy => "text-green-400",
            AgentStatus::Unhealthy => "text-red-400",
            AgentStatus::Unknown => "text-theme-muted",
        }
    }
    
    /// Human-readable label
    pub fn label(&self) -> &'static str {
        match self {
            AgentStatus::Healthy => "Healthy",
            AgentStatus::Unhealthy => "Unhealthy",
            AgentStatus::Unknown => "Unknown",
        }
    }
}

/// Fetch agents from API
pub async fn fetch_agents() -> Result<Vec<AgentInfo>, String> {
    let base_url = web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_else(|| "http://localhost:8080".to_string());
    
    let response = gloo_net::http::Request::get(&format!("{}/api/v1/agents", base_url))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json::<Vec<AgentInfo>>().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Ok(vec![])
    }
}

/// Delete an agent
pub async fn delete_agent(id: &str) -> Result<(), String> {
    let base_url = web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_else(|| "http://localhost:8080".to_string());
    
    let response = gloo_net::http::Request::delete(&format!("{}/api/v1/agents/{}", base_url, id))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        Ok(())
    } else {
        Err(format!("Failed to delete agent: {}", response.status()))
    }
}

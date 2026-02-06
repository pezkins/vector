//! Health Monitoring Service
//!
//! Background service that monitors the health of all registered Vector agents.
//! - Periodically polls agent health endpoints
//! - Records health check results in the database
//! - Updates agent status (healthy/unhealthy/unreachable)
//! - Collects metrics from agents

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::db::repository::AgentRepository;

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub agent_id: String,
    pub agent_name: String,
    pub healthy: bool,
    pub latency_ms: Option<i64>,
    pub error: Option<String>,
    pub vector_version: Option<String>,
    pub uptime_seconds: Option<u64>,
    pub components_running: Option<u32>,
}

/// Agent metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMetrics {
    pub agent_id: String,
    pub events_processed_total: Option<u64>,
    pub events_in_rate: Option<f64>,
    pub events_out_rate: Option<f64>,
    pub bytes_processed_total: Option<u64>,
    pub component_errors_total: Option<u64>,
    pub uptime_seconds: Option<u64>,
}

/// Aggregated health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    pub total_agents: u32,
    pub healthy_agents: u32,
    pub unhealthy_agents: u32,
    pub unreachable_agents: u32,
    pub last_check: String,
}

/// Health monitor configuration
#[derive(Debug, Clone)]
pub struct HealthMonitorConfig {
    /// How often to check agent health (in seconds)
    pub check_interval_secs: u64,
    /// Timeout for health check requests (in seconds)
    pub timeout_secs: u64,
    /// Number of failed checks before marking unreachable
    pub failure_threshold: u32,
}

impl Default for HealthMonitorConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 30,
            timeout_secs: 10,
            failure_threshold: 3,
        }
    }
}

/// Health monitoring service
pub struct HealthMonitor {
    db: Database,
    http_client: reqwest::Client,
    config: HealthMonitorConfig,
    running: Arc<RwLock<bool>>,
    latest_results: Arc<RwLock<Vec<HealthCheckResult>>>,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new(db: Database, config: HealthMonitorConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            db,
            http_client,
            config,
            running: Arc::new(RwLock::new(false)),
            latest_results: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Start the background health monitoring task
    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let monitor = self.clone();
        
        tokio::spawn(async move {
            {
                let mut running = monitor.running.write().await;
                *running = true;
            }
            
            info!(
                "Starting health monitor (interval: {}s, timeout: {}s)",
                monitor.config.check_interval_secs,
                monitor.config.timeout_secs
            );
            
            let mut interval = tokio::time::interval(Duration::from_secs(monitor.config.check_interval_secs));
            
            loop {
                interval.tick().await;
                
                {
                    let running = monitor.running.read().await;
                    if !*running {
                        info!("Health monitor stopping");
                        break;
                    }
                }
                
                monitor.check_all_agents().await;
            }
        })
    }
    
    /// Stop the health monitor
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }
    
    /// Check health of all registered agents
    pub async fn check_all_agents(&self) {
        debug!("Running health check for all agents");
        
        let pool = self.db.pool();
        
        // Get all agents
        let agents = match AgentRepository::list(pool).await {
            Ok(agents) => agents,
            Err(e) => {
                error!("Failed to list agents for health check: {}", e);
                return;
            }
        };
        
        if agents.is_empty() {
            debug!("No agents to check");
            return;
        }
        
        let mut results = Vec::new();
        
        // Check each agent in parallel
        let mut handles = Vec::new();
        
        for agent in agents {
            let client = self.http_client.clone();
            let pool = pool.clone();
            
            let handle = tokio::spawn(async move {
                let result = check_agent_health(&client, &agent.id, &agent.name, &agent.url).await;
                
                // Record the health check
                let _ = AgentRepository::record_health_check(
                    &pool,
                    &agent.id,
                    result.healthy,
                    result.latency_ms,
                    result.error.as_deref(),
                ).await;
                
                // Update agent status
                let status = if result.healthy { "healthy" } else { "unhealthy" };
                let _ = AgentRepository::update_status(
                    &pool,
                    &agent.id,
                    status,
                    result.vector_version.as_deref(),
                ).await;
                
                result
            });
            
            handles.push(handle);
        }
        
        // Wait for all checks to complete
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }
        
        // Store latest results
        {
            let mut latest = self.latest_results.write().await;
            *latest = results;
        }
        
        debug!("Health check completed");
    }
    
    /// Get the latest health check results
    pub async fn get_latest_results(&self) -> Vec<HealthCheckResult> {
        self.latest_results.read().await.clone()
    }
    
    /// Get health summary
    pub async fn get_summary(&self) -> HealthSummary {
        let results = self.latest_results.read().await;
        
        let total = results.len() as u32;
        let healthy = results.iter().filter(|r| r.healthy).count() as u32;
        let unhealthy = total - healthy;
        
        HealthSummary {
            total_agents: total,
            healthy_agents: healthy,
            unhealthy_agents: unhealthy,
            unreachable_agents: results.iter().filter(|r| r.error.is_some()).count() as u32,
            last_check: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Check health of a single agent
async fn check_agent_health(
    client: &reqwest::Client,
    agent_id: &str,
    agent_name: &str,
    agent_url: &str,
) -> HealthCheckResult {
    let health_url = format!("{}/health", agent_url.trim_end_matches('/'));
    
    let start = std::time::Instant::now();
    
    match client.get(&health_url).send().await {
        Ok(response) => {
            let latency = start.elapsed().as_millis() as i64;
            
            if response.status().is_success() {
                // Try to get more info from the GraphQL API
                let (version, uptime, components) = get_agent_info(client, agent_url).await;
                
                HealthCheckResult {
                    agent_id: agent_id.to_string(),
                    agent_name: agent_name.to_string(),
                    healthy: true,
                    latency_ms: Some(latency),
                    error: None,
                    vector_version: version,
                    uptime_seconds: uptime,
                    components_running: components,
                }
            } else {
                HealthCheckResult {
                    agent_id: agent_id.to_string(),
                    agent_name: agent_name.to_string(),
                    healthy: false,
                    latency_ms: Some(latency),
                    error: Some(format!("HTTP {}", response.status())),
                    vector_version: None,
                    uptime_seconds: None,
                    components_running: None,
                }
            }
        }
        Err(e) => {
            let error_msg = if e.is_timeout() {
                "Connection timeout".to_string()
            } else if e.is_connect() {
                "Connection refused".to_string()
            } else {
                e.to_string()
            };
            
            warn!("Health check failed for {}: {}", agent_name, error_msg);
            
            HealthCheckResult {
                agent_id: agent_id.to_string(),
                agent_name: agent_name.to_string(),
                healthy: false,
                latency_ms: None,
                error: Some(error_msg),
                vector_version: None,
                uptime_seconds: None,
                components_running: None,
            }
        }
    }
}

/// Get additional info from agent's GraphQL API
async fn get_agent_info(
    client: &reqwest::Client,
    agent_url: &str,
) -> (Option<String>, Option<u64>, Option<u32>) {
    let graphql_url = format!("{}/graphql", agent_url.trim_end_matches('/'));
    
    let query = r#"
        query {
            meta {
                versionString
                uptimeSeconds
            }
            componentInfo: components {
                totalCount
            }
        }
    "#;
    
    let body = serde_json::json!({ "query": query });
    
    match client.post(&graphql_url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(response) => {
            if let Ok(json) = response.json::<serde_json::Value>().await {
                let version = json["data"]["meta"]["versionString"]
                    .as_str()
                    .map(|s| s.to_string());
                
                let uptime = json["data"]["meta"]["uptimeSeconds"]
                    .as_u64();
                
                let components = json["data"]["componentInfo"]["totalCount"]
                    .as_u64()
                    .map(|n| n as u32);
                
                (version, uptime, components)
            } else {
                (None, None, None)
            }
        }
        Err(_) => (None, None, None),
    }
}

/// Fetch metrics from an agent
pub async fn fetch_agent_metrics(
    client: &reqwest::Client,
    agent_id: &str,
    agent_url: &str,
) -> AgentMetrics {
    let graphql_url = format!("{}/graphql", agent_url.trim_end_matches('/'));
    
    let query = r#"
        query {
            meta {
                uptimeSeconds
            }
            componentInfo: components {
                edges {
                    node {
                        componentId
                        componentType
                        ... on Source {
                            metrics {
                                sentEventsTotal {
                                    sentEventsTotal
                                }
                                sentBytesTotal {
                                    sentBytesTotal
                                }
                            }
                        }
                        ... on Transform {
                            metrics {
                                sentEventsTotal {
                                    sentEventsTotal
                                }
                            }
                        }
                        ... on Sink {
                            metrics {
                                sentEventsTotal {
                                    sentEventsTotal
                                }
                                sentBytesTotal {
                                    sentBytesTotal
                                }
                            }
                        }
                    }
                }
            }
        }
    "#;
    
    let body = serde_json::json!({ "query": query });
    
    match client.post(&graphql_url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(response) => {
            if let Ok(json) = response.json::<serde_json::Value>().await {
                let uptime = json["data"]["meta"]["uptimeSeconds"].as_u64();
                
                // Aggregate metrics from all components
                let mut total_events: u64 = 0;
                let mut total_bytes: u64 = 0;
                
                if let Some(edges) = json["data"]["componentInfo"]["edges"].as_array() {
                    for edge in edges {
                        if let Some(sent) = edge["node"]["metrics"]["sentEventsTotal"]["sentEventsTotal"].as_u64() {
                            total_events += sent;
                        }
                        if let Some(bytes) = edge["node"]["metrics"]["sentBytesTotal"]["sentBytesTotal"].as_u64() {
                            total_bytes += bytes;
                        }
                    }
                }
                
                AgentMetrics {
                    agent_id: agent_id.to_string(),
                    events_processed_total: Some(total_events),
                    bytes_processed_total: Some(total_bytes),
                    uptime_seconds: uptime,
                    ..Default::default()
                }
            } else {
                AgentMetrics {
                    agent_id: agent_id.to_string(),
                    ..Default::default()
                }
            }
        }
        Err(_) => AgentMetrics {
            agent_id: agent_id.to_string(),
            ..Default::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_health_monitor_config_default() {
        let config = HealthMonitorConfig::default();
        assert_eq!(config.check_interval_secs, 30);
        assert_eq!(config.timeout_secs, 10);
        assert_eq!(config.failure_threshold, 3);
    }
    
    #[test]
    fn test_health_summary() {
        let summary = HealthSummary {
            total_agents: 5,
            healthy_agents: 3,
            unhealthy_agents: 2,
            unreachable_agents: 1,
            last_check: "2026-02-03T12:00:00Z".to_string(),
        };
        
        assert_eq!(summary.total_agents, 5);
        assert_eq!(summary.healthy_agents, 3);
    }
}

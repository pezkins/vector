//! Vectorize Agent Sidecar
//!
//! A lightweight agent that runs alongside Vector instances to:
//! - Register with the Vectorize control plane
//! - Pull configuration updates
//! - Report health status
//! - Enable remote management

use std::path::PathBuf;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Vectorize control plane URL
    pub control_plane_url: String,
    /// Agent name (defaults to hostname)
    pub name: Option<String>,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Worker group to join (optional - can be assigned by control plane)
    pub group: Option<String>,
    /// Vector API URL (local Vector instance)
    pub vector_url: String,
    /// Path to Vector config file (for writing updates)
    pub vector_config_path: PathBuf,
    /// Health check interval in seconds
    pub health_interval: u64,
    /// Config poll interval in seconds
    pub config_poll_interval: u64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            control_plane_url: "http://localhost:8080".to_string(),
            name: None,
            api_key: None,
            group: None,
            vector_url: "http://localhost:8686".to_string(),
            vector_config_path: PathBuf::from("/etc/vector/vector.toml"),
            health_interval: 30,
            config_poll_interval: 60,
        }
    }
}

/// Agent state
pub struct Agent {
    config: AgentConfig,
    client: reqwest::Client,
    agent_id: Option<String>,
    current_config_version: Option<String>,
}

impl Agent {
    /// Create a new agent
    pub fn new(config: AgentConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            config,
            client,
            agent_id: None,
            current_config_version: None,
        }
    }
    
    /// Get the agent name (from config or hostname)
    fn get_name(&self) -> String {
        self.config.name.clone().unwrap_or_else(|| {
            hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| "unknown-agent".to_string())
        })
    }
    
    /// Register with the control plane
    pub async fn register(&mut self) -> Result<(), AgentError> {
        let name = self.get_name();
        info!("Registering agent '{}' with control plane at {}", name, self.config.control_plane_url);
        
        let url = format!("{}/api/v1/agents", self.config.control_plane_url);
        
        let body = serde_json::json!({
            "name": name,
            "url": self.config.vector_url,
            "group_id": self.config.group,
        });
        
        let mut request = self.client.post(&url).json(&body);
        
        if let Some(ref api_key) = self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let response = request.send().await.map_err(|e| {
            AgentError::Network(format!("Failed to connect to control plane: {}", e))
        })?;
        
        if response.status().is_success() {
            let result: serde_json::Value = response.json().await.map_err(|e| {
                AgentError::Parse(format!("Failed to parse response: {}", e))
            })?;
            
            if let Some(agent) = result.get("agent") {
                if let Some(id) = agent.get("id").and_then(|v| v.as_str()) {
                    self.agent_id = Some(id.to_string());
                    info!("Registered successfully with ID: {}", id);
                }
            }
            
            Ok(())
        } else if response.status().as_u16() == 409 {
            // Already registered, try to find our ID
            warn!("Agent already registered, fetching existing registration");
            self.fetch_self().await
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(AgentError::Registration(format!("Registration failed ({}): {}", status, body)))
        }
    }
    
    /// Fetch our own agent info
    async fn fetch_self(&mut self) -> Result<(), AgentError> {
        let name = self.get_name();
        let url = format!("{}/api/v1/agents", self.config.control_plane_url);
        
        let mut request = self.client.get(&url);
        
        if let Some(ref api_key) = self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let response = request.send().await.map_err(|e| {
            AgentError::Network(format!("Failed to fetch agents: {}", e))
        })?;
        
        if response.status().is_success() {
            let agents: Vec<serde_json::Value> = response.json().await.map_err(|e| {
                AgentError::Parse(format!("Failed to parse agents: {}", e))
            })?;
            
            // Find our agent by name (unique identifier)
            for agent in agents {
                if agent.get("name").and_then(|v| v.as_str()) == Some(&name) {
                    if let Some(id) = agent.get("id").and_then(|v| v.as_str()) {
                        self.agent_id = Some(id.to_string());
                        info!("Found existing agent ID: {}", id);
                        return Ok(());
                    }
                }
            }
            
            Err(AgentError::Registration("Agent not found in registry".to_string()))
        } else {
            Err(AgentError::Network("Failed to fetch agent list".to_string()))
        }
    }
    
    /// Check for config updates and apply if needed
    pub async fn check_config_update(&mut self) -> Result<bool, AgentError> {
        let agent_id = self.agent_id.as_ref().ok_or_else(|| {
            AgentError::NotRegistered
        })?;
        
        // Get agent info to find group
        let url = format!("{}/api/v1/agents/{}", self.config.control_plane_url, agent_id);
        
        let mut request = self.client.get(&url);
        if let Some(ref api_key) = self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let response = request.send().await.map_err(|e| {
            AgentError::Network(format!("Failed to fetch agent info: {}", e))
        })?;
        
        if !response.status().is_success() {
            return Err(AgentError::Network("Failed to fetch agent info".to_string()));
        }
        
        let agent: serde_json::Value = response.json().await.map_err(|e| {
            AgentError::Parse(format!("Failed to parse agent: {}", e))
        })?;
        
        let group_id = match agent.get("group_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => {
                info!("Agent not assigned to a group, skipping config check");
                return Ok(false);
            }
        };
        
        // Fetch group config
        let url = format!("{}/api/v1/groups/{}/config", self.config.control_plane_url, group_id);
        
        let mut request = self.client.get(&url);
        if let Some(ref api_key) = self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let response = request.send().await.map_err(|e| {
            AgentError::Network(format!("Failed to fetch config: {}", e))
        })?;
        
        if !response.status().is_success() {
            return Err(AgentError::Network("Failed to fetch group config".to_string()));
        }
        
        let config_response: serde_json::Value = response.json().await.map_err(|e| {
            AgentError::Parse(format!("Failed to parse config: {}", e))
        })?;
        
        let new_version = config_response.get("version").and_then(|v| v.as_str());
        let new_config = config_response.get("config").and_then(|v| v.as_str());
        
        // Check if config has changed
        if new_version == self.current_config_version.as_deref() {
            return Ok(false);
        }
        
        if let Some(config) = new_config {
            info!("New configuration available (version: {:?})", new_version);
            
            // Write new config to file
            std::fs::write(&self.config.vector_config_path, config).map_err(|e| {
                AgentError::ConfigWrite(format!("Failed to write config: {}", e))
            })?;
            
            self.current_config_version = new_version.map(|s| s.to_string());
            
            info!("Configuration updated successfully. Vector will auto-reload via --watch-config");
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Report health status to control plane
    pub async fn report_health(&self) -> Result<(), AgentError> {
        // Check Vector health
        let vector_health = self.check_vector_health().await;
        
        info!("Vector health: {:?}", vector_health);
        
        // In a full implementation, we'd POST this to the control plane
        // For now, the control plane polls agents directly
        
        Ok(())
    }
    
    /// Check local Vector instance health
    async fn check_vector_health(&self) -> VectorHealth {
        let url = format!("{}/health", self.config.vector_url);
        
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => VectorHealth::Healthy,
            Ok(_) => VectorHealth::Unhealthy,
            Err(_) => VectorHealth::Unreachable,
        }
    }
    
    /// Run the agent main loop
    pub async fn run(&mut self) -> Result<(), AgentError> {
        info!("Starting Vectorize agent...");
        
        // Register with control plane
        self.register().await?;
        
        let health_interval = Duration::from_secs(self.config.health_interval);
        let config_interval = Duration::from_secs(self.config.config_poll_interval);
        
        let mut health_timer = tokio::time::interval(health_interval);
        let mut config_timer = tokio::time::interval(config_interval);
        
        loop {
            tokio::select! {
                _ = health_timer.tick() => {
                    if let Err(e) = self.report_health().await {
                        warn!("Health report failed: {}", e);
                    }
                }
                _ = config_timer.tick() => {
                    match self.check_config_update().await {
                        Ok(true) => info!("Configuration updated"),
                        Ok(false) => {}  // No change
                        Err(e) => warn!("Config check failed: {}", e),
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("Shutting down agent...");
                    break;
                }
            }
        }
        
        Ok(())
    }
}

/// Vector health status
#[derive(Debug)]
pub enum VectorHealth {
    Healthy,
    Unhealthy,
    Unreachable,
}

/// Agent errors
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Registration error: {0}")]
    Registration(String),
    
    #[error("Agent not registered")]
    NotRegistered,
    
    #[error("Config write error: {0}")]
    ConfigWrite(String),
}

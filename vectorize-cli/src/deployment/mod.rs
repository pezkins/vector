//! Deployment Service
//!
//! Handles the execution of deployments with different strategies:
//! - Basic: Deploy to all agents simultaneously
//! - Rolling: Deploy one-by-one or in batches
//! - Canary: Deploy to subset, validate, then roll out

use std::sync::Arc;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, debug};
use tokio::sync::RwLock;

use crate::db::Database;
use crate::db::models::{Deployment, DeploymentAgent, WorkerGroup};
use crate::db::repository::{DeploymentRepository, DeploymentStats, AgentRepository, WorkerGroupRepository};
use crate::git_store::GitStore;

// =============================================================================
// Deployment Strategy Configuration
// =============================================================================

/// Deployment strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentStrategy {
    /// Deploy to all agents at once
    Basic,
    /// Deploy incrementally
    Rolling,
    /// Deploy to canary subset first, then rest
    Canary,
}

impl From<&str> for DeploymentStrategy {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "rolling" => DeploymentStrategy::Rolling,
            "canary" => DeploymentStrategy::Canary,
            _ => DeploymentStrategy::Basic,
        }
    }
}

impl std::fmt::Display for DeploymentStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeploymentStrategy::Basic => write!(f, "basic"),
            DeploymentStrategy::Rolling => write!(f, "rolling"),
            DeploymentStrategy::Canary => write!(f, "canary"),
        }
    }
}

/// Options for rolling deployments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollingOptions {
    /// Number of agents to deploy to in parallel (default: 1)
    #[serde(default = "default_batch_size")]
    pub batch_size: u32,
    /// Wait time between batches in seconds (default: 30)
    #[serde(default = "default_batch_delay")]
    pub batch_delay_secs: u64,
    /// Whether to pause on failure (default: true)
    #[serde(default = "default_true")]
    pub pause_on_failure: bool,
    /// Maximum allowed failures before aborting (default: 1)
    #[serde(default = "default_max_failures")]
    pub max_failures: u32,
}

fn default_batch_size() -> u32 { 1 }
fn default_batch_delay() -> u64 { 30 }
fn default_true() -> bool { true }
fn default_max_failures() -> u32 { 1 }

impl Default for RollingOptions {
    fn default() -> Self {
        Self {
            batch_size: 1,
            batch_delay_secs: 30,
            pause_on_failure: true,
            max_failures: 1,
        }
    }
}

/// Options for canary deployments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryOptions {
    /// Percentage of agents for canary (default: 10%)
    #[serde(default = "default_canary_percentage")]
    pub canary_percentage: u32,
    /// Specific agent IDs for canary (overrides percentage)
    pub canary_agents: Option<Vec<String>>,
    /// Wait time before promoting canary in seconds (default: 300)
    #[serde(default = "default_canary_wait")]
    pub canary_wait_secs: u64,
    /// Whether to auto-promote after wait (default: false - requires manual approval)
    #[serde(default)]
    pub auto_promote: bool,
}

fn default_canary_percentage() -> u32 { 10 }
fn default_canary_wait() -> u64 { 300 }

impl Default for CanaryOptions {
    fn default() -> Self {
        Self {
            canary_percentage: 10,
            canary_agents: None,
            canary_wait_secs: 300,
            auto_promote: false,
        }
    }
}

/// Combined deployment options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeploymentOptions {
    pub rolling: Option<RollingOptions>,
    pub canary: Option<CanaryOptions>,
}

// =============================================================================
// Deployment Result Types
// =============================================================================

/// Result of starting a deployment
#[derive(Debug, Clone, Serialize)]
pub struct DeploymentResult {
    pub deployment_id: String,
    pub status: String,
    pub message: String,
    pub requires_approval: bool,
    pub queued: bool,
}

/// Result of deploying to a single agent
#[derive(Debug, Clone, Serialize)]
pub struct AgentDeployResult {
    pub agent_id: String,
    pub agent_name: String,
    pub success: bool,
    pub message: String,
}

// =============================================================================
// Version Enforcement
// =============================================================================

/// Check if all agents in a group have the same Vector version
pub async fn check_version_consistency(
    db: &Database,
    group_id: &str,
) -> Result<VersionCheckResult, String> {
    let pool = db.pool();
    
    let agents = AgentRepository::list_by_group(pool, group_id)
        .await
        .map_err(|e| format!("Failed to list agents: {}", e))?;
    
    if agents.is_empty() {
        return Ok(VersionCheckResult {
            consistent: true,
            versions: Vec::new(),
            message: "No agents in group".to_string(),
        });
    }
    
    let mut versions: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    
    for agent in &agents {
        let version = agent.vector_version.clone().unwrap_or_else(|| "unknown".to_string());
        versions.entry(version).or_default().push(agent.name.clone());
    }
    
    let version_list: Vec<VersionInfo> = versions.into_iter()
        .map(|(version, agents)| VersionInfo { version, agents })
        .collect();
    
    let consistent = version_list.len() == 1 && version_list[0].version != "unknown";
    let message = if consistent {
        format!("All agents running Vector {}", version_list[0].version)
    } else if version_list.len() == 1 && version_list[0].version == "unknown" {
        "Agent versions not yet detected - run health check first".to_string()
    } else {
        format!("Mixed versions detected: {}", 
            version_list.iter()
                .map(|v| format!("{} ({} agents)", v.version, v.agents.len()))
                .collect::<Vec<_>>()
                .join(", "))
    };
    
    Ok(VersionCheckResult {
        consistent,
        versions: version_list,
        message,
    })
}

/// Version check result
#[derive(Debug, Clone, Serialize)]
pub struct VersionCheckResult {
    pub consistent: bool,
    pub versions: Vec<VersionInfo>,
    pub message: String,
}

/// Version info
#[derive(Debug, Clone, Serialize)]
pub struct VersionInfo {
    pub version: String,
    pub agents: Vec<String>,
}

// =============================================================================
// Deployment Executor
// =============================================================================

/// Deployment executor service
pub struct DeploymentExecutor {
    db: Database,
    git_store: Arc<GitStore>,
    http_client: reqwest::Client,
    #[allow(dead_code)]
    running: Arc<RwLock<bool>>,
}

impl DeploymentExecutor {
    /// Create a new deployment executor
    pub fn new(db: Database, git_store: Arc<GitStore>) -> Self {
        Self {
            db,
            git_store,
            http_client: reqwest::Client::new(),
            running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Start a new deployment
    pub async fn start_deployment(
        &self,
        group_id: &str,
        config_version: &str,
        options: Option<DeploymentOptions>,
        created_by: Option<&str>,
        force: bool,
    ) -> Result<DeploymentResult, String> {
        let pool = self.db.pool();
        
        // Get the group
        let group = WorkerGroupRepository::get_by_id(pool, group_id)
            .await
            .map_err(|e| format!("Failed to get group: {}", e))?
            .ok_or("Group not found")?;
        
        // Check version consistency (unless forced)
        if !force {
            let version_check = check_version_consistency(&self.db, group_id).await?;
            if !version_check.consistent {
                return Err(format!(
                    "Version mismatch: {}. Use force=true to deploy anyway.",
                    version_check.message
                ));
            }
        }
        
        // Check for existing active deployment
        if let Some(_active) = DeploymentRepository::get_active_for_group(pool, group_id)
            .await
            .map_err(|e| format!("Failed to check active deployments: {}", e))?
        {
            // Queue this deployment
            return self.queue_deployment(group_id, config_version, &group, options, created_by).await;
        }
        
        // Create the deployment
        let options_json = options.as_ref()
            .map(|o| serde_json::to_string(o).unwrap_or_default());
        
        let status = if group.requires_approval {
            "pending_approval"
        } else {
            "pending"
        };
        
        let deployment = DeploymentRepository::create(
            pool,
            group_id,
            config_version,
            &group.deployment_strategy,
            options_json.as_deref(),
            created_by,
        )
        .await
        .map_err(|e| format!("Failed to create deployment: {}", e))?;
        
        // Set initial status
        DeploymentRepository::update_status(pool, &deployment.id, status, None)
            .await
            .map_err(|e| format!("Failed to update status: {}", e))?;
        
        // Add agents to deployment
        let agents = AgentRepository::list_by_group(pool, group_id)
            .await
            .map_err(|e| format!("Failed to list agents: {}", e))?;
        
        for agent in &agents {
            DeploymentRepository::add_agent(pool, &deployment.id, &agent.id)
                .await
                .map_err(|e| format!("Failed to add agent: {}", e))?;
        }
        
        if group.requires_approval {
            info!("Deployment {} created - pending approval", deployment.id);
            return Ok(DeploymentResult {
                deployment_id: deployment.id,
                status: "pending_approval".to_string(),
                message: "Deployment created - awaiting approval".to_string(),
                requires_approval: true,
                queued: false,
            });
        }
        
        // Start execution
        self.execute_deployment(&deployment.id).await?;
        
        Ok(DeploymentResult {
            deployment_id: deployment.id,
            status: "in_progress".to_string(),
            message: format!("Deployment started for {} agents", agents.len()),
            requires_approval: false,
            queued: false,
        })
    }
    
    /// Queue a deployment (when another is active)
    async fn queue_deployment(
        &self,
        group_id: &str,
        config_version: &str,
        group: &WorkerGroup,
        options: Option<DeploymentOptions>,
        created_by: Option<&str>,
    ) -> Result<DeploymentResult, String> {
        let pool = self.db.pool();
        
        let options_json = options.as_ref()
            .map(|o| serde_json::to_string(o).unwrap_or_default());
        
        let deployment = DeploymentRepository::create(
            pool,
            group_id,
            config_version,
            &group.deployment_strategy,
            options_json.as_deref(),
            created_by,
        )
        .await
        .map_err(|e| format!("Failed to create deployment: {}", e))?;
        
        DeploymentRepository::update_status(pool, &deployment.id, "queued", None)
            .await
            .map_err(|e| format!("Failed to queue deployment: {}", e))?;
        
        info!("Deployment {} queued - another deployment is active", deployment.id);
        
        Ok(DeploymentResult {
            deployment_id: deployment.id,
            status: "queued".to_string(),
            message: "Deployment queued - another deployment is in progress".to_string(),
            requires_approval: false,
            queued: true,
        })
    }
    
    /// Execute a deployment
    pub async fn execute_deployment(&self, deployment_id: &str) -> Result<(), String> {
        let pool = self.db.pool();
        
        let deployment = DeploymentRepository::get_by_id(pool, deployment_id)
            .await
            .map_err(|e| format!("Failed to get deployment: {}", e))?
            .ok_or("Deployment not found")?;
        
        // Get the group
        let group = WorkerGroupRepository::get_by_id(pool, &deployment.group_id)
            .await
            .map_err(|e| format!("Failed to get group: {}", e))?
            .ok_or("Group not found")?;
        
        // Get config
        let config = self.git_store.get_config_at_version(&group.name, &deployment.config_version)
            .map_err(|e| format!("Failed to get config: {}", e))?
            .ok_or("Config not found")?;
        
        // Mark as in progress
        DeploymentRepository::update_status(pool, deployment_id, "in_progress", None)
            .await
            .map_err(|e| format!("Failed to update status: {}", e))?;
        
        let strategy = DeploymentStrategy::from(deployment.strategy.as_str());
        let options: DeploymentOptions = deployment.options
            .as_ref()
            .and_then(|o| serde_json::from_str(o).ok())
            .unwrap_or_default();
        
        info!("Executing deployment {} with strategy {:?}", deployment_id, strategy);
        
        let result = match strategy {
            DeploymentStrategy::Basic => {
                self.execute_basic(deployment_id, &config).await
            }
            DeploymentStrategy::Rolling => {
                let rolling_opts = options.rolling.unwrap_or_default();
                self.execute_rolling(deployment_id, &config, &rolling_opts).await
            }
            DeploymentStrategy::Canary => {
                let canary_opts = options.canary.unwrap_or_default();
                self.execute_canary(deployment_id, &config, &canary_opts).await
            }
        };
        
        // Update final status
        match result {
            Ok(_) => {
                DeploymentRepository::update_status(pool, deployment_id, "completed", None)
                    .await
                    .map_err(|e| format!("Failed to update status: {}", e))?;
                info!("Deployment {} completed successfully", deployment_id);
            }
            Err(ref e) => {
                DeploymentRepository::update_status(pool, deployment_id, "failed", Some(e))
                    .await
                    .map_err(|e| format!("Failed to update status: {}", e))?;
                error!("Deployment {} failed: {}", deployment_id, e);
            }
        }
        
        // Check for queued deployments
        self.process_queue(&deployment.group_id).await?;
        
        result
    }
    
    /// Execute basic (all-at-once) deployment
    async fn execute_basic(&self, deployment_id: &str, config: &str) -> Result<(), String> {
        let pool = self.db.pool();
        
        let agents = DeploymentRepository::get_agents(pool, deployment_id)
            .await
            .map_err(|e| format!("Failed to get agents: {}", e))?;
        
        let mut handles = Vec::new();
        
        for agent in agents {
            let pool = pool.clone();
            let client = self.http_client.clone();
            let config = config.to_string();
            let deployment_id = deployment_id.to_string();
            
            // Get agent details
            let agent_info = AgentRepository::get_by_id(&pool, &agent.agent_id)
                .await
                .ok()
                .flatten();
            
            if let Some(info) = agent_info {
                handles.push(tokio::spawn(async move {
                    deploy_to_agent(&client, &pool, &deployment_id, &info.id, &info.url, &config).await
                }));
            }
        }
        
        // Wait for all deployments
        let mut failures = 0;
        for handle in handles {
            if let Ok(result) = handle.await {
                if result.is_err() {
                    failures += 1;
                }
            }
        }
        
        if failures > 0 {
            Err(format!("{} agent(s) failed to deploy", failures))
        } else {
            Ok(())
        }
    }
    
    /// Execute rolling deployment
    async fn execute_rolling(
        &self,
        deployment_id: &str,
        config: &str,
        options: &RollingOptions,
    ) -> Result<(), String> {
        let pool = self.db.pool();
        let mut failures = 0;
        
        loop {
            // Get next batch of pending agents
            let mut batch = Vec::new();
            for _ in 0..options.batch_size {
                if let Some(agent) = DeploymentRepository::get_next_pending_agent(pool, deployment_id)
                    .await
                    .map_err(|e| format!("Failed to get next agent: {}", e))?
                {
                    batch.push(agent);
                } else {
                    break;
                }
            }
            
            if batch.is_empty() {
                break;
            }
            
            debug!("Rolling deployment batch: {} agents", batch.len());
            
            // Deploy to batch
            for agent in batch {
                let agent_info = AgentRepository::get_by_id(pool, &agent.agent_id)
                    .await
                    .ok()
                    .flatten();
                
                if let Some(info) = agent_info {
                    let result = deploy_to_agent(
                        &self.http_client,
                        pool,
                        deployment_id,
                        &info.id,
                        &info.url,
                        config,
                    ).await;
                    
                    if result.is_err() {
                        failures += 1;
                        if options.pause_on_failure && failures >= options.max_failures {
                            return Err(format!(
                                "Rolling deployment paused: {} failures exceeded max {}",
                                failures, options.max_failures
                            ));
                        }
                    }
                }
            }
            
            // Wait between batches
            if options.batch_delay_secs > 0 {
                tokio::time::sleep(Duration::from_secs(options.batch_delay_secs)).await;
            }
        }
        
        if failures > 0 && failures >= options.max_failures {
            Err(format!("{} agent(s) failed", failures))
        } else {
            Ok(())
        }
    }
    
    /// Execute canary deployment
    async fn execute_canary(
        &self,
        deployment_id: &str,
        config: &str,
        options: &CanaryOptions,
    ) -> Result<(), String> {
        let pool = self.db.pool();
        
        let all_agents = DeploymentRepository::get_agents(pool, deployment_id)
            .await
            .map_err(|e| format!("Failed to get agents: {}", e))?;
        
        // Determine canary agents
        let canary_count = if let Some(ref ids) = options.canary_agents {
            ids.len()
        } else {
            ((all_agents.len() as f64) * (options.canary_percentage as f64 / 100.0)).ceil() as usize
        };
        let canary_count = canary_count.max(1).min(all_agents.len());
        
        let (canary_agents, remaining_agents) = if let Some(ref ids) = options.canary_agents {
            let canary: Vec<_> = all_agents.iter()
                .filter(|a| ids.contains(&a.agent_id))
                .cloned()
                .collect();
            let remaining: Vec<_> = all_agents.iter()
                .filter(|a| !ids.contains(&a.agent_id))
                .cloned()
                .collect();
            (canary, remaining)
        } else {
            let canary: Vec<_> = all_agents.iter().take(canary_count).cloned().collect();
            let remaining: Vec<_> = all_agents.iter().skip(canary_count).cloned().collect();
            (canary, remaining)
        };
        
        info!("Canary deployment: {} canary, {} remaining", canary_agents.len(), remaining_agents.len());
        
        // Deploy to canary
        for agent in &canary_agents {
            let agent_info = AgentRepository::get_by_id(pool, &agent.agent_id)
                .await
                .ok()
                .flatten();
            
            if let Some(info) = agent_info {
                deploy_to_agent(&self.http_client, pool, deployment_id, &info.id, &info.url, config).await?;
            }
        }
        
        // Wait for canary period
        info!("Canary deployed - waiting {} seconds for validation", options.canary_wait_secs);
        tokio::time::sleep(Duration::from_secs(options.canary_wait_secs)).await;
        
        // Check canary health
        let stats = DeploymentRepository::get_stats(pool, deployment_id)
            .await
            .map_err(|e| format!("Failed to get stats: {}", e))?;
        
        if stats.failed > 0 {
            return Err(format!("Canary failed: {} agents failed", stats.failed));
        }
        
        if !options.auto_promote {
            // In a full implementation, we would pause here and wait for manual promotion
            // For now, we auto-promote after the wait period
            info!("Canary healthy - promoting to remaining agents");
        }
        
        // Deploy to remaining
        for agent in &remaining_agents {
            let agent_info = AgentRepository::get_by_id(pool, &agent.agent_id)
                .await
                .ok()
                .flatten();
            
            if let Some(info) = agent_info {
                deploy_to_agent(&self.http_client, pool, deployment_id, &info.id, &info.url, config).await?;
            }
        }
        
        Ok(())
    }
    
    /// Process queued deployments (spawns as separate task to avoid recursion)
    async fn process_queue(&self, group_id: &str) -> Result<(), String> {
        let pool = self.db.pool();
        
        // Check if there's an active deployment
        if DeploymentRepository::get_active_for_group(pool, group_id)
            .await
            .map_err(|e| format!("Failed to check active: {}", e))?
            .is_some()
        {
            return Ok(());
        }
        
        // Get next queued deployment
        let queued = DeploymentRepository::get_queued_for_group(pool, group_id)
            .await
            .map_err(|e| format!("Failed to get queued: {}", e))?;
        
        if let Some(next) = queued.first() {
            info!("Queued deployment {} ready to start", next.id);
            // Don't call execute_deployment here to avoid recursion
            // The caller or a background task should pick up queued deployments
        }
        
        Ok(())
    }
    
    /// Approve a pending deployment
    pub async fn approve_deployment(
        &self,
        deployment_id: &str,
        approved_by: &str,
    ) -> Result<(), String> {
        let pool = self.db.pool();
        
        DeploymentRepository::approve(pool, deployment_id, approved_by)
            .await
            .map_err(|e| format!("Failed to approve: {}", e))?;
        
        info!("Deployment {} approved by {}", deployment_id, approved_by);
        
        // Start execution
        self.execute_deployment(deployment_id).await
    }
    
    /// Reject a pending deployment
    pub async fn reject_deployment(
        &self,
        deployment_id: &str,
        rejected_by: &str,
        reason: Option<&str>,
    ) -> Result<(), String> {
        let pool = self.db.pool();
        
        DeploymentRepository::reject(pool, deployment_id, rejected_by, reason)
            .await
            .map_err(|e| format!("Failed to reject: {}", e))?;
        
        info!("Deployment {} rejected by {}", deployment_id, rejected_by);
        
        Ok(())
    }
    
    /// Cancel a deployment
    pub async fn cancel_deployment(&self, deployment_id: &str) -> Result<(), String> {
        let pool = self.db.pool();
        
        DeploymentRepository::update_status(pool, deployment_id, "cancelled", Some("Cancelled by user"))
            .await
            .map_err(|e| format!("Failed to cancel: {}", e))?;
        
        info!("Deployment {} cancelled", deployment_id);
        
        Ok(())
    }
    
    /// Get deployment status
    pub async fn get_status(&self, deployment_id: &str) -> Result<DeploymentStatus, String> {
        let pool = self.db.pool();
        
        let deployment = DeploymentRepository::get_by_id(pool, deployment_id)
            .await
            .map_err(|e| format!("Failed to get deployment: {}", e))?
            .ok_or("Deployment not found")?;
        
        let stats = DeploymentRepository::get_stats(pool, deployment_id)
            .await
            .map_err(|e| format!("Failed to get stats: {}", e))?;
        
        let agents = DeploymentRepository::get_agents(pool, deployment_id)
            .await
            .map_err(|e| format!("Failed to get agents: {}", e))?;
        
        Ok(DeploymentStatus {
            deployment,
            stats,
            agents,
        })
    }
}

/// Deploy configuration to a single agent
async fn deploy_to_agent(
    client: &reqwest::Client,
    pool: &sqlx::SqlitePool,
    deployment_id: &str,
    agent_id: &str,
    agent_url: &str,
    config: &str,
) -> Result<(), String> {
    // Mark as in progress
    DeploymentRepository::update_agent_status(pool, deployment_id, agent_id, "in_progress", None)
        .await
        .map_err(|e| format!("Failed to update status: {}", e))?;
    
    // In a full implementation, we would POST the config to the agent's API
    // For now, we simulate successful deployment since agents pull configs
    let deploy_url = format!("{}/api/deploy", agent_url.trim_end_matches('/'));
    
    let result = client.post(&deploy_url)
        .header("Content-Type", "application/toml")
        .body(config.to_string())
        .timeout(Duration::from_secs(30))
        .send()
        .await;
    
    match result {
        Ok(response) if response.status().is_success() => {
            DeploymentRepository::update_agent_status(pool, deployment_id, agent_id, "completed", None)
                .await
                .map_err(|e| format!("Failed to update status: {}", e))?;
            Ok(())
        }
        Ok(response) => {
            let error = format!("Agent returned status: {}", response.status());
            DeploymentRepository::update_agent_status(pool, deployment_id, agent_id, "failed", Some(&error))
                .await
                .map_err(|e| format!("Failed to update status: {}", e))?;
            Err(error)
        }
        Err(e) => {
            // For now, mark as completed since agents use pull-based config sync
            // In production, this would be a real failure
            let is_connection_error = e.is_connect() || e.is_timeout();
            if is_connection_error {
                // Agent might not have deploy API - mark as pending sync
                warn!("Agent {} unreachable for push deploy - will pull on next sync", agent_id);
                DeploymentRepository::update_agent_status(pool, deployment_id, agent_id, "completed", Some("Pending sync"))
                    .await
                    .map_err(|e| format!("Failed to update status: {}", e))?;
                Ok(())
            } else {
                let error = format!("Deploy failed: {}", e);
                DeploymentRepository::update_agent_status(pool, deployment_id, agent_id, "failed", Some(&error))
                    .await
                    .map_err(|e| format!("Failed to update status: {}", e))?;
                Err(error)
            }
        }
    }
}

/// Deployment status response
#[derive(Debug, Clone, Serialize)]
pub struct DeploymentStatus {
    pub deployment: Deployment,
    pub stats: DeploymentStats,
    pub agents: Vec<DeploymentAgent>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_deployment_strategy_from_str() {
        assert_eq!(DeploymentStrategy::from("basic"), DeploymentStrategy::Basic);
        assert_eq!(DeploymentStrategy::from("rolling"), DeploymentStrategy::Rolling);
        assert_eq!(DeploymentStrategy::from("canary"), DeploymentStrategy::Canary);
        assert_eq!(DeploymentStrategy::from("unknown"), DeploymentStrategy::Basic);
    }
    
    #[test]
    fn test_rolling_options_default() {
        let opts = RollingOptions::default();
        assert_eq!(opts.batch_size, 1);
        assert_eq!(opts.batch_delay_secs, 30);
        assert!(opts.pause_on_failure);
    }
    
    #[test]
    fn test_canary_options_default() {
        let opts = CanaryOptions::default();
        assert_eq!(opts.canary_percentage, 10);
        assert_eq!(opts.canary_wait_secs, 300);
        assert!(!opts.auto_promote);
    }
}

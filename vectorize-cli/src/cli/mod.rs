//! CLI subcommands for Vectorize management
//!
//! Provides commands for managing:
//! - Agents (list, register, delete)
//! - Groups (list, create, delete)
//! - Config (get, set, validate)
//! - Deployments (create, status, approve)

use clap::Subcommand;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

/// Base URL for API calls
fn get_api_url(url: &str) -> String {
    format!("{}/api/v1", url.trim_end_matches('/'))
}

/// CLI client for Vectorize API
pub struct CliClient {
    client: Client,
    base_url: String,
}

impl CliClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            base_url: get_api_url(base_url),
        }
    }
}

// =============================================================================
// Agents Commands
// =============================================================================

#[derive(Subcommand, Debug)]
pub enum AgentCommands {
    /// List all registered agents
    List {
        /// Output format (json, table)
        #[arg(short, long, default_value = "table")]
        format: String,
    },
    /// Get details of a specific agent
    Get {
        /// Agent ID
        id: String,
    },
    /// Register a new agent
    Register {
        /// Agent name
        #[arg(short, long)]
        name: String,
        /// Agent URL (Vector API endpoint)
        #[arg(short, long)]
        url: String,
        /// Worker group ID to assign
        #[arg(short, long)]
        group: Option<String>,
    },
    /// Delete an agent
    Delete {
        /// Agent ID
        id: String,
        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },
}

impl AgentCommands {
    pub async fn execute(&self, client: &CliClient) -> anyhow::Result<()> {
        match self {
            AgentCommands::List { format } => {
                let resp = client.client
                    .get(format!("{}/agents", client.base_url))
                    .send()
                    .await?;
                
                let agents: Vec<serde_json::Value> = resp.json().await?;
                
                if format == "json" {
                    println!("{}", serde_json::to_string_pretty(&agents)?);
                } else {
                    println!("{:<36} {:<20} {:<30} {:<10}", "ID", "NAME", "URL", "STATUS");
                    println!("{}", "-".repeat(100));
                    for agent in agents {
                        println!("{:<36} {:<20} {:<30} {:<10}",
                            agent["id"].as_str().unwrap_or("-"),
                            agent["name"].as_str().unwrap_or("-"),
                            agent["url"].as_str().unwrap_or("-"),
                            agent["status"].as_str().unwrap_or("-"),
                        );
                    }
                }
                Ok(())
            }
            AgentCommands::Get { id } => {
                let resp = client.client
                    .get(format!("{}/agents/{}", client.base_url, id))
                    .send()
                    .await?;
                
                if resp.status().is_success() {
                    let agent: serde_json::Value = resp.json().await?;
                    println!("{}", serde_json::to_string_pretty(&agent)?);
                } else {
                    let error: serde_json::Value = resp.json().await?;
                    eprintln!("Error: {}", error["error"].as_str().unwrap_or("Unknown error"));
                }
                Ok(())
            }
            AgentCommands::Register { name, url, group } => {
                let mut body = json!({
                    "name": name,
                    "url": url,
                });
                if let Some(g) = group {
                    body["group_id"] = json!(g);
                }
                
                let resp = client.client
                    .post(format!("{}/agents", client.base_url))
                    .json(&body)
                    .send()
                    .await?;
                
                if resp.status().is_success() {
                    let result: serde_json::Value = resp.json().await?;
                    println!("Agent registered successfully!");
                    println!("ID: {}", result["agent"]["id"].as_str().unwrap_or("-"));
                } else {
                    let error: serde_json::Value = resp.json().await?;
                    eprintln!("Error: {}", error["error"].as_str().unwrap_or("Unknown error"));
                }
                Ok(())
            }
            AgentCommands::Delete { id, force } => {
                if !force {
                    println!("Are you sure you want to delete agent {}? Use --force to confirm.", id);
                    return Ok(());
                }
                
                let resp = client.client
                    .delete(format!("{}/agents/{}", client.base_url, id))
                    .send()
                    .await?;
                
                if resp.status().is_success() {
                    println!("Agent {} deleted successfully.", id);
                } else {
                    let error: serde_json::Value = resp.json().await?;
                    eprintln!("Error: {}", error["error"].as_str().unwrap_or("Unknown error"));
                }
                Ok(())
            }
        }
    }
}

// =============================================================================
// Groups Commands
// =============================================================================

#[derive(Subcommand, Debug)]
pub enum GroupCommands {
    /// List all worker groups
    List {
        /// Output format (json, table)
        #[arg(short, long, default_value = "table")]
        format: String,
    },
    /// Get details of a specific group
    Get {
        /// Group ID
        id: String,
    },
    /// Create a new worker group
    Create {
        /// Group name
        #[arg(short, long)]
        name: String,
        /// Description
        #[arg(short, long)]
        description: Option<String>,
        /// Deployment strategy (basic, rolling, canary)
        #[arg(short, long, default_value = "basic")]
        strategy: String,
        /// Require approval for deployments
        #[arg(long)]
        requires_approval: bool,
    },
    /// Delete a worker group
    Delete {
        /// Group ID
        id: String,
        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },
    /// List agents in a group
    Agents {
        /// Group ID
        id: String,
    },
}

impl GroupCommands {
    pub async fn execute(&self, client: &CliClient) -> anyhow::Result<()> {
        match self {
            GroupCommands::List { format } => {
                let resp = client.client
                    .get(format!("{}/groups", client.base_url))
                    .send()
                    .await?;
                
                let groups: Vec<serde_json::Value> = resp.json().await?;
                
                if format == "json" {
                    println!("{}", serde_json::to_string_pretty(&groups)?);
                } else {
                    println!("{:<36} {:<20} {:<10} {:<10} {:<10}", "ID", "NAME", "STRATEGY", "APPROVAL", "AGENTS");
                    println!("{}", "-".repeat(90));
                    for group in groups {
                        println!("{:<36} {:<20} {:<10} {:<10} {:<10}",
                            group["id"].as_str().unwrap_or("-"),
                            group["name"].as_str().unwrap_or("-"),
                            group["deployment_strategy"].as_str().unwrap_or("-"),
                            if group["requires_approval"].as_bool().unwrap_or(false) { "yes" } else { "no" },
                            group["agent_count"].as_i64().unwrap_or(0),
                        );
                    }
                }
                Ok(())
            }
            GroupCommands::Get { id } => {
                let resp = client.client
                    .get(format!("{}/groups/{}", client.base_url, id))
                    .send()
                    .await?;
                
                if resp.status().is_success() {
                    let group: serde_json::Value = resp.json().await?;
                    println!("{}", serde_json::to_string_pretty(&group)?);
                } else {
                    let error: serde_json::Value = resp.json().await?;
                    eprintln!("Error: {}", error["error"].as_str().unwrap_or("Unknown error"));
                }
                Ok(())
            }
            GroupCommands::Create { name, description, strategy, requires_approval } => {
                let body = json!({
                    "name": name,
                    "description": description,
                    "deployment_strategy": strategy,
                    "requires_approval": requires_approval,
                });
                
                let resp = client.client
                    .post(format!("{}/groups", client.base_url))
                    .json(&body)
                    .send()
                    .await?;
                
                if resp.status().is_success() {
                    let result: serde_json::Value = resp.json().await?;
                    println!("Group created successfully!");
                    println!("ID: {}", result["id"].as_str().unwrap_or("-"));
                } else {
                    let error: serde_json::Value = resp.json().await?;
                    eprintln!("Error: {}", error["error"].as_str().unwrap_or("Unknown error"));
                }
                Ok(())
            }
            GroupCommands::Delete { id, force } => {
                if !force {
                    println!("Are you sure you want to delete group {}? Use --force to confirm.", id);
                    return Ok(());
                }
                
                let resp = client.client
                    .delete(format!("{}/groups/{}", client.base_url, id))
                    .send()
                    .await?;
                
                if resp.status().is_success() {
                    println!("Group {} deleted successfully.", id);
                } else {
                    let error: serde_json::Value = resp.json().await?;
                    eprintln!("Error: {}", error["error"].as_str().unwrap_or("Unknown error"));
                }
                Ok(())
            }
            GroupCommands::Agents { id } => {
                let resp = client.client
                    .get(format!("{}/groups/{}/agents", client.base_url, id))
                    .send()
                    .await?;
                
                if resp.status().is_success() {
                    let agents: Vec<serde_json::Value> = resp.json().await?;
                    println!("{:<36} {:<20} {:<30} {:<10}", "ID", "NAME", "URL", "STATUS");
                    println!("{}", "-".repeat(100));
                    for agent in agents {
                        println!("{:<36} {:<20} {:<30} {:<10}",
                            agent["id"].as_str().unwrap_or("-"),
                            agent["name"].as_str().unwrap_or("-"),
                            agent["url"].as_str().unwrap_or("-"),
                            agent["status"].as_str().unwrap_or("-"),
                        );
                    }
                } else {
                    let error: serde_json::Value = resp.json().await?;
                    eprintln!("Error: {}", error["error"].as_str().unwrap_or("Unknown error"));
                }
                Ok(())
            }
        }
    }
}

// =============================================================================
// Config Commands
// =============================================================================

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Get configuration for a group
    Get {
        /// Group ID
        group_id: String,
        /// Specific version (commit hash)
        #[arg(short, long)]
        version: Option<String>,
    },
    /// Set configuration for a group
    Set {
        /// Group ID
        group_id: String,
        /// Path to config file
        #[arg(short, long)]
        file: String,
    },
    /// Validate a configuration
    Validate {
        /// Path to config file (or - for stdin)
        file: String,
        /// Validation mode (quick, syntax, full)
        #[arg(short, long, default_value = "full")]
        mode: String,
    },
    /// Show configuration history
    History {
        /// Group ID
        group_id: String,
        /// Number of entries to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Rollback to a previous version
    Rollback {
        /// Group ID
        group_id: String,
        /// Version to rollback to
        version: String,
    },
}

impl ConfigCommands {
    pub async fn execute(&self, client: &CliClient) -> anyhow::Result<()> {
        match self {
            ConfigCommands::Get { group_id, version } => {
                let url = if let Some(v) = version {
                    format!("{}/groups/{}/config/{}", client.base_url, group_id, v)
                } else {
                    format!("{}/groups/{}/config", client.base_url, group_id)
                };
                
                let resp = client.client.get(&url).send().await?;
                
                if resp.status().is_success() {
                    let result: serde_json::Value = resp.json().await?;
                    if let Some(config) = result["config"].as_str() {
                        println!("{}", config);
                    } else {
                        println!("No configuration set for this group.");
                    }
                } else {
                    let error: serde_json::Value = resp.json().await?;
                    eprintln!("Error: {}", error["error"].as_str().unwrap_or("Unknown error"));
                }
                Ok(())
            }
            ConfigCommands::Set { group_id, file } => {
                let config = std::fs::read_to_string(file)?;
                
                let resp = client.client
                    .put(format!("{}/groups/{}/config", client.base_url, group_id))
                    .json(&json!({ "config": config }))
                    .send()
                    .await?;
                
                if resp.status().is_success() {
                    let result: serde_json::Value = resp.json().await?;
                    println!("Configuration updated successfully!");
                    if let Some(version) = result["version"].as_str() {
                        println!("Version: {}", version);
                    }
                } else {
                    let error: serde_json::Value = resp.json().await?;
                    eprintln!("Error: {}", error["message"].as_str()
                        .or(error["error"].as_str())
                        .unwrap_or("Unknown error"));
                }
                Ok(())
            }
            ConfigCommands::Validate { file, mode } => {
                let config = if file == "-" {
                    use std::io::Read;
                    let mut buffer = String::new();
                    std::io::stdin().read_to_string(&mut buffer)?;
                    buffer
                } else {
                    std::fs::read_to_string(file)?
                };
                
                // Use /validate or /validate/quick based on mode
                let endpoint = if mode == "quick" {
                    format!("{}/validate/quick", client.base_url)
                } else {
                    format!("{}/validate", client.base_url)
                };
                
                let resp = client.client
                    .post(&endpoint)
                    .json(&json!({ "config": config }))
                    .send()
                    .await?;
                
                // Try to parse as JSON, fall back to text
                let text = resp.text().await?;
                let result: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(_) => {
                        // Plain text response
                        println!("{}", text);
                        return Ok(());
                    }
                };
                
                if result["valid"].as_bool().unwrap_or(false) {
                    println!("✓ Configuration is valid");
                } else {
                    println!("✗ Configuration has errors:");
                    if let Some(errors) = result["errors"].as_array() {
                        for error in errors {
                            println!("  - {}", error["message"].as_str().unwrap_or("-"));
                        }
                    }
                }
                
                if let Some(warnings) = result["warnings"].as_array() {
                    if !warnings.is_empty() {
                        println!("\nWarnings:");
                        for warning in warnings {
                            println!("  - {}", warning["message"].as_str().unwrap_or("-"));
                        }
                    }
                }
                
                Ok(())
            }
            ConfigCommands::History { group_id, limit } => {
                let resp = client.client
                    .get(format!("{}/groups/{}/history?limit={}", client.base_url, group_id, limit))
                    .send()
                    .await?;
                
                if resp.status().is_success() {
                    let history: Vec<serde_json::Value> = resp.json().await?;
                    println!("{:<12} {:<20} {:<50}", "VERSION", "DATE", "MESSAGE");
                    println!("{}", "-".repeat(85));
                    for entry in history {
                        println!("{:<12} {:<20} {:<50}",
                            entry["hash"].as_str().unwrap_or("-").chars().take(8).collect::<String>(),
                            entry["timestamp"].as_str().unwrap_or("-"),
                            entry["message"].as_str().unwrap_or("-").chars().take(50).collect::<String>(),
                        );
                    }
                } else {
                    let error: serde_json::Value = resp.json().await?;
                    eprintln!("Error: {}", error["error"].as_str().unwrap_or("Unknown error"));
                }
                Ok(())
            }
            ConfigCommands::Rollback { group_id, version } => {
                let resp = client.client
                    .post(format!("{}/groups/{}/rollback", client.base_url, group_id))
                    .json(&json!({ "version": version }))
                    .send()
                    .await?;
                
                if resp.status().is_success() {
                    let result: serde_json::Value = resp.json().await?;
                    println!("Rollback successful!");
                    if let Some(new_version) = result["version"].as_str() {
                        println!("New version: {}", new_version);
                    }
                } else {
                    let error: serde_json::Value = resp.json().await?;
                    eprintln!("Error: {}", error["message"].as_str()
                        .or(error["error"].as_str())
                        .unwrap_or("Unknown error"));
                }
                Ok(())
            }
        }
    }
}

// =============================================================================
// Deploy Commands
// =============================================================================

#[derive(Subcommand, Debug)]
pub enum DeployCommands {
    /// Create a new deployment
    Create {
        /// Group ID to deploy to
        group_id: String,
        /// Config version to deploy (defaults to current)
        #[arg(short, long)]
        version: Option<String>,
        /// Force deployment even with version mismatch
        #[arg(long)]
        force: bool,
    },
    /// Get deployment status
    Status {
        /// Deployment ID
        id: String,
    },
    /// List deployments for a group
    List {
        /// Group ID
        group_id: String,
        /// Number of entries to show
        #[arg(short, long, default_value = "10")]
        limit: i64,
    },
    /// Approve a pending deployment
    Approve {
        /// Deployment ID
        id: String,
    },
    /// Reject a pending deployment
    Reject {
        /// Deployment ID
        id: String,
        /// Rejection reason
        #[arg(short, long)]
        reason: Option<String>,
    },
    /// Cancel a deployment
    Cancel {
        /// Deployment ID
        id: String,
    },
    /// Check version consistency for a group
    Versions {
        /// Group ID
        group_id: String,
    },
}

impl DeployCommands {
    pub async fn execute(&self, client: &CliClient, username: &str) -> anyhow::Result<()> {
        match self {
            DeployCommands::Create { group_id, version, force } => {
                let mut body = json!({ "force": force });
                if let Some(v) = version {
                    body["config_version"] = json!(v);
                }
                
                let resp = client.client
                    .post(format!("{}/groups/{}/deployments", client.base_url, group_id))
                    .json(&body)
                    .send()
                    .await?;
                
                if resp.status().is_success() {
                    let result: serde_json::Value = resp.json().await?;
                    println!("Deployment created!");
                    println!("ID: {}", result["deployment_id"].as_str().unwrap_or("-"));
                    println!("Status: {}", result["status"].as_str().unwrap_or("-"));
                    if result["requires_approval"].as_bool().unwrap_or(false) {
                        println!("\nNote: This deployment requires approval before execution.");
                    }
                } else {
                    let error: serde_json::Value = resp.json().await?;
                    eprintln!("Error: {}", error["error"].as_str().unwrap_or("Unknown error"));
                }
                Ok(())
            }
            DeployCommands::Status { id } => {
                let resp = client.client
                    .get(format!("{}/deployments/{}", client.base_url, id))
                    .send()
                    .await?;
                
                if resp.status().is_success() {
                    let result: serde_json::Value = resp.json().await?;
                    println!("Deployment: {}", id);
                    println!("Status: {}", result["status"].as_str().unwrap_or("-"));
                    println!("Strategy: {}", result["strategy"].as_str().unwrap_or("-"));
                    println!("Config Version: {}", result["config_version"].as_str().unwrap_or("-"));
                    println!("Created: {}", result["created_at"].as_str().unwrap_or("-"));
                    
                    if let Some(stats) = result.get("stats") {
                        println!("\nProgress:");
                        println!("  Total: {}", stats["total"].as_i64().unwrap_or(0));
                        println!("  Completed: {}", stats["completed"].as_i64().unwrap_or(0));
                        println!("  Failed: {}", stats["failed"].as_i64().unwrap_or(0));
                        println!("  In Progress: {}", stats["in_progress"].as_i64().unwrap_or(0));
                        println!("  Pending: {}", stats["pending"].as_i64().unwrap_or(0));
                    }
                } else {
                    let error: serde_json::Value = resp.json().await?;
                    eprintln!("Error: {}", error["error"].as_str().unwrap_or("Unknown error"));
                }
                Ok(())
            }
            DeployCommands::List { group_id, limit } => {
                let resp = client.client
                    .get(format!("{}/groups/{}/deployments?limit={}", client.base_url, group_id, limit))
                    .send()
                    .await?;
                
                if resp.status().is_success() {
                    let result: serde_json::Value = resp.json().await?;
                    let deployments = result["deployments"].as_array();
                    
                    println!("{:<36} {:<15} {:<10} {:<20}", "ID", "STATUS", "STRATEGY", "CREATED");
                    println!("{}", "-".repeat(85));
                    
                    if let Some(deps) = deployments {
                        for dep in deps {
                            println!("{:<36} {:<15} {:<10} {:<20}",
                                dep["id"].as_str().unwrap_or("-"),
                                dep["status"].as_str().unwrap_or("-"),
                                dep["strategy"].as_str().unwrap_or("-"),
                                dep["created_at"].as_str().unwrap_or("-"),
                            );
                        }
                    }
                } else {
                    let error: serde_json::Value = resp.json().await?;
                    eprintln!("Error: {}", error["error"].as_str().unwrap_or("Unknown error"));
                }
                Ok(())
            }
            DeployCommands::Approve { id } => {
                let resp = client.client
                    .post(format!("{}/deployments/{}/approve", client.base_url, id))
                    .json(&json!({ "approved_by": username }))
                    .send()
                    .await?;
                
                if resp.status().is_success() {
                    println!("Deployment {} approved and started.", id);
                } else {
                    let error: serde_json::Value = resp.json().await?;
                    eprintln!("Error: {}", error["error"].as_str().unwrap_or("Unknown error"));
                }
                Ok(())
            }
            DeployCommands::Reject { id, reason } => {
                let body = json!({
                    "rejected_by": username,
                    "reason": reason,
                });
                
                let resp = client.client
                    .post(format!("{}/deployments/{}/reject", client.base_url, id))
                    .json(&body)
                    .send()
                    .await?;
                
                if resp.status().is_success() {
                    println!("Deployment {} rejected.", id);
                } else {
                    let error: serde_json::Value = resp.json().await?;
                    eprintln!("Error: {}", error["error"].as_str().unwrap_or("Unknown error"));
                }
                Ok(())
            }
            DeployCommands::Cancel { id } => {
                let resp = client.client
                    .post(format!("{}/deployments/{}/cancel", client.base_url, id))
                    .send()
                    .await?;
                
                if resp.status().is_success() {
                    println!("Deployment {} cancelled.", id);
                } else {
                    let error: serde_json::Value = resp.json().await?;
                    eprintln!("Error: {}", error["error"].as_str().unwrap_or("Unknown error"));
                }
                Ok(())
            }
            DeployCommands::Versions { group_id } => {
                let resp = client.client
                    .get(format!("{}/groups/{}/versions", client.base_url, group_id))
                    .send()
                    .await?;
                
                if resp.status().is_success() {
                    let result: serde_json::Value = resp.json().await?;
                    
                    if result["consistent"].as_bool().unwrap_or(false) {
                        println!("✓ {}", result["message"].as_str().unwrap_or("Version check passed"));
                    } else {
                        println!("✗ Version mismatch detected!");
                        println!("{}", result["message"].as_str().unwrap_or(""));
                    }
                    
                    if let Some(versions) = result["versions"].as_array() {
                        println!("\nVersions:");
                        for v in versions {
                            println!("  {} - {} agents",
                                v["version"].as_str().unwrap_or("-"),
                                v["agents"].as_array().map(|a| a.len()).unwrap_or(0),
                            );
                        }
                    }
                } else {
                    let error: serde_json::Value = resp.json().await?;
                    eprintln!("Error: {}", error["error"].as_str().unwrap_or("Unknown error"));
                }
                Ok(())
            }
        }
    }
}

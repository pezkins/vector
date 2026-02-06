//! Database models for Vectorize
//!
//! These structs map to database tables and are used for queries.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// =============================================================================
// Agent Models
// =============================================================================

/// Agent status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Healthy,
    Unhealthy,
    Unreachable,
    Unknown,
}

impl From<String> for AgentStatus {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "healthy" => AgentStatus::Healthy,
            "unhealthy" => AgentStatus::Unhealthy,
            "unreachable" => AgentStatus::Unreachable,
            _ => AgentStatus::Unknown,
        }
    }
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStatus::Healthy => write!(f, "healthy"),
            AgentStatus::Unhealthy => write!(f, "unhealthy"),
            AgentStatus::Unreachable => write!(f, "unreachable"),
            AgentStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Agent database model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub url: String,
    pub group_id: Option<String>,
    pub status: String,
    pub vector_version: Option<String>,
    pub last_seen: Option<String>,
    pub registered_at: String,
    pub metadata: Option<String>,
}

/// Agent for API responses (with parsed status)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub id: String,
    pub name: String,
    pub url: String,
    pub group_id: Option<String>,
    pub status: AgentStatus,
    pub vector_version: Option<String>,
    pub last_seen: Option<String>,
    pub registered_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl From<Agent> for AgentResponse {
    fn from(agent: Agent) -> Self {
        Self {
            id: agent.id,
            name: agent.name,
            url: agent.url,
            group_id: agent.group_id,
            status: AgentStatus::from(agent.status),
            vector_version: agent.vector_version,
            last_seen: agent.last_seen,
            registered_at: agent.registered_at,
            metadata: agent.metadata.and_then(|m| serde_json::from_str(&m).ok()),
        }
    }
}

/// Health check record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct HealthCheck {
    pub id: i64,
    pub agent_id: String,
    pub healthy: bool,
    pub latency_ms: Option<i64>,
    pub error: Option<String>,
    pub checked_at: String,
}

// =============================================================================
// User & Auth Models
// =============================================================================

/// Role database model (used in future RBAC phase)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub permissions: String,  // JSON array
    pub is_builtin: bool,
    pub created_at: String,
    pub updated_at: Option<String>,
}

/// Role for API responses (used in future RBAC phase)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub is_builtin: bool,
    pub created_at: String,
}

impl From<Role> for RoleResponse {
    fn from(role: Role) -> Self {
        Self {
            id: role.id,
            name: role.name,
            description: role.description,
            permissions: serde_json::from_str(&role.permissions).unwrap_or_default(),
            is_builtin: role.is_builtin,
            created_at: role.created_at,
        }
    }
}

/// User database model
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct User {
    pub id: String,
    pub username: Option<String>,
    pub email: Option<String>,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub sso_provider: Option<String>,
    pub sso_subject: Option<String>,
    pub role_id: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub last_login: Option<String>,
}

/// User for API responses (no password hash)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub sso_provider: Option<String>,
    pub role_id: String,
    pub is_active: bool,
    pub created_at: String,
    pub last_login: Option<String>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            sso_provider: user.sso_provider,
            role_id: user.role_id,
            is_active: user.is_active,
            created_at: user.created_at,
            last_login: user.last_login,
        }
    }
}

/// API key database model (used in future auth phase)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing)]
    pub key_hash: String,
    pub key_prefix: String,
    pub user_id: Option<String>,
    pub permissions: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub last_used: Option<String>,
    pub revoked_at: Option<String>,
}

/// API key for API responses (used in future auth phase)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyResponse {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub user_id: Option<String>,
    pub permissions: Option<Vec<String>>,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub last_used: Option<String>,
    pub is_revoked: bool,
}

impl From<ApiKey> for ApiKeyResponse {
    fn from(key: ApiKey) -> Self {
        Self {
            id: key.id,
            name: key.name,
            key_prefix: key.key_prefix,
            user_id: key.user_id,
            permissions: key.permissions.and_then(|p| serde_json::from_str(&p).ok()),
            created_at: key.created_at,
            expires_at: key.expires_at,
            last_used: key.last_used,
            is_revoked: key.revoked_at.is_some(),
        }
    }
}

/// Session database model (used in future auth phase)
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub refresh_token_hash: String,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: String,
    pub expires_at: String,
    pub revoked_at: Option<String>,
}

// =============================================================================
// Worker Group Models
// =============================================================================

/// Worker group database model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkerGroup {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub deployment_strategy: String,
    pub requires_approval: bool,
    pub approvers: Option<String>,  // JSON array
    pub config_path: Option<String>,
    pub current_config_version: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub created_by: Option<String>,
}

/// Worker group for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerGroupResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub deployment_strategy: String,
    pub requires_approval: bool,
    pub approvers: Vec<String>,
    pub config_path: Option<String>,
    pub current_config_version: Option<String>,
    pub created_at: String,
    pub agent_count: Option<i64>,
    pub healthy_count: Option<i64>,
    pub unhealthy_count: Option<i64>,
}

impl From<WorkerGroup> for WorkerGroupResponse {
    fn from(group: WorkerGroup) -> Self {
        Self {
            id: group.id,
            name: group.name,
            description: group.description,
            deployment_strategy: group.deployment_strategy,
            requires_approval: group.requires_approval,
            approvers: group.approvers.and_then(|a| serde_json::from_str(&a).ok()).unwrap_or_default(),
            config_path: group.config_path,
            current_config_version: group.current_config_version,
            created_at: group.created_at,
            agent_count: None,
            healthy_count: None,
            unhealthy_count: None,
        }
    }
}

// =============================================================================
// Deployment Models
// =============================================================================

/// Deployment status enum (used in future deployment phase)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentStatus {
    Pending,
    PendingApproval,
    Queued,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl From<String> for DeploymentStatus {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => DeploymentStatus::Pending,
            "pending_approval" => DeploymentStatus::PendingApproval,
            "queued" => DeploymentStatus::Queued,
            "in_progress" => DeploymentStatus::InProgress,
            "completed" => DeploymentStatus::Completed,
            "failed" => DeploymentStatus::Failed,
            "cancelled" => DeploymentStatus::Cancelled,
            _ => DeploymentStatus::Pending,
        }
    }
}

impl std::fmt::Display for DeploymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeploymentStatus::Pending => write!(f, "pending"),
            DeploymentStatus::PendingApproval => write!(f, "pending_approval"),
            DeploymentStatus::Queued => write!(f, "queued"),
            DeploymentStatus::InProgress => write!(f, "in_progress"),
            DeploymentStatus::Completed => write!(f, "completed"),
            DeploymentStatus::Failed => write!(f, "failed"),
            DeploymentStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Deployment database model (used in future deployment phase)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Deployment {
    pub id: String,
    pub group_id: String,
    pub config_version: String,
    pub strategy: String,
    pub status: String,
    pub options: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_by: Option<String>,
    pub approved_by: Option<String>,
    pub approved_at: Option<String>,
    pub rejected_by: Option<String>,
    pub rejected_at: Option<String>,
    pub rejection_reason: Option<String>,
    pub error: Option<String>,
}

/// Deployment agent status (used in future deployment phase)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeploymentAgent {
    pub id: i64,
    pub deployment_id: String,
    pub agent_id: String,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error: Option<String>,
}

// =============================================================================
// Audit Log Models
// =============================================================================

/// Audit log entry (used in future audit phase)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLogEntry {
    pub id: String,
    pub timestamp: String,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub details: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub result: String,
}

/// Audit log for API responses (used in future audit phase)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogResponse {
    pub id: String,
    pub timestamp: String,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub result: String,
}

impl From<AuditLogEntry> for AuditLogResponse {
    fn from(entry: AuditLogEntry) -> Self {
        Self {
            id: entry.id,
            timestamp: entry.timestamp,
            actor_type: entry.actor_type,
            actor_id: entry.actor_id,
            actor_name: entry.actor_name,
            action: entry.action,
            resource_type: entry.resource_type,
            resource_id: entry.resource_id,
            details: entry.details.and_then(|d| serde_json::from_str(&d).ok()),
            ip_address: entry.ip_address,
            result: entry.result,
        }
    }
}

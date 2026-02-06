//! Audit logging API endpoints
//!
//! Provides endpoints for viewing audit logs.

use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::error;

use crate::AppState;
use crate::db::models::AuditLogResponse;
use crate::db::repository::AuditLogRepository;
use crate::rbac::{AuthenticatedUser, require_permission};

/// Query parameters for audit log listing
#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    /// Filter by actor ID (user ID)
    pub actor_id: Option<String>,
    /// Filter by action type
    pub action: Option<String>,
    /// Filter by resource type
    pub resource_type: Option<String>,
    /// Number of entries to return (default 50, max 1000)
    pub limit: Option<i64>,
    /// Offset for pagination
    pub offset: Option<i64>,
}

/// Response with audit log entries
#[derive(Debug, Serialize)]
pub struct AuditLogListResponse {
    pub entries: Vec<AuditLogResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// List audit log entries
pub async fn list_audit_logs(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(query): Query<AuditLogQuery>,
) -> impl IntoResponse {
    if let Err(resp) = require_permission(&user, "audit_read") {
        return resp;
    }
    
    let limit = query.limit.unwrap_or(50).min(1000);
    let offset = query.offset.unwrap_or(0);
    
    let pool = state.db.pool();
    
    // Get entries
    let entries = match AuditLogRepository::list(
        pool,
        query.actor_id.as_deref(),
        query.action.as_deref(),
        query.resource_type.as_deref(),
        limit,
        offset,
    ).await {
        Ok(entries) => entries,
        Err(e) => {
            error!("Failed to list audit logs: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to list audit logs"
            }))).into_response();
        }
    };
    
    // Get total count for pagination
    let total: i64 = match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM audit_log")
        .fetch_one(pool)
        .await
    {
        Ok(count) => count,
        Err(_) => entries.len() as i64,
    };
    
    let entry_responses: Vec<AuditLogResponse> = entries.into_iter().map(AuditLogResponse::from).collect();
    
    (StatusCode::OK, Json(AuditLogListResponse {
        entries: entry_responses,
        total,
        limit,
        offset,
    })).into_response()
}

/// Available actions for filtering
#[derive(Debug, Serialize)]
pub struct AuditActionsResponse {
    pub actions: Vec<AuditAction>,
}

/// Audit action info
#[derive(Debug, Serialize)]
pub struct AuditAction {
    pub name: String,
    pub description: String,
    pub category: String,
}

/// List available audit actions
pub async fn list_audit_actions(
    Extension(user): Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    if let Err(resp) = require_permission(&user, "audit_read") {
        return resp;
    }
    
    let actions = vec![
        // Authentication
        AuditAction { name: "login".into(), description: "User logged in".into(), category: "Auth".into() },
        AuditAction { name: "logout".into(), description: "User logged out".into(), category: "Auth".into() },
        AuditAction { name: "login_failed".into(), description: "Failed login attempt".into(), category: "Auth".into() },
        
        // Users
        AuditAction { name: "user.create".into(), description: "User created".into(), category: "Users".into() },
        AuditAction { name: "user.update".into(), description: "User updated".into(), category: "Users".into() },
        AuditAction { name: "user.delete".into(), description: "User deleted".into(), category: "Users".into() },
        
        // Roles
        AuditAction { name: "role.create".into(), description: "Role created".into(), category: "Roles".into() },
        AuditAction { name: "role.update".into(), description: "Role updated".into(), category: "Roles".into() },
        AuditAction { name: "role.delete".into(), description: "Role deleted".into(), category: "Roles".into() },
        
        // Agents
        AuditAction { name: "agent.register".into(), description: "Agent registered".into(), category: "Agents".into() },
        AuditAction { name: "agent.update".into(), description: "Agent updated".into(), category: "Agents".into() },
        AuditAction { name: "agent.delete".into(), description: "Agent deleted".into(), category: "Agents".into() },
        
        // Groups
        AuditAction { name: "group.create".into(), description: "Group created".into(), category: "Groups".into() },
        AuditAction { name: "group.update".into(), description: "Group updated".into(), category: "Groups".into() },
        AuditAction { name: "group.delete".into(), description: "Group deleted".into(), category: "Groups".into() },
        
        // Configurations
        AuditAction { name: "config.update".into(), description: "Configuration updated".into(), category: "Config".into() },
        AuditAction { name: "config.rollback".into(), description: "Configuration rolled back".into(), category: "Config".into() },
        AuditAction { name: "config.deploy".into(), description: "Configuration deployed".into(), category: "Config".into() },
        
        // API Keys
        AuditAction { name: "api_key.create".into(), description: "API key created".into(), category: "API Keys".into() },
        AuditAction { name: "api_key.revoke".into(), description: "API key revoked".into(), category: "API Keys".into() },
        
        // Alerts
        AuditAction { name: "alert.create".into(), description: "Alert rule created".into(), category: "Alerts".into() },
        AuditAction { name: "alert.delete".into(), description: "Alert rule deleted".into(), category: "Alerts".into() },
    ];
    
    (StatusCode::OK, Json(AuditActionsResponse { actions })).into_response()
}

// =============================================================================
// Audit Helper Functions
// =============================================================================

/// Log an audit event
#[allow(dead_code)]
pub async fn log_audit_event(
    pool: &sqlx::SqlitePool,
    actor_type: &str,
    actor_id: Option<&str>,
    actor_name: Option<&str>,
    action: &str,
    resource_type: Option<&str>,
    resource_id: Option<&str>,
    details: Option<serde_json::Value>,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
    result: &str,
) {
    let details_str = details.map(|d| d.to_string());
    
    if let Err(e) = AuditLogRepository::create(
        pool,
        actor_type,
        actor_id,
        actor_name,
        action,
        resource_type,
        resource_id,
        details_str.as_deref(),
        ip_address,
        user_agent,
        result,
    ).await {
        error!("Failed to create audit log: {}", e);
    }
}

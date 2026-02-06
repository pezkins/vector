//! Role-Based Access Control (RBAC) Module
//!
//! Provides fine-grained permission management:
//! - Permission definitions and actions
//! - Role management (built-in and custom)
//! - Permission checking middleware
//! - JWT authentication middleware

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, warn};

use crate::AppState;
use crate::db::repository::RoleRepository;

// =============================================================================
// Permission Definitions
// =============================================================================

/// All available permissions in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    // Agent permissions
    AgentsRead,
    AgentsWrite,
    AgentsDelete,
    
    // Worker Group permissions
    GroupsRead,
    GroupsWrite,
    GroupsDelete,
    GroupsDeploy,
    
    // Configuration permissions
    ConfigsRead,
    ConfigsWrite,
    ConfigsRollback,
    ConfigsValidate,
    
    // User management permissions
    UsersRead,
    UsersWrite,
    UsersDelete,
    
    // Role management permissions
    RolesRead,
    RolesWrite,
    RolesDelete,
    
    // API Key permissions
    ApiKeysRead,
    ApiKeysWrite,
    ApiKeysDelete,
    
    // Audit log permissions
    AuditRead,
    
    // Alert permissions
    AlertsRead,
    AlertsWrite,
    AlertsDelete,
    
    // System permissions
    SystemRead,
    SystemAdmin,
}

impl Permission {
    /// Get all available permissions
    pub fn all() -> Vec<Permission> {
        vec![
            Permission::AgentsRead,
            Permission::AgentsWrite,
            Permission::AgentsDelete,
            Permission::GroupsRead,
            Permission::GroupsWrite,
            Permission::GroupsDelete,
            Permission::GroupsDeploy,
            Permission::ConfigsRead,
            Permission::ConfigsWrite,
            Permission::ConfigsRollback,
            Permission::ConfigsValidate,
            Permission::UsersRead,
            Permission::UsersWrite,
            Permission::UsersDelete,
            Permission::RolesRead,
            Permission::RolesWrite,
            Permission::RolesDelete,
            Permission::ApiKeysRead,
            Permission::ApiKeysWrite,
            Permission::ApiKeysDelete,
            Permission::AuditRead,
            Permission::AlertsRead,
            Permission::AlertsWrite,
            Permission::AlertsDelete,
            Permission::SystemRead,
            Permission::SystemAdmin,
        ]
    }
    
    /// Get permissions for the admin role
    pub fn admin_permissions() -> Vec<String> {
        Self::all().iter().map(|p| format!("{:?}", p).to_lowercase()).collect()
    }
    
    /// Get permissions for the operator role
    pub fn operator_permissions() -> Vec<String> {
        vec![
            "agents_read", "agents_write",
            "groups_read", "groups_write", "groups_deploy",
            "configs_read", "configs_write", "configs_validate",
            "alerts_read", "alerts_write",
            "system_read",
        ].into_iter().map(String::from).collect()
    }
    
    /// Get permissions for the viewer role
    pub fn viewer_permissions() -> Vec<String> {
        vec![
            "agents_read",
            "groups_read",
            "configs_read",
            "alerts_read",
            "system_read",
        ].into_iter().map(String::from).collect()
    }
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

impl std::str::FromStr for Permission {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "agents_read" | "agentsread" => Ok(Permission::AgentsRead),
            "agents_write" | "agentswrite" => Ok(Permission::AgentsWrite),
            "agents_delete" | "agentsdelete" => Ok(Permission::AgentsDelete),
            "groups_read" | "groupsread" => Ok(Permission::GroupsRead),
            "groups_write" | "groupswrite" => Ok(Permission::GroupsWrite),
            "groups_delete" | "groupsdelete" => Ok(Permission::GroupsDelete),
            "groups_deploy" | "groupsdeploy" => Ok(Permission::GroupsDeploy),
            "configs_read" | "configsread" => Ok(Permission::ConfigsRead),
            "configs_write" | "configswrite" => Ok(Permission::ConfigsWrite),
            "configs_rollback" | "configsrollback" => Ok(Permission::ConfigsRollback),
            "configs_validate" | "configsvalidate" => Ok(Permission::ConfigsValidate),
            "users_read" | "usersread" => Ok(Permission::UsersRead),
            "users_write" | "userswrite" => Ok(Permission::UsersWrite),
            "users_delete" | "usersdelete" => Ok(Permission::UsersDelete),
            "roles_read" | "rolesread" => Ok(Permission::RolesRead),
            "roles_write" | "roleswrite" => Ok(Permission::RolesWrite),
            "roles_delete" | "rolesdelete" => Ok(Permission::RolesDelete),
            "api_keys_read" | "apikeysread" => Ok(Permission::ApiKeysRead),
            "api_keys_write" | "apikeyswrite" => Ok(Permission::ApiKeysWrite),
            "api_keys_delete" | "apikeysdelete" => Ok(Permission::ApiKeysDelete),
            "audit_read" | "auditread" => Ok(Permission::AuditRead),
            "alerts_read" | "alertsread" => Ok(Permission::AlertsRead),
            "alerts_write" | "alertswrite" => Ok(Permission::AlertsWrite),
            "alerts_delete" | "alertsdelete" => Ok(Permission::AlertsDelete),
            "system_read" | "systemread" => Ok(Permission::SystemRead),
            "system_admin" | "systemadmin" => Ok(Permission::SystemAdmin),
            _ => Err(format!("Unknown permission: {}", s)),
        }
    }
}

// =============================================================================
// JWT Claims
// =============================================================================

/// JWT token claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Role ID
    pub role: String,
    /// Expiration time (Unix timestamp)
    pub exp: usize,
    /// Issued at (Unix timestamp)
    pub iat: usize,
    /// Token type (access or refresh)
    #[serde(default)]
    pub token_type: String,
}

/// Authenticated user info (available in request extensions)
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub role_id: String,
    pub permissions: Vec<String>,
}

impl AuthenticatedUser {
    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        // System admin has all permissions
        if self.permissions.contains(&"system_admin".to_string()) {
            return true;
        }
        self.permissions.contains(&permission.to_string())
    }
    
    /// Check if user has any of the specified permissions
    pub fn has_any_permission(&self, permissions: &[&str]) -> bool {
        if self.permissions.contains(&"system_admin".to_string()) {
            return true;
        }
        permissions.iter().any(|p| self.permissions.contains(&p.to_string()))
    }
}

// =============================================================================
// Authentication Middleware
// =============================================================================

/// Get JWT secret from environment
fn get_jwt_secret() -> String {
    std::env::var("VECTORIZE_JWT_SECRET")
        .unwrap_or_else(|_| "vectorize-development-secret-change-in-production".to_string())
}

/// Authentication middleware - extracts and validates JWT token
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract token from Authorization header
    let token = match extract_token(&request) {
        Some(token) => token,
        None => {
            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
                "error": "Missing or invalid authorization header"
            }))).into_response();
        }
    };
    
    // Decode and validate token
    let claims = match decode_token(&token) {
        Ok(claims) => claims,
        Err(e) => {
            debug!("Token validation failed: {}", e);
            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
                "error": "Invalid or expired token"
            }))).into_response();
        }
    };
    
    // Get role permissions
    let permissions = match get_role_permissions(&state, &claims.role).await {
        Ok(perms) => perms,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to load permissions"
            }))).into_response();
        }
    };
    
    // Add authenticated user to request extensions
    let user = AuthenticatedUser {
        user_id: claims.sub,
        role_id: claims.role,
        permissions,
    };
    
    request.extensions_mut().insert(user);
    
    next.run(request).await
}

/// Optional auth middleware - allows unauthenticated requests but adds user if token present
pub async fn optional_auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Try to extract and validate token
    if let Some(token) = extract_token(&request) {
        if let Ok(claims) = decode_token(&token) {
            if let Ok(permissions) = get_role_permissions(&state, &claims.role).await {
                let user = AuthenticatedUser {
                    user_id: claims.sub,
                    role_id: claims.role,
                    permissions,
                };
                request.extensions_mut().insert(user);
            }
        }
    }
    
    next.run(request).await
}

/// Extract bearer token from request
fn extract_token(request: &Request) -> Option<String> {
    // Try Authorization header first
    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                return Some(auth_str[7..].to_string());
            }
        }
    }
    
    // Try X-API-Key header
    if let Some(api_key) = request.headers().get("X-API-Key") {
        if let Ok(key) = api_key.to_str() {
            return Some(key.to_string());
        }
    }
    
    None
}

/// Decode and validate JWT token
fn decode_token(token: &str) -> Result<Claims, String> {
    let secret = get_jwt_secret();
    
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| e.to_string())?;
    
    Ok(token_data.claims)
}

/// Get permissions for a role
async fn get_role_permissions(state: &AppState, role_id: &str) -> Result<Vec<String>, String> {
    let pool = state.db.pool();
    
    match RoleRepository::get_by_id(pool, role_id).await {
        Ok(Some(role)) => {
            let perms: Vec<String> = serde_json::from_str(&role.permissions)
                .unwrap_or_default();
            Ok(perms)
        }
        Ok(None) => {
            warn!("Role not found: {}", role_id);
            Ok(Vec::new())
        }
        Err(e) => {
            Err(format!("Database error: {}", e))
        }
    }
}

// =============================================================================
// Permission Checking
// =============================================================================

/// Require specific permission(s) - returns 403 if not authorized
pub fn require_permission(
    user: &AuthenticatedUser,
    permission: &str,
) -> Result<(), Response> {
    if user.has_permission(permission) {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, Json(serde_json::json!({
            "error": "Insufficient permissions",
            "required": permission
        }))).into_response())
    }
}

/// Require any of the specified permissions
pub fn require_any_permission(
    user: &AuthenticatedUser,
    permissions: &[&str],
) -> Result<(), Response> {
    if user.has_any_permission(permissions) {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, Json(serde_json::json!({
            "error": "Insufficient permissions",
            "required_any": permissions
        }))).into_response())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_permission_display() {
        assert_eq!(Permission::AgentsRead.to_string(), "agentsread");
        assert_eq!(Permission::SystemAdmin.to_string(), "systemadmin");
    }
    
    #[test]
    fn test_permission_from_str() {
        assert_eq!("agents_read".parse::<Permission>().unwrap(), Permission::AgentsRead);
        assert_eq!("agentsread".parse::<Permission>().unwrap(), Permission::AgentsRead);
        assert_eq!("system_admin".parse::<Permission>().unwrap(), Permission::SystemAdmin);
    }
    
    #[test]
    fn test_admin_permissions() {
        let perms = Permission::admin_permissions();
        assert!(perms.contains(&"systemadmin".to_string()));
        assert!(perms.len() >= 20); // Should have many permissions
    }
    
    #[test]
    fn test_authenticated_user_has_permission() {
        let user = AuthenticatedUser {
            user_id: "user1".to_string(),
            role_id: "admin".to_string(),
            permissions: vec!["agents_read".to_string(), "system_admin".to_string()],
        };
        
        // Direct permission
        assert!(user.has_permission("agents_read"));
        
        // System admin has all permissions
        assert!(user.has_permission("anything"));
    }
    
    #[test]
    fn test_viewer_permissions() {
        let perms = Permission::viewer_permissions();
        assert!(perms.contains(&"agents_read".to_string()));
        assert!(!perms.contains(&"agents_write".to_string()));
    }
}

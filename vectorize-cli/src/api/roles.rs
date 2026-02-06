//! Role management API endpoints
//!
//! Provides endpoints for managing roles and permissions.

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, error};

use crate::AppState;
use crate::db::models::RoleResponse;
use crate::db::repository::RoleRepository;
use crate::rbac::{AuthenticatedUser, Permission, require_permission};

/// Request to create a new role
#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
}

/// Request to update a role
#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
}

/// Response with role list
#[derive(Debug, Serialize)]
pub struct RolesResponse {
    pub roles: Vec<RoleResponse>,
}

/// Response with available permissions
#[derive(Debug, Serialize)]
pub struct PermissionsResponse {
    pub permissions: Vec<PermissionInfo>,
}

/// Permission info
#[derive(Debug, Serialize)]
pub struct PermissionInfo {
    pub name: String,
    pub description: String,
    pub category: String,
}

/// List all roles
pub async fn list_roles(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    if let Err(resp) = require_permission(&user, "roles_read") {
        return resp;
    }
    
    let pool = state.db.pool();
    
    match RoleRepository::list(pool).await {
        Ok(roles) => {
            let role_responses: Vec<RoleResponse> = roles.into_iter().map(RoleResponse::from).collect();
            (StatusCode::OK, Json(RolesResponse { roles: role_responses })).into_response()
        }
        Err(e) => {
            error!("Failed to list roles: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to list roles"
            }))).into_response()
        }
    }
}

/// Get a specific role
pub async fn get_role(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(role_id): Path<String>,
) -> impl IntoResponse {
    if let Err(resp) = require_permission(&user, "roles_read") {
        return resp;
    }
    
    let pool = state.db.pool();
    
    match RoleRepository::get_by_id(pool, &role_id).await {
        Ok(Some(role)) => {
            (StatusCode::OK, Json(RoleResponse::from(role))).into_response()
        }
        Ok(None) => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Role not found"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to get role: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get role"
            }))).into_response()
        }
    }
}

/// Create a new custom role
pub async fn create_role(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(request): Json<CreateRoleRequest>,
) -> impl IntoResponse {
    if let Err(resp) = require_permission(&user, "roles_write") {
        return resp;
    }
    
    // Validate role name
    if request.name.is_empty() || request.name.len() > 50 {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "Role name must be between 1 and 50 characters"
        }))).into_response();
    }
    
    // Validate permissions
    let valid_permissions: Vec<String> = Permission::all()
        .iter()
        .map(|p| p.to_string())
        .collect();
    
    for perm in &request.permissions {
        let normalized = perm.to_lowercase().replace("_", "");
        if !valid_permissions.iter().any(|p| p == &normalized) {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": format!("Invalid permission: {}", perm),
                "valid_permissions": valid_permissions
            }))).into_response();
        }
    }
    
    let pool = state.db.pool();
    
    // Check if role name already exists
    if let Ok(Some(_)) = RoleRepository::get_by_id(pool, &request.name.to_lowercase()).await {
        return (StatusCode::CONFLICT, Json(serde_json::json!({
            "error": "Role with this name already exists"
        }))).into_response();
    }
    
    // Create role
    match RoleRepository::create(pool, &request.name, request.description.as_deref(), &request.permissions).await {
        Ok(role) => {
            info!("Role created: {} by {}", request.name, user.user_id);
            (StatusCode::CREATED, Json(RoleResponse::from(role))).into_response()
        }
        Err(e) => {
            error!("Failed to create role: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to create role"
            }))).into_response()
        }
    }
}

/// Update a custom role
pub async fn update_role(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(role_id): Path<String>,
    Json(request): Json<UpdateRoleRequest>,
) -> impl IntoResponse {
    if let Err(resp) = require_permission(&user, "roles_write") {
        return resp;
    }
    
    let pool = state.db.pool();
    
    // Get existing role
    let existing = match RoleRepository::get_by_id(pool, &role_id).await {
        Ok(Some(role)) => role,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Role not found"
            }))).into_response();
        }
        Err(e) => {
            error!("Failed to get role: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Database error"
            }))).into_response();
        }
    };
    
    // Cannot modify built-in roles
    if existing.is_builtin {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({
            "error": "Cannot modify built-in roles"
        }))).into_response();
    }
    
    // Validate permissions if provided
    if let Some(ref perms) = request.permissions {
        let valid_permissions: Vec<String> = Permission::all()
            .iter()
            .map(|p| p.to_string())
            .collect();
        
        for perm in perms {
            let normalized = perm.to_lowercase().replace("_", "");
            if !valid_permissions.iter().any(|p| p == &normalized) {
                return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                    "error": format!("Invalid permission: {}", perm)
                }))).into_response();
            }
        }
    }
    
    // Build update
    let new_name = request.name.as_ref().unwrap_or(&existing.name);
    let new_description = request.description.as_ref().or(existing.description.as_ref());
    let new_permissions = request.permissions.unwrap_or_else(|| {
        serde_json::from_str(&existing.permissions).unwrap_or_default()
    });
    let permissions_json = serde_json::to_string(&new_permissions).unwrap_or_else(|_| "[]".to_string());
    
    // Execute update
    let result = sqlx::query_as::<_, crate::db::models::Role>(
        r#"
        UPDATE roles SET 
            name = ?,
            description = ?,
            permissions = ?,
            updated_at = datetime('now')
        WHERE id = ? AND is_builtin = 0
        RETURNING *
        "#
    )
    .bind(new_name)
    .bind(new_description)
    .bind(&permissions_json)
    .bind(&role_id)
    .fetch_optional(pool)
    .await;
    
    match result {
        Ok(Some(updated)) => {
            info!("Role updated: {} by {}", role_id, user.user_id);
            (StatusCode::OK, Json(RoleResponse::from(updated))).into_response()
        }
        Ok(None) => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Role not found or is built-in"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to update role: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to update role"
            }))).into_response()
        }
    }
}

/// Delete a custom role
pub async fn delete_role(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(role_id): Path<String>,
) -> impl IntoResponse {
    if let Err(resp) = require_permission(&user, "roles_delete") {
        return resp;
    }
    
    let pool = state.db.pool();
    
    // Check if role is built-in
    if let Ok(Some(role)) = RoleRepository::get_by_id(pool, &role_id).await {
        if role.is_builtin {
            return (StatusCode::FORBIDDEN, Json(serde_json::json!({
                "error": "Cannot delete built-in roles"
            }))).into_response();
        }
    }
    
    // Check if any users have this role
    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE role_id = ?")
        .bind(&role_id)
        .fetch_one(pool)
        .await
        .unwrap_or((0,));
    
    if user_count.0 > 0 {
        return (StatusCode::CONFLICT, Json(serde_json::json!({
            "error": "Cannot delete role that is assigned to users",
            "user_count": user_count.0
        }))).into_response();
    }
    
    match RoleRepository::delete(pool, &role_id).await {
        Ok(true) => {
            info!("Role deleted: {} by {}", role_id, user.user_id);
            (StatusCode::NO_CONTENT).into_response()
        }
        Ok(false) => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Role not found or is built-in"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to delete role: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to delete role"
            }))).into_response()
        }
    }
}

/// List all available permissions
pub async fn list_permissions(
    Extension(user): Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    if let Err(resp) = require_permission(&user, "roles_read") {
        return resp;
    }
    
    let permissions: Vec<PermissionInfo> = vec![
        // Agent permissions
        PermissionInfo { name: "agents_read".into(), description: "View agents".into(), category: "Agents".into() },
        PermissionInfo { name: "agents_write".into(), description: "Create and update agents".into(), category: "Agents".into() },
        PermissionInfo { name: "agents_delete".into(), description: "Delete agents".into(), category: "Agents".into() },
        
        // Group permissions
        PermissionInfo { name: "groups_read".into(), description: "View worker groups".into(), category: "Groups".into() },
        PermissionInfo { name: "groups_write".into(), description: "Create and update groups".into(), category: "Groups".into() },
        PermissionInfo { name: "groups_delete".into(), description: "Delete groups".into(), category: "Groups".into() },
        PermissionInfo { name: "groups_deploy".into(), description: "Deploy configurations".into(), category: "Groups".into() },
        
        // Config permissions
        PermissionInfo { name: "configs_read".into(), description: "View configurations".into(), category: "Configs".into() },
        PermissionInfo { name: "configs_write".into(), description: "Edit configurations".into(), category: "Configs".into() },
        PermissionInfo { name: "configs_rollback".into(), description: "Rollback configurations".into(), category: "Configs".into() },
        PermissionInfo { name: "configs_validate".into(), description: "Validate configurations".into(), category: "Configs".into() },
        
        // User permissions
        PermissionInfo { name: "users_read".into(), description: "View users".into(), category: "Users".into() },
        PermissionInfo { name: "users_write".into(), description: "Create and update users".into(), category: "Users".into() },
        PermissionInfo { name: "users_delete".into(), description: "Delete users".into(), category: "Users".into() },
        
        // Role permissions
        PermissionInfo { name: "roles_read".into(), description: "View roles".into(), category: "Roles".into() },
        PermissionInfo { name: "roles_write".into(), description: "Create and update roles".into(), category: "Roles".into() },
        PermissionInfo { name: "roles_delete".into(), description: "Delete roles".into(), category: "Roles".into() },
        
        // API Key permissions
        PermissionInfo { name: "api_keys_read".into(), description: "View API keys".into(), category: "API Keys".into() },
        PermissionInfo { name: "api_keys_write".into(), description: "Create API keys".into(), category: "API Keys".into() },
        PermissionInfo { name: "api_keys_delete".into(), description: "Revoke API keys".into(), category: "API Keys".into() },
        
        // Audit permissions
        PermissionInfo { name: "audit_read".into(), description: "View audit logs".into(), category: "Audit".into() },
        
        // Alert permissions
        PermissionInfo { name: "alerts_read".into(), description: "View alerts".into(), category: "Alerts".into() },
        PermissionInfo { name: "alerts_write".into(), description: "Manage alerts".into(), category: "Alerts".into() },
        PermissionInfo { name: "alerts_delete".into(), description: "Delete alerts".into(), category: "Alerts".into() },
        
        // System permissions
        PermissionInfo { name: "system_read".into(), description: "View system info".into(), category: "System".into() },
        PermissionInfo { name: "system_admin".into(), description: "Full system access".into(), category: "System".into() },
    ];
    
    (StatusCode::OK, Json(PermissionsResponse { permissions })).into_response()
}

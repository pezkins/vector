//! User management API endpoints
//!
//! Provides endpoints for managing users in the system.

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, error};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

use crate::AppState;
use crate::db::models::UserResponse;
use crate::db::repository::UserRepository;
use crate::rbac::{AuthenticatedUser, require_permission};

/// Request to create a new user
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub role_id: String,
}

/// Request to update a user
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub role_id: Option<String>,
    pub is_active: Option<bool>,
}

/// Response with user list
#[derive(Debug, Serialize)]
pub struct UsersResponse {
    pub users: Vec<UserResponse>,
}

/// List all users
pub async fn list_users(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    if let Err(resp) = require_permission(&user, "users_read") {
        return resp;
    }
    
    let pool = state.db.pool();
    
    match UserRepository::list(pool).await {
        Ok(users) => {
            let user_responses: Vec<UserResponse> = users.into_iter().map(UserResponse::from).collect();
            (StatusCode::OK, Json(UsersResponse { users: user_responses })).into_response()
        }
        Err(e) => {
            error!("Failed to list users: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to list users"
            }))).into_response()
        }
    }
}

/// Get a specific user by ID
pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    // Users can always view their own profile
    if user.user_id != user_id {
        if let Err(resp) = require_permission(&user, "users_read") {
            return resp;
        }
    }
    
    let pool = state.db.pool();
    
    match UserRepository::get_by_id(pool, &user_id).await {
        Ok(Some(u)) => {
            (StatusCode::OK, Json(UserResponse::from(u))).into_response()
        }
        Ok(None) => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "User not found"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to get user: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get user"
            }))).into_response()
        }
    }
}

/// Create a new user
pub async fn create_user(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(request): Json<CreateUserRequest>,
) -> impl IntoResponse {
    if let Err(resp) = require_permission(&user, "users_write") {
        return resp;
    }
    
    // Validate input
    if request.username.len() < 3 {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "Username must be at least 3 characters"
        }))).into_response();
    }
    
    if request.password.len() < 8 {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "Password must be at least 8 characters"
        }))).into_response();
    }
    
    if !request.email.contains('@') || !request.email.contains('.') {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "Invalid email format"
        }))).into_response();
    }
    
    let pool = state.db.pool();
    
    // Check if username already exists
    if let Ok(Some(_)) = UserRepository::get_by_username(pool, &request.username).await {
        return (StatusCode::CONFLICT, Json(serde_json::json!({
            "error": "Username already exists"
        }))).into_response();
    }
    
    // Check if email already exists
    if let Ok(Some(_)) = UserRepository::get_by_email(pool, &request.email).await {
        return (StatusCode::CONFLICT, Json(serde_json::json!({
            "error": "Email already exists"
        }))).into_response();
    }
    
    // Hash password
    let password_hash = match hash_password(&request.password) {
        Ok(hash) => hash,
        Err(e) => {
            error!("Failed to hash password: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to process password"
            }))).into_response();
        }
    };
    
    // Create user
    match UserRepository::create(pool, &request.username, &request.email, &password_hash, &request.role_id).await {
        Ok(new_user) => {
            info!("User created: {} by {}", request.username, user.user_id);
            (StatusCode::CREATED, Json(UserResponse::from(new_user))).into_response()
        }
        Err(e) => {
            error!("Failed to create user: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to create user"
            }))).into_response()
        }
    }
}

/// Update a user
pub async fn update_user(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path(user_id): Path<String>,
    Json(request): Json<UpdateUserRequest>,
) -> impl IntoResponse {
    // Users can update their own profile (limited fields)
    // Admin permission required for full updates
    let is_self = auth_user.user_id == user_id;
    
    if !is_self {
        if let Err(resp) = require_permission(&auth_user, "users_write") {
            return resp;
        }
    }
    
    // Non-admins cannot change their own role
    if is_self && request.role_id.is_some() && !auth_user.has_permission("users_write") {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({
            "error": "Cannot change your own role"
        }))).into_response();
    }
    
    let pool = state.db.pool();
    
    // Get existing user
    let existing = match UserRepository::get_by_id(pool, &user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "User not found"
            }))).into_response();
        }
        Err(e) => {
            error!("Failed to get user: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Database error"
            }))).into_response();
        }
    };
    
    // Build update query
    let existing_username = existing.username.unwrap_or_default();
    let existing_email = existing.email.unwrap_or_default();
    let new_username = request.username.as_ref().unwrap_or(&existing_username);
    let new_email = request.email.as_ref().unwrap_or(&existing_email);
    let new_role = request.role_id.as_ref().unwrap_or(&existing.role_id);
    let new_active = request.is_active.unwrap_or(existing.is_active);
    
    // Hash new password if provided
    let new_password_hash = if let Some(ref password) = request.password {
        if password.len() < 8 {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Password must be at least 8 characters"
            }))).into_response();
        }
        Some(match hash_password(password) {
            Ok(hash) => hash,
            Err(_) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                    "error": "Failed to process password"
                }))).into_response();
            }
        })
    } else {
        None
    };
    
    // Execute update
    let result = sqlx::query_as::<_, crate::db::models::User>(
        r#"
        UPDATE users SET 
            username = ?,
            email = ?,
            password_hash = COALESCE(?, password_hash),
            role_id = ?,
            is_active = ?,
            updated_at = datetime('now')
        WHERE id = ?
        RETURNING *
        "#
    )
    .bind(new_username)
    .bind(new_email)
    .bind(&new_password_hash)
    .bind(new_role)
    .bind(new_active)
    .bind(&user_id)
    .fetch_optional(pool)
    .await;
    
    match result {
        Ok(Some(updated)) => {
            info!("User updated: {} by {}", user_id, auth_user.user_id);
            (StatusCode::OK, Json(UserResponse::from(updated))).into_response()
        }
        Ok(None) => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "User not found"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to update user: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to update user"
            }))).into_response()
        }
    }
}

/// Delete a user
pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    if let Err(resp) = require_permission(&user, "users_delete") {
        return resp;
    }
    
    // Cannot delete yourself
    if user.user_id == user_id {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "Cannot delete your own account"
        }))).into_response();
    }
    
    let pool = state.db.pool();
    
    match UserRepository::delete(pool, &user_id).await {
        Ok(true) => {
            info!("User deleted: {} by {}", user_id, user.user_id);
            (StatusCode::NO_CONTENT).into_response()
        }
        Ok(false) => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "User not found"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to delete user: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to delete user"
            }))).into_response()
        }
    }
}

/// Hash a password using Argon2
fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| e.to_string())
}

//! Authentication and setup API endpoints
//!
//! Provides endpoints for:
//! - First-time setup wizard
//! - User login/logout
//! - Session management

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn, error};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use crate::AppState;
use crate::db::models::UserResponse;
use crate::db::repository::UserRepository;

/// Response for setup status check
#[derive(Debug, Serialize)]
pub struct SetupStatusResponse {
    /// Whether setup has been completed (admin user exists)
    pub is_setup: bool,
    /// Application version
    pub version: String,
}

/// Request to initialize the application (create first admin)
#[derive(Debug, Deserialize)]
pub struct SetupInitRequest {
    /// Username for the admin user
    pub username: String,
    /// Email for the admin user  
    pub email: String,
    /// Password for the admin user
    pub password: String,
}

/// Response for setup initialization
#[derive(Debug, Serialize)]
pub struct SetupInitResponse {
    pub success: bool,
    pub message: String,
    pub user: Option<UserResponse>,
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Username or email
    pub identifier: String,
    /// Password
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
    pub token: Option<String>,
    pub user: Option<UserResponse>,
}

/// Check if initial setup has been completed
pub async fn setup_status(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.db.is_fresh().await {
        Ok(is_fresh) => {
            (StatusCode::OK, Json(SetupStatusResponse {
                is_setup: !is_fresh,
                version: env!("CARGO_PKG_VERSION").to_string(),
            })).into_response()
        }
        Err(e) => {
            error!("Failed to check setup status: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to check setup status"
            }))).into_response()
        }
    }
}

/// Initialize the application with the first admin user
pub async fn setup_init(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SetupInitRequest>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Check if setup has already been completed
    match state.db.is_fresh().await {
        Ok(true) => {}, // Fresh database, proceed with setup
        Ok(false) => {
            return (StatusCode::CONFLICT, Json(SetupInitResponse {
                success: false,
                message: "Setup has already been completed".to_string(),
                user: None,
            })).into_response();
        }
        Err(e) => {
            error!("Failed to check setup status: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(SetupInitResponse {
                success: false,
                message: "Failed to check setup status".to_string(),
                user: None,
            })).into_response();
        }
    }
    
    // Validate input
    if request.username.len() < 3 {
        return (StatusCode::BAD_REQUEST, Json(SetupInitResponse {
            success: false,
            message: "Username must be at least 3 characters".to_string(),
            user: None,
        })).into_response();
    }
    
    if request.password.len() < 8 {
        return (StatusCode::BAD_REQUEST, Json(SetupInitResponse {
            success: false,
            message: "Password must be at least 8 characters".to_string(),
            user: None,
        })).into_response();
    }
    
    // Validate email format (basic check)
    if !request.email.contains('@') || !request.email.contains('.') {
        return (StatusCode::BAD_REQUEST, Json(SetupInitResponse {
            success: false,
            message: "Invalid email format".to_string(),
            user: None,
        })).into_response();
    }
    
    // Hash the password
    let password_hash = match hash_password(&request.password) {
        Ok(hash) => hash,
        Err(e) => {
            error!("Failed to hash password: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(SetupInitResponse {
                success: false,
                message: "Failed to process password".to_string(),
                user: None,
            })).into_response();
        }
    };
    
    // Create the admin user
    match UserRepository::create(
        pool,
        &request.username,
        &request.email,
        &password_hash,
        "admin", // Use the built-in admin role
    ).await {
        Ok(user) => {
            info!("Initial admin user created: {}", request.username);
            (StatusCode::CREATED, Json(SetupInitResponse {
                success: true,
                message: "Admin user created successfully".to_string(),
                user: Some(UserResponse::from(user)),
            })).into_response()
        }
        Err(e) => {
            error!("Failed to create admin user: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(SetupInitResponse {
                success: false,
                message: format!("Failed to create admin user: {}", e),
                user: None,
            })).into_response()
        }
    }
}

/// Login with username/email and password
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LoginRequest>,
) -> impl IntoResponse {
    let pool = state.db.pool();
    
    // Try to find user by username or email
    let user = if request.identifier.contains('@') {
        UserRepository::get_by_email(pool, &request.identifier).await
    } else {
        UserRepository::get_by_username(pool, &request.identifier).await
    };
    
    let user = match user {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (StatusCode::UNAUTHORIZED, Json(LoginResponse {
                success: false,
                message: "Invalid credentials".to_string(),
                token: None,
                user: None,
            })).into_response();
        }
        Err(e) => {
            error!("Database error during login: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(LoginResponse {
                success: false,
                message: "Login failed".to_string(),
                token: None,
                user: None,
            })).into_response();
        }
    };
    
    // Check if user is active
    if !user.is_active {
        return (StatusCode::UNAUTHORIZED, Json(LoginResponse {
            success: false,
            message: "Account is disabled".to_string(),
            token: None,
            user: None,
        })).into_response();
    }
    
    // Verify password
    let password_hash = match &user.password_hash {
        Some(hash) => hash,
        None => {
            // SSO user trying to login with password
            return (StatusCode::UNAUTHORIZED, Json(LoginResponse {
                success: false,
                message: "Please use SSO to login".to_string(),
                token: None,
                user: None,
            })).into_response();
        }
    };
    
    if !verify_password(&request.password, password_hash) {
        return (StatusCode::UNAUTHORIZED, Json(LoginResponse {
            success: false,
            message: "Invalid credentials".to_string(),
            token: None,
            user: None,
        })).into_response();
    }
    
    // Update last login time
    if let Err(e) = UserRepository::update_last_login(pool, &user.id).await {
        warn!("Failed to update last login time: {}", e);
    }
    
    // Generate JWT token
    let token = match generate_jwt(&user.id, &user.role_id) {
        Ok(token) => token,
        Err(e) => {
            error!("Failed to generate JWT: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(LoginResponse {
                success: false,
                message: "Failed to create session".to_string(),
                token: None,
                user: None,
            })).into_response();
        }
    };
    
    info!("User logged in: {}", user.username.as_deref().unwrap_or("unknown"));
    
    (StatusCode::OK, Json(LoginResponse {
        success: true,
        message: "Login successful".to_string(),
        token: Some(token),
        user: Some(UserResponse::from(user)),
    })).into_response()
}

/// Logout (invalidate session)
pub async fn logout() -> impl IntoResponse {
    // For JWT-based auth, client just discards the token
    // In the future, we could add token blacklisting
    (StatusCode::OK, Json(serde_json::json!({
        "success": true,
        "message": "Logged out successfully"
    }))).into_response()
}

/// Get current user info (from JWT token)
pub async fn current_user() -> impl IntoResponse {
    // For now, return unauthorized - will be implemented with auth middleware
    // TODO: Extract user from JWT token in auth middleware
    (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
        "error": "Not authenticated"
    }))).into_response()
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

/// Verify a password against a hash
fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(hash) => hash,
        Err(_) => return false,
    };
    
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

/// Generate a JWT token for a user
fn generate_jwt(user_id: &str, role_id: &str) -> Result<String, String> {
    use jsonwebtoken::{encode, Header, EncodingKey};
    
    #[derive(Serialize)]
    struct Claims {
        sub: String,      // User ID
        role: String,     // Role ID
        exp: usize,       // Expiration time
        iat: usize,       // Issued at
    }
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as usize;
    
    let claims = Claims {
        sub: user_id.to_string(),
        role: role_id.to_string(),
        exp: now + 24 * 60 * 60, // 24 hours
        iat: now,
    };
    
    // In production, this should be loaded from environment/config
    let secret = std::env::var("VECTORIZE_JWT_SECRET")
        .unwrap_or_else(|_| "vectorize-development-secret-change-in-production".to_string());
    
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| e.to_string())
}

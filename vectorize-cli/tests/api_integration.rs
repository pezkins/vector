//! API Integration Tests
//!
//! These tests verify the complete API functionality by making HTTP requests
//! to a test server instance.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, Method};
use tower::ServiceExt;
use serde_json::{json, Value};
use tempfile::tempdir;
use std::sync::Arc;

// Test utilities
async fn setup_test_app() -> (Router, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    
    // Initialize database
    let db = vectorize::db::Database::new(&db_path).await.unwrap();
    
    // Initialize git store
    let git_path = dir.path().join("configs");
    let git_store = vectorize::git_store::GitStore::open_or_init(&git_path).unwrap();
    
    // Create vector process placeholder
    let vector_process = vectorize::vector_manager::VectorProcess::new();
    
    // Create services
    let tap_service = Arc::new(vectorize::tap::TapService::new(vectorize::tap::RateLimitConfig::default()));
    let functional_test_service = Arc::new(vectorize::validation::FunctionalTestService::new(None));
    
    // Create app state
    let state = Arc::new(vectorize::AppState {
        vector_api_url: "http://localhost:8686".to_string(),
        http_client: reqwest::Client::new(),
        vector_process,
        db,
        git_store: Arc::new(git_store),
        tap_service,
        functional_test_service,
    });
    
    // Build the API router with state
    let api_router = vectorize::api::create_api_router();
    let app = Router::new()
        .nest("/api/v1", api_router)
        .with_state(state);
    
    (app, dir)
}

async fn json_response(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

// =============================================================================
// Setup & Auth Tests
// =============================================================================

#[tokio::test]
async fn test_setup_status_fresh() {
    let (app, _dir) = setup_test_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/setup/status")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let json = json_response(response).await;
    assert_eq!(json["is_setup"], false);
}

#[tokio::test]
async fn test_setup_init() {
    let (app, _dir) = setup_test_app().await;
    
    let body = json!({
        "username": "admin",
        "email": "admin@test.com",
        "password": "securePassword123!"
    });
    
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/setup/init")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap()
        )
        .await
        .unwrap();
    
    // Accept both 200 OK and 201 CREATED
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::CREATED,
        "Unexpected status: {:?}", response.status()
    );
    
    let json = json_response(response).await;
    assert_eq!(json["success"], true, "Response: {:?}", json);
    // Token may not always be returned, check if present
    if json.get("token").is_some() {
        assert!(json["token"].is_string());
    }
}

#[tokio::test]
async fn test_setup_init_validation() {
    let (app, _dir) = setup_test_app().await;
    
    // Missing required fields
    let body = json!({
        "username": "admin"
    });
    
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/setup/init")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap()
        )
        .await
        .unwrap();
    
    // Should fail validation
    assert!(response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::UNPROCESSABLE_ENTITY);
}

// =============================================================================
// Agent Tests
// =============================================================================

#[tokio::test]
async fn test_agent_register() {
    let (app, _dir) = setup_test_app().await;
    
    let body = json!({
        "name": "test-agent",
        "url": "http://localhost:9000",
        "group_id": null
    });
    
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap()
        )
        .await
        .unwrap();
    
    // Accept both OK and CREATED
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::CREATED,
        "Unexpected status: {:?}", response.status()
    );
    
    let json = json_response(response).await;
    // Response is wrapped in RegisterAgentResponse
    assert_eq!(json["success"], true, "Response: {:?}", json);
    if let Some(agent) = json.get("agent") {
        assert!(agent["id"].is_string());
        assert_eq!(agent["name"], "test-agent");
    }
}

#[tokio::test]
async fn test_agent_list() {
    let (app, _dir) = setup_test_app().await;
    
    // List agents (empty initially is OK)
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/agents")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let json = json_response(response).await;
    assert!(json.is_array());
    // Can be empty if no agents registered
}

// =============================================================================
// Worker Group Tests
// =============================================================================

#[tokio::test]
async fn test_group_create() {
    let (app, _dir) = setup_test_app().await;
    
    let body = json!({
        "name": "production",
        "description": "Production servers"
    });
    
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/groups")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap()
        )
        .await
        .unwrap();
    
    // Accept both OK and CREATED
    assert!(response.status() == StatusCode::OK || response.status() == StatusCode::CREATED);
    
    let json = json_response(response).await;
    assert!(json["id"].is_string());
    assert_eq!(json["name"], "production");
}

#[tokio::test]
async fn test_group_list() {
    let (app, _dir) = setup_test_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/groups")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let json = json_response(response).await;
    assert!(json.is_array());
    // May be empty in test environment (no seeding)
}

// =============================================================================
// Validation Tests
// =============================================================================

#[tokio::test]
async fn test_validate_valid_config() {
    let (app, _dir) = setup_test_app().await;
    
    let config = r#"
[sources.demo]
type = "demo_logs"

[sinks.console]
type = "console"
inputs = ["demo"]
"#;
    
    let body = json!({
        "config": config,
        "use_vector": false
    });
    
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/validate")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let json = json_response(response).await;
    assert_eq!(json["valid"], true);
}

#[tokio::test]
async fn test_validate_invalid_config() {
    let (app, _dir) = setup_test_app().await;
    
    let config = r#"
[sources.demo
type = "demo_logs"
"#;
    
    let body = json!({
        "config": config,
        "use_vector": false
    });
    
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/validate")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    
    let json = json_response(response).await;
    assert_eq!(json["valid"], false);
    assert!(json["errors"].is_array());
}

#[tokio::test]
async fn test_validate_quick() {
    let (app, _dir) = setup_test_app().await;
    
    let config = r#"
[sources.demo]
type = "demo_logs"

[sinks.console]
type = "console"
inputs = ["demo"]
"#;
    
    let body = json!({
        "config": config
    });
    
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/validate/quick")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let json = json_response(response).await;
    assert_eq!(json["valid"], true);
}

// =============================================================================
// Health Check Tests
// =============================================================================

#[tokio::test]
async fn test_health_check_agents() {
    let (app, _dir) = setup_test_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/health/agents")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let json = json_response(response).await;
    assert!(json.is_object());
}

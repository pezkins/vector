//! Web Server for Vectorize UI
//!
//! Serves the embedded web UI and proxies API requests to Vector.

use axum::{
    body::Body,
    extract::State,
    http::{header, Request, Response, StatusCode},
    response::IntoResponse,
    routing::{any, get},
    Router,
    Json,
};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, error};

use crate::api;
use crate::db::Database;
use crate::git_store::GitStore;
use crate::tap::{TapService, RateLimitConfig};
use crate::validation::FunctionalTestService;
use crate::vector_manager::VectorProcess;

/// Embedded UI assets (compiled WASM app)
#[derive(RustEmbed)]
#[folder = "../ui/dist/"]
struct UiAssets;

/// Server state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub vector_api_url: String,
    pub http_client: reqwest::Client,
    pub vector_process: VectorProcess,
    pub db: Database,
    pub git_store: Arc<GitStore>,
    pub tap_service: Arc<TapService>,
    pub functional_test_service: Arc<FunctionalTestService>,
}

/// Start the web server
pub async fn start_server(
    port: u16,
    vector_api_port: u16,
    vector_process: VectorProcess,
    db: Database,
    git_store: GitStore,
) -> anyhow::Result<tokio::task::JoinHandle<anyhow::Result<()>>> {
    // Create services
    let vector_bin = vector_process.get_binary_path();
    let tap_service = Arc::new(TapService::new(RateLimitConfig::default()));
    let functional_test_service = Arc::new(FunctionalTestService::new(vector_bin));
    
    let state = Arc::new(AppState {
        vector_api_url: format!("http://127.0.0.1:{}", vector_api_port),
        http_client: reqwest::Client::new(),
        vector_process,
        db,
        git_store: Arc::new(git_store),
        tap_service,
        functional_test_service,
    });

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Create the control plane API router
    let control_plane_api = api::create_api_router();

    let app = Router::new()
        // Health check
        .route("/health", get(health_check))
        // API info endpoint
        .route("/api/info", get(api_info))
        // Proxy to Vector's health endpoint
        .route("/api/health", get(proxy_health))
        // Proxy to Vector's GraphQL API
        .route("/api/graphql", any(proxy_graphql))
        // Config endpoints - GET to read current config, POST to deploy new config
        .route("/api/config", get(get_config).post(deploy_config))
        // Control plane API (agents, groups, auth)
        .nest("/api/v1", control_plane_api)
        // Serve UI assets - index.html for root
        .route("/", get(serve_index))
        // Use fallback for all other paths (static files and SPA routing)
        .fallback(serve_static)
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    info!("UI server listening on {}", addr);

    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .map_err(|e| anyhow::anyhow!("Server error: {}", e))
    });

    Ok(handle)
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    tracing::info!("Health check endpoint called");
    (StatusCode::OK, "OK")
}

/// API info endpoint - returns Vector API URL and capabilities for the UI
async fn api_info(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Check if we can deploy (have config path access)
    let can_deploy = state.vector_process.config_path().await.is_some();
    
    let info = serde_json::json!({
        "vector_api_url": state.vector_api_url,
        "version": env!("CARGO_PKG_VERSION"),
        "can_deploy": can_deploy,
        "mode": if can_deploy { "managed" } else { "standalone" }
    });
    (StatusCode::OK, axum::Json(info))
}

/// Proxy health check to Vector's API
async fn proxy_health(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let url = format!("{}/health", state.vector_api_url);
    match state.http_client.get(&url).send().await {
        Ok(response) if response.status().is_success() => {
            (StatusCode::OK, "OK").into_response()
        }
        Ok(response) => {
            (StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY), 
             "Vector health check failed").into_response()
        }
        Err(e) => {
            (StatusCode::BAD_GATEWAY, format!("Failed to reach Vector: {}", e)).into_response()
        }
    }
}

/// Proxy GraphQL requests to Vector's API
async fn proxy_graphql(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> impl IntoResponse {
    let url = format!("{}/graphql", state.vector_api_url);
    proxy_request(&state.http_client, &url, req).await
}

/// Deploy configuration request/response types
/// Accepts either raw TOML, a pipeline object, or the full PipelineConfig format
#[derive(Debug, Deserialize)]
struct DeployConfigRequest {
    /// TOML configuration content (direct TOML string)
    #[serde(default)]
    toml: Option<String>,
    /// Sources configuration
    #[serde(default)]
    sources: Option<serde_json::Value>,
    /// Transforms configuration  
    #[serde(default)]
    transforms: Option<serde_json::Value>,
    /// Sinks configuration
    #[serde(default)]
    sinks: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct DeployConfigResponse {
    success: bool,
    message: String,
    /// Whether this instance can deploy (has Vector process access)
    #[serde(skip_serializing_if = "Option::is_none")]
    can_deploy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Get current configuration from disk
/// Reads the TOML config file and converts it to JSON format
async fn get_config(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Get the config path - only available in managed mode
    let config_path = match state.vector_process.config_path().await {
        Some(path) => path,
        None => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Config not available in standalone mode"
            }))).into_response();
        }
    };
    
    // Read the config file
    let toml_content = match std::fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("Failed to read config file: {}", e)
            }))).into_response();
        }
    };
    
    // Parse TOML to structured config
    let config: toml::Value = match toml::from_str(&toml_content) {
        Ok(config) => config,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("Failed to parse config TOML: {}", e)
            }))).into_response();
        }
    };
    
    // Convert TOML value to JSON for easier handling
    let json_config = toml_to_json(&config);
    
    (StatusCode::OK, Json(json_config)).into_response()
}

/// Convert TOML value to JSON value
fn toml_to_json(toml_val: &toml::Value) -> serde_json::Value {
    match toml_val {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::json!(*i),
        toml::Value::Float(f) => serde_json::json!(*f),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
        toml::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(toml_to_json).collect())
        }
        toml::Value::Table(table) => {
            let mut map = serde_json::Map::new();
            for (k, v) in table {
                map.insert(k.clone(), toml_to_json(v));
            }
            serde_json::Value::Object(map)
        }
    }
}

/// Deploy configuration - writes to disk and reloads Vector via SIGHUP
/// This only works when Vectorize is managing Vector (managed mode).
/// In standalone mode (connecting to external Vector), deployment is not supported.
async fn deploy_config(
    State(state): State<Arc<AppState>>,
    Json(request): Json<DeployConfigRequest>,
) -> impl IntoResponse {
    // Get the config path - only available in managed mode
    let config_path = match state.vector_process.config_path().await {
        Some(path) => path,
        None => {
            return (StatusCode::BAD_REQUEST, Json(DeployConfigResponse {
                success: false,
                message: "Deployment not supported in standalone mode".to_string(),
                can_deploy: Some(false),
                error: Some("Connect to a Vectorize-managed Vector instance to deploy configuration changes. In standalone mode, update Vector's configuration files directly.".to_string()),
            }));
        }
    };
    
    // Generate TOML content
    let toml_content = if let Some(toml) = request.toml {
        // Direct TOML string provided
        toml
    } else if request.sources.is_some() || request.transforms.is_some() || request.sinks.is_some() {
        // PipelineConfig format - convert to TOML
        let pipeline = serde_json::json!({
            "sources": request.sources.unwrap_or(serde_json::json!({})),
            "transforms": request.transforms.unwrap_or(serde_json::json!({})),
            "sinks": request.sinks.unwrap_or(serde_json::json!({})),
        });
        match pipeline_to_toml(&pipeline) {
            Ok(toml) => toml,
            Err(e) => {
                return (StatusCode::BAD_REQUEST, Json(DeployConfigResponse {
                    success: false,
                    message: "Failed to convert pipeline to TOML".to_string(),
                    can_deploy: Some(true),
                    error: Some(e),
                }));
            }
        }
    } else {
        return (StatusCode::BAD_REQUEST, Json(DeployConfigResponse {
            success: false,
            message: "No configuration provided".to_string(),
            can_deploy: Some(true),
            error: Some("Provide 'toml', or 'sources'/'transforms'/'sinks' fields".to_string()),
        }));
    };
    
    info!("Writing config to {:?}", config_path);
    
    // Write the config to disk
    // Vector is started with --watch-config (-w) flag, so it will auto-reload
    if let Err(e) = std::fs::write(&config_path, &toml_content) {
        error!("Failed to write config: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(DeployConfigResponse {
            success: false,
            message: "Failed to write config file".to_string(),
            can_deploy: Some(true),
            error: Some(e.to_string()),
        }));
    }
    
    info!("Config written to {:?} - Vector will auto-reload", config_path);
    (StatusCode::OK, Json(DeployConfigResponse {
        success: true,
        message: "Configuration deployed - Vector will auto-reload".to_string(),
        can_deploy: Some(true),
        error: None,
    }))
}

/// Convert pipeline JSON to Vector TOML config
fn pipeline_to_toml(pipeline: &serde_json::Value) -> Result<String, String> {
    let mut toml = String::new();
    
    // Add API config header
    toml.push_str("# Vectorize Pipeline Configuration\n");
    toml.push_str("# Auto-generated - do not edit manually\n\n");
    toml.push_str("[api]\n");
    toml.push_str("enabled = true\n");
    toml.push_str("address = \"127.0.0.1:8686\"\n");
    toml.push_str("playground = true\n\n");
    
    // Process sources
    if let Some(sources) = pipeline.get("sources").and_then(|s| s.as_object()) {
        for (name, config) in sources {
            toml.push_str(&format!("[sources.{}]\n", name));
            if let Some(obj) = config.as_object() {
                for (key, value) in obj {
                    toml.push_str(&format_toml_value(key, value));
                }
            }
            toml.push('\n');
        }
    }
    
    // Process transforms
    toml.push_str("[transforms]\n\n");
    if let Some(transforms) = pipeline.get("transforms").and_then(|t| t.as_object()) {
        for (name, config) in transforms {
            toml.push_str(&format!("[transforms.{}]\n", name));
            if let Some(obj) = config.as_object() {
                // Check if this is a filter with condition_type
                let is_filter = obj.get("type").and_then(|v| v.as_str()) == Some("filter");
                let condition_type = obj.get("condition_type").and_then(|v| v.as_str());
                let condition_value = obj.get("condition").and_then(|v| v.as_str());
                
                for (key, value) in obj {
                    // Skip condition_type - we'll handle it with condition
                    if key == "condition_type" {
                        continue;
                    }
                    
                    // For filter transforms, combine condition_type and condition
                    if is_filter && key == "condition" {
                        if let Some(cond) = condition_value {
                            if let Some(ctype) = condition_type {
                                if ctype != "vrl" {
                                    // Non-VRL condition type - use object syntax
                                    toml.push_str(&format!("condition = {{ type = \"{}\", source = \"{}\" }}\n", ctype, cond));
                                } else {
                                    // VRL - use shorthand
                                    toml.push_str(&format_toml_value(key, value));
                                }
                            } else {
                                // No condition_type, use shorthand
                                toml.push_str(&format_toml_value(key, value));
                            }
                        } else {
                            // No condition value, skip
                        }
                        continue;
                    }
                    
                    toml.push_str(&format_toml_value(key, value));
                }
            }
            toml.push('\n');
        }
    }
    
    // Process sinks
    if let Some(sinks) = pipeline.get("sinks").and_then(|s| s.as_object()) {
        for (name, config) in sinks {
            toml.push_str(&format!("[sinks.{}]\n", name));
            if let Some(obj) = config.as_object() {
                for (key, value) in obj {
                    toml.push_str(&format_toml_value(key, value));
                }
            }
            toml.push('\n');
        }
    }
    
    Ok(toml)
}

/// Format a single TOML key-value pair
fn format_toml_value(key: &str, value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => {
            // Use triple single quotes for strings containing quotes (VRL expressions, etc.)
            // This avoids escaping issues with complex expressions
            if s.contains('"') || s.contains('\\') {
                format!("{} = '''\n{}\n'''\n", key, s)
            } else {
                format!("{} = \"{}\"\n", key, s)
            }
        }
        serde_json::Value::Number(n) => format!("{} = {}\n", key, n),
        serde_json::Value::Bool(b) => format!("{} = {}\n", key, b),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter()
                .filter_map(|v| v.as_str().map(|s| format!("\"{}\"", s)))
                .collect();
            format!("{} = [{}]\n", key, items.join(", "))
        }
        serde_json::Value::Object(obj) => {
            // Handle nested objects like encoding.codec
            let mut result = String::new();
            for (k, v) in obj {
                result.push_str(&format_toml_value(&format!("{}.{}", key, k), v));
            }
            result
        }
        serde_json::Value::Null => String::new(),
    }
}

/// Generic request proxy
async fn proxy_request(
    client: &reqwest::Client,
    url: &str,
    req: Request<Body>,
) -> impl IntoResponse {
    let method = req.method().clone();
    let headers = req.headers().clone();
    
    // Read the body
    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read request body: {}", e),
            ).into_response();
        }
    };

    // Build the proxied request
    let mut proxy_req = client.request(method, url);
    
    // Copy relevant headers
    for (name, value) in headers.iter() {
        if name != header::HOST {
            proxy_req = proxy_req.header(name, value);
        }
    }
    
    proxy_req = proxy_req.body(body_bytes);

    // Execute the request
    match proxy_req.send().await {
        Ok(response) => {
            let status = StatusCode::from_u16(response.status().as_u16())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let headers = response.headers().clone();
            let body = response.bytes().await.unwrap_or_default();

            let mut res = Response::builder().status(status);
            for (name, value) in headers.iter() {
                res = res.header(name, value);
            }
            res.body(Body::from(body)).unwrap().into_response()
        }
        Err(e) => {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to proxy request to Vector: {}", e),
            ).into_response()
        }
    }
}

/// Serve index.html
async fn serve_index() -> impl IntoResponse {
    serve_file("index.html")
}

/// Serve static files from embedded assets (fallback handler)
async fn serve_static(req: Request<Body>) -> impl IntoResponse {
    let path = req.uri().path().to_string();
    tracing::debug!("Fallback handler called for path: {}", path);
    
    // Try the exact path first
    if let Some(response) = try_serve_file(&path) {
        return response;
    }
    
    // For SPA routing, serve index.html for non-asset paths
    if !path.contains('.') {
        return serve_file("index.html");
    }

    // 404 for missing assets
    (StatusCode::NOT_FOUND, "Not Found").into_response()
}

fn try_serve_file(path: &str) -> Option<Response<Body>> {
    let path = path.trim_start_matches('/');
    UiAssets::get(path).map(|content| {
        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();
        
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .body(Body::from(content.data.to_vec()))
            .unwrap()
    })
}

fn serve_file(path: &str) -> Response<Body> {
    match UiAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string();
            
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .body(Body::from(content.data.to_vec()))
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not Found"))
            .unwrap(),
    }
}

//! Web Server for Vectorize UI
//!
//! Serves the embedded web UI and proxies API requests to Vector.

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, Request, Response, StatusCode},
    response::IntoResponse,
    routing::{any, get},
    Router,
};
use rust_embed::RustEmbed;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

/// Embedded UI assets (compiled WASM app)
#[derive(RustEmbed)]
#[folder = "../ui/dist/"]
struct UiAssets;

/// Server state
#[derive(Clone)]
struct AppState {
    vector_api_url: String,
    http_client: reqwest::Client,
}

/// Start the web server
pub async fn start_server(
    port: u16,
    vector_api_port: u16,
) -> anyhow::Result<tokio::task::JoinHandle<anyhow::Result<()>>> {
    let state = Arc::new(AppState {
        vector_api_url: format!("http://127.0.0.1:{}", vector_api_port),
        http_client: reqwest::Client::new(),
    });

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Health check
        .route("/health", get(health_check))
        // API info endpoint
        .route("/api/info", get(api_info))
        // Proxy to Vector's health endpoint
        .route("/api/health", get(proxy_health))
        // Proxy to Vector's GraphQL API
        .route("/api/graphql", any(proxy_graphql))
        // Proxy to Vector's config reload endpoint (RFC 541)
        .route("/api/config", any(proxy_config))
        // Serve UI assets
        .route("/", get(serve_index))
        .route("/*path", get(serve_static))
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
    (StatusCode::OK, "OK")
}

/// API info endpoint - returns Vector API URL for the UI
async fn api_info(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let info = serde_json::json!({
        "vector_api_url": state.vector_api_url,
        "version": env!("CARGO_PKG_VERSION"),
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

/// Proxy config requests to Vector (RFC 541)
async fn proxy_config(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> impl IntoResponse {
    let url = format!("{}/config", state.vector_api_url);
    proxy_request(&state.http_client, &url, req).await
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

/// Serve static files from embedded assets
async fn serve_static(Path(path): Path<String>) -> impl IntoResponse {
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

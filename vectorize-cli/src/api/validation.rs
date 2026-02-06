//! Configuration validation API endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, error};

use crate::AppState;
use crate::validation::{ConfigValidator, ValidationResult, FunctionalTestRequest};

/// Request to validate a configuration
#[derive(Debug, Deserialize)]
pub struct ValidateRequest {
    /// Configuration content (TOML)
    pub config: String,
    /// Whether to run Vector's validate command (slower but more thorough)
    #[serde(default)]
    pub use_vector: bool,
}

/// Response for validation
#[derive(Debug, Serialize)]
pub struct ValidateResponse {
    /// Whether validation passed
    pub valid: bool,
    /// List of errors
    pub errors: Vec<ValidationErrorResponse>,
    /// List of warnings
    pub warnings: Vec<ValidationWarningResponse>,
    /// Summary message
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ValidationErrorResponse {
    pub code: String,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub component: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ValidationWarningResponse {
    pub code: String,
    pub message: String,
    pub component: Option<String>,
}

impl From<ValidationResult> for ValidateResponse {
    fn from(result: ValidationResult) -> Self {
        Self {
            valid: result.valid,
            errors: result.errors.into_iter().map(|e| ValidationErrorResponse {
                code: e.code,
                message: e.message,
                line: e.line,
                column: e.column,
                component: e.component,
            }).collect(),
            warnings: result.warnings.into_iter().map(|w| ValidationWarningResponse {
                code: w.code,
                message: w.message,
                component: w.component,
            }).collect(),
            message: result.message,
        }
    }
}

/// Validate a configuration
pub async fn validate_config(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ValidateRequest>,
) -> impl IntoResponse {
    info!("Validating configuration (use_vector: {})", request.use_vector);
    
    // Get Vector binary path from state if available
    let vector_bin = state.vector_process.get_binary_path();
    let validator = ConfigValidator::new(vector_bin);
    
    let result = if request.use_vector {
        validator.validate_with_vector(&request.config)
    } else {
        validator.validate(&request.config)
    };
    
    let status = if result.valid {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    
    (status, Json(ValidateResponse::from(result)))
}

/// Quick validation (TOML syntax only)
pub async fn validate_quick(
    Json(request): Json<ValidateRequest>,
) -> impl IntoResponse {
    let validator = ConfigValidator::new(None);
    let result = validator.validate(&request.config);
    
    let status = if result.valid {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    
    (status, Json(ValidateResponse::from(result)))
}

// =============================================================================
// Functional Testing Endpoints (Layer 4)
// =============================================================================

/// Request to start a functional test
#[derive(Debug, Deserialize)]
pub struct StartTestRequest {
    /// Configuration content (TOML)
    pub config: String,
    /// Sample events to test with
    pub sample_events: Vec<serde_json::Value>,
    /// Source ID to inject events into (optional)
    pub source_id: Option<String>,
    /// Timeout in seconds (default: 30)
    #[serde(default = "default_test_timeout")]
    pub timeout_secs: u64,
}

fn default_test_timeout() -> u64 { 30 }

/// Response when starting a test
#[derive(Debug, Serialize)]
pub struct StartTestResponse {
    pub test_id: String,
    pub status: String,
    pub message: String,
}

/// Response for test status/results
#[derive(Debug, Serialize)]
pub struct TestResultResponse {
    pub test_id: String,
    pub status: String,
    pub input_events: usize,
    pub output_events: Vec<serde_json::Value>,
    pub output_count: usize,
    pub dropped_count: usize,
    pub duration_ms: u64,
    pub errors: Vec<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

/// Start a functional test
pub async fn start_functional_test(
    State(state): State<Arc<AppState>>,
    Json(request): Json<StartTestRequest>,
) -> impl IntoResponse {
    info!("Starting functional test with {} sample events", request.sample_events.len());
    
    // Build the test request
    let test_request = FunctionalTestRequest {
        config: request.config,
        sample_events: request.sample_events,
        source_id: request.source_id,
        timeout_secs: request.timeout_secs,
    };
    
    // Run the test
    match state.functional_test_service.run_test(test_request).await {
        Ok(test_id) => {
            (StatusCode::ACCEPTED, Json(StartTestResponse {
                test_id,
                status: "running".to_string(),
                message: "Functional test started".to_string(),
            })).into_response()
        }
        Err(e) => {
            error!("Failed to start functional test: {}", e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": e.to_string()
            }))).into_response()
        }
    }
}

/// Get functional test status/results
pub async fn get_test_result(
    State(state): State<Arc<AppState>>,
    Path(test_id): Path<String>,
) -> impl IntoResponse {
    match state.functional_test_service.get_result(&test_id).await {
        Some(result) => {
            (StatusCode::OK, Json(TestResultResponse {
                test_id: result.test_id,
                status: format!("{:?}", result.status).to_lowercase(),
                input_events: result.input_events,
                output_events: result.output_events,
                output_count: result.output_count,
                dropped_count: result.dropped_count,
                duration_ms: result.duration_ms,
                errors: result.errors,
                started_at: result.started_at,
                completed_at: result.completed_at,
            })).into_response()
        }
        None => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": format!("Test {} not found", test_id)
            }))).into_response()
        }
    }
}

/// List recent test results
pub async fn list_test_results(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let results = state.functional_test_service.list_results(20).await;
    
    let response: Vec<TestResultResponse> = results.into_iter().map(|r| TestResultResponse {
        test_id: r.test_id,
        status: format!("{:?}", r.status).to_lowercase(),
        input_events: r.input_events,
        output_events: r.output_events,
        output_count: r.output_count,
        dropped_count: r.dropped_count,
        duration_ms: r.duration_ms,
        errors: r.errors,
        started_at: r.started_at,
        completed_at: r.completed_at,
    }).collect();
    
    (StatusCode::OK, Json(response))
}

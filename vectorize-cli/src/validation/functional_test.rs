//! Functional Testing Service (Layer 4)
//!
//! Runs sample data through Vector configurations to verify transforms
//! produce expected output. This is the highest level of validation that
//! actually executes the pipeline with test data.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{info, warn, debug};
use uuid::Uuid;

// =============================================================================
// Types
// =============================================================================

/// Request to run a functional test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionalTestRequest {
    /// The Vector configuration to test (TOML string)
    pub config: String,
    /// Sample events to send through the pipeline
    pub sample_events: Vec<serde_json::Value>,
    /// Source component to inject events into (must be of type 'stdin')
    #[serde(default)]
    pub source_id: Option<String>,
    /// Timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 { 30 }

/// Result of a functional test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionalTestResult {
    /// Unique test ID
    pub test_id: String,
    /// Test status
    pub status: TestStatus,
    /// Input events count
    pub input_events: usize,
    /// Output events captured
    pub output_events: Vec<serde_json::Value>,
    /// Number of output events
    pub output_count: usize,
    /// Events dropped (input - output)
    pub dropped_count: usize,
    /// Test duration in milliseconds
    pub duration_ms: u64,
    /// Any errors encountered
    pub errors: Vec<String>,
    /// Transform-level results (if available)
    pub transform_results: HashMap<String, TransformResult>,
    /// Timestamp when test started
    pub started_at: String,
    /// Timestamp when test completed
    pub completed_at: Option<String>,
}

/// Test status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TestStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Timeout,
}

/// Results for a specific transform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformResult {
    pub component_id: String,
    pub input_count: usize,
    pub output_count: usize,
    pub dropped_count: usize,
    pub sample_output: Vec<serde_json::Value>,
}

// =============================================================================
// Functional Test Service
// =============================================================================

/// Service for running functional tests on Vector configurations
pub struct FunctionalTestService {
    /// Path to Vector binary
    vector_bin: String,
    /// Active test results (stored in memory)
    results: Arc<RwLock<HashMap<String, FunctionalTestResult>>>,
    /// Maximum tests to keep in memory
    max_results: usize,
}

impl FunctionalTestService {
    /// Create a new functional test service
    pub fn new(vector_bin: Option<String>) -> Self {
        Self {
            vector_bin: vector_bin.unwrap_or_else(|| "vector".to_string()),
            results: Arc::new(RwLock::new(HashMap::new())),
            max_results: 100,
        }
    }
    
    /// Run a functional test
    pub async fn run_test(&self, request: FunctionalTestRequest) -> Result<String, FunctionalTestError> {
        let test_id = Uuid::new_v4().to_string();
        let start = Instant::now();
        let started_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        
        info!("Starting functional test {}", test_id);
        
        // Initialize result
        let mut result = FunctionalTestResult {
            test_id: test_id.clone(),
            status: TestStatus::Running,
            input_events: request.sample_events.len(),
            output_events: Vec::new(),
            output_count: 0,
            dropped_count: 0,
            duration_ms: 0,
            errors: Vec::new(),
            transform_results: HashMap::new(),
            started_at,
            completed_at: None,
        };
        
        // Store initial result
        {
            let mut results = self.results.write().await;
            // Clean up old results if needed
            if results.len() >= self.max_results {
                // Remove oldest completed tests
                let to_remove: Vec<String> = results.iter()
                    .filter(|(_, r)| r.status == TestStatus::Completed || r.status == TestStatus::Failed)
                    .take(10)
                    .map(|(k, _)| k.clone())
                    .collect();
                for key in to_remove {
                    results.remove(&key);
                }
            }
            results.insert(test_id.clone(), result.clone());
        }
        
        // Generate test configuration
        let test_config = self.generate_test_config(&request)?;
        
        // Write config to temp file
        let temp_dir = std::env::temp_dir();
        let config_file = temp_dir.join(format!("vectorize-test-{}.toml", test_id));
        
        if let Err(e) = std::fs::write(&config_file, &test_config) {
            result.status = TestStatus::Failed;
            result.errors.push(format!("Failed to write config: {}", e));
            self.update_result(&test_id, result.clone()).await;
            return Err(FunctionalTestError::ConfigError(e.to_string()));
        }
        
        // Run Vector with the test config
        match self.run_vector_test(&config_file, &request, Duration::from_secs(request.timeout_secs)).await {
            Ok(output_events) => {
                result.output_events = output_events.clone();
                result.output_count = output_events.len();
                result.dropped_count = request.sample_events.len().saturating_sub(output_events.len());
                result.status = TestStatus::Completed;
            }
            Err(e) => {
                result.status = if matches!(e, FunctionalTestError::Timeout) {
                    TestStatus::Timeout
                } else {
                    TestStatus::Failed
                };
                result.errors.push(e.to_string());
            }
        }
        
        // Clean up temp file
        let _ = std::fs::remove_file(&config_file);
        
        // Update final result
        result.duration_ms = start.elapsed().as_millis() as u64;
        result.completed_at = Some(chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());
        
        self.update_result(&test_id, result).await;
        
        info!("Functional test {} completed", test_id);
        Ok(test_id)
    }
    
    /// Get test result by ID
    pub async fn get_result(&self, test_id: &str) -> Option<FunctionalTestResult> {
        let results = self.results.read().await;
        results.get(test_id).cloned()
    }
    
    /// List recent test results
    pub async fn list_results(&self, limit: usize) -> Vec<FunctionalTestResult> {
        let results = self.results.read().await;
        let mut all: Vec<_> = results.values().cloned().collect();
        all.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        all.truncate(limit);
        all
    }
    
    /// Update a test result
    async fn update_result(&self, test_id: &str, result: FunctionalTestResult) {
        let mut results = self.results.write().await;
        results.insert(test_id.to_string(), result);
    }
    
    /// Generate a test configuration that uses stdin source and stdout sink
    fn generate_test_config(&self, request: &FunctionalTestRequest) -> Result<String, FunctionalTestError> {
        // Parse the original config
        let original: toml::Value = toml::from_str(&request.config)
            .map_err(|e| FunctionalTestError::ConfigError(format!("Invalid TOML: {}", e)))?;
        
        let mut config = original.as_table()
            .ok_or_else(|| FunctionalTestError::ConfigError("Config must be a table".to_string()))?
            .clone();
        
        // Find the first source or use provided source_id
        let source_id = if let Some(ref id) = request.source_id {
            id.clone()
        } else if let Some(sources) = config.get("sources").and_then(|v| v.as_table()) {
            sources.keys().next()
                .ok_or_else(|| FunctionalTestError::ConfigError("No sources in config".to_string()))?
                .clone()
        } else {
            return Err(FunctionalTestError::ConfigError("No sources in config".to_string()));
        };
        
        // Replace the source with a stdin source
        let mut sources = config.get("sources")
            .and_then(|v| v.as_table())
            .cloned()
            .unwrap_or_default();
        
        let mut stdin_source = toml::map::Map::new();
        stdin_source.insert("type".to_string(), toml::Value::String("stdin".to_string()));
        stdin_source.insert("decoding".to_string(), {
            let mut decoding = toml::map::Map::new();
            decoding.insert("codec".to_string(), toml::Value::String("json".to_string()));
            toml::Value::Table(decoding)
        });
        
        sources.insert(source_id.clone(), toml::Value::Table(stdin_source));
        config.insert("sources".to_string(), toml::Value::Table(sources));
        
        // Find all sinks and replace with a single stdout sink
        let sink_inputs: Vec<String> = if let Some(sinks) = config.get("sinks").and_then(|v| v.as_table()) {
            sinks.values()
                .filter_map(|s| s.get("inputs"))
                .filter_map(|i| i.as_array())
                .flatten()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        } else {
            vec![source_id]
        };
        
        // Create stdout sink
        let mut sinks = toml::map::Map::new();
        let mut stdout_sink = toml::map::Map::new();
        stdout_sink.insert("type".to_string(), toml::Value::String("console".to_string()));
        stdout_sink.insert("inputs".to_string(), toml::Value::Array(
            sink_inputs.iter()
                .map(|s| toml::Value::String(s.clone()))
                .collect()
        ));
        stdout_sink.insert("encoding".to_string(), {
            let mut encoding = toml::map::Map::new();
            encoding.insert("codec".to_string(), toml::Value::String("json".to_string()));
            toml::Value::Table(encoding)
        });
        
        sinks.insert("__test_output".to_string(), toml::Value::Table(stdout_sink));
        config.insert("sinks".to_string(), toml::Value::Table(sinks));
        
        // Disable API for test runs
        let mut api = toml::map::Map::new();
        api.insert("enabled".to_string(), toml::Value::Boolean(false));
        config.insert("api".to_string(), toml::Value::Table(api));
        
        toml::to_string(&config)
            .map_err(|e| FunctionalTestError::ConfigError(format!("Failed to serialize config: {}", e)))
    }
    
    /// Run Vector with the test configuration
    async fn run_vector_test(
        &self,
        config_file: &PathBuf,
        request: &FunctionalTestRequest,
        timeout: Duration,
    ) -> Result<Vec<serde_json::Value>, FunctionalTestError> {
        debug!("Running Vector with config: {:?}", config_file);
        
        // Start Vector process
        let mut child = Command::new(&self.vector_bin)
            .args([
                "--config",
                config_file.to_str().unwrap_or(""),
                "--quiet",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| FunctionalTestError::VectorError(format!("Failed to start Vector: {}", e)))?;
        
        let mut stdin = child.stdin.take()
            .ok_or_else(|| FunctionalTestError::VectorError("Failed to get stdin".to_string()))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| FunctionalTestError::VectorError("Failed to get stdout".to_string()))?;
        
        // Send input events
        let events_to_send = request.sample_events.clone();
        let send_task = tokio::spawn(async move {
            // Give Vector a moment to start
            tokio::time::sleep(Duration::from_millis(500)).await;
            
            for event in events_to_send {
                let line = serde_json::to_string(&event).unwrap_or_default();
                if let Err(e) = stdin.write_all(format!("{}\n", line).as_bytes()).await {
                    warn!("Failed to write event: {}", e);
                    break;
                }
            }
            
            // Close stdin to signal EOF
            drop(stdin);
        });
        
        // Read output events with timeout
        let output_events = Arc::new(RwLock::new(Vec::new()));
        let output_events_clone = output_events.clone();
        
        let read_task = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) {
                    output_events_clone.write().await.push(event);
                }
            }
        });
        
        // Wait with timeout
        let result = tokio::time::timeout(timeout, async {
            let _ = send_task.await;
            // Give Vector time to process
            tokio::time::sleep(Duration::from_secs(2)).await;
            // Kill Vector process
            let _ = child.kill().await;
            let _ = read_task.await;
        }).await;
        
        if result.is_err() {
            let _ = child.kill().await;
            return Err(FunctionalTestError::Timeout);
        }
        
        let events = output_events.read().await.clone();
        Ok(events)
    }
}

/// Functional test error
#[derive(Debug)]
pub enum FunctionalTestError {
    ConfigError(String),
    VectorError(String),
    Timeout,
    NotFound(String),
}

impl std::fmt::Display for FunctionalTestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionalTestError::ConfigError(e) => write!(f, "Config error: {}", e),
            FunctionalTestError::VectorError(e) => write!(f, "Vector error: {}", e),
            FunctionalTestError::Timeout => write!(f, "Test timed out"),
            FunctionalTestError::NotFound(id) => write!(f, "Test not found: {}", id),
        }
    }
}

impl std::error::Error for FunctionalTestError {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_timeout() {
        assert_eq!(default_timeout(), 30);
    }
    
    #[test]
    fn test_test_status_serialization() {
        assert_eq!(
            serde_json::to_string(&TestStatus::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&TestStatus::Failed).unwrap(),
            "\"failed\""
        );
    }
    
    #[test]
    fn test_functional_test_request_deserialization() {
        let json = r#"{
            "config": "[sources.test]\ntype = \"stdin\"",
            "sample_events": [{"message": "test"}]
        }"#;
        
        let request: FunctionalTestRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.timeout_secs, 30);
        assert_eq!(request.sample_events.len(), 1);
    }
    
    #[test]
    fn test_generate_test_config() {
        let service = FunctionalTestService::new(None);
        
        let request = FunctionalTestRequest {
            config: r#"
[sources.demo]
type = "demo_logs"

[transforms.parse]
type = "remap"
inputs = ["demo"]
source = ". = parse_json!(.message)"

[sinks.console]
type = "console"
inputs = ["parse"]
"#.to_string(),
            sample_events: vec![serde_json::json!({"message": "test"})],
            source_id: None,
            timeout_secs: 30,
        };
        
        let result = service.generate_test_config(&request);
        assert!(result.is_ok());
        
        let config = result.unwrap();
        assert!(config.contains("stdin"));
        assert!(config.contains("__test_output"));
    }
}

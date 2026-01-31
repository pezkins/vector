//! Vector Runtime Integration
//!
//! This module starts Vector as an embedded runtime within the same process.
//! No separate Vector binary is needed.

use std::path::PathBuf;
use tracing::{info, error};
use tokio::task::JoinHandle;

/// Start the embedded Vector runtime
pub async fn start_vector(
    config_path: Option<&PathBuf>,
    api_port: u16,
) -> anyhow::Result<JoinHandle<anyhow::Result<()>>> {
    info!("Initializing embedded Vector runtime...");

    // Set environment variables for Vector's API
    std::env::set_var("VECTOR_API_ENABLED", "true");
    std::env::set_var("VECTOR_API_ADDRESS", format!("127.0.0.1:{}", api_port));
    std::env::set_var("VECTOR_API_PLAYGROUND", "true");

    // Build Vector's command-line arguments
    let mut args = vec!["vectorize".to_string()];
    
    if let Some(config) = config_path {
        args.push("--config".to_string());
        args.push(config.to_string_lossy().to_string());
    }

    let handle = tokio::task::spawn_blocking(move || {
        // Parse args as if Vector was called from CLI
        // This lets Vector initialize with all its normal setup
        
        info!("Starting Vector with args: {:?}", args);
        
        // Use Vector's application entry point
        let status = vector::app::Application::run(vector::extra_context::ExtraContext::default());
        
        if status.success() {
            info!("Vector exited successfully");
            Ok(())
        } else {
            error!("Vector exited with status: {:?}", status);
            Err(anyhow::anyhow!("Vector exited with error"))
        }
    });

    Ok(handle)
}

/// Generate a default Vector configuration for demo purposes
pub fn default_config() -> String {
    r#"
# Vectorize default configuration
# This config is used when no config file is provided

[api]
enabled = true
address = "127.0.0.1:8686"
playground = true

[sources.demo]
type = "demo_logs"
format = "json"
interval = 1.0

[transforms.parse]
type = "remap"
inputs = ["demo"]
source = '''
. = parse_json!(.message)
'''

[sinks.console]
type = "console"
inputs = ["parse"]
encoding.codec = "json"
"#.to_string()
}

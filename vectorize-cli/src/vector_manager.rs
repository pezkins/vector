//! Vector Process Manager
//!
//! Manages the Vector process lifecycle - starting, stopping, and monitoring.
//! Vector is built from the same workspace and runs as a subprocess.

use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, warn, error};

/// Find the Vector binary
fn find_vector_binary(specified: &Option<PathBuf>) -> PathBuf {
    // 1. Use specified path if provided
    if let Some(path) = specified {
        return path.clone();
    }

    // 2. Check same directory as vectorize binary
    if let Ok(exe_path) = std::env::current_exe() {
        let same_dir = exe_path.parent().map(|p| p.join("vector"));
        if let Some(path) = same_dir {
            if path.exists() {
                return path;
            }
        }
    }

    // 3. Fall back to PATH
    PathBuf::from("vector")
}

/// Start Vector as a subprocess
pub async fn start_vector(
    vector_bin: &Option<PathBuf>,
    config: Option<&PathBuf>,
    api_port: u16,
) -> anyhow::Result<tokio::task::JoinHandle<anyhow::Result<()>>> {
    let vector_path = find_vector_binary(vector_bin);

    info!("Starting Vector from: {:?}", vector_path);

    let mut cmd = Command::new(&vector_path);

    // Add config if provided
    if let Some(config_path) = config {
        cmd.arg("--config").arg(config_path);
    }

    // Enable API
    cmd.env("VECTOR_API_ENABLED", "true");
    cmd.env("VECTOR_API_ADDRESS", format!("127.0.0.1:{}", api_port));
    cmd.env("VECTOR_API_PLAYGROUND", "true");

    // Configure stdio
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let handle = tokio::spawn(async move {
        match cmd.spawn() {
            Ok(mut child) => {
                info!("Vector started with PID: {:?}", child.id());
                
                // Wait for Vector to exit
                match child.wait().await {
                    Ok(status) => {
                        if status.success() {
                            info!("Vector exited successfully");
                        } else {
                            warn!("Vector exited with status: {}", status);
                        }
                    }
                    Err(e) => {
                        error!("Error waiting for Vector: {}", e);
                        return Err(anyhow::anyhow!("Vector process error: {}", e));
                    }
                }
            }
            Err(e) => {
                error!("Failed to start Vector: {}", e);
                error!("");
                error!("Vector binary not found. When building from source, both 'vector' and");
                error!("'vectorize' binaries should be in target/release/");
                error!("");
                error!("Build with: cargo build --release -p vector -p vectorize");
                error!("");
                return Err(anyhow::anyhow!("Failed to start Vector: {}", e));
            }
        }
        Ok(())
    });

    Ok(handle)
}

/// Run Vector with passthrough arguments (for `vectorize vector ...` command)
pub async fn run_vector_passthrough(
    vector_bin: &Option<PathBuf>,
    args: Vec<String>,
) -> anyhow::Result<()> {
    let vector_path = find_vector_binary(vector_bin);

    info!("Running Vector with args: {:?}", args);

    let mut cmd = Command::new(&vector_path);
    cmd.args(&args);
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    cmd.stdin(Stdio::inherit());

    let status = cmd.status().await?;
    
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

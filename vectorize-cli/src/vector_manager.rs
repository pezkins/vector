//! Vector Process Manager
//!
//! Manages the Vector process lifecycle - starting, stopping, and monitoring.
//! Vector is built from the same workspace and runs as a subprocess.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// Shared state for the Vector process
#[derive(Clone)]
pub struct VectorProcess {
    inner: Arc<RwLock<VectorProcessInner>>,
    binary_path: Option<String>,
}

struct VectorProcessInner {
    pid: Option<u32>,
    config_path: Option<PathBuf>,
}

impl VectorProcess {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(VectorProcessInner {
                pid: None,
                config_path: None,
            })),
            binary_path: None,
        }
    }
    
    /// Create with a specific binary path
    pub fn with_binary_path(binary_path: PathBuf) -> Self {
        Self {
            inner: Arc::new(RwLock::new(VectorProcessInner {
                pid: None,
                config_path: None,
            })),
            binary_path: Some(binary_path.to_string_lossy().to_string()),
        }
    }
    
    /// Get the Vector binary path
    pub fn get_binary_path(&self) -> Option<String> {
        self.binary_path.clone()
    }
    
    /// Set the Vector process info
    pub async fn set_process(&self, pid: u32, config_path: Option<PathBuf>) {
        let mut inner = self.inner.write().await;
        inner.pid = Some(pid);
        inner.config_path = config_path;
    }
    
    /// Get the config path
    pub async fn config_path(&self) -> Option<PathBuf> {
        self.inner.read().await.config_path.clone()
    }
    
    /// Reload Vector configuration by sending SIGHUP
    #[cfg(unix)]
    pub async fn reload_config(&self) -> Result<(), String> {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;
        
        let inner = self.inner.read().await;
        if let Some(pid) = inner.pid {
            info!("Sending SIGHUP to Vector (PID: {})", pid);
            kill(Pid::from_raw(pid as i32), Signal::SIGHUP)
                .map_err(|e| format!("Failed to send SIGHUP: {}", e))?;
            Ok(())
        } else {
            Err("Vector process not running".to_string())
        }
    }
    
    #[cfg(not(unix))]
    pub async fn reload_config(&self) -> Result<(), String> {
        Err("Config reload via SIGHUP not supported on this platform".to_string())
    }
}

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
    process_state: VectorProcess,
) -> anyhow::Result<(tokio::task::JoinHandle<anyhow::Result<()>>, PathBuf)> {
    let vector_path = find_vector_binary(vector_bin);
    let config_path = config.cloned();
    let vector_path_clone = vector_path.clone();

    info!("Starting Vector from: {:?}", vector_path);

    let mut cmd = Command::new(&vector_path);

    // Add config if provided - use -w to watch for config changes
    if let Some(ref config_path) = config_path {
        cmd.arg("--config").arg(config_path);
        cmd.arg("-w");  // Watch for config file changes
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
                let pid = child.id();
                info!("Vector started with PID: {:?}", pid);
                
                // Store the PID in shared state
                if let Some(pid) = pid {
                    process_state.set_process(pid, config_path).await;
                }
                
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

    Ok((handle, vector_path_clone))
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

//! Vectorize - Visual Pipeline Builder for Vector
//!
//! A unified tool that runs Vector with an embedded web UI for building
//! and managing observability pipelines visually.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod agent;
mod cli;

// Use library modules
use vectorize::db;
use vectorize::git_store;
use vectorize::server;
use vectorize::vector_manager;

#[derive(Parser, Debug)]
#[command(name = "vectorize")]
#[command(author = "Vectorize Team")]
#[command(version)]
#[command(about = "Visual Pipeline Builder for Vector", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Port for the web UI
    #[arg(short, long, default_value = "8080", env = "VECTORIZE_PORT", global = true)]
    port: u16,

    /// Vector API port (Vector's GraphQL API)
    #[arg(long, default_value = "8686", env = "VECTOR_API_PORT", global = true)]
    vector_api_port: u16,

    /// Path to Vector configuration file
    #[arg(short, long, env = "VECTOR_CONFIG", global = true)]
    config: Option<PathBuf>,

    /// Path to Vector binary (defaults to 'vector' in PATH or same directory)
    #[arg(long, env = "VECTOR_BIN", global = true)]
    vector_bin: Option<PathBuf>,

    /// Don't open browser automatically (click URL in terminal to open in Cursor)
    #[arg(long, global = true)]
    no_browser: bool,

    /// Enable verbose logging
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,
    
    /// Vectorize server URL for CLI commands
    #[arg(long, default_value = "http://localhost:8080", env = "VECTORIZE_URL", global = true)]
    url: String,
    
    /// Username for CLI commands (for audit logging)
    #[arg(long, default_value = "cli-user", env = "VECTORIZE_USER", global = true)]
    user: String,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start Vectorize (Vector + Web UI)
    Start {
        /// Don't open browser automatically
        #[arg(long)]
        no_open: bool,
    },
    /// Run Vector only (without web UI)
    Vector {
        /// Arguments to pass to Vector
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Run as a sidecar agent alongside Vector
    Agent {
        /// Vectorize control plane URL
        #[arg(long, default_value = "http://localhost:8080", env = "VECTORIZE_CONTROL_PLANE")]
        control_plane: String,
        
        /// Agent name (defaults to hostname)
        #[arg(long, env = "VECTORIZE_AGENT_NAME")]
        name: Option<String>,
        
        /// API key for authentication
        #[arg(long, env = "VECTORIZE_API_KEY")]
        api_key: Option<String>,
        
        /// Worker group to join
        #[arg(long, env = "VECTORIZE_GROUP")]
        group: Option<String>,
        
        /// Vector API URL (local Vector instance)
        #[arg(long, default_value = "http://localhost:8686", env = "VECTOR_API_URL")]
        vector_url: String,
        
        /// Path to Vector config file
        #[arg(long, default_value = "/etc/vector/vector.toml", env = "VECTOR_CONFIG_PATH")]
        vector_config_path: PathBuf,
        
        /// Health check interval in seconds
        #[arg(long, default_value = "30")]
        health_interval: u64,
        
        /// Config poll interval in seconds
        #[arg(long, default_value = "60")]
        config_poll_interval: u64,
    },
    
    /// Manage Vector agents
    Agents {
        #[command(subcommand)]
        command: cli::AgentCommands,
    },
    
    /// Manage worker groups
    Groups {
        #[command(subcommand)]
        command: cli::GroupCommands,
    },
    
    /// Manage configurations
    Config {
        #[command(subcommand)]
        command: cli::ConfigCommands,
    },
    
    /// Manage deployments
    Deploy {
        #[command(subcommand)]
        command: cli::DeployCommands,
    },
    
    /// Show version information
    Version,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = match cli.verbose {
        0 => Level::INFO,
        1 => Level::DEBUG,
        _ => Level::TRACE,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Create CLI client for management commands
    let cli_client = cli::CliClient::new(&cli.url);
    
    match cli.command {
        Some(Commands::Start { no_open }) => {
            start_vectorize(&cli, !no_open && !cli.no_browser).await?;
        }
        Some(Commands::Vector { args }) => {
            vector_manager::run_vector_passthrough(&cli.vector_bin, args).await?;
        }
        Some(Commands::Agent {
            control_plane,
            name,
            api_key,
            group,
            vector_url,
            vector_config_path,
            health_interval,
            config_poll_interval,
        }) => {
            run_agent(agent::AgentConfig {
                control_plane_url: control_plane,
                name,
                api_key,
                group,
                vector_url,
                vector_config_path,
                health_interval,
                config_poll_interval,
            }).await?;
        }
        Some(Commands::Agents { command }) => {
            command.execute(&cli_client).await?;
        }
        Some(Commands::Groups { command }) => {
            command.execute(&cli_client).await?;
        }
        Some(Commands::Config { command }) => {
            command.execute(&cli_client).await?;
        }
        Some(Commands::Deploy { command }) => {
            command.execute(&cli_client, &cli.user).await?;
        }
        Some(Commands::Version) => {
            println!("Vectorize {}", env!("CARGO_PKG_VERSION"));
            println!("Visual Pipeline Builder for Vector");
        }
        None => {
            // Default: start Vectorize (no browser by default - click URL in terminal)
            start_vectorize(&cli, !cli.no_browser).await?;
        }
    }

    Ok(())
}

async fn start_vectorize(cli: &Cli, open_browser: bool) -> anyhow::Result<()> {
    info!("Starting Vectorize...");

    // Initialize data directory
    let data_dir = get_data_dir();
    info!("Data directory: {}", data_dir.display());
    
    // Initialize database
    let db_path = data_dir.join("vectorize.db");
    let db = db::Database::new(&db_path).await
        .map_err(|e| anyhow::anyhow!("Failed to initialize database: {}", e))?;
    
    // Check if this is a fresh installation
    let is_fresh = db.is_fresh().await.unwrap_or(true);
    if is_fresh {
        info!("Fresh installation detected - setup wizard will be available at /setup");
    }
    
    // Initialize git store for configurations
    let configs_dir = data_dir.join("configs");
    let git_store = git_store::GitStore::open_or_init(&configs_dir)
        .map_err(|e| anyhow::anyhow!("Failed to initialize git store: {}", e))?;

    // Start Vector process and get the binary path
    let vector_process_temp = vector_manager::VectorProcess::new();
    let (vector_handle, vector_binary_path) = vector_manager::start_vector(
        &cli.vector_bin,
        cli.config.as_ref(),
        cli.vector_api_port,
        vector_process_temp.clone(),
    )
    .await?;
    
    // Create the final VectorProcess with the binary path
    let vector_process = vector_manager::VectorProcess::with_binary_path(vector_binary_path);

    // Wait a bit for Vector to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Start web UI server
    let ui_url = format!("http://127.0.0.1:{}", cli.port);
    info!("Starting web UI at {}", ui_url);

    let server_handle = server::start_server(cli.port, cli.vector_api_port, vector_process, db.clone(), git_store).await?;

    // Open browser
    if open_browser {
        info!("Opening browser...");
        if let Err(e) = open::that(&ui_url) {
            tracing::warn!("Failed to open browser: {}", e);
            info!("Please open {} in your browser", ui_url);
        }
    }

    info!("");
    info!("╔════════════════════════════════════════════════════════════╗");
    info!("║                                                            ║");
    info!("║   🚀 Vectorize is running!                                 ║");
    info!("║                                                            ║");
    info!("╚════════════════════════════════════════════════════════════╝");
    info!("");
    info!("   Click to open → http://127.0.0.1:{}", cli.port);
    info!("   Vector API:     http://127.0.0.1:{}", cli.vector_api_port);
    info!("");
    info!("   Press Ctrl+C to stop");
    info!("");

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Shutting down...");
        }
        result = vector_handle => {
            if let Err(e) = result {
                tracing::error!("Vector process error: {}", e);
            }
        }
        result = server_handle => {
            if let Err(e) = result {
                tracing::error!("Server error: {}", e);
            }
        }
    }

    info!("Vectorize stopped.");
    Ok(())
}

/// Run as a sidecar agent
async fn run_agent(config: agent::AgentConfig) -> anyhow::Result<()> {
    info!("Starting Vectorize agent...");
    info!("Control plane: {}", config.control_plane_url);
    info!("Vector URL: {}", config.vector_url);
    info!("Vector config: {}", config.vector_config_path.display());
    
    let mut agent = agent::Agent::new(config);
    agent.run().await.map_err(|e| anyhow::anyhow!("Agent error: {}", e))?;
    
    Ok(())
}

/// Get the data directory for Vectorize
/// Uses: $VECTORIZE_DATA_DIR > ~/.vectorize > ./data
fn get_data_dir() -> PathBuf {
    // Check environment variable first
    if let Ok(dir) = std::env::var("VECTORIZE_DATA_DIR") {
        return PathBuf::from(dir);
    }
    
    // Try ~/.vectorize
    if let Some(home) = dirs::home_dir() {
        let vectorize_dir = home.join(".vectorize");
        if std::fs::create_dir_all(&vectorize_dir).is_ok() {
            return vectorize_dir;
        }
    }
    
    // Fall back to ./data
    let local_dir = PathBuf::from("./data");
    let _ = std::fs::create_dir_all(&local_dir);
    local_dir
}

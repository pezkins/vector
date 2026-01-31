//! Vectorize - Visual Pipeline Builder for Vector
//!
//! A unified tool that runs Vector with an embedded web UI for building
//! and managing observability pipelines visually.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod server;
mod vector_manager;

#[derive(Parser, Debug)]
#[command(name = "vectorize")]
#[command(author = "Vectorize Team")]
#[command(version)]
#[command(about = "Visual Pipeline Builder for Vector", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Port for the web UI
    #[arg(short, long, default_value = "8080", env = "VECTORIZE_PORT")]
    port: u16,

    /// Vector API port (Vector's GraphQL API)
    #[arg(long, default_value = "8686", env = "VECTOR_API_PORT")]
    vector_api_port: u16,

    /// Path to Vector configuration file
    #[arg(short, long, env = "VECTOR_CONFIG")]
    config: Option<PathBuf>,

    /// Path to Vector binary (defaults to 'vector' in PATH or same directory)
    #[arg(long, env = "VECTOR_BIN")]
    vector_bin: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
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

    match cli.command {
        Some(Commands::Start { no_open }) => {
            start_vectorize(&cli, !no_open).await?;
        }
        Some(Commands::Vector { args }) => {
            vector_manager::run_vector_passthrough(&cli.vector_bin, args).await?;
        }
        Some(Commands::Version) => {
            println!("Vectorize {}", env!("CARGO_PKG_VERSION"));
            println!("Visual Pipeline Builder for Vector");
        }
        None => {
            // Default: start Vectorize
            start_vectorize(&cli, true).await?;
        }
    }

    Ok(())
}

async fn start_vectorize(cli: &Cli, open_browser: bool) -> anyhow::Result<()> {
    info!("Starting Vectorize...");

    // Start Vector process
    let vector_handle = vector_manager::start_vector(
        &cli.vector_bin,
        cli.config.as_ref(),
        cli.vector_api_port,
    )
    .await?;

    // Wait a bit for Vector to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Start web UI server
    let ui_url = format!("http://127.0.0.1:{}", cli.port);
    info!("Starting web UI at {}", ui_url);

    let server_handle = server::start_server(cli.port, cli.vector_api_port).await?;

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
    info!("║   Web UI:     http://127.0.0.1:{}                       ║", cli.port);
    info!("║   Vector API: http://127.0.0.1:{}                       ║", cli.vector_api_port);
    info!("║                                                            ║");
    info!("║   Press Ctrl+C to stop                                     ║");
    info!("║                                                            ║");
    info!("╚════════════════════════════════════════════════════════════╝");
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

//! Global State Management
//!
//! This module manages the global application state including:
//! - Connection state
//! - Pipeline state
//! - Event buffers

use leptos::*;
use vectorize_shared::{ConnectionMode, Pipeline, Topology};

use crate::client::{DirectClient, VectorClient, VectorClientError};

/// Global application state
#[derive(Clone)]
pub struct AppState {
    /// Current connection mode
    pub connection_mode: RwSignal<ConnectionMode>,
    
    /// Whether connected to a Vector instance or control plane
    pub connected: RwSignal<bool>,
    
    /// Connection URL
    pub url: RwSignal<String>,
    
    /// Current pipeline configuration
    pub pipeline: RwSignal<Pipeline>,
    
    /// Selected node ID in the pipeline editor
    pub selected_node: RwSignal<Option<String>>,
    
    /// Current topology from Vector
    pub topology: RwSignal<Option<Topology>>,
    
    /// Connection error message
    pub error: RwSignal<Option<String>>,
    
    /// The active client (stored as a resource that can be updated)
    client: RwSignal<Option<DirectClient>>,
}

impl AppState {
    /// Create a new app state with default values
    pub fn new() -> Self {
        Self {
            connection_mode: create_rw_signal(ConnectionMode::Direct),
            connected: create_rw_signal(false),
            url: create_rw_signal(String::new()),
            pipeline: create_rw_signal(Pipeline::new()),
            selected_node: create_rw_signal(None),
            topology: create_rw_signal(None),
            error: create_rw_signal(None),
            client: create_rw_signal(None),
        }
    }
    
    /// Connect to a Vector instance directly
    pub async fn connect_direct(&self, url: &str) -> Result<(), VectorClientError> {
        let client = DirectClient::new(url);
        
        // Test connection by fetching health
        client.health().await?;
        
        // Fetch initial topology
        let topology = client.get_topology().await?;
        
        // Update state
        self.url.set(url.to_string());
        self.topology.set(Some(topology));
        self.connected.set(true);
        self.error.set(None);
        self.client.set(Some(client));
        
        Ok(())
    }
    
    /// Disconnect from the current connection
    pub fn disconnect(&self) {
        self.connected.set(false);
        self.client.set(None);
        self.topology.set(None);
        self.url.set(String::new());
    }
    
    /// Get the current client if connected
    pub fn client(&self) -> Option<DirectClient> {
        self.client.get()
    }
    
    /// Deploy the current pipeline configuration
    pub async fn deploy_pipeline(&self) -> Result<(), VectorClientError> {
        let client = self.client.get().ok_or_else(|| {
            VectorClientError::NotConnected
        })?;
        
        let config = self.pipeline.get().to_pipeline_config();
        let result = client.deploy_config(&config).await?;
        if result.success {
            Ok(())
        } else {
            Err(VectorClientError::ConfigError(
                result.error.unwrap_or_else(|| "Deployment failed".to_string())
            ))
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

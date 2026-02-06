//! Vector Client Abstraction Layer
//!
//! This module provides a unified interface for connecting to Vector,
//! whether directly to a single instance or through a control plane.
//!
//! # Connection Modes
//!
//! - **DirectClient**: Connects directly to Vector's GraphQL API
//! - **ControlPlaneClient**: Connects to the Vectorize control plane
//!
//! Both implement the same `VectorClient` trait, allowing the UI to
//! work identically in both modes.

mod direct;
mod subscription;
mod types;

pub use direct::DirectClient;
pub use subscription::{SubscriptionClient, SubscriptionHandle};
// Re-export types for external use
#[allow(unused_imports)]
pub use types::*;

use async_trait::async_trait;
use vectorize_shared::{
    ComponentMetrics, DeployResult, PipelineConfig, Topology, VectorNode,
};

/// Error types for Vector client operations
#[derive(Debug, thiserror::Error)]
pub enum VectorClientError {
    #[error("Not connected to Vector")]
    NotConnected,
    
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Request failed: {0}")]
    RequestFailed(String),
    
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("GraphQL error: {0}")]
    GraphQLError(String),
}

/// Trait for Vector client implementations
///
/// Both DirectClient and ControlPlaneClient implement this trait,
/// providing a unified interface for the UI.
#[allow(dead_code)]
#[async_trait(?Send)]
pub trait VectorClient {
    /// Check if the connection is healthy
    async fn health(&self) -> Result<(), VectorClientError>;
    
    /// Get the current pipeline topology
    async fn get_topology(&self) -> Result<Topology, VectorClientError>;
    
    /// Get metrics for all components
    async fn get_metrics(&self) -> Result<Vec<ComponentMetrics>, VectorClientError>;
    
    /// Deploy a pipeline configuration
    async fn deploy_config(&self, config: &PipelineConfig) -> Result<DeployResult, VectorClientError>;
    
    /// Get the list of connected nodes (only relevant for control plane mode)
    async fn get_nodes(&self) -> Result<Vec<VectorNode>, VectorClientError>;
}

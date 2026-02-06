//! API message types for communication between UI and backend
//!
//! These types are used for:
//! - REST API requests/responses
//! - WebSocket messages
//! - GraphQL types

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Connection mode for the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionMode {
    /// Connect directly to a single Vector instance
    Direct,
    /// Connect to a control plane managing multiple Vector instances
    ControlPlane,
}

impl Default for ConnectionMode {
    fn default() -> Self {
        Self::Direct
    }
}

/// Vector node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorNode {
    /// Unique node identifier
    pub id: String,
    
    /// Node display name
    pub name: String,
    
    /// Node API URL
    pub url: String,
    
    /// Node health status
    pub status: NodeHealthStatus,
    
    /// Last health check time
    pub last_seen: Option<DateTime<Utc>>,
    
    /// Node version
    pub version: Option<String>,
}

/// Node health status (for multi-node management)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeHealthStatus {
    /// Node is healthy and responsive
    Healthy,
    /// Node is degraded but operational
    Degraded,
    /// Node is not responding
    Unhealthy,
    /// Node status is unknown
    Unknown,
}

impl Default for NodeHealthStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Component topology from Vector's GraphQL API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topology {
    /// All components in the topology
    pub components: Vec<Component>,
}

/// A Vector component (source, transform, or sink)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    /// Component ID
    pub component_id: String,
    
    /// Component type (e.g., "file", "remap", "console")
    pub component_type: String,
    
    /// Component kind (source, transform, or sink)
    pub component_kind: ComponentKind,
    
    /// Output connections
    pub outputs: Vec<String>,
}

/// Kind of Vector component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentKind {
    Source,
    Transform,
    Sink,
}

/// Component metrics from Vector's GraphQL API
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentMetrics {
    /// Component ID
    pub component_id: String,
    
    /// Total events received
    pub received_events_total: u64,
    
    /// Total events sent
    pub sent_events_total: u64,
    
    /// Total bytes received
    pub received_bytes_total: u64,
    
    /// Total bytes sent
    pub sent_bytes_total: u64,
    
    /// Total errors
    pub errors_total: u64,
}

/// A log event from Vector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Event message (if present)
    pub message: Option<String>,
    
    /// All event fields
    pub fields: HashMap<String, serde_json::Value>,
    
    /// Source component ID
    pub source_component: Option<String>,
    
    /// Source node ID (for multi-node mode)
    pub source_node: Option<String>,
}

/// A metric event from Vector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricEvent {
    /// Metric name
    pub name: String,
    
    /// Metric namespace
    pub namespace: Option<String>,
    
    /// Metric kind
    pub kind: MetricKind,
    
    /// Metric value
    pub value: MetricValue,
    
    /// Metric tags
    pub tags: HashMap<String, String>,
    
    /// Event timestamp
    pub timestamp: Option<DateTime<Utc>>,
}

/// Kind of metric
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetricKind {
    Incremental,
    Absolute,
}

/// Metric value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetricValue {
    Counter { value: f64 },
    Gauge { value: f64 },
    Set { values: Vec<String> },
    Distribution { samples: Vec<f64>, statistic: String },
    Histogram { buckets: Vec<(f64, u64)>, count: u64, sum: f64 },
    Summary { quantiles: Vec<(f64, f64)>, count: u64, sum: f64 },
}

/// Configuration deployment request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployRequest {
    /// Pipeline configuration to deploy
    pub config: crate::config::PipelineConfig,
    
    /// Target node IDs (empty for all nodes in control plane mode)
    #[serde(default)]
    pub target_nodes: Vec<String>,
    
    /// Whether to validate only without deploying
    #[serde(default)]
    pub validate_only: bool,
}

/// Configuration deployment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployResult {
    /// Whether deployment was successful
    pub success: bool,
    
    /// Per-node deployment status
    pub node_results: HashMap<String, NodeDeployResult>,
    
    /// Overall error message if failed
    pub error: Option<String>,
}

/// Per-node deployment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDeployResult {
    /// Node ID
    pub node_id: String,
    
    /// Whether this node's deployment was successful
    pub success: bool,
    
    /// Error message if failed
    pub error: Option<String>,
    
    /// Validation errors if any
    pub validation_errors: Vec<ValidationError>,
}

/// Configuration validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Component that has the error
    pub component: Option<String>,
    
    /// Error message
    pub message: String,
    
    /// Error details
    pub details: Option<String>,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Whether the service is healthy
    pub healthy: bool,
    
    /// Service uptime in seconds
    pub uptime_seconds: u64,
    
    /// Service version
    pub version: String,
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsMessage {
    /// Subscribe to event stream
    Subscribe { component_ids: Vec<String> },
    
    /// Unsubscribe from event stream
    Unsubscribe { component_ids: Vec<String> },
    
    /// Event data
    Event(LogEvent),
    
    /// Metrics update
    Metrics(Vec<ComponentMetrics>),
    
    /// Topology change
    TopologyChange(Topology),
    
    /// Error message
    Error { message: String },
    
    /// Ping/pong for keepalive
    Ping,
    Pong,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ws_message_serialization() {
        let msg = WsMessage::Subscribe {
            component_ids: vec!["source1".to_string(), "transform1".to_string()],
        };
        
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"Subscribe\""));
        
        let parsed: WsMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            WsMessage::Subscribe { component_ids } => {
                assert_eq!(component_ids.len(), 2);
            }
            _ => panic!("Wrong message type"),
        }
    }
}

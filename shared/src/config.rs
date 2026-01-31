//! Vector pipeline configuration types
//!
//! These types represent Vector pipeline configurations that can be
//! serialized to TOML/JSON for deployment to Vector instances.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A complete Vector pipeline configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Pipeline sources (data ingestion points)
    #[serde(default)]
    pub sources: HashMap<String, SourceConfig>,
    
    /// Pipeline transforms (data processing)
    #[serde(default)]
    pub transforms: HashMap<String, TransformConfig>,
    
    /// Pipeline sinks (data destinations)
    #[serde(default)]
    pub sinks: HashMap<String, SinkConfig>,
}

impl PipelineConfig {
    /// Create a new empty pipeline configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a source to the pipeline
    pub fn add_source(&mut self, id: impl Into<String>, config: SourceConfig) {
        self.sources.insert(id.into(), config);
    }
    
    /// Add a transform to the pipeline
    pub fn add_transform(&mut self, id: impl Into<String>, config: TransformConfig) {
        self.transforms.insert(id.into(), config);
    }
    
    /// Add a sink to the pipeline
    pub fn add_sink(&mut self, id: impl Into<String>, config: SinkConfig) {
        self.sinks.insert(id.into(), config);
    }
    
    /// Convert to TOML string for Vector configuration
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
    
    /// Parse from TOML string
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }
}

/// Source component configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// Source type (e.g., "file", "http", "kafka")
    #[serde(rename = "type")]
    pub source_type: String,
    
    /// Source-specific configuration options
    #[serde(flatten)]
    pub options: HashMap<String, serde_json::Value>,
}

impl SourceConfig {
    pub fn new(source_type: impl Into<String>) -> Self {
        Self {
            source_type: source_type.into(),
            options: HashMap::new(),
        }
    }
    
    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }
}

/// Transform component configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformConfig {
    /// Transform type (e.g., "remap", "filter", "route")
    #[serde(rename = "type")]
    pub transform_type: String,
    
    /// Input components for this transform
    pub inputs: Vec<String>,
    
    /// Transform-specific configuration options
    #[serde(flatten)]
    pub options: HashMap<String, serde_json::Value>,
}

impl TransformConfig {
    pub fn new(transform_type: impl Into<String>, inputs: Vec<String>) -> Self {
        Self {
            transform_type: transform_type.into(),
            inputs,
            options: HashMap::new(),
        }
    }
    
    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }
}

/// Sink component configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkConfig {
    /// Sink type (e.g., "console", "file", "http", "kafka")
    #[serde(rename = "type")]
    pub sink_type: String,
    
    /// Input components for this sink
    pub inputs: Vec<String>,
    
    /// Sink-specific configuration options
    #[serde(flatten)]
    pub options: HashMap<String, serde_json::Value>,
}

impl SinkConfig {
    pub fn new(sink_type: impl Into<String>, inputs: Vec<String>) -> Self {
        Self {
            sink_type: sink_type.into(),
            inputs,
            options: HashMap::new(),
        }
    }
    
    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }
}

/// Node position in the pipeline canvas UI
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// A pipeline node with UI metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineNode {
    /// Unique node identifier
    pub id: String,
    
    /// Display name
    pub name: String,
    
    /// Node type and configuration
    pub node_type: NodeType,
    
    /// Position on the canvas
    pub position: Position,
}

impl PipelineNode {
    pub fn new(name: impl Into<String>, node_type: NodeType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            node_type,
            position: Position::default(),
        }
    }
    
    pub fn with_position(mut self, x: f64, y: f64) -> Self {
        self.position = Position { x, y };
        self
    }
}

/// Type of pipeline node
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "category", content = "config")]
pub enum NodeType {
    Source(SourceConfig),
    Transform(TransformConfig),
    Sink(SinkConfig),
}

impl NodeType {
    /// Get the display name for this node type
    pub fn display_name(&self) -> &str {
        match self {
            NodeType::Source(c) => &c.source_type,
            NodeType::Transform(c) => &c.transform_type,
            NodeType::Sink(c) => &c.sink_type,
        }
    }
    
    /// Get the category name
    pub fn category(&self) -> &'static str {
        match self {
            NodeType::Source(_) => "source",
            NodeType::Transform(_) => "transform",
            NodeType::Sink(_) => "sink",
        }
    }
}

/// A connection between pipeline nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    /// Source node ID (with optional output port)
    pub from: String,
    
    /// Target node ID
    pub to: String,
}

impl Connection {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }
}

/// Complete pipeline state including UI metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Pipeline {
    /// All nodes in the pipeline
    pub nodes: HashMap<String, PipelineNode>,
    
    /// Connections between nodes
    pub connections: Vec<Connection>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a node to the pipeline
    pub fn add_node(&mut self, node: PipelineNode) {
        self.nodes.insert(node.id.clone(), node);
    }
    
    /// Remove a node and its connections
    pub fn remove_node(&mut self, id: &str) {
        self.nodes.remove(id);
        self.connections.retain(|c| c.from != id && c.to != id);
    }
    
    /// Add a connection between nodes
    pub fn connect(&mut self, from: impl Into<String>, to: impl Into<String>) {
        self.connections.push(Connection::new(from, to));
    }
    
    /// Convert to Vector pipeline configuration
    pub fn to_pipeline_config(&self) -> PipelineConfig {
        let mut config = PipelineConfig::new();
        
        for (id, node) in &self.nodes {
            match &node.node_type {
                NodeType::Source(source) => {
                    config.add_source(id, source.clone());
                }
                NodeType::Transform(transform) => {
                    config.add_transform(id, transform.clone());
                }
                NodeType::Sink(sink) => {
                    config.add_sink(id, sink.clone());
                }
            }
        }
        
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pipeline_to_toml() {
        let mut pipeline = PipelineConfig::new();
        
        pipeline.add_source("stdin", SourceConfig::new("stdin"));
        pipeline.add_transform(
            "parse",
            TransformConfig::new("remap", vec!["stdin".to_string()])
                .with_option("source", ". = parse_json!(string!(.message))"),
        );
        pipeline.add_sink(
            "console",
            SinkConfig::new("console", vec!["parse".to_string()])
                .with_option("encoding", serde_json::json!({"codec": "json"})),
        );
        
        let toml = pipeline.to_toml().unwrap();
        assert!(toml.contains("[sources.stdin]"));
        assert!(toml.contains("[transforms.parse]"));
        assert!(toml.contains("[sinks.console]"));
    }
}

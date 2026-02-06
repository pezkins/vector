//! Direct Vector Client
//!
//! This client connects directly to a single Vector instance's GraphQL API.
//! Used for single-node (laptop) deployments.

use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use vectorize_shared::{
    Component, ComponentKind, ComponentMetrics, DeployResult, NodeDeployResult,
    NodeHealthStatus, Pipeline, PipelineConfig, PipelineNode, Position, Topology, VectorNode,
    NodeType, SourceConfig, TransformConfig, SinkConfig, Connection,
};
use std::collections::HashMap;

use super::{VectorClient, VectorClientError};

/// Direct client for connecting to a single Vector instance
#[derive(Debug, Clone)]
pub struct DirectClient {
    /// Vector API base URL
    base_url: String,
}

impl DirectClient {
    /// Create a new direct client
    pub fn new(url: &str) -> Self {
        // Normalize URL (remove trailing slash)
        let base_url = url.trim_end_matches('/').to_string();
        Self { base_url }
    }
    
    /// Get the GraphQL endpoint URL
    fn graphql_url(&self) -> String {
        format!("{}/graphql", self.base_url)
    }
    
    /// Get the config reload endpoint URL (RFC 541)
    fn config_url(&self) -> String {
        format!("{}/config", self.base_url)
    }
    
    /// Get the health endpoint URL
    fn health_url(&self) -> String {
        format!("{}/health", self.base_url)
    }
    
    /// Execute a GraphQL query
    async fn graphql_query<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
    ) -> Result<T, VectorClientError> {
        let request = GraphQLRequest {
            query: query.to_string(),
            variables: None,
        };
        
        let response = Request::post(&self.graphql_url())
            .header("Content-Type", "application/json")
            .json(&request)
            .map_err(|e| VectorClientError::RequestFailed(e.to_string()))?
            .send()
            .await
            .map_err(|e| VectorClientError::ConnectionFailed(e.to_string()))?;
        
        if !response.ok() {
            return Err(VectorClientError::RequestFailed(format!(
                "HTTP {}: {}",
                response.status(),
                response.status_text()
            )));
        }
        
        let result: GraphQLResponse<T> = response
            .json()
            .await
            .map_err(|e| VectorClientError::InvalidResponse(e.to_string()))?;
        
        if let Some(errors) = result.errors {
            let error_msg = errors
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(VectorClientError::GraphQLError(error_msg));
        }
        
        result.data.ok_or_else(|| {
            VectorClientError::InvalidResponse("No data in response".to_string())
        })
    }
    
    /// Fetch the current pipeline configuration from Vector
    /// Returns a Pipeline with auto-layouted nodes and configuration options loaded from the config file
    pub async fn fetch_pipeline(&self) -> Result<Pipeline, VectorClientError> {
        let query = r#"
            query {
                sources { 
                    nodes { 
                        componentId 
                        componentType 
                    } 
                }
                transforms { 
                    nodes { 
                        componentId 
                        componentType 
                        sources { componentId }
                        transforms { componentId }
                    } 
                }
                sinks { 
                    nodes { 
                        componentId 
                        componentType 
                        sources { componentId }
                        transforms { componentId }
                    } 
                }
            }
        "#;
        
        let data: PipelineQueryResponse = self.graphql_query(query).await?;
        
        // Also fetch the actual config to get component options
        let config = self.fetch_config().await.unwrap_or_default();
        let sources_config = config.get("sources").and_then(|v| v.as_object());
        let transforms_config = config.get("transforms").and_then(|v| v.as_object());
        let sinks_config = config.get("sinks").and_then(|v| v.as_object());
        
        web_sys::console::log_1(&format!(
            "Fetched config - sources: {:?}, transforms: {:?}, sinks: {:?}",
            sources_config.map(|s| s.keys().collect::<Vec<_>>()),
            transforms_config.map(|t| t.keys().collect::<Vec<_>>()),
            sinks_config.map(|s| s.keys().collect::<Vec<_>>())
        ).into());
        
        // Build the pipeline from the response
        let mut pipeline = Pipeline::new();
        let mut node_inputs: HashMap<String, Vec<String>> = HashMap::new();
        
        // Add sources (no inputs)
        for source in &data.sources.nodes {
            // Get options from config if available
            let options = self.extract_component_options(
                sources_config,
                &source.component_id,
                &["type", "inputs"]
            );
            
            let node = PipelineNode {
                id: source.component_id.clone(),
                name: source.component_id.clone(),
                node_type: NodeType::Source(SourceConfig {
                    source_type: source.component_type.clone(),
                    options,
                }),
                position: Position { x: 0.0, y: 0.0 },
            };
            pipeline.nodes.insert(source.component_id.clone(), node);
        }
        
        // Add transforms (has inputs from sources and other transforms)
        for transform in &data.transforms.nodes {
            let mut inputs = Vec::new();
            for src in &transform.sources {
                inputs.push(src.component_id.clone());
            }
            for tr in &transform.transforms {
                inputs.push(tr.component_id.clone());
            }
            node_inputs.insert(transform.component_id.clone(), inputs.clone());
            
            // Get options from config if available
            let options = self.extract_component_options(
                transforms_config,
                &transform.component_id,
                &["type", "inputs"]
            );
            
            web_sys::console::log_1(&format!(
                "Transform '{}' options from config: {:?}",
                transform.component_id, options
            ).into());
            
            let node = PipelineNode {
                id: transform.component_id.clone(),
                name: transform.component_id.clone(),
                node_type: NodeType::Transform(TransformConfig {
                    transform_type: transform.component_type.clone(),
                    inputs,
                    options,
                }),
                position: Position { x: 0.0, y: 0.0 },
            };
            pipeline.nodes.insert(transform.component_id.clone(), node);
        }
        
        // Add sinks (has inputs from sources and transforms)
        for sink in &data.sinks.nodes {
            let mut inputs = Vec::new();
            for src in &sink.sources {
                inputs.push(src.component_id.clone());
            }
            for tr in &sink.transforms {
                inputs.push(tr.component_id.clone());
            }
            node_inputs.insert(sink.component_id.clone(), inputs.clone());
            
            // Get options from config if available
            let options = self.extract_component_options(
                sinks_config,
                &sink.component_id,
                &["type", "inputs"]
            );
            
            let node = PipelineNode {
                id: sink.component_id.clone(),
                name: sink.component_id.clone(),
                node_type: NodeType::Sink(SinkConfig {
                    sink_type: sink.component_type.clone(),
                    inputs,
                    options,
                }),
                position: Position { x: 0.0, y: 0.0 },
            };
            pipeline.nodes.insert(sink.component_id.clone(), node);
        }
        
        // Build connections from inputs
        for (node_id, inputs) in &node_inputs {
            for input_id in inputs {
                pipeline.connections.push(Connection::new(input_id.clone(), node_id.clone()));
            }
        }
        
        // Auto-layout the nodes left-to-right
        self.auto_layout_pipeline(&mut pipeline);
        
        Ok(pipeline)
    }
    
    /// Fetch the current configuration from the Vectorize API
    async fn fetch_config(&self) -> Result<serde_json::Value, VectorClientError> {
        let url = format!("{}/config", self.base_url);
        
        let response = Request::get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| VectorClientError::ConnectionFailed(e.to_string()))?;
        
        if !response.ok() {
            // Config endpoint may not be available in standalone mode
            web_sys::console::log_1(&"Config endpoint not available, using defaults".into());
            return Ok(serde_json::json!({}));
        }
        
        let config: serde_json::Value = response
            .json()
            .await
            .map_err(|e| VectorClientError::InvalidResponse(e.to_string()))?;
        
        Ok(config)
    }
    
    /// Extract component options from config, excluding reserved fields
    fn extract_component_options(
        &self,
        section_config: Option<&serde_json::Map<String, serde_json::Value>>,
        component_id: &str,
        exclude_fields: &[&str],
    ) -> HashMap<String, serde_json::Value> {
        let mut options = HashMap::new();
        
        if let Some(section) = section_config {
            if let Some(component_config) = section.get(component_id) {
                if let Some(obj) = component_config.as_object() {
                    for (key, value) in obj {
                        // Skip reserved fields that are handled separately
                        if !exclude_fields.contains(&key.as_str()) {
                            options.insert(key.clone(), value.clone());
                        }
                    }
                }
            }
        }
        
        options
    }
    
    /// Auto-layout nodes in a left-to-right flow
    fn auto_layout_pipeline(&self, pipeline: &mut Pipeline) {
        // Build adjacency for topological sort
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        
        for node_id in pipeline.nodes.keys() {
            in_degree.insert(node_id.clone(), 0);
            adj.insert(node_id.clone(), Vec::new());
        }
        
        for conn in &pipeline.connections {
            if let Some(degree) = in_degree.get_mut(&conn.to_node) {
                *degree += 1;
            }
            if let Some(neighbors) = adj.get_mut(&conn.from_node) {
                neighbors.push(conn.to_node.clone());
            }
        }
        
        // Topological sort to determine levels (columns)
        let mut level: HashMap<String, usize> = HashMap::new();
        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(id, _)| id.clone())
            .collect();
        
        // Initialize sources at level 0
        for node_id in &queue {
            level.insert(node_id.clone(), 0);
        }
        
        // BFS to compute levels
        let mut idx = 0;
        while idx < queue.len() {
            let node_id = queue[idx].clone();
            let node_level = *level.get(&node_id).unwrap_or(&0);
            
            if let Some(neighbors) = adj.get(&node_id) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        
                        // Update level to be max of current level and parent level + 1
                        let new_level = node_level + 1;
                        let current_level = level.entry(neighbor.clone()).or_insert(0);
                        if new_level > *current_level {
                            *current_level = new_level;
                        }
                        
                        if *degree == 0 {
                            queue.push(neighbor.clone());
                        }
                    }
                }
            }
            idx += 1;
        }
        
        // Group nodes by level
        let mut levels: HashMap<usize, Vec<String>> = HashMap::new();
        for (node_id, lvl) in &level {
            levels.entry(*lvl).or_default().push(node_id.clone());
        }
        
        // Position nodes: x based on level, y based on position within level
        let x_spacing = 250.0;
        let y_spacing = 120.0;
        let x_start = 200.0;
        let y_start = 150.0;
        
        for (lvl, nodes) in &levels {
            let x = x_start + (*lvl as f64) * x_spacing;
            for (i, node_id) in nodes.iter().enumerate() {
                let y = y_start + (i as f64) * y_spacing;
                if let Some(node) = pipeline.nodes.get_mut(node_id) {
                    node.position = Position { x, y };
                }
            }
        }
    }
}

#[async_trait::async_trait(?Send)]
impl VectorClient for DirectClient {
    async fn health(&self) -> Result<(), VectorClientError> {
        let response = Request::get(&self.health_url())
            .send()
            .await
            .map_err(|e| VectorClientError::ConnectionFailed(e.to_string()))?;
        
        if response.ok() {
            Ok(())
        } else {
            Err(VectorClientError::ConnectionFailed(format!(
                "Health check failed: HTTP {}",
                response.status()
            )))
        }
    }
    
    async fn get_topology(&self) -> Result<Topology, VectorClientError> {
        let query = r#"
            query {
                components {
                    edges {
                        node {
                            componentId
                            componentType
                            __typename
                        }
                    }
                }
            }
        "#;
        
        let data: TopologyResponse = self.graphql_query(query).await?;
        
        let components = data
            .components
            .edges
            .into_iter()
            .map(|edge| {
                let node = edge.node;
                Component {
                    component_id: node.component_id,
                    component_type: node.component_type,
                    component_kind: match node.typename.as_str() {
                        "Source" => ComponentKind::Source,
                        "Transform" => ComponentKind::Transform,
                        "Sink" => ComponentKind::Sink,
                        _ => ComponentKind::Transform,
                    },
                    outputs: vec![],
                }
            })
            .collect();
        
        Ok(Topology { components })
    }
    
    async fn get_metrics(&self) -> Result<Vec<ComponentMetrics>, VectorClientError> {
        let query = r#"
            query {
                components {
                    edges {
                        node {
                            componentId
                            ... on Source {
                                metrics {
                                    receivedEventsTotal {
                                        receivedEventsTotal
                                    }
                                    sentEventsTotal {
                                        sentEventsTotal
                                    }
                                }
                            }
                            ... on Transform {
                                metrics {
                                    receivedEventsTotal {
                                        receivedEventsTotal
                                    }
                                    sentEventsTotal {
                                        sentEventsTotal
                                    }
                                }
                            }
                            ... on Sink {
                                metrics {
                                    receivedEventsTotal {
                                        receivedEventsTotal
                                    }
                                    sentEventsTotal {
                                        sentEventsTotal
                                    }
                                }
                            }
                        }
                    }
                }
            }
        "#;
        
        // For now, return empty metrics - full implementation would parse the response
        let _data: serde_json::Value = self.graphql_query(query).await?;
        Ok(vec![])
    }
    
    async fn deploy_config(&self, config: &PipelineConfig) -> Result<DeployResult, VectorClientError> {
        let response = Request::post(&self.config_url())
            .header("Content-Type", "application/json")
            .json(config)
            .map_err(|e| VectorClientError::RequestFailed(e.to_string()))?
            .send()
            .await
            .map_err(|e| VectorClientError::ConnectionFailed(e.to_string()))?;
        
        let success = response.ok();
        let error = if !success {
            let body = response.text().await.unwrap_or_default();
            Some(format!("Deployment failed: {}", body))
        } else {
            None
        };
        
        let mut node_results = std::collections::HashMap::new();
        node_results.insert(
            "local".to_string(),
            NodeDeployResult {
                node_id: "local".to_string(),
                success,
                error: error.clone(),
                validation_errors: vec![],
            },
        );
        
        Ok(DeployResult {
            success,
            node_results,
            error,
        })
    }
    
    async fn get_nodes(&self) -> Result<Vec<VectorNode>, VectorClientError> {
        // In direct mode, there's only one "node" - the connected Vector instance
        Ok(vec![VectorNode {
            id: "local".to_string(),
            name: "Local Vector".to_string(),
            url: self.base_url.clone(),
            status: NodeHealthStatus::Healthy,
            last_seen: Some(chrono::Utc::now()),
            version: None,
        }])
    }
}

// GraphQL request/response types

#[derive(Debug, Serialize)]
struct GraphQLRequest {
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    variables: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

// Vector GraphQL response types

#[derive(Debug, Deserialize)]
struct TopologyResponse {
    components: ComponentsConnection,
}

#[derive(Debug, Deserialize)]
struct ComponentsConnection {
    edges: Vec<ComponentEdge>,
}

#[derive(Debug, Deserialize)]
struct ComponentEdge {
    node: ComponentNode,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComponentNode {
    component_id: String,
    component_type: String,
    #[serde(rename = "__typename")]
    typename: String,
}

// Pipeline query response types

#[derive(Debug, Deserialize)]
struct PipelineQueryResponse {
    sources: SourcesResponse,
    transforms: TransformsResponse,
    sinks: SinksResponse,
}

#[derive(Debug, Deserialize)]
struct SourcesResponse {
    nodes: Vec<SourceNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceNode {
    component_id: String,
    component_type: String,
}

#[derive(Debug, Deserialize)]
struct TransformsResponse {
    nodes: Vec<TransformNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransformNode {
    component_id: String,
    component_type: String,
    sources: Vec<ComponentRef>,
    transforms: Vec<ComponentRef>,
}

#[derive(Debug, Deserialize)]
struct SinksResponse {
    nodes: Vec<SinkNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SinkNode {
    component_id: String,
    component_type: String,
    sources: Vec<ComponentRef>,
    transforms: Vec<ComponentRef>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComponentRef {
    component_id: String,
}

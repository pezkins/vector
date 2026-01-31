//! Direct Vector Client
//!
//! This client connects directly to a single Vector instance's GraphQL API.
//! Used for single-node (laptop) deployments.

use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use vectorize_shared::{
    Component, ComponentKind, ComponentMetrics, DeployResult, NodeDeployResult,
    NodeStatus, PipelineConfig, Topology, VectorNode,
};

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
                            componentKind
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
                    component_kind: match node.component_kind.as_str() {
                        "source" => ComponentKind::Source,
                        "transform" => ComponentKind::Transform,
                        "sink" => ComponentKind::Sink,
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
            status: NodeStatus::Healthy,
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
    component_kind: String,
}

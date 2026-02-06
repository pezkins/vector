//! Global State Management
//!
//! This module manages the global application state including:
//! - Connection state
//! - Pipeline state (loaded from Vector API, not localStorage)
//! - Event buffers
//! - Node execution status
//! - UI state (theme, sidebar, bottom panel)

use leptos::*;
use std::collections::HashMap;
use vectorize_shared::{ConnectionMode, NodeEvent, NodeStatus, Pipeline, Topology};

use crate::client::{DirectClient, VectorClient, VectorClientError};

/// Maximum events to cache per node
#[allow(dead_code)]
const MAX_EVENTS_PER_NODE: usize = 100;

/// Default height for bottom panel
const DEFAULT_BOTTOM_PANEL_HEIGHT: f64 = 256.0;

/// Theme options
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Theme {
    #[default]
    Dark,
    Light,
    System,
}

/// Bottom panel tab options
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum BottomPanelTab {
    #[default]
    DataPreview,
    Logs,
    TestResults,
}

/// Global application state
#[derive(Clone)]
pub struct AppState {
    // ========================================================================
    // Connection State
    // ========================================================================
    
    /// Current connection mode
    pub connection_mode: RwSignal<ConnectionMode>,
    
    /// Whether connected to a Vector instance or control plane
    pub connected: RwSignal<bool>,
    
    /// Connection URL
    pub url: RwSignal<String>,
    
    /// Connection error message
    pub error: RwSignal<Option<String>>,
    
    /// The active client (stored as a resource that can be updated)
    client: RwSignal<Option<DirectClient>>,
    
    // ========================================================================
    // Pipeline State
    // ========================================================================
    
    /// Current pipeline configuration
    pub pipeline: RwSignal<Pipeline>,
    
    /// Selected node ID in the pipeline editor
    pub selected_node: RwSignal<Option<String>>,
    
    /// Current topology from Vector
    pub topology: RwSignal<Option<Topology>>,
    
    /// Node execution statuses for visualization
    pub node_statuses: RwSignal<HashMap<String, NodeStatus>>,
    
    /// Cached events per node for data preview
    pub node_events: RwSignal<HashMap<String, Vec<NodeEvent>>>,
    
    /// Currently active config panel tab (reserved for future tabbed UI)
    #[allow(dead_code)]
    pub config_panel_tab: RwSignal<ConfigPanelTab>,
    
    // ========================================================================
    // UI State
    // ========================================================================
    
    /// Current theme (Dark, Light, or System)
    pub theme: RwSignal<Theme>,
    
    /// Whether the sidebar is collapsed
    pub sidebar_collapsed: RwSignal<bool>,
    
    /// Bottom panel height in pixels (0 = collapsed)
    pub bottom_panel_height: RwSignal<f64>,
    
    /// Currently active bottom panel tab
    pub bottom_panel_tab: RwSignal<BottomPanelTab>,
}

/// Configuration panel tabs (reserved for future tabbed UI)
#[allow(dead_code)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum ConfigPanelTab {
    #[default]
    Settings,
    Input,
    Output,
}

impl AppState {
    /// Create a new app state with default values
    /// Pipeline is empty initially and will be loaded from Vector API on connection
    pub fn new() -> Self {
        // Try to load theme preference from localStorage
        let initial_theme = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item("vectorize_theme").ok().flatten())
            .and_then(|t| match t.as_str() {
                "dark" => Some(Theme::Dark),
                "light" => Some(Theme::Light),
                "system" => Some(Theme::System),
                _ => None,
            })
            .unwrap_or(Theme::Dark);
        
        // Try to load sidebar collapsed state from localStorage
        let initial_sidebar_collapsed = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item("vectorize_sidebar_collapsed").ok().flatten())
            .map(|v| v == "true")
            .unwrap_or(false);
        
        // Try to load bottom panel height from localStorage
        let initial_bottom_panel_height = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item("vectorize_bottom_panel_height").ok().flatten())
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(DEFAULT_BOTTOM_PANEL_HEIGHT);
        
        Self {
            // Connection state
            connection_mode: create_rw_signal(ConnectionMode::Direct),
            connected: create_rw_signal(false),
            url: create_rw_signal(String::new()),
            error: create_rw_signal(None),
            client: create_rw_signal(None),
            
            // Pipeline state
            pipeline: create_rw_signal(Pipeline::new()),
            selected_node: create_rw_signal(None),
            topology: create_rw_signal(None),
            node_statuses: create_rw_signal(HashMap::new()),
            node_events: create_rw_signal(HashMap::new()),
            config_panel_tab: create_rw_signal(ConfigPanelTab::Settings),
            
            // UI state
            theme: create_rw_signal(initial_theme),
            sidebar_collapsed: create_rw_signal(initial_sidebar_collapsed),
            bottom_panel_height: create_rw_signal(initial_bottom_panel_height),
            bottom_panel_tab: create_rw_signal(BottomPanelTab::DataPreview),
        }
    }
    
    /// Save UI preferences to localStorage
    pub fn save_ui_preferences(&self) {
        if let Some(storage) = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
        {
            // Save theme
            let theme_str = match self.theme.get_untracked() {
                Theme::Dark => "dark",
                Theme::Light => "light",
                Theme::System => "system",
            };
            let _ = storage.set_item("vectorize_theme", theme_str);
            
            // Save sidebar state
            let _ = storage.set_item(
                "vectorize_sidebar_collapsed",
                if self.sidebar_collapsed.get_untracked() { "true" } else { "false" }
            );
            
            // Save bottom panel height
            let _ = storage.set_item(
                "vectorize_bottom_panel_height",
                &self.bottom_panel_height.get_untracked().to_string()
            );
        }
    }
    
    /// Update status for a node (reserved for future status tracking)
    #[allow(dead_code)]
    pub fn set_node_status(&self, node_id: &str, status: NodeStatus) {
        self.node_statuses.update(|statuses| {
            statuses.insert(node_id.to_string(), status);
        });
    }
    
    /// Add an event for a node (maintains max buffer size) (reserved for future event caching)
    #[allow(dead_code)]
    pub fn add_node_event(&self, node_id: &str, event: NodeEvent) {
        self.node_events.update(|events| {
            let node_events = events.entry(node_id.to_string()).or_insert_with(Vec::new);
            node_events.push(event);
            // Keep only the last N events
            if node_events.len() > MAX_EVENTS_PER_NODE {
                node_events.remove(0);
            }
        });
    }
    
    /// Clear events for a node (reserved for future event caching)
    #[allow(dead_code)]
    pub fn clear_node_events(&self, node_id: &str) {
        self.node_events.update(|events| {
            events.remove(node_id);
        });
    }
    
    /// Get events for the selected node (reserved for future event caching)
    #[allow(dead_code)]
    pub fn selected_node_events(&self) -> Vec<NodeEvent> {
        if let Some(node_id) = self.selected_node.get() {
            self.node_events.get().get(&node_id).cloned().unwrap_or_default()
        } else {
            vec![]
        }
    }
    
    /// Get input events for the selected node (from connected input nodes) (reserved for future)
    #[allow(dead_code)]
    pub fn selected_node_input_events(&self) -> Vec<NodeEvent> {
        if let Some(node_id) = self.selected_node.get() {
            let pipeline = self.pipeline.get();
            let inputs = pipeline.get_inputs(&node_id);
            let events = self.node_events.get();
            
            inputs.iter()
                .flat_map(|input_id| events.get(input_id).cloned().unwrap_or_default())
                .collect()
        } else {
            vec![]
        }
    }
    
    /// Connect to a Vector instance directly
    /// Also loads the current pipeline configuration from Vector
    pub async fn connect_direct(&self, url: &str) -> Result<(), VectorClientError> {
        let client = DirectClient::new(url);
        
        // Test connection by fetching health
        client.health().await?;
        
        // Fetch initial topology
        let topology = client.get_topology().await?;
        
        // Load the current pipeline configuration from Vector
        let pipeline = client.fetch_pipeline().await?;
        web_sys::console::log_1(&format!(
            "Loaded pipeline from Vector: {} sources, {} transforms, {} sinks",
            pipeline.nodes.values().filter(|n| matches!(n.node_type, vectorize_shared::NodeType::Source(_))).count(),
            pipeline.nodes.values().filter(|n| matches!(n.node_type, vectorize_shared::NodeType::Transform(_))).count(),
            pipeline.nodes.values().filter(|n| matches!(n.node_type, vectorize_shared::NodeType::Sink(_))).count(),
        ).into());
        
        // Update state
        self.url.set(url.to_string());
        self.topology.set(Some(topology));
        self.pipeline.set(pipeline);  // Set the loaded pipeline
        self.connected.set(true);
        self.error.set(None);
        self.client.set(Some(client));
        
        Ok(())
    }
    
    /// Reload the pipeline from Vector (useful after external changes) (reserved for future)
    #[allow(dead_code)]
    pub async fn reload_pipeline(&self) -> Result<(), VectorClientError> {
        let client = self.client.get().ok_or(VectorClientError::NotConnected)?;
        
        let pipeline = client.fetch_pipeline().await?;
        self.pipeline.set(pipeline);
        
        Ok(())
    }
    
    /// Disconnect from the current connection
    pub fn disconnect(&self) {
        self.connected.set(false);
        self.client.set(None);
        self.topology.set(None);
        self.url.set(String::new());
    }
    
    /// Get the current client if connected (reserved for future)
    #[allow(dead_code)]
    pub fn client(&self) -> Option<DirectClient> {
        self.client.get()
    }
    
    /// Deploy the current pipeline configuration
    pub async fn deploy_pipeline(&self) -> Result<(), VectorClientError> {
        let client = self.client.get().ok_or(VectorClientError::NotConnected)?;
        
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

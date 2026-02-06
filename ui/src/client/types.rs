//! Client type definitions
//!
//! Additional types used by the Vector clients.
//! These types are prepared for future filter/subscription enhancements.

use serde::{Deserialize, Serialize};

/// Event filter for subscriptions (prepared for future use)
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventFilter {
    /// Filter by component IDs
    pub component_ids: Option<Vec<String>>,
    
    /// Filter by log level
    pub level: Option<String>,
    
    /// Search text
    pub search: Option<String>,
    
    /// Maximum events to return
    pub limit: Option<usize>,
}

#[allow(dead_code)]
impl EventFilter {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_components(mut self, ids: Vec<String>) -> Self {
        self.component_ids = Some(ids);
        self
    }
    
    pub fn with_level(mut self, level: impl Into<String>) -> Self {
        self.level = Some(level.into());
        self
    }
    
    pub fn with_search(mut self, search: impl Into<String>) -> Self {
        self.search = Some(search.into());
        self
    }
    
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Subscription options (prepared for future use)
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct SubscriptionOptions {
    /// Event filter
    pub filter: EventFilter,
    
    /// Buffer size for backpressure
    pub buffer_size: usize,
}

#[allow(dead_code)]
impl SubscriptionOptions {
    pub fn new() -> Self {
        Self {
            filter: EventFilter::default(),
            buffer_size: 1000,
        }
    }
}

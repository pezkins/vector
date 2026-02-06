//! Observe Components
//!
//! Observability and monitoring components for the Vectorize UI:
//! - `AlertsManagement`: Alert configuration and history

mod alerts;
mod audit;
mod metrics;

pub use alerts::AlertsManagement;

// Re-export when these components are used in the app:
// pub use audit::AuditLogs;
// pub use metrics::MetricsDashboard;

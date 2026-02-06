//! Management UI components for the Vectorize control plane
//!
//! Provides UI for:
//! - Worker group management
//! - Agent registry
//! - Configuration history and versioning
//! - Deployment controls

mod groups;
mod history;

pub use groups::*;
// ConfigHistory is used internally by groups module
#[allow(unused_imports)]
pub use history::ConfigHistory;

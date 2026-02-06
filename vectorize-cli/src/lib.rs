//! Vectorize Library
//!
//! Core modules for the Vectorize control plane.

pub mod alerts;
pub mod api;
pub mod db;
pub mod deployment;
pub mod git_store;
pub mod health;
pub mod rbac;
pub mod server;
pub mod sso;
pub mod tap;
pub mod validation;
pub mod vector_manager;

// Re-export AppState for convenience
pub use server::AppState;

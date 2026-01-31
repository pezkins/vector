//! Shared types for Vectorize UI and Control Plane
//!
//! This crate contains common types used across the Vectorize platform:
//! - Pipeline configuration types
//! - API message types
//! - Vector component definitions

pub mod config;
pub mod messages;

pub use config::*;
pub use messages::*;

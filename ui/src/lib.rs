//! Vectorize UI Library
//!
//! This crate provides the Vectorize user interface - a real-time pipeline
//! management tool for Vector observability pipelines.
//!
//! # Architecture
//!
//! The UI supports two connection modes:
//! - **Direct Mode**: Connect directly to a single Vector instance
//! - **Control Plane Mode**: Connect to a control plane managing multiple Vector instances
//!
//! # Modules
//!
//! - [`app`]: Root application component and routing
//! - [`client`]: Connection abstraction layer (DirectClient, ControlPlaneClient)
//! - [`components`]: UI components (pipeline builder, data view, etc.)
//! - [`state`]: Global state management

pub mod app;
pub mod client;
pub mod components;
pub mod state;

pub use app::App;

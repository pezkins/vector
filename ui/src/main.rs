//! Vectorize UI Entry Point
//!
//! This is the main entry point for the Vectorize UI WASM application.
//! It initializes logging and mounts the Leptos app to the DOM.

use leptos::*;
use tracing_wasm::WASMLayerConfigBuilder;

mod app;
mod client;
mod components;
mod state;

pub use app::App;

fn main() {
    // Initialize WASM tracing
    let config = WASMLayerConfigBuilder::default()
        .set_max_level(tracing::Level::DEBUG)
        .build();
    tracing_wasm::set_as_global_default_with_config(config);
    
    tracing::info!("Starting Vectorize UI");
    
    // Mount the app
    mount_to_body(|| view! { <App /> });
}

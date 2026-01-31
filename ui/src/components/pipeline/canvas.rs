//! Pipeline Canvas Component
//!
//! The main visual canvas for building pipelines via drag-and-drop.

use leptos::*;
use leptos::wasm_bindgen::JsCast;
use vectorize_shared::{NodeType, PipelineNode as PipelineNodeData, Position, SourceConfig};

use super::PipelineNode;
use crate::state::AppState;

/// Main pipeline canvas with drag-and-drop support
#[component]
pub fn PipelineCanvas() -> impl IntoView {
    let app_state = expect_context::<AppState>();
    let (drag_over, set_drag_over) = create_signal(false);
    
    // Handle drop from palette
    let app_state_drop = app_state.clone();
    let on_drop = move |e: web_sys::DragEvent| {
        e.prevent_default();
        set_drag_over.set(false);
        
        if let Some(data_transfer) = e.data_transfer() {
            if let Ok(data) = data_transfer.get_data("application/json") {
                if let Ok(component_type) = serde_json::from_str::<String>(&data) {
                    // Get drop position - use client coordinates as fallback
                    let x = e.client_x() as f64 - 280.0; // Approximate offset for sidebar
                    let y = e.client_y() as f64 - 120.0; // Approximate offset for header + toolbar
                    
                    // Create new node
                    let node = create_node_from_type(&component_type, Position { x: x.max(50.0), y: y.max(50.0) });
                    
                    // Add to pipeline
                    let mut pipeline = app_state_drop.pipeline.get();
                    let node_id = node.id.clone();
                    pipeline.add_node(node);
                    app_state_drop.pipeline.set(pipeline);
                    
                    // Select the new node
                    app_state_drop.selected_node.set(Some(node_id));
                }
            }
        }
    };
    
    view! {
        <div
            class=move || {
                let base = "relative w-full h-full bg-slate-900";
                let grid = "bg-grid";
                let drag = if drag_over.get() { "ring-2 ring-inset ring-blue-500" } else { "" };
                format!("{} {} {}", base, grid, drag)
            }
            on:dragover=move |e: web_sys::DragEvent| {
                e.prevent_default();
                set_drag_over.set(true);
            }
            on:dragleave=move |_| {
                set_drag_over.set(false);
            }
            on:drop=on_drop
        >
            // Render pipeline nodes
            {
                let app_state = app_state.clone();
                move || {
                    let app_state = app_state.clone();
                    let pipeline = app_state.pipeline.get();
                    pipeline.nodes.iter().map(|(id, node)| {
                        let node = node.clone();
                        let id2 = id.clone();
                        let app_state = app_state.clone();
                        let app_state2 = app_state.clone();
                        view! {
                            <PipelineNode
                                node=node
                                on_select=move |node_id| app_state.selected_node.set(Some(node_id))
                                selected=move || app_state2.selected_node.get() == Some(id2.clone())
                            />
                        }
                    }).collect_view()
                }
            }
            
            // Empty state
            {
                let app_state = app_state.clone();
                view! {
                    <Show when=move || app_state.pipeline.get().nodes.is_empty()>
                        <div class="absolute inset-0 flex items-center justify-center pointer-events-none">
                            <div class="text-center text-slate-500">
                                <p class="text-lg mb-2">"Drag components here to build your pipeline"</p>
                                <p class="text-sm">"Start with a source, add transforms, and end with a sink"</p>
                            </div>
                        </div>
                    </Show>
                }
            }
        </div>
    }
}

/// Create a new pipeline node from a component type
fn create_node_from_type(component_type: &str, position: Position) -> PipelineNodeData {
    let node_type = match component_type {
        // Sources
        "stdin" => NodeType::Source(SourceConfig::new("stdin")),
        "file" => NodeType::Source(
            SourceConfig::new("file")
                .with_option("include", serde_json::json!(["/var/log/**/*.log"]))
        ),
        "http_server" => NodeType::Source(
            SourceConfig::new("http_server")
                .with_option("address", "0.0.0.0:8080")
        ),
        "demo_logs" => NodeType::Source(
            SourceConfig::new("demo_logs")
                .with_option("format", "json")
        ),
        
        // Transforms
        "remap" => NodeType::Transform(
            vectorize_shared::TransformConfig::new("remap", vec![])
                .with_option("source", ". = parse_json!(.message)")
        ),
        "filter" => NodeType::Transform(
            vectorize_shared::TransformConfig::new("filter", vec![])
                .with_option("condition", ".level == \"error\"")
        ),
        "route" => NodeType::Transform(
            vectorize_shared::TransformConfig::new("route", vec![])
        ),
        
        // Sinks
        "console" => NodeType::Sink(
            vectorize_shared::SinkConfig::new("console", vec![])
                .with_option("encoding", serde_json::json!({"codec": "json"}))
        ),
        "file_sink" => NodeType::Sink(
            vectorize_shared::SinkConfig::new("file", vec![])
                .with_option("path", "/var/log/output.log")
        ),
        "http" => NodeType::Sink(
            vectorize_shared::SinkConfig::new("http", vec![])
                .with_option("uri", "http://localhost:9000")
        ),
        
        // Default to stdin source
        _ => NodeType::Source(SourceConfig::new(component_type)),
    };
    
    PipelineNodeData::new(component_type, node_type).with_position(position.x, position.y)
}

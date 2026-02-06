//! Pipeline Node Component
//!
//! Visual representation of a node in the pipeline canvas.
//! Features n8n-inspired design with structured sections:
//! - Header: icon, name, status, menu
//! - Type badge: component type with category color
//! - Config preview: 2-3 key configuration options
//! - Metrics footer: event/error counts, run button
//! - Ports: input/output connection points

use leptos::*;
use vectorize_shared::{NodeStatus, NodeType, PipelineNode as PipelineNodeData, SourceConfig, TransformConfig, SinkConfig};

use crate::components::common::*;
use crate::state::AppState;

/// Node dimensions (exported for canvas calculations)
pub const NODE_WIDTH: f64 = 240.0;
#[allow(dead_code)]
pub const NODE_HEIGHT: f64 = 140.0;

/// Port radius for connection calculations
#[allow(dead_code)]
pub const PORT_RADIUS: f64 = 6.0;

/// Extract key configuration options for preview display
fn get_config_preview(node_type: &NodeType) -> Vec<(String, String)> {
    let mut preview = Vec::new();
    
    let options = match node_type {
        NodeType::Source(SourceConfig { options, source_type, .. }) => {
            // Add source-specific key fields first
            match source_type.as_str() {
                "demo_logs" => {
                    if let Some(v) = options.get("interval") {
                        preview.push(("interval".to_string(), format_value(v)));
                    }
                    if let Some(v) = options.get("format") {
                        preview.push(("format".to_string(), format_value(v)));
                    }
                    if let Some(v) = options.get("lines") {
                        preview.push(("lines".to_string(), format_value(v)));
                    }
                }
                "file" => {
                    if let Some(v) = options.get("include") {
                        preview.push(("include".to_string(), format_value(v)));
                    }
                }
                "http_server" => {
                    if let Some(v) = options.get("address") {
                        preview.push(("address".to_string(), format_value(v)));
                    }
                }
                "kafka" => {
                    if let Some(v) = options.get("bootstrap_servers") {
                        preview.push(("servers".to_string(), format_value(v)));
                    }
                    if let Some(v) = options.get("topics") {
                        preview.push(("topics".to_string(), format_value(v)));
                    }
                }
                _ => {}
            }
            options
        }
        NodeType::Transform(TransformConfig { options, transform_type, .. }) => {
            match transform_type.as_str() {
                "remap" => {
                    if let Some(v) = options.get("source") {
                        let src = format_value(v);
                        // Truncate long VRL source
                        let truncated = if src.len() > 30 {
                            format!("{}...", &src[..27])
                        } else {
                            src
                        };
                        preview.push(("source".to_string(), truncated));
                    }
                }
                "filter" => {
                    if let Some(v) = options.get("condition") {
                        preview.push(("condition".to_string(), format_value(v)));
                    }
                }
                "route" => {
                    if let Some(v) = options.get("route") {
                        let count = if let serde_json::Value::Object(m) = v {
                            m.len()
                        } else {
                            0
                        };
                        preview.push(("routes".to_string(), format!("{} defined", count)));
                    }
                }
                "sample" => {
                    if let Some(v) = options.get("rate") {
                        preview.push(("rate".to_string(), format_value(v)));
                    }
                }
                "reduce" => {
                    if let Some(v) = options.get("group_by") {
                        preview.push(("group_by".to_string(), format_value(v)));
                    }
                }
                _ => {}
            }
            options
        }
        NodeType::Sink(SinkConfig { options, sink_type, .. }) => {
            match sink_type.as_str() {
                "console" => {
                    if let Some(v) = options.get("encoding") {
                        preview.push(("encoding".to_string(), format_value(v)));
                    }
                }
                "file" => {
                    if let Some(v) = options.get("path") {
                        preview.push(("path".to_string(), format_value(v)));
                    }
                }
                "http" => {
                    if let Some(v) = options.get("uri") {
                        preview.push(("uri".to_string(), format_value(v)));
                    }
                }
                "kafka" => {
                    if let Some(v) = options.get("bootstrap_servers") {
                        preview.push(("servers".to_string(), format_value(v)));
                    }
                    if let Some(v) = options.get("topic") {
                        preview.push(("topic".to_string(), format_value(v)));
                    }
                }
                "elasticsearch" => {
                    if let Some(v) = options.get("endpoints") {
                        preview.push(("endpoints".to_string(), format_value(v)));
                    }
                }
                _ => {}
            }
            options
        }
    };
    
    // If we haven't found specific fields, add generic options
    if preview.is_empty() {
        for (key, value) in options.iter().take(2) {
            // Skip internal/less useful fields
            if key != "type" && key != "inputs" {
                preview.push((key.clone(), format_value(value)));
            }
        }
    }
    
    // Limit to 3 items max
    preview.truncate(3);
    preview
}

/// Format a JSON value for display
fn format_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => {
            if s.len() > 25 {
                format!("{}...", &s[..22])
            } else {
                s.clone()
            }
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Array(arr) => {
            if arr.len() == 1 {
                format_value(&arr[0])
            } else {
                format!("[{} items]", arr.len())
            }
        }
        serde_json::Value::Object(obj) => {
            // For encoding objects, try to get codec
            if let Some(codec) = obj.get("codec") {
                format_value(codec)
            } else {
                "{...}".to_string()
            }
        }
        serde_json::Value::Null => "null".to_string(),
    }
}

/// Format large numbers with K/M suffix
fn format_count(count: usize) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

/// Pipeline node on the canvas
#[component]
pub fn PipelineNode(
    node: PipelineNodeData,
    #[prop(into)] on_select: Callback<String>,
    #[prop(into)] selected: Signal<bool>,
    #[prop(into)] on_output_port_drag_start: Callback<()>,
    #[prop(into)] on_input_port_drop: Callback<()>,
) -> impl IntoView {
    let app_state = expect_context::<AppState>();
    let (port_hover, set_port_hover) = create_signal::<Option<&'static str>>(None);
    let (collapsed, set_collapsed) = create_signal(false);
    let (menu_open, set_menu_open) = create_signal(false);
    
    let node_id = node.id.clone();
    let node_id2 = node.id.clone();
    let node_id_click = node_id.clone();
    let node_id_menu = node_id.clone();
    let node_id_position = node_id.clone();
    
    let category = match &node.node_type {
        NodeType::Source(_) => "source",
        NodeType::Transform(_) => "transform",
        NodeType::Sink(_) => "sink",
    };
    
    // Get node status from state (default to Idle)
    let node_id_status = node_id.clone();
    let status = create_memo(move |_| {
        app_state.node_statuses.get().get(&node_id_status).copied().unwrap_or_default()
    });
    
    // Get event count for this node
    let node_id_events = node_id.clone();
    let event_count = create_memo(move |_| {
        app_state.node_events.get().get(&node_id_events).map(|v| v.len()).unwrap_or(0)
    });
    
    // Get error count (placeholder - would come from metrics)
    let error_count = create_memo(move |_| 0usize);
    
    // Selection ring styling
    let selection_ring = move || {
        if selected.get() {
            "ring-2 ring-accent ring-offset-2 ring-offset-theme-bg"
        } else {
            ""
        }
    };
    
    // Status-based border styling
    let status_border = move || {
        match status.get() {
            NodeStatus::Running => "border-warning animate-pulse",
            NodeStatus::Success => "border-success",
            NodeStatus::Error => "border-error",
            NodeStatus::Warning => "border-warning",
            NodeStatus::Idle => "border-theme-border",
        }
    };
    
    // Category-specific styling
    let (bg_class, accent_color, icon_class) = match category {
        "source" => (
            "bg-source/10 hover:bg-source/20",
            "text-source",
            "bg-source/20 text-source",
        ),
        "transform" => (
            "bg-transform/10 hover:bg-transform/20",
            "text-transform",
            "bg-transform/20 text-transform",
        ),
        "sink" => (
            "bg-sink/10 hover:bg-sink/20",
            "text-sink",
            "bg-sink/20 text-sink",
        ),
        _ => (
            "bg-theme-surface hover:bg-theme-surface-hover",
            "text-theme-muted",
            "bg-theme-surface-hover text-theme-muted",
        ),
    };
    
    // Port colors based on category
    let port_bg = match category {
        "source" => "bg-source",
        "transform" => "bg-transform",
        "sink" => "bg-sink",
        _ => "bg-theme-muted",
    };
    
    let has_input = category != "source";
    let has_output = category != "sink";
    
    let style = {
        let pos = node.position;
        move || {
            format!(
                "position: absolute; left: {}px; top: {}px; width: {}px;",
                pos.x,
                pos.y,
                NODE_WIDTH
            )
        }
    };
    
    // Handle node dragging for repositioning
    let app_state_drag = app_state.clone();
    let on_drag_end = move |e: web_sys::DragEvent| {
        let x = e.client_x() as f64 - 280.0;
        let y = e.client_y() as f64 - 120.0;
        
        let mut pipeline = app_state_drag.pipeline.get();
        pipeline.update_node_position(&node_id_position, x.max(0.0), y.max(0.0));
        app_state_drag.pipeline.set(pipeline);
    };
    
    let node_type_name = node.node_type.display_name().to_string();
    let node_name = node.name.clone();
    let config_preview = get_config_preview(&node.node_type);
    let has_config = !config_preview.is_empty();
    
    view! {
        <div
            class=move || format!(
                "group relative rounded-xl border shadow-lg backdrop-blur-sm overflow-visible {} {} {} transition-all duration-150 z-10",
                bg_class,
                status_border(),
                selection_ring()
            )
            style=style
            on:click=move |e| {
                e.stop_propagation();
                set_menu_open.set(false);
                on_select.call(node_id_click.clone());
            }
            draggable="true"
            on:dragend=on_drag_end
        >
            // === INPUT PORT (left side) ===
            {if has_input {
                let on_drop = on_input_port_drop;
                let is_hovering = move || port_hover.get() == Some("input");
                view! {
                    <div
                        class=move || format!(
                            "absolute -left-1.5 top-1/2 -translate-y-1/2 w-3 h-3 rounded-full {} border-2 border-theme-bg cursor-crosshair transition-all duration-150 z-50 {}",
                            port_bg,
                            if is_hovering() { "scale-150 ring-2 ring-accent" } else { "hover:scale-125" }
                        )
                        title="Drop connection here (input)"
                        on:mouseenter=move |_| set_port_hover.set(Some("input"))
                        on:mouseleave=move |_| set_port_hover.set(None)
                        on:dragover=move |e: web_sys::DragEvent| {
                            e.prevent_default();
                            e.stop_propagation();
                            set_port_hover.set(Some("input"));
                        }
                        on:dragleave=move |_| set_port_hover.set(None)
                        on:drop=move |e: web_sys::DragEvent| {
                            e.prevent_default();
                            e.stop_propagation();
                            
                            if let Some(dt) = e.data_transfer() {
                                if let Ok(data) = dt.get_data("application/connection") {
                                    if data == "output" {
                                        set_port_hover.set(None);
                                        on_drop.call(());
                                    }
                                }
                            }
                        }
                    />
                }.into_view()
            } else {
                view! {}.into_view()
            }}
            
            // === OUTPUT PORT (right side) ===
            {if has_output {
                let on_drag_start = on_output_port_drag_start;
                let is_hovering = move || port_hover.get() == Some("output");
                view! {
                    <div
                        class=move || format!(
                            "absolute -right-1.5 top-1/2 -translate-y-1/2 w-3 h-3 rounded-full {} border-2 border-theme-bg cursor-grab transition-all duration-150 z-50 {}",
                            port_bg,
                            if is_hovering() { "scale-150" } else { "hover:scale-125" }
                        )
                        title="Drag to connect (output)"
                        draggable="true"
                        on:mouseenter=move |_| set_port_hover.set(Some("output"))
                        on:mouseleave=move |_| set_port_hover.set(None)
                        on:dragstart=move |e: web_sys::DragEvent| {
                            e.stop_propagation();
                            if let Some(dt) = e.data_transfer() {
                                let _ = dt.set_data("application/connection", "output");
                            }
                            on_drag_start.call(());
                        }
                    />
                }.into_view()
            } else {
                view! {}.into_view()
            }}
            
            // === HEADER SECTION ===
            <div class="flex items-center gap-2 px-3 py-2 border-b border-theme-border/50">
                // Icon container
                <div class=format!("flex-shrink-0 w-7 h-7 rounded-lg flex items-center justify-center {}", icon_class)>
                    {match category {
                        "source" => view! { <SourceIcon class="w-4 h-4" /> }.into_view(),
                        "transform" => view! { <TransformIcon class="w-4 h-4" /> }.into_view(),
                        "sink" => view! { <SinkIcon class="w-4 h-4" /> }.into_view(),
                        _ => view! { <TransformIcon class="w-4 h-4" /> }.into_view(),
                    }}
                </div>
                
                // Node name
                <div class="flex-1 min-w-0">
                    <div class="text-sm font-semibold text-theme truncate">{node_name}</div>
                </div>
                
                // Status indicator
                <div class="flex-shrink-0">
                    {move || {
                        let (color, title) = match status.get() {
                            NodeStatus::Running => ("bg-warning animate-pulse", "Running"),
                            NodeStatus::Success => ("bg-success", "Success"),
                            NodeStatus::Error => ("bg-error", "Error"),
                            NodeStatus::Warning => ("bg-warning", "Warning"),
                            NodeStatus::Idle => ("bg-theme-muted", "Idle"),
                        };
                        view! {
                            <div class=format!("w-2 h-2 rounded-full {}", color) title=title />
                        }
                    }}
                </div>
                
                // Dropdown menu button
                <button
                    class="flex-shrink-0 p-1 rounded hover:bg-theme-surface-hover text-theme-muted hover:text-theme transition-colors opacity-0 group-hover:opacity-100"
                    on:click=move |e| {
                        e.stop_propagation();
                        set_menu_open.update(|v| *v = !*v);
                    }
                >
                    <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="12" cy="12" r="1" />
                        <circle cx="12" cy="5" r="1" />
                        <circle cx="12" cy="19" r="1" />
                    </svg>
                </button>
            </div>
            
            // === TYPE BADGE ===
            <div class="px-3 py-1.5 border-b border-theme-border/50">
                <span class=format!("inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {}", accent_color)>
                    {node_type_name}
                </span>
            </div>
            
            // === CONFIG PREVIEW SECTION (collapsible) ===
            {if has_config {
                view! {
                    <div class=move || format!(
                        "border-b border-theme-border/50 overflow-hidden transition-all duration-200 {}",
                        if collapsed.get() { "max-h-0" } else { "max-h-24" }
                    )>
                        <div class="px-3 py-2 space-y-1">
                            {config_preview.iter().map(|(key, value)| {
                                let key = key.clone();
                                let value = value.clone();
                                view! {
                                    <div class="flex items-center text-xs">
                                        <span class="text-theme-muted w-16 truncate">{key}":"</span>
                                        <span class="text-theme-secondary truncate flex-1 font-mono">{value}</span>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    </div>
                }.into_view()
            } else {
                view! {}.into_view()
            }}
            
            // === METRICS FOOTER ===
            <div class="flex items-center justify-between px-3 py-2 text-xs">
                <div class="flex items-center gap-3">
                    // Events count
                    <span class="flex items-center gap-1 text-theme-secondary">
                        <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M22 12h-4l-3 9L9 3l-3 9H2" />
                        </svg>
                        {move || format_count(event_count.get())}
                    </span>
                    
                    // Errors count (only show if > 0)
                    {move || {
                        let errors = error_count.get();
                        if errors > 0 {
                            view! {
                                <span class="flex items-center gap-1 text-error">
                                    <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                        <circle cx="12" cy="12" r="10" />
                                        <line x1="12" y1="8" x2="12" y2="12" />
                                        <line x1="12" y1="16" x2="12.01" y2="16" />
                                    </svg>
                                    {format_count(errors)}
                                </span>
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                </div>
                
                // Collapse/expand button
                {if has_config {
                    view! {
                        <button
                            class="p-1 rounded hover:bg-theme-surface-hover text-theme-muted hover:text-theme transition-colors"
                            on:click=move |e| {
                                e.stop_propagation();
                                set_collapsed.update(|v| *v = !*v);
                            }
                            title=move || if collapsed.get() { "Expand" } else { "Collapse" }
                        >
                            <svg 
                                class=move || format!("w-3 h-3 transition-transform {}", if collapsed.get() { "rotate-180" } else { "" })
                                viewBox="0 0 24 24" 
                                fill="none" 
                                stroke="currentColor" 
                                stroke-width="2"
                            >
                                <polyline points="18 15 12 9 6 15" />
                            </svg>
                        </button>
                    }.into_view()
                } else {
                    view! {}.into_view()
                }}
            </div>
            
            // === DROPDOWN MENU ===
            {
                let node_id_menu_inner = node_id_menu.clone();
                let node_id_menu_delete = node_id_menu.clone();
                let app_state_menu = app_state.clone();
                move || {
                    if menu_open.get() {
                        let node_id_for_select = node_id_menu_inner.clone();
                        let node_id_for_delete = node_id_menu_delete.clone();
                        let app_state_for_delete = app_state_menu.clone();
                        view! {
                            <div class="absolute right-0 top-10 w-36 rounded-lg bg-theme-surface border border-theme-border shadow-xl z-50 overflow-hidden animate-fade-in">
                                <button
                                    class="w-full px-3 py-2 text-left text-sm text-theme hover:bg-theme-surface-hover transition-colors flex items-center gap-2"
                                    on:click=move |e| {
                                        e.stop_propagation();
                                        set_menu_open.set(false);
                                        // Trigger select to open config panel
                                        on_select.call(node_id_for_select.clone());
                                    }
                                >
                                    <SettingsIcon class="w-4 h-4 text-theme-muted" />
                                    "Configure"
                                </button>
                                <div class="border-t border-theme-border" />
                                <button
                                    class="w-full px-3 py-2 text-left text-sm text-error hover:bg-error/10 transition-colors flex items-center gap-2"
                                    on:click=move |e| {
                                        e.stop_propagation();
                                        set_menu_open.set(false);
                                        let mut pipeline = app_state_for_delete.pipeline.get();
                                        pipeline.remove_node(&node_id_for_delete);
                                        app_state_for_delete.pipeline.set(pipeline);
                                        app_state_for_delete.selected_node.set(None);
                                    }
                                >
                                    <TrashIcon class="w-4 h-4" />
                                    "Delete"
                                </button>
                            </div>
                        }.into_view()
                    } else {
                        view! {}.into_view()
                    }
                }
            }
            
            // === DELETE BUTTON (quick access on hover) ===
            {
                let node_id_delete = node_id2.clone();
                let app_state_delete = app_state.clone();
                view! {
                    <button
                        class="absolute -top-2 -right-2 w-5 h-5 rounded-full bg-error text-white text-xs opacity-0 group-hover:opacity-100 transition-opacity duration-150 hover:bg-red-400 flex items-center justify-center shadow-lg"
                        on:click=move |e| {
                            e.stop_propagation();
                            let mut pipeline = app_state_delete.pipeline.get();
                            pipeline.remove_node(&node_id_delete);
                            app_state_delete.pipeline.set(pipeline);
                            app_state_delete.selected_node.set(None);
                        }
                        title="Delete node"
                    >
                        "×"
                    </button>
                }
            }
        </div>
    }
}

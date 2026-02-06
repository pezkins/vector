//! Pipeline Canvas Component
//!
//! The main visual canvas for building pipelines via drag-and-drop.
//! Features n8n-style visual connections between nodes with zoom/pan controls.

use leptos::*;
use wasm_bindgen::JsCast;
use vectorize_shared::{NodeType, PipelineNode as PipelineNodeData, Position, SourceConfig};

use super::PipelineNode;
use crate::state::AppState;

/// Node dimensions for connection calculations
const NODE_WIDTH: f64 = 200.0;
#[allow(dead_code)]
const NODE_HEIGHT: f64 = 80.0;
const PORT_OFFSET_Y: f64 = 40.0; // Center of node

/// Zoom constraints
const MIN_ZOOM: f64 = 0.25;
const MAX_ZOOM: f64 = 2.0;
const ZOOM_STEP: f64 = 0.1;

/// Grid settings
const GRID_SIZE: f64 = 20.0; // Size of grid cells in pixels

/// Minimap settings
const MINIMAP_WIDTH: f64 = 180.0;
const MINIMAP_HEIGHT: f64 = 120.0;
const MINIMAP_SCALE: f64 = 0.08;

/// Main pipeline canvas with drag-and-drop support
#[component]
pub fn PipelineCanvas() -> impl IntoView {
    let app_state = expect_context::<AppState>();
    let (drag_over, set_drag_over) = create_signal(false);
    
    // Zoom and pan state
    let (zoom, set_zoom) = create_signal(1.0_f64);
    let (pan_x, set_pan_x) = create_signal(0.0_f64);
    let (pan_y, set_pan_y) = create_signal(0.0_f64);
    
    // Pan interaction state
    let (is_panning, set_is_panning) = create_signal(false);
    let (pan_start_x, set_pan_start_x) = create_signal(0.0_f64);
    let (pan_start_y, set_pan_start_y) = create_signal(0.0_f64);
    let (space_held, set_space_held) = create_signal(false);
    
    // Minimap visibility
    let (show_minimap, set_show_minimap) = create_signal(true);
    
    // Track connection being drawn
    let (drawing_connection, set_drawing_connection) = create_signal::<Option<ConnectionDraft>>(None);
    let (mouse_pos, set_mouse_pos) = create_signal(Position { x: 0.0, y: 0.0 });
    
    // Canvas ref for coordinate calculations
    let canvas_ref = create_node_ref::<html::Div>();
    
    // Zoom in function
    let zoom_in = move |_| {
        set_zoom.update(|z| *z = (*z + ZOOM_STEP).min(MAX_ZOOM));
    };
    
    // Zoom out function
    let zoom_out = move |_| {
        set_zoom.update(|z| *z = (*z - ZOOM_STEP).max(MIN_ZOOM));
    };
    
    // Zoom to fit function
    let app_state_fit = app_state.clone();
    let zoom_to_fit = move |_| {
        let pipeline = app_state_fit.pipeline.get();
        if pipeline.nodes.is_empty() {
            set_zoom.set(1.0);
            set_pan_x.set(0.0);
            set_pan_y.set(0.0);
            return;
        }
        
        // Calculate bounding box of all nodes
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        
        for node in pipeline.nodes.values() {
            min_x = min_x.min(node.position.x);
            min_y = min_y.min(node.position.y);
            max_x = max_x.max(node.position.x + NODE_WIDTH);
            max_y = max_y.max(node.position.y + NODE_HEIGHT);
        }
        
        // Add padding
        let padding = 50.0;
        min_x -= padding;
        min_y -= padding;
        max_x += padding;
        max_y += padding;
        
        // Calculate zoom to fit
        if let Some(canvas) = canvas_ref.get() {
            let canvas_width = canvas.client_width() as f64;
            let canvas_height = canvas.client_height() as f64;
            
            let content_width = max_x - min_x;
            let content_height = max_y - min_y;
            
            let zoom_x = canvas_width / content_width;
            let zoom_y = canvas_height / content_height;
            let new_zoom = zoom_x.min(zoom_y).clamp(MIN_ZOOM, MAX_ZOOM);
            
            set_zoom.set(new_zoom);
            set_pan_x.set(-min_x);
            set_pan_y.set(-min_y);
        }
    };
    
    // Handle mouse wheel zoom (with Ctrl)
    let on_wheel = move |e: web_sys::WheelEvent| {
        if e.ctrl_key() || e.meta_key() {
            e.prevent_default();
            
            let delta = -e.delta_y().signum() * ZOOM_STEP;
            let old_zoom = zoom.get();
            let new_zoom = (old_zoom + delta).clamp(MIN_ZOOM, MAX_ZOOM);
            
            if (new_zoom - old_zoom).abs() > 0.001 {
                // Get mouse position relative to canvas
                if let Some(canvas) = canvas_ref.get() {
                    let rect = canvas.get_bounding_client_rect();
                    let mouse_x = e.client_x() as f64 - rect.left();
                    let mouse_y = e.client_y() as f64 - rect.top();
                    
                    // Calculate world position under cursor before zoom
                    let world_x = (mouse_x / old_zoom) - pan_x.get();
                    let world_y = (mouse_y / old_zoom) - pan_y.get();
                    
                    // Update zoom
                    set_zoom.set(new_zoom);
                    
                    // Calculate new pan to keep cursor position stable
                    let new_pan_x = (mouse_x / new_zoom) - world_x;
                    let new_pan_y = (mouse_y / new_zoom) - world_y;
                    
                    set_pan_x.set(new_pan_x);
                    set_pan_y.set(new_pan_y);
                }
            }
        }
    };
    
    // Handle keyboard events for space key
    let on_keydown = move |e: web_sys::KeyboardEvent| {
        if e.code() == "Space" && !e.repeat() {
            e.prevent_default();
            set_space_held.set(true);
        }
    };
    
    let on_keyup = move |e: web_sys::KeyboardEvent| {
        if e.code() == "Space" {
            set_space_held.set(false);
            set_is_panning.set(false);
        }
    };
    
    // Handle pan start (middle click or space + click)
    let on_mousedown = move |e: web_sys::MouseEvent| {
        // Middle mouse button (1) or space held + left click (0)
        if e.button() == 1 || (e.button() == 0 && space_held.get()) {
            e.prevent_default();
            set_is_panning.set(true);
            set_pan_start_x.set(e.client_x() as f64 - pan_x.get() * zoom.get());
            set_pan_start_y.set(e.client_y() as f64 - pan_y.get() * zoom.get());
        }
    };
    
    // Handle pan movement
    let on_mouse_move = move |e: web_sys::MouseEvent| {
        if is_panning.get() {
            let z = zoom.get();
            set_pan_x.set((e.client_x() as f64 - pan_start_x.get()) / z);
            set_pan_y.set((e.client_y() as f64 - pan_start_y.get()) / z);
        } else if drawing_connection.get().is_some() {
            // Transform mouse position to canvas coordinates
            let z = zoom.get();
            let px = pan_x.get();
            let py = pan_y.get();
            
            if let Some(canvas) = canvas_ref.get() {
                let rect = canvas.get_bounding_client_rect();
                let x = (e.client_x() as f64 - rect.left()) / z - px;
                let y = (e.client_y() as f64 - rect.top()) / z - py;
                set_mouse_pos.set(Position { x, y });
            }
        }
    };
    
    // Handle pan end
    let on_mouseup = move |e: web_sys::MouseEvent| {
        if e.button() == 1 || (e.button() == 0 && is_panning.get()) {
            set_is_panning.set(false);
        }
    };
    
    // Cancel connection drawing on escape or click on empty space
    let on_canvas_click = move |_: web_sys::MouseEvent| {
        if !is_panning.get() {
            set_drawing_connection.set(None);
        }
    };
    
    // Handle drop from palette
    let app_state_drop = app_state.clone();
    let on_drop = move |e: web_sys::DragEvent| {
        e.prevent_default();
        e.stop_propagation();
        set_drag_over.set(false);
        
        if let Some(data_transfer) = e.data_transfer() {
            match data_transfer.get_data("application/json") {
                Ok(data) => {
                    match serde_json::from_str::<String>(&data) {
                        Ok(component_type) => {
                            // Transform drop position to canvas coordinates
                            if let Some(canvas) = canvas_ref.get() {
                                let rect = canvas.get_bounding_client_rect();
                                let z = zoom.get();
                                let px = pan_x.get();
                                let py = pan_y.get();
                                
                                let x = (e.client_x() as f64 - rect.left()) / z - px;
                                let y = (e.client_y() as f64 - rect.top()) / z - py;
                                
                                // Create new node
                                let node = create_node_from_type(&component_type, Position { x: x.max(20.0), y: y.max(20.0) });
                                
                                // Add to pipeline
                                let mut pipeline = app_state_drop.pipeline.get();
                                let node_id = node.id.clone();
                                pipeline.add_node(node);
                                app_state_drop.pipeline.set(pipeline);
                                
                                // Select the new node
                                app_state_drop.selected_node.set(Some(node_id));
                            }
                        }
                        Err(e) => {
                            web_sys::console::log_1(&format!("Failed to parse component type: {:?}", e).into());
                        }
                    }
                }
                Err(e) => {
                    web_sys::console::log_1(&format!("Failed to get data: {:?}", e).into());
                }
            }
        }
    };
    
    // Grid background style
    let grid_style = move || {
        let z = zoom.get();
        let scaled_grid = GRID_SIZE * z;
        let px = pan_x.get() * z;
        let py = pan_y.get() * z;
        
        format!(
            "background-image: radial-gradient(circle, var(--color-border) 1px, transparent 1px); \
             background-size: {scaled_grid}px {scaled_grid}px; \
             background-position: {px}px {py}px;",
        )
    };
    
    // Transform style for canvas content
    let content_transform = move || {
        format!(
            "transform: scale({}) translate({}px, {}px); transform-origin: 0 0;",
            zoom.get(),
            pan_x.get(),
            pan_y.get()
        )
    };
    
    // Cursor style based on state
    let cursor_style = move || {
        if is_panning.get() {
            "cursor: grabbing;"
        } else if space_held.get() {
            "cursor: grab;"
        } else {
            "cursor: default;"
        }
    };
    
    view! {
        <div
            id="pipeline-canvas"
            node_ref=canvas_ref
            tabindex="0"
            style=move || format!("min-height: 400px; {} {}", grid_style(), cursor_style())
            class=move || {
                let base = "relative w-full h-full bg-theme-bg overflow-hidden focus:outline-none";
                let drag = if drag_over.get() { "ring-2 ring-inset ring-blue-500" } else { "" };
                format!("{} {}", base, drag)
            }
            on:dragover=move |e: web_sys::DragEvent| {
                e.prevent_default();
                e.stop_propagation();
                if let Some(dt) = e.data_transfer() {
                    dt.set_drop_effect("copy");
                }
                set_drag_over.set(true);
            }
            on:dragleave=move |e: web_sys::DragEvent| {
                e.prevent_default();
                set_drag_over.set(false);
            }
            on:drop=on_drop
            on:mousemove=on_mouse_move
            on:mousedown=on_mousedown
            on:mouseup=on_mouseup
            on:click=on_canvas_click
            on:wheel=on_wheel
            on:keydown=on_keydown
            on:keyup=on_keyup
            on:contextmenu=move |e: web_sys::MouseEvent| e.prevent_default()
        >
            // Transformed content container
            <div
                class="absolute inset-0 pointer-events-none"
                style=content_transform
            >
                // SVG layer for connections
                <ConnectionsLayer
                    drawing_connection=drawing_connection
                    mouse_pos=mouse_pos
                />
                
                // Render pipeline nodes
                <div class="pointer-events-auto">
                    {
                        let app_state = app_state.clone();
                        move || {
                            let app_state = app_state.clone();
                            let pipeline = app_state.pipeline.get();
                            
                            pipeline.nodes.iter().map(|(id, node)| {
                                let node = node.clone();
                                let id2 = id.clone();
                                let id3 = id.clone();
                                let id4 = id.clone();
                                let app_state = app_state.clone();
                                let app_state3 = app_state.clone();
                                
                                // Create signals for the node
                                let selected_signal = Signal::derive(move || {
                                    app_state.selected_node.get() == Some(id2.clone())
                                });
                                
                                view! {
                                    <PipelineNode
                                        node=node
                                        on_select=Callback::new(move |node_id: String| app_state.selected_node.set(Some(node_id)))
                                        selected=selected_signal
                                        on_output_port_drag_start=Callback::new(move |_: ()| {
                                            set_drawing_connection.set(Some(ConnectionDraft {
                                                from_node: id3.clone(),
                                                from_port: None,
                                            }));
                                        })
                                        on_input_port_drop=Callback::new(move |_: ()| {
                                            // Complete the connection
                                            if let Some(draft) = drawing_connection.get() {
                                                if draft.from_node != id4 {
                                                    let mut pipeline = app_state3.pipeline.get();
                                                    // Check if connection doesn't already exist
                                                    if !pipeline.has_connection(&draft.from_node, &id4) {
                                                        pipeline.connect(&draft.from_node, &id4);
                                                        app_state3.pipeline.set(pipeline);
                                                    }
                                                }
                                            }
                                            set_drawing_connection.set(None);
                                        })
                                    />
                                }
                            }).collect_view()
                        }
                    }
                </div>
            </div>
            
            // Empty state (not transformed)
            {
                let app_state = app_state.clone();
                view! {
                    <Show when=move || app_state.pipeline.get().nodes.is_empty()>
                        <div class="absolute inset-0 flex items-center justify-center pointer-events-none">
                            <div class="text-center text-theme-muted">
                                <p class="text-lg mb-2">"Drag components here to build your pipeline"</p>
                                <p class="text-sm">"Start with a source, add transforms, and end with a sink"</p>
                            </div>
                        </div>
                    </Show>
                }
            }
            
            // Zoom controls (bottom-right corner)
            <ZoomControls
                zoom=zoom
                on_zoom_in=zoom_in
                on_zoom_out=zoom_out
                on_zoom_fit=zoom_to_fit
            />
            
            // Minimap (bottom-left corner)
            <Show when=move || show_minimap.get()>
                <Minimap
                    zoom=zoom
                    pan_x=pan_x
                    pan_y=pan_y
                    canvas_ref=canvas_ref
                    set_pan_x=set_pan_x
                    set_pan_y=set_pan_y
                />
            </Show>
            
            // Minimap toggle button
            <button
                class="absolute bottom-4 left-4 w-8 h-8 rounded-lg bg-theme-surface border border-theme-border 
                       flex items-center justify-center text-theme-muted hover:text-theme hover:bg-theme-surface-hover 
                       transition-colors z-20"
                style=move || if show_minimap.get() { "bottom: 140px;" } else { "" }
                on:click=move |_| set_show_minimap.update(|v| *v = !*v)
                title=move || if show_minimap.get() { "Hide minimap" } else { "Show minimap" }
            >
                <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <rect x="3" y="3" width="18" height="18" rx="2" />
                    <rect x="5" y="5" width="6" height="4" rx="1" />
                </svg>
            </button>
        </div>
    }
}

/// Zoom control toolbar component
#[component]
fn ZoomControls(
    zoom: ReadSignal<f64>,
    on_zoom_in: impl Fn(web_sys::MouseEvent) + 'static,
    on_zoom_out: impl Fn(web_sys::MouseEvent) + 'static,
    on_zoom_fit: impl Fn(web_sys::MouseEvent) + 'static,
) -> impl IntoView {
    view! {
        <div class="absolute bottom-4 right-4 flex items-center gap-1 p-1 rounded-lg bg-theme-surface border border-theme-border shadow-lg z-20">
            // Zoom out button
            <button
                class="w-8 h-8 rounded flex items-center justify-center text-theme-muted hover:text-theme hover:bg-theme-surface-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                on:click=on_zoom_out
                disabled=move || zoom.get() <= MIN_ZOOM
                title="Zoom out"
            >
                <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <line x1="5" y1="12" x2="19" y2="12" />
                </svg>
            </button>
            
            // Zoom percentage display
            <div class="w-14 text-center text-sm font-mono text-theme-secondary">
                {move || format!("{}%", (zoom.get() * 100.0).round() as i32)}
            </div>
            
            // Zoom in button
            <button
                class="w-8 h-8 rounded flex items-center justify-center text-theme-muted hover:text-theme hover:bg-theme-surface-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                on:click=on_zoom_in
                disabled=move || zoom.get() >= MAX_ZOOM
                title="Zoom in"
            >
                <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <line x1="12" y1="5" x2="12" y2="19" />
                    <line x1="5" y1="12" x2="19" y2="12" />
                </svg>
            </button>
            
            // Separator
            <div class="w-px h-6 bg-theme-border mx-1"></div>
            
            // Zoom to fit button
            <button
                class="w-8 h-8 rounded flex items-center justify-center text-theme-muted hover:text-theme hover:bg-theme-surface-hover transition-colors"
                on:click=on_zoom_fit
                title="Zoom to fit"
            >
                <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M8 3H5a2 2 0 0 0-2 2v3" />
                    <path d="M21 8V5a2 2 0 0 0-2-2h-3" />
                    <path d="M3 16v3a2 2 0 0 0 2 2h3" />
                    <path d="M16 21h3a2 2 0 0 0 2-2v-3" />
                </svg>
            </button>
        </div>
    }
}

/// Minimap component for navigation overview
#[component]
fn Minimap(
    zoom: ReadSignal<f64>,
    pan_x: ReadSignal<f64>,
    pan_y: ReadSignal<f64>,
    canvas_ref: NodeRef<html::Div>,
    set_pan_x: WriteSignal<f64>,
    set_pan_y: WriteSignal<f64>,
) -> impl IntoView {
    let app_state = expect_context::<AppState>();
    let (is_dragging, set_is_dragging) = create_signal(false);
    
    // Calculate viewport rectangle
    let viewport_style = move || {
        let z = zoom.get();
        let px = pan_x.get();
        let py = pan_y.get();
        
        if let Some(canvas) = canvas_ref.get() {
            let canvas_width = canvas.client_width() as f64;
            let canvas_height = canvas.client_height() as f64;
            
            // Viewport in world coordinates
            let view_width = canvas_width / z;
            let view_height = canvas_height / z;
            let view_x = -px;
            let view_y = -py;
            
            // Scale to minimap
            let x = view_x * MINIMAP_SCALE;
            let y = view_y * MINIMAP_SCALE;
            let w = view_width * MINIMAP_SCALE;
            let h = view_height * MINIMAP_SCALE;
            
            format!(
                "left: {}px; top: {}px; width: {}px; height: {}px;",
                x.clamp(0.0, MINIMAP_WIDTH - 20.0),
                y.clamp(0.0, MINIMAP_HEIGHT - 15.0),
                w.min(MINIMAP_WIDTH),
                h.min(MINIMAP_HEIGHT)
            )
        } else {
            String::new()
        }
    };
    
    // Handle click on minimap to pan
    let on_minimap_click = move |e: web_sys::MouseEvent| {
        let target: Option<web_sys::EventTarget> = e.target();
        if let Some(t) = target {
            if let Ok(element) = t.dyn_into::<web_sys::Element>() {
                let rect = element.get_bounding_client_rect();
                let click_x = e.client_x() as f64 - rect.left();
                let click_y = e.client_y() as f64 - rect.top();
                
                // Convert minimap coordinates to world coordinates
                let world_x = click_x / MINIMAP_SCALE;
                let world_y = click_y / MINIMAP_SCALE;
                
                // Center the viewport on the clicked position
                if let Some(canvas) = canvas_ref.get() {
                    let canvas_width = canvas.client_width() as f64;
                    let canvas_height = canvas.client_height() as f64;
                    let z = zoom.get();
                    
                    let view_width = canvas_width / z;
                    let view_height = canvas_height / z;
                    
                    set_pan_x.set(-(world_x - view_width / 2.0));
                    set_pan_y.set(-(world_y - view_height / 2.0));
                }
            }
        }
    };
    
    // Handle drag on minimap
    let on_minimap_mousedown = move |e: web_sys::MouseEvent| {
        e.prevent_default();
        set_is_dragging.set(true);
    };
    
    let on_minimap_mousemove = move |e: web_sys::MouseEvent| {
        if is_dragging.get() {
            on_minimap_click(e);
        }
    };
    
    let on_minimap_mouseup = move |_: web_sys::MouseEvent| {
        set_is_dragging.set(false);
    };
    
    view! {
        <div 
            class="absolute bottom-4 left-4 rounded-lg bg-theme-surface border border-theme-border shadow-lg overflow-hidden z-20"
            style=format!("width: {}px; height: {}px;", MINIMAP_WIDTH, MINIMAP_HEIGHT)
            on:click=on_minimap_click
            on:mousedown=on_minimap_mousedown
            on:mousemove=on_minimap_mousemove
            on:mouseup=on_minimap_mouseup
            on:mouseleave=move |_| set_is_dragging.set(false)
        >
            // Minimap background with subtle grid
            <div 
                class="absolute inset-0 opacity-30"
                style="background-image: radial-gradient(circle, var(--color-border) 0.5px, transparent 0.5px); background-size: 8px 8px;"
            ></div>
            
            // Render nodes as small rectangles
            {
                move || {
                    let pipeline = app_state.pipeline.get();
                    
                    pipeline.nodes.values().map(|node| {
                        let x = node.position.x * MINIMAP_SCALE;
                        let y = node.position.y * MINIMAP_SCALE;
                        let w = NODE_WIDTH * MINIMAP_SCALE;
                        let h = NODE_HEIGHT * MINIMAP_SCALE;
                        
                        let color = match &node.node_type {
                            NodeType::Source(_) => "var(--color-source)",
                            NodeType::Transform(_) => "var(--color-transform)",
                            NodeType::Sink(_) => "var(--color-sink)",
                        };
                        
                        view! {
                            <div
                                class="absolute rounded-sm"
                                style=format!(
                                    "left: {}px; top: {}px; width: {}px; height: {}px; background-color: {};",
                                    x, y, w.max(4.0), h.max(3.0), color
                                )
                            ></div>
                        }
                    }).collect_view()
                }
            }
            
            // Viewport indicator
            <div
                class="absolute border-2 border-accent rounded pointer-events-none"
                style=viewport_style
            ></div>
        </div>
    }
}

/// Draft connection being drawn
#[derive(Clone, Debug)]
pub struct ConnectionDraft {
    pub from_node: String,
    #[allow(dead_code)]
    pub from_port: Option<String>,
}

/// SVG layer for rendering connections
#[component]
fn ConnectionsLayer(
    drawing_connection: ReadSignal<Option<ConnectionDraft>>,
    mouse_pos: ReadSignal<Position>,
) -> impl IntoView {
    let app_state = expect_context::<AppState>();
    let (hovered_connection, set_hovered_connection) = create_signal::<Option<String>>(None);
    
    view! {
        <svg 
            class="absolute inset-0 w-full h-full pointer-events-none"
            style="z-index: 5; overflow: visible;"
        >
            <defs>
                // Arrow marker for connection direction
                <marker
                    id="arrow"
                    markerWidth="10"
                    markerHeight="10"
                    refX="8"
                    refY="3"
                    orient="auto"
                    markerUnits="strokeWidth"
                >
                    <path d="M0,0 L0,6 L9,3 z" fill="var(--color-muted)" />
                </marker>
                
                // Gradient for active connections
                <linearGradient id="connection-gradient" x1="0%" y1="0%" x2="100%" y2="0%">
                    <stop offset="0%" stop-color="var(--color-accent)" />
                    <stop offset="100%" stop-color="var(--color-source)" />
                </linearGradient>
            </defs>
            
            // Render existing connections
            {
                let app_state = app_state.clone();
                move || {
                    let app_state = app_state.clone();
                    let pipeline = app_state.pipeline.get();
                    let hovered = hovered_connection.get();
                    
                    pipeline.connections.iter().map(|conn| {
                        let conn = conn.clone();
                        let conn_id = conn.id.clone();
                        let conn_id2 = conn.id.clone();
                        let _conn_id3 = conn.id.clone();
                        let is_hovered = hovered.as_ref() == Some(&conn_id);
                        
                        // Get node positions
                        let from_node = pipeline.nodes.get(&conn.from_node);
                        let to_node = pipeline.nodes.get(&conn.to_node);
                        
                        if let (Some(from), Some(to)) = (from_node, to_node) {
                            let path = calculate_bezier_path(
                                from.position.x + NODE_WIDTH,
                                from.position.y + PORT_OFFSET_Y,
                                to.position.x,
                                to.position.y + PORT_OFFSET_Y,
                            );
                            
                            let app_state_delete = app_state.clone();
                            
                            view! {
                                <g class="connection-group">
                                    // Invisible wider path for easier interaction
                                    <path
                                        d=path.clone()
                                        fill="none"
                                        stroke="transparent"
                                        stroke-width="20"
                                        class="pointer-events-auto cursor-pointer"
                                        on:mouseenter=move |_| set_hovered_connection.set(Some(conn_id.clone()))
                                        on:mouseleave=move |_| set_hovered_connection.set(None)
                                        on:click=move |e| {
                                            e.stop_propagation();
                                            // Delete connection on click
                                            let mut pipeline = app_state_delete.pipeline.get();
                                            pipeline.disconnect(&conn_id2);
                                            app_state_delete.pipeline.set(pipeline);
                                        }
                                    />
                                    // Visible connection path
                                    <path
                                        d=path
                                        fill="none"
                                        stroke=if is_hovered { "var(--color-error)" } else { "var(--color-muted)" }
                                        stroke-width=if is_hovered { "3" } else { "2" }
                                        stroke-linecap="round"
                                        class="transition-all duration-150"
                                    />
                                    // Delete indicator when hovered
                                    {
                                        if is_hovered {
                                            let from_pos = pipeline.nodes.get(&conn.from_node).map(|n| n.position);
                                            let to_pos = pipeline.nodes.get(&conn.to_node).map(|n| n.position);
                                            if let (Some(from), Some(to)) = (from_pos, to_pos) {
                                                let mid_x = (from.x + NODE_WIDTH + to.x) / 2.0;
                                                let mid_y = (from.y + to.y) / 2.0 + PORT_OFFSET_Y;
                                                view! {
                                                    <g transform=format!("translate({}, {})", mid_x, mid_y)>
                                                        <circle r="12" fill="var(--color-error)" class="pointer-events-auto cursor-pointer" />
                                                        <text 
                                                            x="0" 
                                                            y="4" 
                                                            text-anchor="middle" 
                                                            fill="white" 
                                                            font-size="14"
                                                            font-weight="bold"
                                                        >"×"</text>
                                                    </g>
                                                }.into_view()
                                            } else {
                                                view! {}.into_view()
                                            }
                                        } else {
                                            view! {}.into_view()
                                        }
                                    }
                                </g>
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }).collect_view()
                }
            }
            
            // Draw connection being created
            {
                let app_state = app_state.clone();
                move || {
                    if let Some(draft) = drawing_connection.get() {
                        let pipeline = app_state.pipeline.get();
                        if let Some(from_node) = pipeline.nodes.get(&draft.from_node) {
                            let from_x = from_node.position.x + NODE_WIDTH;
                            let from_y = from_node.position.y + PORT_OFFSET_Y;
                            let to = mouse_pos.get();
                            
                            let path = calculate_bezier_path(from_x, from_y, to.x, to.y);
                            
                            view! {
                                <path
                                    d=path
                                    fill="none"
                                    stroke="var(--color-accent)"
                                    stroke-width="2"
                                    stroke-dasharray="5,5"
                                    stroke-linecap="round"
                                />
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    } else {
                        view! {}.into_view()
                    }
                }
            }
        </svg>
    }
}

/// Calculate bezier curve path between two points
fn calculate_bezier_path(x1: f64, y1: f64, x2: f64, y2: f64) -> String {
    // Control point offset based on horizontal distance
    let dx = (x2 - x1).abs();
    let offset = (dx / 2.0).clamp(50.0, 150.0);
    
    // Control points for smooth S-curve
    let cx1 = x1 + offset;
    let cy1 = y1;
    let cx2 = x2 - offset;
    let cy2 = y2;
    
    format!(
        "M {} {} C {} {}, {} {}, {} {}",
        x1, y1,
        cx1, cy1,
        cx2, cy2,
        x2, y2
    )
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
        "kafka" => NodeType::Source(
            SourceConfig::new("kafka")
                .with_option("bootstrap_servers", "localhost:9092")
                .with_option("topics", serde_json::json!(["events"]))
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
        "sample" => NodeType::Transform(
            vectorize_shared::TransformConfig::new("sample", vec![])
                .with_option("rate", 10)
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
        "kafka_sink" => NodeType::Sink(
            vectorize_shared::SinkConfig::new("kafka", vec![])
                .with_option("bootstrap_servers", "localhost:9092")
                .with_option("topic", "output")
        ),
        
        // Default to stdin source
        _ => NodeType::Source(SourceConfig::new(component_type)),
    };
    
    PipelineNodeData::new(component_type, node_type).with_position(position.x, position.y)
}

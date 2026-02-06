//! Bottom Panel Component
//!
//! Resizable panel at the bottom of the screen for data preview, logs, test results, etc.
//! Features:
//! - Drag handle for resizing
//! - Tab bar for switching between views
//! - Collapse/expand button
//! - Persists height in state
//! - Data Preview panel with toolbar (filter, search, pause, clear)

use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::MouseEvent;

use crate::components::pipeline::data_view::{DataView, ViewMode};
use crate::state::{AppState, BottomPanelTab};
use vectorize_shared::{NodeEvent, NodeType};

/// Minimum and maximum panel heights
const MIN_HEIGHT: f64 = 100.0;
const MAX_HEIGHT: f64 = 600.0;
const DEFAULT_HEIGHT: f64 = 256.0;

/// Component filter options
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum ComponentFilter {
    #[default]
    All,
    Sources,
    Transforms,
    Sinks,
}

/// Resizable bottom panel with tabs
#[component]
pub fn BottomPanel(
    /// Panel content for each tab
    children: Children,
) -> impl IntoView {
    let app_state = expect_context::<AppState>();
    
    // Dragging state
    let (is_dragging, set_is_dragging) = create_signal(false);
    let (drag_start_y, set_drag_start_y) = create_signal(0.0);
    let (drag_start_height, set_drag_start_height) = create_signal(0.0);
    
    // Handle drag start
    let on_drag_start = move |e: MouseEvent| {
        e.prevent_default();
        set_is_dragging.set(true);
        set_drag_start_y.set(e.client_y() as f64);
        set_drag_start_height.set(app_state.bottom_panel_height.get());
    };
    
    // Handle drag move (attached to window)
    create_effect(move |_| {
        if is_dragging.get() {
            let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                let delta = drag_start_y.get() - e.client_y() as f64;
                let new_height = (drag_start_height.get() + delta).clamp(MIN_HEIGHT, MAX_HEIGHT);
                app_state.bottom_panel_height.set(new_height);
            }) as Box<dyn FnMut(web_sys::MouseEvent)>);
            
            let window = web_sys::window().unwrap();
            window.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref()).unwrap();
            closure.forget();
        }
    });
    
    // Handle drag end (attached to window)
    create_effect(move |_| {
        if is_dragging.get() {
            let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
                set_is_dragging.set(false);
            }) as Box<dyn FnMut(web_sys::MouseEvent)>);
            
            let window = web_sys::window().unwrap();
            window.add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref()).unwrap();
            closure.forget();
        }
    });
    
    // Toggle collapsed state
    let toggle_collapsed = move |_| {
        if app_state.bottom_panel_height.get() < MIN_HEIGHT + 10.0 {
            app_state.bottom_panel_height.set(DEFAULT_HEIGHT);
        } else {
            app_state.bottom_panel_height.set(0.0);
        }
    };
    
    let is_collapsed = move || app_state.bottom_panel_height.get() < MIN_HEIGHT;
    
    // Store children in a StoredValue so it can be accessed from the reactive view
    let children = store_value(children());
    
    view! {
        <div 
            class="flex flex-col bg-theme-surface border-t border-theme-border flex-shrink-0"
            style=move || format!("height: {}px;", app_state.bottom_panel_height.get().max(0.0))
        >
            // Drag handle and tab bar
            <div class="flex items-center justify-between border-b border-theme-border bg-theme-bg flex-shrink-0">
                // Drag handle
                <div 
                    class="w-full h-1 cursor-ns-resize hover:bg-accent/50 transition-colors absolute -top-0.5 left-0 right-0"
                    on:mousedown=on_drag_start
                />
                
                // Tab bar
                <div class="flex items-center gap-1 px-2 h-9">
                    <TabButton
                        tab=BottomPanelTab::DataPreview
                        label="Data Preview"
                        current_tab=app_state.bottom_panel_tab
                        badge=use_event_count()
                    />
                    <TabButton
                        tab=BottomPanelTab::Logs
                        label="Logs"
                        current_tab=app_state.bottom_panel_tab
                    />
                    <TabButton
                        tab=BottomPanelTab::TestResults
                        label="Test Results"
                        current_tab=app_state.bottom_panel_tab
                    />
                </div>
                
                // Right side controls
                <div class="flex items-center gap-1 px-2">
                    // Collapse/expand button
                    <button
                        class="p-1 rounded hover:bg-theme-surface-hover text-theme-secondary hover:text-theme transition-colors"
                        on:click=toggle_collapsed
                        title=move || if is_collapsed() { "Expand panel" } else { "Collapse panel" }
                    >
                        {move || if is_collapsed() {
                            view! { <ChevronUpIcon class="w-4 h-4" /> }
                        } else {
                            view! { <ChevronDownIcon class="w-4 h-4" /> }
                        }}
                    </button>
                </div>
            </div>
            
            // Panel content
            <Show when=move || !is_collapsed()>
                <div class="flex-1 overflow-hidden">
                    {children.get_value()}
                </div>
            </Show>
        </div>
    }
}

/// Tab button component with optional badge
#[component]
fn TabButton(
    tab: BottomPanelTab,
    label: &'static str,
    current_tab: RwSignal<BottomPanelTab>,
    /// Optional badge count signal
    #[prop(optional, into)] badge: Option<Signal<usize>>,
) -> impl IntoView {
    let is_active = move || current_tab.get() == tab;
    
    view! {
        <button
            class=move || {
                let base = "px-3 py-1 text-sm font-medium rounded transition-colors flex items-center gap-1.5";
                if is_active() {
                    format!("{} bg-theme-surface text-theme", base)
                } else {
                    format!("{} text-theme-secondary hover:text-theme", base)
                }
            }
            on:click=move |_| current_tab.set(tab)
        >
            {label}
            {move || {
                if let Some(badge_signal) = badge {
                    let count = badge_signal.get();
                    if count > 0 {
                        view! {
                            <span class="px-1.5 py-0.5 text-xs font-medium rounded-full bg-accent/20 text-accent min-w-[1.25rem] text-center">
                                {if count > 999 { "999+".to_string() } else { count.to_string() }}
                            </span>
                        }.into_view()
                    } else {
                        view! {}.into_view()
                    }
                } else {
                    view! {}.into_view()
                }
            }}
        </button>
    }
}

/// Empty panel content - placeholder when no data
#[component]
pub fn EmptyPanelContent(
    /// Message to display
    message: &'static str,
) -> impl IntoView {
    view! {
        <div class="flex items-center justify-center h-full text-theme-muted">
            <div class="text-center">
                <p class="text-sm">{message}</p>
            </div>
        </div>
    }
}

// Icon components

#[component]
fn ChevronUpIcon(#[prop(default = "w-4 h-4")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="18 15 12 9 6 15" />
        </svg>
    }
}

#[component]
fn ChevronDownIcon(#[prop(default = "w-4 h-4")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="6 9 12 15 18 9" />
        </svg>
    }
}

// ============================================================================
// Data Preview Panel
// ============================================================================

/// Data Preview Panel with toolbar and event display
#[component]
pub fn DataPreviewPanel() -> impl IntoView {
    let app_state = expect_context::<AppState>();
    
    // Local state for the panel
    let is_paused = create_rw_signal(false);
    let component_filter = create_rw_signal(ComponentFilter::All);
    let search_query = create_rw_signal(String::new());
    let view_mode = create_rw_signal(ViewMode::Table);
    
    // Collect all events from all nodes, with filtering
    let all_events = create_memo(move |_| {
        let events_map = app_state.node_events.get();
        let pipeline = app_state.pipeline.get();
        let filter = component_filter.get();
        
        let mut all: Vec<(String, NodeEvent)> = Vec::new();
        
        for (node_id, events) in events_map.iter() {
            // Check if node matches the filter
            let matches_filter = match filter {
                ComponentFilter::All => true,
                ComponentFilter::Sources => {
                    pipeline.nodes.get(node_id)
                        .map(|n| matches!(n.node_type, NodeType::Source(_)))
                        .unwrap_or(false)
                }
                ComponentFilter::Transforms => {
                    pipeline.nodes.get(node_id)
                        .map(|n| matches!(n.node_type, NodeType::Transform(_)))
                        .unwrap_or(false)
                }
                ComponentFilter::Sinks => {
                    pipeline.nodes.get(node_id)
                        .map(|n| matches!(n.node_type, NodeType::Sink(_)))
                        .unwrap_or(false)
                }
            };
            
            if matches_filter {
                for event in events {
                    all.push((node_id.clone(), event.clone()));
                }
            }
        }
        
        // Sort by timestamp (most recent last)
        all.sort_by(|a, b| a.1.timestamp.cmp(&b.1.timestamp));
        
        // Apply search filter
        let search = search_query.get().to_lowercase();
        if !search.is_empty() {
            all.retain(|(_, event)| {
                let json_str = serde_json::to_string(&event.data).unwrap_or_default().to_lowercase();
                json_str.contains(&search)
            });
        }
        
        // Return just the events (without node_id for now)
        all.into_iter().map(|(_, e)| e).collect::<Vec<_>>()
    });
    
    // Create memo for DataView
    let events_memo = create_memo(move |_| all_events.get());
    
    // Get available components for the filter dropdown
    let components = create_memo(move |_| {
        let pipeline = app_state.pipeline.get();
        let mut sources = Vec::new();
        let mut transforms = Vec::new();
        let mut sinks = Vec::new();
        
        for (id, node) in pipeline.nodes.iter() {
            match &node.node_type {
                NodeType::Source(_) => sources.push(id.clone()),
                NodeType::Transform(_) => transforms.push(id.clone()),
                NodeType::Sink(_) => sinks.push(id.clone()),
            }
        }
        
        (sources, transforms, sinks)
    });
    
    // Clear all events
    let clear_events = move |_| {
        app_state.node_events.set(std::collections::HashMap::new());
    };
    
    view! {
        <div class="flex flex-col h-full">
            // Toolbar
            <div class="flex items-center gap-3 px-4 py-2 border-b border-theme-border bg-theme-bg flex-shrink-0">
                // Component filter dropdown
                <div class="flex items-center gap-2">
                    <label class="text-xs text-theme-muted">"Filter:"</label>
                    <select
                        class="px-2 py-1 text-xs rounded bg-theme-surface border border-theme-border text-theme focus:outline-none focus:ring-1 focus:ring-accent"
                        on:change=move |e| {
                            let value = event_target_value(&e);
                            let filter = match value.as_str() {
                                "sources" => ComponentFilter::Sources,
                                "transforms" => ComponentFilter::Transforms,
                                "sinks" => ComponentFilter::Sinks,
                                _ => ComponentFilter::All,
                            };
                            component_filter.set(filter);
                        }
                    >
                        <option value="all" selected=move || component_filter.get() == ComponentFilter::All>"All Components"</option>
                        <option value="sources" selected=move || component_filter.get() == ComponentFilter::Sources>
                            {move || {
                                let (sources, _, _) = components.get();
                                format!("Sources ({})", sources.len())
                            }}
                        </option>
                        <option value="transforms" selected=move || component_filter.get() == ComponentFilter::Transforms>
                            {move || {
                                let (_, transforms, _) = components.get();
                                format!("Transforms ({})", transforms.len())
                            }}
                        </option>
                        <option value="sinks" selected=move || component_filter.get() == ComponentFilter::Sinks>
                            {move || {
                                let (_, _, sinks) = components.get();
                                format!("Sinks ({})", sinks.len())
                            }}
                        </option>
                    </select>
                </div>
                
                // Search input
                <div class="flex-1 max-w-xs">
                    <div class="relative">
                        <SearchIcon class="absolute left-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-theme-muted" />
                        <input
                            type="text"
                            placeholder="Search events..."
                            class="w-full pl-7 pr-3 py-1 text-xs rounded bg-theme-surface border border-theme-border text-theme placeholder:text-theme-muted focus:outline-none focus:ring-1 focus:ring-accent"
                            prop:value=move || search_query.get()
                            on:input=move |e| search_query.set(event_target_value(&e))
                        />
                    </div>
                </div>
                
                // Spacer
                <div class="flex-1" />
                
                // Pause/Resume button
                <button
                    class=move || format!(
                        "flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium rounded transition-colors {}",
                        if is_paused.get() {
                            "bg-yellow-500/20 text-yellow-400 hover:bg-yellow-500/30"
                        } else {
                            "bg-theme-surface hover:bg-theme-surface-hover text-theme-secondary"
                        }
                    )
                    on:click=move |_| is_paused.update(|p| *p = !*p)
                    title=move || if is_paused.get() { "Resume" } else { "Pause" }
                >
                    {move || if is_paused.get() {
                        view! { <PlayIcon class="w-3.5 h-3.5" /> }.into_view()
                    } else {
                        view! { <PauseIcon class="w-3.5 h-3.5" /> }.into_view()
                    }}
                    {move || if is_paused.get() { "Resume" } else { "Pause" }}
                </button>
                
                // Clear button
                <button
                    class="flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium rounded bg-theme-surface hover:bg-theme-surface-hover text-theme-secondary transition-colors"
                    on:click=clear_events
                    title="Clear all events"
                >
                    <TrashIcon class="w-3.5 h-3.5" />
                    "Clear"
                </button>
            </div>
            
            // Data view content
            <div class="flex-1 p-4 overflow-hidden min-h-0">
                <DataView
                    events=events_memo
                    title=None
                    empty_message="No events captured. Connect to Vector and run your pipeline to see events."
                    view_mode=view_mode
                    search_query=Signal::from(search_query)
                    is_paused=Signal::from(is_paused)
                    show_toggle=true
                />
            </div>
        </div>
    }
}

/// Get the event count signal for the Data Preview badge
pub fn use_event_count() -> Signal<usize> {
    let app_state = expect_context::<AppState>();
    Signal::derive(move || {
        app_state.node_events.get().values().map(|v| v.len()).sum::<usize>()
    })
}

// Additional icons for the toolbar

#[component]
fn SearchIcon(#[prop(default = "w-4 h-4")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8" />
            <path d="m21 21-4.35-4.35" />
        </svg>
    }
}

#[component]
fn PauseIcon(#[prop(default = "w-4 h-4")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="6" y="4" width="4" height="16" />
            <rect x="14" y="4" width="4" height="16" />
        </svg>
    }
}

#[component]
fn PlayIcon(#[prop(default = "w-4 h-4")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polygon points="5 3 19 12 5 21 5 3" />
        </svg>
    }
}

#[component]
fn TrashIcon(#[prop(default = "w-4 h-4")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="3 6 5 6 21 6" />
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
        </svg>
    }
}

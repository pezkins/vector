//! Pipeline View - Main pipeline builder page
//!
//! n8n-style visual pipeline builder with drag-and-drop canvas,
//! visual connections, and real-time data preview.

use leptos::*;
use std::rc::Rc;
use std::cell::RefCell;
use vectorize_shared::NodeType;

use super::{ComponentPalette, ConfigPanel, PipelineCanvas};
use crate::client::{SubscriptionClient, SubscriptionHandle};
use crate::components::common::*;
use crate::state::AppState;

/// Main pipeline builder view
#[component]
pub fn PipelineView() -> impl IntoView {
    let app_state = expect_context::<AppState>();
    
    // Calculate node and connection counts
    let node_count = create_memo(move |_| app_state.pipeline.get().nodes.len());
    let connection_count = create_memo(move |_| app_state.pipeline.get().connections.len());
    
    view! {
        // Use flexbox with Tailwind classes - 3 column layout
        <div class="flex-1 flex overflow-hidden">
            // Left sidebar - Component palette (fixed 280px width)
            <aside class="w-72 flex-shrink-0 border-r border-slate-700 flex flex-col bg-slate-800/50">
                <div class="p-4 border-b border-slate-700 flex-shrink-0">
                    <h2 class="text-sm font-semibold text-slate-400 uppercase tracking-wide flex items-center gap-2">
                        <ComponentsIcon class="w-4 h-4" />
                        "Components"
                    </h2>
                </div>
                <div class="flex-1 overflow-y-auto custom-scrollbar">
                    <ComponentPalette />
                </div>
            </aside>
            
            // Main canvas area - flexible middle
            <div class="flex-1 flex flex-col min-w-0 overflow-hidden">
                // Toolbar
                <div class="h-14 border-b border-slate-700 flex items-center px-4 gap-3 bg-slate-800/30 flex-shrink-0">
                    // Clear canvas
                    <button 
                        class="btn-secondary flex items-center gap-2"
                        on:click=move |_| {
                            app_state.pipeline.set(vectorize_shared::Pipeline::new());
                            app_state.selected_node.set(None);
                        }
                    >
                        <TrashIcon class="w-4 h-4" />
                        "Clear Canvas"
                    </button>
                    
                    <div class="flex-1" />
                    
                    // Pipeline stats
                    <div class="flex items-center gap-4 text-sm text-slate-400">
                        <span class="flex items-center gap-1.5">
                            <NodeIcon class="w-4 h-4" />
                            {move || format!("{} nodes", node_count.get())}
                        </span>
                        <span class="flex items-center gap-1.5">
                            <ConnectionIcon class="w-4 h-4" />
                            {move || format!("{} connections", connection_count.get())}
                        </span>
                    </div>
                    
                    // View TOML button
                    <button class="btn-ghost text-sm flex items-center gap-2">
                        <CodeIcon class="w-4 h-4" />
                        "View TOML"
                    </button>
                </div>
                
                // Canvas (top section - takes remaining space)
                <div class="flex-1 relative overflow-hidden min-h-0">
                    <PipelineCanvas />
                </div>
                
                // Data Preview Panel (bottom section - fixed 256px height)
                <DataPreviewPanel />
            </div>
            
            // Right sidebar - Configuration Panel (fixed 320px width)
            <div class="w-80 flex-shrink-0">
                <ConfigPanel />
            </div>
        </div>
    }
}

// Additional icons

#[component]
fn ComponentsIcon(#[prop(optional)] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6.429 9.75L2.25 12l4.179 2.25m0-4.5l5.571 3 5.571-3m-11.142 0L2.25 7.5 12 2.25l9.75 5.25-4.179 2.25m0 0L21.75 12l-4.179 2.25m0 0l4.179 2.25L12 21.75 2.25 16.5l4.179-2.25m11.142 0l-5.571 3-5.571-3" />
        </svg>
    }
}

#[component]
fn CheckIcon(#[prop(optional)] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" d="M4.5 12.75l6 6 9-13.5" />
        </svg>
    }
}

#[component]
fn XIcon(#[prop(optional)] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
        </svg>
    }
}

#[component]
fn NodeIcon(#[prop(optional)] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" d="M21 7.5l-9-5.25L3 7.5m18 0l-9 5.25m9-5.25v9l-9 5.25M3 7.5l9 5.25M3 7.5v9l9 5.25m0-9v9" />
        </svg>
    }
}

#[component]
fn ConnectionIcon(#[prop(optional)] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" d="M13.19 8.688a4.5 4.5 0 011.242 7.244l-4.5 4.5a4.5 4.5 0 01-6.364-6.364l1.757-1.757m13.35-.622l1.757-1.757a4.5 4.5 0 00-6.364-6.364l-4.5 4.5a4.5 4.5 0 001.242 7.244" />
        </svg>
    }
}

#[component]
fn CodeIcon(#[prop(optional)] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" d="M17.25 6.75L22.5 12l-5.25 5.25m-10.5 0L1.5 12l5.25-5.25m7.5-3l-4.5 16.5" />
        </svg>
    }
}

/// Bottom panel showing live data preview for selected component
#[component]
fn DataPreviewPanel() -> impl IntoView {
    let app_state = expect_context::<AppState>();
    // Separate signals for input and output events
    let (input_events, set_input_events) = create_signal::<Vec<serde_json::Value>>(Vec::new());
    let (output_events, set_output_events) = create_signal::<Vec<serde_json::Value>>(Vec::new());
    let (streaming, set_streaming) = create_signal(false);
    let (last_event_time, set_last_event_time) = create_signal(String::new());
    let (is_collapsed, set_is_collapsed) = create_signal(false);
    let (connection_status, set_connection_status) = create_signal("Disconnected".to_string());
    
    // Divider position - stored here so it persists across refreshes
    let (_divider_position, _set_divider_position) = create_signal(60.0_f64);
    
    // Store the subscription handle so we can cancel it
    let subscription_handle: Rc<RefCell<Option<SubscriptionHandle>>> = Rc::new(RefCell::new(None));
    
    // Get selected node info
    let selected_node = create_memo(move |_| {
        let selected_id = app_state.selected_node.get();
        let pipeline = app_state.pipeline.get();
        selected_id.and_then(|id| pipeline.nodes.get(&id).cloned())
    });
    
    // Determine component type for different display layouts
    let is_transform = create_memo(move |_| {
        selected_node.get().map(|n| {
            matches!(n.node_type, NodeType::Transform(_))
        }).unwrap_or(false)
    });
    
    let is_sink = create_memo(move |_| {
        selected_node.get().map(|n| {
            matches!(n.node_type, NodeType::Sink(_))
        }).unwrap_or(false)
    });
    
    // Combined check for subscription logic (transforms and sinks both need input events)
    let is_transform_or_sink = create_memo(move |_| {
        is_transform.get() || is_sink.get()
    });
    
    // Start/stop streaming based on selected node
    let subscription_handle_effect = subscription_handle.clone();
    create_effect(move |prev_node_id: Option<Option<String>>| {
        let current_node = selected_node.get();
        let current_node_id = current_node.as_ref().map(|n| n.id.clone());
        
        // Only restart subscription if node changed
        if prev_node_id.is_some() && prev_node_id.as_ref() == Some(&current_node_id) {
            return current_node_id;
        }
        
        // Cancel any existing subscription
        if let Some(handle) = subscription_handle_effect.borrow_mut().take() {
            handle.cancel();
        }
        
        // Clear events when node changes
        set_input_events.set(Vec::new());
        set_output_events.set(Vec::new());
        
        if let Some(node) = current_node {
            // Get the Vector API URL - we need to connect directly to Vector for WebSocket
            // The app_state.url is the Vectorize proxy URL, but WebSockets need direct connection
            let proxy_url = app_state.url.get();
            if proxy_url.is_empty() {
                set_connection_status.set("Not connected".to_string());
                return current_node_id;
            }
            
            // Convert proxy URL to Vector's direct GraphQL URL
            // http://127.0.0.1:8080/api -> http://127.0.0.1:8686
            let vector_url = proxy_url
                .replace("/api", "")
                .replace(":8080", ":8686");
            
            // Get the component name for patterns
            let component_name = node.name.clone();
            
            set_connection_status.set("Connecting...".to_string());
            set_streaming.set(true);
            
            // Create subscription client with Vector's direct URL
            let client = SubscriptionClient::new(&vector_url);
            
            // Create callback for events - sorts into input or output based on componentKind
            let set_input_events_cb = set_input_events;
            let set_output_events_cb = set_output_events;
            let set_last_event_time_cb = set_last_event_time;
            let set_connection_status_cb = set_connection_status;
            let node_name_for_cb = component_name.clone();
            
            let callback = Rc::new(move |event: serde_json::Value| {
                // Get the componentId and componentKind from the event
                let component_id = event.get("componentId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let component_kind = event.get("componentKind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
                // Determine if this is an input or output event based on componentId
                // Output events have the same componentId as the selected node
                // Input events come from upstream components (sources feeding into transforms)
                let is_output = component_id == node_name_for_cb;
                
                // DEBUG: Log every event received for debugging input/output classification
                web_sys::console::log_1(&format!(
                    "EVENT: componentId='{}', componentKind='{}', selected='{}', is_output={}",
                    component_id, component_kind, node_name_for_cb, is_output
                ).into());
                
                if is_output || component_kind == "sink" {
                    // This is an output event from the selected component
                    set_output_events_cb.update(|events| {
                        events.push(event.clone());
                        if events.len() > 100 {
                            events.remove(0);
                        }
                    });
                } else {
                    // This is an input event (from upstream source/transform)
                    set_input_events_cb.update(|events| {
                        events.push(event.clone());
                        if events.len() > 100 {
                            events.remove(0);
                        }
                    });
                }
                
                // Update timestamp
                let now = js_sys::Date::new_0();
                let time_str = format!("{:02}:{:02}:{:02}", 
                    now.get_hours(), 
                    now.get_minutes(), 
                    now.get_seconds()
                );
                set_last_event_time_cb.set(time_str);
                set_connection_status_cb.set("Streaming".to_string());
            });
            
            // For transforms/sinks, subscribe to both input and output events
            // For sources, just subscribe to output events
            let is_transform = matches!(node.node_type, NodeType::Transform(_) | NodeType::Sink(_));
            
            if is_transform {
                // Find the input components (sources that feed into this transform)
                let pipeline = app_state.pipeline.get();
                let input_component_names: Vec<String> = pipeline.connections.iter()
                    .filter(|c| c.to_node == node.id)
                    .filter_map(|c| pipeline.nodes.get(&c.from_node).map(|n| n.name.clone()))
                    .collect();
                
                web_sys::console::log_1(&format!(
                    "Transform '{}' - subscribing to inputs: {:?}, outputs: [{}]",
                    component_name, input_component_names, component_name
                ).into());
                
                // FIXED: Subscribe to BOTH the selected component AND its inputs in outputsPatterns
                // Vector's inputsPatterns doesn't return events from input components - it only filters
                // We need to include input components in outputsPatterns to get their events
                let mut all_patterns = vec![component_name.clone()];
                all_patterns.extend(input_component_names.clone());
                
                web_sys::console::log_1(&format!(
                    "Subscribing to outputsPatterns: {:?}",
                    all_patterns
                ).into());
                
                let handle = client.subscribe_output_events(
                    all_patterns,  // Subscribe to both this component AND its inputs
                    callback,
                );
                
                *subscription_handle_effect.borrow_mut() = Some(handle);
            } else {
                // Source - just subscribe to output events
                web_sys::console::log_1(&format!("Source '{}' - subscribing to output only", component_name).into());
                
                let handle = client.subscribe_output_events(
                    vec![component_name],
                    callback,
                );
                
                *subscription_handle_effect.borrow_mut() = Some(handle);
            }
        } else {
            set_streaming.set(false);
            set_connection_status.set("No component selected".to_string());
        }
        
        current_node_id
    });
    
    // Cleanup subscription on unmount
    let subscription_handle_cleanup = subscription_handle.clone();
    on_cleanup(move || {
        if let Some(handle) = subscription_handle_cleanup.borrow_mut().take() {
            handle.cancel();
        }
    });
    
    // Manual clear events
    let clear_events = move |_| {
        set_input_events.set(Vec::new());
        set_output_events.set(Vec::new());
    };
    
    // Combined events for sources (no input/output split)
    let _combined_events = create_memo(move |_| {
        // Both branches return output_events - this is intentional for now
        // as we display the same events regardless of component type
        output_events.get()
    });
    
    view! {
        // Fixed height panel - NEVER grows beyond 256px
        <div 
            class="border-t border-slate-700 bg-slate-800/80 flex flex-col flex-shrink-0"
            style=move || if is_collapsed.get() { 
                "height: 40px; min-height: 40px; max-height: 40px; overflow: hidden;" 
            } else { 
                "height: 256px; min-height: 256px; max-height: 256px; overflow: hidden;" 
            }
        >
            // Header
            <div class="h-10 flex items-center justify-between px-4 border-b border-slate-700 bg-slate-800 flex-shrink-0">
                <div class="flex items-center gap-3">
                    <button
                        class="text-slate-400 hover:text-white transition-colors"
                        on:click=move |_| set_is_collapsed.update(|v| *v = !*v)
                    >
                        <ChevronIcon class="w-4 h-4" rotated=is_collapsed />
                    </button>
                    <h3 class="text-sm font-semibold text-slate-300 flex items-center gap-2">
                        <DataIcon class="w-4 h-4 text-blue-400" />
                        "Data Preview"
                    </h3>
                    {move || selected_node.get().map(|node| view! {
                        <span class="text-xs text-slate-500">
                            " — "
                            <span class="text-slate-400">{node.name}</span>
                        </span>
                    })}
                </div>
                
                <div class="flex items-center gap-3">
                    // Connection status indicator
                    <div class="flex items-center gap-2">
                        <div class=move || format!(
                            "w-2 h-2 rounded-full {}",
                            if streaming.get() && connection_status.get() == "Streaming" {
                                "bg-green-500 animate-pulse"
                            } else if connection_status.get() == "Connecting..." {
                                "bg-yellow-500 animate-pulse"
                            } else {
                                "bg-slate-500"
                            }
                        ) />
                        <span class="text-xs text-slate-400">
                            {move || connection_status.get()}
                        </span>
                    </div>
                    
                    // Last event time
                    {move || {
                        let time = last_event_time.get();
                        if !time.is_empty() {
                            Some(view! {
                                <span class="text-xs text-slate-500">
                                    "Last event: " {time}
                                </span>
                            })
                        } else {
                            None
                        }
                    }}
                    
                    // Event count
                    <span class="text-xs text-slate-500">
                        {move || {
                            let input_count = input_events.get().len();
                            let output_count = output_events.get().len();
                            if is_transform_or_sink.get() {
                                format!("{} in / {} out", input_count, output_count)
                            } else {
                                format!("{} events", output_count)
                            }
                        }}
                    </span>
                    
                    // Clear events button
                    <button
                        class="px-3 py-1 rounded text-xs bg-slate-700 hover:bg-slate-600 text-slate-300 transition-colors flex items-center gap-1"
                        on:click=clear_events
                    >
                        <TrashIcon class="w-3 h-3" />
                        "Clear"
                    </button>
                </div>
            </div>
            
            // Content (when not collapsed)
            <Show when=move || !is_collapsed.get()>
                <div class="flex-1 flex min-h-0 overflow-hidden">
                    {move || {
                        if selected_node.get().is_none() {
                            // No node selected - show placeholder
                            view! {
                                <div class="flex-1 flex items-center justify-center text-slate-500">
                                    <div class="text-center">
                                        <DataIcon class="w-12 h-12 mx-auto mb-3 opacity-30" />
                                        <p class="text-sm">"Select a component to preview its data"</p>
                                    </div>
                                </div>
                            }.into_view()
                        } else if is_sink.get() {
                            // Sinks: show input and output events only (no parsed fields)
                            view! {
                                <SinkPreviewPanels 
                                    input_events=input_events
                                    output_events=output_events
                                />
                            }.into_view()
                        } else if is_transform.get() {
                            // Transforms: show input events, output events, AND parsed fields
                            view! {
                                <TransformPreviewPanels 
                                    input_events=input_events
                                    output_events=output_events
                                />
                            }.into_view()
                        } else {
                            // Sources: show output events with parsed fields
                            let current_events = output_events.get();
                            view! {
                                <ResizablePanels 
                                    events=output_events 
                                    current_events_len=current_events.len() 
                                />
                            }.into_view()
                        }
                    }}
                </div>
            </Show>
        </div>
    }
}

/// Split view for transforms showing Input Events, Output Events, and Parsed Fields (3 columns)
#[component]
fn TransformPreviewPanels(
    input_events: ReadSignal<Vec<serde_json::Value>>,
    output_events: ReadSignal<Vec<serde_json::Value>>,
) -> impl IntoView {
    view! {
        // Container with strict overflow control - never grows
        <div class="flex-1 flex" style="min-height: 0; overflow: hidden;">
            // Input Events (left - 35%)
            <div class="flex flex-col border-r border-slate-700" style="width: 35%; min-width: 0; overflow: hidden;">
                <div class="px-3 py-2 bg-slate-800/50 border-b border-slate-700 flex-shrink-0">
                    <div class="flex items-center gap-2">
                        <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">
                            "Input Events"
                        </span>
                        <span class="text-xs text-slate-500">
                            "(" {move || input_events.get().len()} ")"
                        </span>
                    </div>
                </div>
                <div class="flex-1 p-3 custom-scrollbar" style="min-height: 0; overflow-x: hidden; overflow-y: auto;">
                    <Show
                        when=move || !input_events.get().is_empty()
                        fallback=|| view! {
                            <div class="text-center text-slate-500 py-4">
                                <NoInputIcon class="w-8 h-8 mx-auto mb-2 opacity-50" />
                                <p class="text-sm">"Waiting for input events..."</p>
                                <p class="text-xs mt-1">"Events from upstream will appear here"</p>
                            </div>
                        }
                    >
                        <div class="space-y-2">
                            <For
                                each=move || {
                                    let mut evts = input_events.get();
                                    evts.reverse();
                                    evts
                                }
                                key=|item| serde_json::to_string(item).unwrap_or_default()
                                children=move |event| {
                                    view! { <EventRow event=event /> }
                                }
                            />
                        </div>
                    </Show>
                </div>
            </div>
            
            // Output Events (middle - 35%)
            <div class="flex flex-col border-r border-slate-700" style="width: 35%; min-width: 0; overflow: hidden;">
                <div class="px-3 py-2 bg-slate-800/50 border-b border-slate-700 flex-shrink-0">
                    <div class="flex items-center gap-2">
                        <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">
                            "Output Events"
                        </span>
                        <span class="text-xs text-slate-500">
                            "(" {move || output_events.get().len()} ")"
                        </span>
                    </div>
                </div>
                <div class="flex-1 p-3 custom-scrollbar" style="min-height: 0; overflow-x: hidden; overflow-y: auto;">
                    <Show
                        when=move || !output_events.get().is_empty()
                        fallback=|| view! {
                            <div class="text-center text-slate-500 py-4">
                                <FilteredIcon class="w-8 h-8 mx-auto mb-2 opacity-50" />
                                <p class="text-sm">"No output events yet"</p>
                                <p class="text-xs mt-1">"Processed events will appear here"</p>
                            </div>
                        }
                    >
                        <div class="space-y-2">
                            <For
                                each=move || {
                                    let mut evts = output_events.get();
                                    evts.reverse();
                                    evts
                                }
                                key=|item| serde_json::to_string(item).unwrap_or_default()
                                children=move |event| {
                                    view! { <EventRow event=event /> }
                                }
                            />
                        </div>
                    </Show>
                </div>
            </div>
            
            // Parsed Fields (right - 30%)
            <div class="flex flex-col bg-slate-900/50" style="width: 30%; min-width: 0; overflow: hidden;">
                <div class="px-3 py-2 bg-slate-800/50 border-b border-slate-700 flex-shrink-0">
                    <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">
                        "Parsed Fields"
                    </span>
                </div>
                <div class="flex-1 p-3 custom-scrollbar" style="min-height: 0; overflow-x: hidden; overflow-y: auto;">
                    <ParsedFieldsView events=output_events />
                </div>
            </div>
        </div>
    }
}

/// Split view for sinks showing Input and Output events side by side (no parsed fields)
#[component]
fn SinkPreviewPanels(
    input_events: ReadSignal<Vec<serde_json::Value>>,
    output_events: ReadSignal<Vec<serde_json::Value>>,
) -> impl IntoView {
    view! {
        // Container with strict overflow control - never grows
        <div class="flex-1 flex" style="min-height: 0; overflow: hidden;">
            // Input Events (left side - 50%)
            <div class="flex flex-col border-r border-slate-700" style="width: 50%; min-width: 0; overflow: hidden;">
                <div class="px-3 py-2 bg-slate-800/50 border-b border-slate-700 flex-shrink-0">
                    <div class="flex items-center gap-2">
                        <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">
                            "Input Events"
                        </span>
                        <span class="text-xs text-slate-500">
                            "(" {move || input_events.get().len()} ")"
                        </span>
                    </div>
                </div>
                <div class="flex-1 p-3 custom-scrollbar" style="min-height: 0; overflow-x: hidden; overflow-y: auto;">
                    <Show
                        when=move || !input_events.get().is_empty()
                        fallback=|| view! {
                            <div class="text-center text-slate-500 py-4">
                                <NoInputIcon class="w-8 h-8 mx-auto mb-2 opacity-50" />
                                <p class="text-sm">"Waiting for input events..."</p>
                                <p class="text-xs mt-1">"Events from upstream will appear here"</p>
                            </div>
                        }
                    >
                        <div class="space-y-2">
                            <For
                                each=move || {
                                    let mut evts = input_events.get();
                                    evts.reverse();
                                    evts
                                }
                                key=|item| serde_json::to_string(item).unwrap_or_default()
                                children=move |event| {
                                    view! { <EventRow event=event /> }
                                }
                            />
                        </div>
                    </Show>
                </div>
            </div>
            
            // Output Events (right side - 50%)
            <div class="flex flex-col" style="width: 50%; min-width: 0; overflow: hidden;">
                <div class="px-3 py-2 bg-slate-800/50 border-b border-slate-700 flex-shrink-0">
                    <div class="flex items-center gap-2">
                        <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">
                            "Output Events"
                        </span>
                        <span class="text-xs text-slate-500">
                            "(" {move || output_events.get().len()} ")"
                        </span>
                    </div>
                </div>
                <div class="flex-1 p-3 custom-scrollbar" style="min-height: 0; overflow-x: hidden; overflow-y: auto;">
                    <Show
                        when=move || !output_events.get().is_empty()
                        fallback=|| view! {
                            <div class="text-center text-slate-500 py-4">
                                <FilteredIcon class="w-8 h-8 mx-auto mb-2 opacity-50" />
                                <p class="text-sm">"No output events yet"</p>
                                <p class="text-xs mt-1">"Events sent to sink will appear here"</p>
                            </div>
                        }
                    >
                        <div class="space-y-2">
                            <For
                                each=move || {
                                    let mut evts = output_events.get();
                                    evts.reverse();
                                    evts
                                }
                                key=|item| serde_json::to_string(item).unwrap_or_default()
                                children=move |event| {
                                    view! { <EventRow event=event /> }
                                }
                            />
                        </div>
                    </Show>
                </div>
            </div>
        </div>
    }
}

#[component]
fn ArrowRightIcon(#[prop(optional)] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" d="M13.5 4.5L21 12m0 0l-7.5 7.5M21 12H3" />
        </svg>
    }
}

#[component]
fn FilteredIcon(#[prop(optional)] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 3c2.755 0 5.455.232 8.083.678.533.09.917.556.917 1.096v1.044a2.25 2.25 0 01-.659 1.591l-5.432 5.432a2.25 2.25 0 00-.659 1.591v2.927a2.25 2.25 0 01-1.244 2.013L9.75 21v-6.568a2.25 2.25 0 00-.659-1.591L3.659 7.409A2.25 2.25 0 013 5.818V4.774c0-.54.384-1.006.917-1.096A48.32 48.32 0 0112 3z" />
        </svg>
    }
}

/// Simple fixed-width panels for events and parsed fields
#[component]
fn ResizablePanels(
    events: ReadSignal<Vec<serde_json::Value>>,
    current_events_len: usize,
) -> impl IntoView {
    view! {
        // Flex container that fills available space but never grows beyond it
        <div class="flex-1 flex" style="min-height: 0; overflow: hidden;">
            // Events list (left side - 70%) - scrollable
            <div style="width: 70%; min-width: 0; overflow-x: hidden; overflow-y: auto;" class="p-3 custom-scrollbar border-r border-slate-700">
                <div class="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">
                    "Output Events (" {current_events_len} ")"
                </div>
                <Show
                    when=move || !events.get().is_empty()
                    fallback=|| view! {
                        <div class="text-center text-slate-500 py-4">
                            <NoInputIcon class="w-8 h-8 mx-auto mb-2 opacity-50" />
                            <p class="text-sm">"No events yet"</p>
                        </div>
                    }
                >
                    <div class="space-y-1">
                        <For
                            each=move || {
                                let mut evts = events.get();
                                evts.reverse();
                                evts
                            }
                            key=|item| serde_json::to_string(item).unwrap_or_default()
                            children=move |event| {
                                view! { <EventRow event=event /> }
                            }
                        />
                    </div>
                </Show>
            </div>
            
            // Parsed fields (right side - 30%) - scrollable
            <div style="width: 30%; min-width: 0; overflow-y: auto;" class="p-3 bg-slate-900/50 custom-scrollbar">
                <div class="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">
                    "Parsed Fields"
                </div>
                <ParsedFieldsView events=events />
            </div>
        </div>
    }
}

/// Generate sample events based on node type and configuration
#[allow(dead_code)]
fn generate_sample_events(node_type: &NodeType, count: usize) -> Vec<serde_json::Value> {
    // Generate base events
    let base_events: Vec<serde_json::Value> = (0..count * 3).map(|i| {
        let levels = ["info", "debug", "warn", "error"];
        let level = levels[i % levels.len()];
        let messages = [
            "Application started successfully",
            "Processing incoming request",
            "Database connection established",
            "Cache hit for user session",
            "Request completed in 45ms",
            "Memory usage at 65%",
            "New user registration",
            "API rate limit check passed",
            "Background job completed",
            "Health check passed",
        ];
        serde_json::json!({
            "timestamp": format!("2026-01-31T17:{}:{:02}Z", 30 + (i / 60), i % 60),
            "message": messages[i % messages.len()],
            "level": level,
            "host": "localhost",
            "service": "demo-app",
            "pid": 12345 + i
        })
    }).collect();
    
    match node_type {
        NodeType::Source(c) if c.source_type == "demo_logs" => {
            base_events.into_iter().take(count).collect()
        }
        NodeType::Transform(c) if c.transform_type == "filter" => {
            // Get the filter condition from configuration
            let condition = c.options.get("condition")
                .and_then(|v| v.as_str())
                .unwrap_or(".level == \"error\"");
            
            // Parse and apply the filter condition
            let filtered: Vec<_> = base_events.into_iter().filter(|event| {
                // Simple parsing of common filter patterns
                if condition.contains(".level") {
                    // Extract the level being filtered for
                    if let Some(level_match) = extract_level_from_condition(condition) {
                        if let Some(event_level) = event.get("level").and_then(|v| v.as_str()) {
                            return event_level == level_match;
                        }
                    }
                }
                // Default: let through if we can't parse condition
                true
            }).take(count).collect();
            
            if filtered.is_empty() {
                // Return at least one event showing no matches
                vec![serde_json::json!({
                    "message": format!("No events match filter: {}", condition),
                    "timestamp": "2026-01-31T17:30:00Z",
                    "level": "info"
                })]
            } else {
                filtered
            }
        }
        NodeType::Transform(c) if c.transform_type == "remap" => {
            (0..count).map(|i| {
                serde_json::json!({
                    "ts": format!("2026-01-31T17:30:{:02}Z", i),
                    "msg": format!("Transformed event #{}", i + 1),
                    "severity": if i % 4 == 3 { "ERROR" } else if i % 4 == 2 { "WARN" } else { "INFO" },
                    "metadata": {
                        "transformed": true,
                        "original_field_count": 6,
                        "new_field_count": 4
                    }
                })
            }).collect()
        }
        NodeType::Sink(c) if c.sink_type == "console" => {
            (0..count).map(|i| {
                serde_json::json!({
                    "status": "delivered",
                    "timestamp": format!("2026-01-31T17:30:{:02}Z", i),
                    "bytes_written": 256 + i * 32,
                    "destination": "stdout"
                })
            }).collect()
        }
        _ => {
            base_events.into_iter().take(count).collect()
        }
    }
}

/// Extract the level value from a filter condition like ".level == \"error\""
#[allow(dead_code)]
fn extract_level_from_condition(condition: &str) -> Option<&str> {
    // Common patterns: .level == "error", .level == "info", etc.
    if condition.contains("\"error\"") {
        Some("error")
    } else if condition.contains("\"warn\"") || condition.contains("\"warning\"") {
        Some("warn")
    } else if condition.contains("\"info\"") {
        Some("info")
    } else if condition.contains("\"debug\"") {
        Some("debug")
    } else {
        None
    }
}


/// Single event row in the data preview - shows raw event data without modification
#[component]
fn EventRow(event: serde_json::Value) -> impl IntoView {
    // Extract timestamp
    let timestamp = event.get("timestamp")
        .and_then(|v| v.as_str())
        .map(|s| {
            // Extract just the time portion (HH:MM:SS) from ISO timestamp
            if let Some(t_pos) = s.find('T') {
                let time_part = &s[t_pos + 1..];
                time_part.chars().take(8).collect::<String>()
            } else {
                s.to_string()
            }
        })
        .unwrap_or_default();
    
    let component_id = event.get("componentId")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    
    // Get the raw message - show it as-is without reformatting
    let message_str = event.get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    
    // Truncate component ID for display
    let short_component = if component_id.len() > 12 {
        format!("{}...", &component_id[..8])
    } else {
        component_id.to_string()
    };
    
    view! {
        <div class="rounded px-2 py-1.5 font-mono text-xs bg-slate-800/50 flex items-start gap-2 hover:bg-slate-700/50 transition-colors">
            <span class="text-slate-500 flex-shrink-0 w-14 text-[10px]">{timestamp}</span>
            <span class="text-cyan-400 flex-shrink-0 w-16 text-[10px]" title=component_id.to_string()>
                {short_component}
            </span>
            // Allow message to wrap - use word-break for long strings
            <span class="text-slate-300 flex-1 min-w-0" style="word-break: break-word; overflow-wrap: anywhere;">{message_str.to_string()}</span>
        </div>
    }
}

/// Parsed fields view showing schema
#[component]
fn ParsedFieldsView(events: ReadSignal<Vec<serde_json::Value>>) -> impl IntoView {
    // Extract unique fields from all events
    let fields = create_memo(move |_| {
        let mut field_map: std::collections::HashMap<String, (String, usize)> = std::collections::HashMap::new();
        
        for event in events.get() {
            if let Some(obj) = event.as_object() {
                for (key, value) in obj {
                    let type_str = match value {
                        serde_json::Value::String(_) => "string",
                        serde_json::Value::Number(_) => "number",
                        serde_json::Value::Bool(_) => "boolean",
                        serde_json::Value::Array(_) => "array",
                        serde_json::Value::Object(_) => "object",
                        serde_json::Value::Null => "null",
                    };
                    
                    let entry = field_map.entry(key.clone()).or_insert((type_str.to_string(), 0));
                    entry.1 += 1;
                }
            }
        }
        
        let mut fields: Vec<_> = field_map.into_iter().collect();
        fields.sort_by(|a, b| b.1.1.cmp(&a.1.1)); // Sort by occurrence count
        fields
    });
    
    view! {
        <Show
            when=move || !fields.get().is_empty()
            fallback=|| view! {
                <div class="text-center text-slate-500 py-4">
                    <p class="text-xs">"No fields detected"</p>
                </div>
            }
        >
            <div class="space-y-1">
                <For
                    each=move || fields.get()
                    key=|(name, _)| name.clone()
                    children=move |(name, (type_str, count))| {
                        let type_color = match type_str.as_str() {
                            "string" => "text-green-400",
                            "number" => "text-blue-400",
                            "boolean" => "text-purple-400",
                            "array" => "text-orange-400",
                            "object" => "text-cyan-400",
                            _ => "text-slate-400",
                        };
                        
                        let total = events.get().len();
                        let percentage = if total > 0 { (count * 100) / total } else { 0 };
                        
                        view! {
                            <div class="flex items-center justify-between py-1.5 px-2 rounded hover:bg-slate-800/50 group">
                                <div class="flex items-center gap-2">
                                    <span class="font-mono text-xs text-slate-300">{name}</span>
                                </div>
                                <div class="flex items-center gap-2">
                                    <span class=format!("text-xs {}", type_color)>{type_str}</span>
                                    <span class="text-xs text-slate-500">{percentage}"%"</span>
                                </div>
                            </div>
                        }
                    }
                />
            </div>
        </Show>
    }
}

#[component]
fn ChevronIcon(#[prop(optional)] class: &'static str, #[prop(into)] rotated: Signal<bool>) -> impl IntoView {
    view! {
        <svg 
            class=move || format!("{} transition-transform duration-200 {}", class, if rotated.get() { "-rotate-90" } else { "rotate-0" })
            xmlns="http://www.w3.org/2000/svg" 
            fill="none" 
            viewBox="0 0 24 24" 
            stroke-width="2" 
            stroke="currentColor"
        >
            <path stroke-linecap="round" stroke-linejoin="round" d="M19 9l-7 7-7-7" />
        </svg>
    }
}

#[component]
fn DataIcon(#[prop(optional)] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
        </svg>
    }
}

#[component]
fn NoInputIcon(#[prop(optional)] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
        </svg>
    }
}

#[component]
fn RefreshIcon(#[prop(optional)] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
        </svg>
    }
}

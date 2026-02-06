//! Data View Component
//!
//! Displays event data in Table, JSON, Raw, or Schema format.
//! Used in the bottom panel for data preview and in the configuration panel.

use leptos::*;
use vectorize_shared::NodeEvent;

/// Maximum events to display (virtual scrolling)
const MAX_DISPLAYED_EVENTS: usize = 100;

/// View mode for data display
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum ViewMode {
    #[default]
    Table,
    Json,
    Raw,
    Schema,
}

/// Data view with Table/JSON/Raw/Schema toggle
#[component]
pub fn DataView(
    /// Events to display
    events: Memo<Vec<NodeEvent>>,
    /// Optional title (hidden when None)
    #[prop(default = None)] title: Option<&'static str>,
    /// Message when no data available
    #[prop(default = "No data available")] empty_message: &'static str,
    /// External view mode signal (if provided, syncs with internal state)
    #[prop(optional)] view_mode: Option<RwSignal<ViewMode>>,
    /// Search query to highlight (optional)
    #[prop(optional)] search_query: Option<Signal<String>>,
    /// Whether data collection is paused
    #[prop(optional)] is_paused: Option<Signal<bool>>,
    /// Show view mode toggle (default true)
    #[prop(default = true)] show_toggle: bool,
) -> impl IntoView {
    // Use external view_mode if provided, otherwise create local state
    let (internal_view_mode, set_internal_view_mode) = create_signal(ViewMode::Table);
    let effective_view_mode = view_mode.map(Signal::from).unwrap_or_else(|| Signal::from(internal_view_mode));
    let set_view_mode = move |mode: ViewMode| {
        if let Some(ext) = view_mode {
            ext.set(mode);
        } else {
            set_internal_view_mode.set(mode);
        }
    };
    
    let (selected_event, set_selected_event) = create_signal::<Option<usize>>(None);
    
    // Compute displayed events with virtual scrolling limit
    let displayed_events = create_memo(move |_| {
        let all_events = events.get();
        let total = all_events.len();
        if total > MAX_DISPLAYED_EVENTS {
            // Take the last MAX_DISPLAYED_EVENTS events (most recent)
            all_events.into_iter().skip(total - MAX_DISPLAYED_EVENTS).collect::<Vec<_>>()
        } else {
            all_events
        }
    });
    
    let total_count = create_memo(move |_| events.get().len());
    let displayed_count = create_memo(move |_| displayed_events.get().len());
    
    view! {
        <div class="flex flex-col h-full">
            // Header with view toggle
            <Show when=move || title.is_some() || show_toggle>
                <div class="flex items-center justify-between mb-3 flex-shrink-0">
                    {title.map(|t| view! {
                        <h4 class="text-xs font-semibold text-theme-muted uppercase tracking-wider">{t}</h4>
                    })}
                    
                    <Show when=move || show_toggle>
                        <div class="flex rounded-lg overflow-hidden border border-theme-border">
                            <ViewModeButton mode=ViewMode::Table label="Table" current=effective_view_mode on_click=set_view_mode />
                            <ViewModeButton mode=ViewMode::Json label="JSON" current=effective_view_mode on_click=set_view_mode />
                            <ViewModeButton mode=ViewMode::Raw label="Raw" current=effective_view_mode on_click=set_view_mode />
                            <ViewModeButton mode=ViewMode::Schema label="Schema" current=effective_view_mode on_click=set_view_mode />
                        </div>
                    </Show>
                </div>
            </Show>
            
            // Event count with virtual scroll indicator
            {move || {
                let total = total_count.get();
                let displayed = displayed_count.get();
                let paused = is_paused.map(|s| s.get()).unwrap_or(false);
                
                if total > 0 || paused {
                    view! {
                        <div class="flex items-center gap-2 text-xs text-theme-muted mb-2 flex-shrink-0">
                            {if displayed < total {
                                view! { <span>{format!("Showing {} of {} events", displayed, total)}</span> }.into_view()
                            } else if total > 0 {
                                view! { <span>{format!("{} events", total)}</span> }.into_view()
                            } else {
                                view! {}.into_view()
                            }}
                            {if paused {
                                view! {
                                    <span class="px-1.5 py-0.5 rounded bg-yellow-500/20 text-yellow-400 text-xs font-medium">
                                        "Paused"
                                    </span>
                                }.into_view()
                            } else {
                                view! {}.into_view()
                            }}
                        </div>
                    }.into_view()
                } else {
                    view! {}.into_view()
                }
            }}
            
            // Content
            <div class="flex-1 rounded-lg border border-theme-border bg-theme-surface-alt overflow-hidden min-h-0">
                {move || {
                    let data = displayed_events.get();
                    let search = search_query.map(|s| s.get()).unwrap_or_default();
                    
                    if data.is_empty() {
                        view! {
                            <EmptyState message=empty_message />
                        }.into_view()
                    } else {
                        match effective_view_mode.get() {
                            ViewMode::Table => {
                                view! { <TableView events=data selected=selected_event on_select=set_selected_event search=search.clone() /> }.into_view()
                            }
                            ViewMode::Json => {
                                view! { <JsonView events=data selected=selected_event.get() search=search.clone() /> }.into_view()
                            }
                            ViewMode::Raw => {
                                view! { <RawView events=data search=search.clone() /> }.into_view()
                            }
                            ViewMode::Schema => {
                                view! { <SchemaView events=data /> }.into_view()
                            }
                        }
                    }
                }}
            </div>
        </div>
    }
}

/// View mode toggle button
#[component]
fn ViewModeButton<F>(
    mode: ViewMode,
    label: &'static str,
    current: Signal<ViewMode>,
    on_click: F,
) -> impl IntoView
where
    F: Fn(ViewMode) + Copy + 'static,
{
    let is_active = move || current.get() == mode;
    
    view! {
        <button
            class=move || format!(
                "px-3 py-1 text-xs font-medium transition-colors {}",
                if is_active() { "bg-accent text-white" } else { "bg-theme-surface text-theme-muted hover:text-theme" }
            )
            on:click=move |_| on_click(mode)
        >
            {label}
        </button>
    }
}

/// Empty state component with theme classes
#[component]
fn EmptyState(message: &'static str) -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center h-full p-8 text-theme-muted">
            <svg class="w-12 h-12 mb-3 opacity-50" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <ellipse cx="12" cy="5" rx="9" ry="3" />
                <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3" />
                <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
            </svg>
            <p class="text-sm text-center">{message}</p>
        </div>
    }
}

/// Table view of events
#[component]
fn TableView(
    events: Vec<NodeEvent>,
    selected: ReadSignal<Option<usize>>,
    on_select: WriteSignal<Option<usize>>,
    #[prop(default = String::new())] search: String,
) -> impl IntoView {
    // Extract all unique keys from events
    let keys: Vec<String> = {
        let mut keys_set = std::collections::HashSet::new();
        for event in &events {
            if let serde_json::Value::Object(obj) = &event.data {
                for key in obj.keys() {
                    keys_set.insert(key.clone());
                }
            }
        }
        let mut keys: Vec<_> = keys_set.into_iter().collect();
        keys.sort();
        keys
    };
    
    let keys_header = keys.clone();
    let search_lower = search.to_lowercase();
    
    view! {
        <div class="overflow-auto h-full custom-scrollbar">
            <table class="w-full text-sm">
                <thead class="sticky top-0 bg-theme-surface text-theme-muted text-xs uppercase z-10">
                    <tr>
                        <th class="px-3 py-2 text-left font-medium">"#"</th>
                        {keys_header.iter().map(|key| {
                            let key = key.clone();
                            view! {
                                <th class="px-3 py-2 text-left font-medium whitespace-nowrap">{key}</th>
                            }
                        }).collect_view()}
                    </tr>
                </thead>
                <tbody class="divide-y divide-theme-border/50">
                    {events.iter().enumerate().map(|(idx, event)| {
                        let event_data = event.data.clone();
                        let is_selected = move || selected.get() == Some(idx);
                        let keys_row = keys.clone();
                        let search_lower = search_lower.clone();
                        
                        // Check if this row matches the search
                        let row_matches = if search_lower.is_empty() {
                            true
                        } else {
                            let json_str = serde_json::to_string(&event_data).unwrap_or_default().to_lowercase();
                            json_str.contains(&search_lower)
                        };
                        
                        // Skip non-matching rows when searching
                        if !row_matches {
                            return view! {}.into_view();
                        }
                        
                        view! {
                            <tr 
                                class=move || format!(
                                    "cursor-pointer transition-colors {}",
                                    if is_selected() { "bg-accent/20" } else { "hover:bg-theme-surface-hover" }
                                )
                                on:click=move |_| on_select.set(Some(idx))
                            >
                                <td class="px-3 py-2 text-theme-muted">{idx + 1}</td>
                                {keys_row.iter().map(|key| {
                                    let value = event_data.get(key)
                                        .map(format_value)
                                        .unwrap_or_else(|| "-".to_string());
                                    
                                    view! {
                                        <td class="px-3 py-2 text-theme-secondary truncate max-w-xs" title=value.clone()>
                                            {value}
                                        </td>
                                    }
                                }).collect_view()}
                            </tr>
                        }.into_view()
                    }).collect_view()}
                </tbody>
            </table>
        </div>
    }
}

/// JSON view of events
#[component]
fn JsonView(
    events: Vec<NodeEvent>,
    selected: Option<usize>,
    #[prop(default = String::new())] search: String,
) -> impl IntoView {
    let json_content = if let Some(idx) = selected {
        events.get(idx)
            .map(|e| serde_json::to_string_pretty(&e.data).unwrap_or_default())
            .unwrap_or_else(|| "Select an event to view".to_string())
    } else if let Some(first) = events.first() {
        serde_json::to_string_pretty(&first.data).unwrap_or_default()
    } else {
        "No data".to_string()
    };
    
    // Simple search highlight (wrap matching text in a span)
    let highlighted_content = if !search.is_empty() {
        json_content.replace(&search, &format!("【{}】", search))
    } else {
        json_content
    };
    
    view! {
        <div class="p-4 h-full overflow-auto custom-scrollbar">
            <pre class="text-xs font-mono text-theme-secondary whitespace-pre-wrap">
                {highlighted_content}
            </pre>
        </div>
    }
}

/// Raw view showing events as line-delimited JSON
#[component]
fn RawView(
    events: Vec<NodeEvent>,
    #[prop(default = String::new())] search: String,
) -> impl IntoView {
    let search_lower = search.to_lowercase();
    
    view! {
        <div class="p-4 h-full overflow-auto custom-scrollbar font-mono text-xs">
            {events.iter().enumerate().map(|(idx, event)| {
                let json_line = serde_json::to_string(&event.data).unwrap_or_else(|_| "{}".to_string());
                
                // Check if line matches search
                let matches_search = search_lower.is_empty() || json_line.to_lowercase().contains(&search_lower);
                
                if !matches_search {
                    return view! {}.into_view();
                }
                
                view! {
                    <div class="py-0.5 text-theme-secondary hover:bg-theme-surface-hover/50 border-l-2 border-transparent hover:border-accent pl-2 -ml-2">
                        <span class="text-theme-muted select-none mr-2">{format!("{:>4}", idx + 1)}</span>
                        <span class="break-all">{json_line}</span>
                    </div>
                }.into_view()
            }).collect_view()}
        </div>
    }
}

/// Schema view showing field types
#[component]
fn SchemaView(events: Vec<NodeEvent>) -> impl IntoView {
    // Infer schema from events
    let schema: Vec<(String, &'static str)> = {
        let mut schema_map = std::collections::HashMap::new();
        
        for event in &events {
            if let serde_json::Value::Object(obj) = &event.data {
                for (key, value) in obj {
                    let type_name = match value {
                        serde_json::Value::Null => "null",
                        serde_json::Value::Bool(_) => "boolean",
                        serde_json::Value::Number(_) => "number",
                        serde_json::Value::String(_) => "string",
                        serde_json::Value::Array(_) => "array",
                        serde_json::Value::Object(_) => "object",
                    };
                    schema_map.entry(key.clone()).or_insert(type_name);
                }
            }
        }
        
        let mut items: Vec<_> = schema_map.into_iter().collect();
        items.sort_by(|a, b| a.0.cmp(&b.0));
        items
    };
    
    view! {
        <div class="p-4 space-y-2 h-full overflow-auto custom-scrollbar">
            {if schema.is_empty() {
                view! {
                    <div class="text-sm text-theme-muted">"No schema available"</div>
                }.into_view()
            } else {
                schema.iter().map(|(key, type_name)| {
                    let type_color = match *type_name {
                        "string" => "text-green-400",
                        "number" => "text-blue-400",
                        "boolean" => "text-yellow-400",
                        "array" => "text-purple-400",
                        "object" => "text-orange-400",
                        _ => "text-theme-muted",
                    };
                    
                    view! {
                        <div class="flex items-center gap-2 text-sm">
                            <span class="text-theme-secondary font-medium">{key.clone()}</span>
                            <span class=format!("text-xs {}", type_color)>{*type_name}</span>
                        </div>
                    }
                }).collect_view()
            }}
        </div>
    }
}

/// Format a JSON value for display
fn format_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            if s.len() > 50 {
                format!("{}...", &s[..47])
            } else {
                s.clone()
            }
        }
        serde_json::Value::Array(arr) => format!("[{} items]", arr.len()),
        serde_json::Value::Object(obj) => format!("{{{} fields}}", obj.len()),
    }
}

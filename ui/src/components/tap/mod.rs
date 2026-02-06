//! Tap/Sample Viewer Components
//!
//! Provides UI components for viewing live data samples from Vector agents.
//! Supports both REST-based sampling and WebSocket streaming.

use leptos::*;
use serde::{Deserialize, Serialize};

/// Sampled event from Vector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampledEvent {
    pub event: serde_json::Value,
    pub component_id: String,
    pub component_kind: String,
    pub sampled_at: String,
}

/// Sample response from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleResponse {
    pub agent_id: String,
    pub events: Vec<SampledEvent>,
    pub count: usize,
    pub limited: bool,
    pub duration_ms: u64,
}

/// WebSocket info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketInfo {
    pub agent_id: String,
    pub agent_url: String,
    pub websocket_url: String,
    pub protocol: String,
}

/// Rate limit status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatus {
    pub agent_id: String,
    pub can_sample: bool,
    pub config: RateLimitConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub max_requests_per_minute: u32,
    pub max_concurrent_per_agent: u32,
    pub global_max_concurrent: u32,
}

/// Agent info for selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub url: String,
    pub status: String,
}

/// Fetch available agents
async fn fetch_agents() -> Result<Vec<AgentInfo>, String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let response = gloo_net::http::Request::get(&format!("{}/api/v1/agents", origin))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json().await.map_err(|e| format!("Parse failed: {}", e))
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

/// Fetch WebSocket info for an agent
async fn fetch_ws_info(agent_id: &str) -> Result<WebSocketInfo, String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let response = gloo_net::http::Request::get(
        &format!("{}/api/v1/tap/{}/ws-info", origin, agent_id)
    )
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json().await.map_err(|e| format!("Parse failed: {}", e))
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

/// Check rate limit status
async fn check_rate_limit(agent_id: &str) -> Result<RateLimitStatus, String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let response = gloo_net::http::Request::get(
        &format!("{}/api/v1/tap/{}/rate-limit", origin, agent_id)
    )
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json().await.map_err(|e| format!("Parse failed: {}", e))
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

/// Sample events from an agent via REST API
async fn sample_events(agent_id: &str, patterns: &str, limit: u32) -> Result<SampleResponse, String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let url = format!(
        "{}/api/v1/tap/{}/sample?patterns={}&limit={}",
        origin, agent_id, patterns, limit
    );
    
    let response = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json().await.map_err(|e| format!("Parse failed: {}", e))
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

/// Tap Viewer - Main component for sampling live data
#[component]
pub fn TapViewer() -> impl IntoView {
    // State
    let (agents, set_agents) = create_signal(Vec::<AgentInfo>::new());
    let (selected_agent, set_selected_agent) = create_signal(Option::<String>::None);
    let (component_pattern, set_component_pattern) = create_signal("*".to_string());
    let (sample_limit, set_sample_limit) = create_signal(10u32);
    let (events, set_events) = create_signal(Vec::<SampledEvent>::new());
    let (ws_info, set_ws_info) = create_signal(Option::<WebSocketInfo>::None);
    let (rate_limit, set_rate_limit) = create_signal(Option::<RateLimitStatus>::None);
    let (loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);
    // Reserved for WebSocket streaming mode
    let (_sampling_active, _set_sampling_active) = create_signal(false);
    
    // Fetch agents on mount
    create_effect(move |_| {
        spawn_local(async move {
            match fetch_agents().await {
                Ok(a) => set_agents.set(a),
                Err(e) => set_error.set(Some(e)),
            }
        });
    });
    
    // Fetch WebSocket info when agent is selected
    create_effect(move |_| {
        if let Some(agent_id) = selected_agent.get() {
            spawn_local(async move {
                // Fetch WebSocket info
                if let Ok(info) = fetch_ws_info(&agent_id).await {
                    set_ws_info.set(Some(info));
                }
                // Check rate limit
                if let Ok(limit) = check_rate_limit(&agent_id).await {
                    set_rate_limit.set(Some(limit));
                }
            });
        }
    });
    
    view! {
        <div class="flex flex-col h-full bg-slate-900 text-white">
            // Header
            <div class="p-4 border-b border-slate-700 bg-slate-800">
                <h2 class="text-lg font-semibold mb-1">"Live Data Sampling"</h2>
                <p class="text-sm text-slate-400">
                    "Sample live events from Vector agents without affecting production."
                </p>
            </div>
            
            // Configuration panel
            <div class="p-4 border-b border-slate-700 bg-slate-800/50">
                <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
                    // Agent selector
                    <div>
                        <label class="block text-xs font-medium text-slate-400 mb-1">"Agent"</label>
                        <select
                            class="w-full px-3 py-2 bg-slate-900 border border-slate-600 rounded text-sm focus:outline-none focus:border-blue-500"
                            on:change=move |ev| {
                                let value = event_target_value(&ev);
                                set_selected_agent.set(if value.is_empty() { None } else { Some(value) });
                                set_events.set(Vec::new());
                                set_ws_info.set(None);
                            }
                        >
                            <option value="">"Select an agent..."</option>
                            {move || agents.get().into_iter().map(|agent| {
                                view! {
                                    <option value={agent.id.clone()}>
                                        {format!("{} ({})", agent.name, agent.status)}
                                    </option>
                                }
                            }).collect::<Vec<_>>()}
                        </select>
                    </div>
                    
                    // Component pattern
                    <div>
                        <label class="block text-xs font-medium text-slate-400 mb-1">"Component Pattern"</label>
                        <input
                            type="text"
                            class="w-full px-3 py-2 bg-slate-900 border border-slate-600 rounded text-sm focus:outline-none focus:border-blue-500"
                            placeholder="* or source_*"
                            prop:value=move || component_pattern.get()
                            on:input=move |ev| set_component_pattern.set(event_target_value(&ev))
                        />
                    </div>
                    
                    // Sample limit
                    <div>
                        <label class="block text-xs font-medium text-slate-400 mb-1">"Event Limit"</label>
                        <input
                            type="number"
                            class="w-full px-3 py-2 bg-slate-900 border border-slate-600 rounded text-sm focus:outline-none focus:border-blue-500"
                            min="1"
                            max="100"
                            prop:value=move || sample_limit.get()
                            on:input=move |ev| {
                                if let Ok(val) = event_target_value(&ev).parse() {
                                    set_sample_limit.set(val);
                                }
                            }
                        />
                    </div>
                    
                    // Sample button
                    <div class="flex items-end">
                        <button
                            class="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-slate-700 disabled:cursor-not-allowed rounded text-sm font-medium transition-colors"
                            disabled=move || selected_agent.get().is_none() || loading.get()
                            on:click=move |_| {
                                if let Some(agent_id) = selected_agent.get() {
                                    let pattern = component_pattern.get();
                                    let limit = sample_limit.get();
                                    
                                    set_loading.set(true);
                                    set_error.set(None);
                                    
                                    spawn_local(async move {
                                        match sample_events(&agent_id, &pattern, limit).await {
                                            Ok(response) => {
                                                set_events.update(|e| {
                                                    e.extend(response.events);
                                                    // Keep only last 100 events
                                                    if e.len() > 100 {
                                                        e.drain(0..e.len()-100);
                                                    }
                                                });
                                            }
                                            Err(e) => set_error.set(Some(e)),
                                        }
                                        set_loading.set(false);
                                    });
                                }
                            }
                        >
                            {move || if loading.get() { "Sampling..." } else { "Sample Events" }}
                        </button>
                    </div>
                </div>
                
                // Rate limit info
                {move || rate_limit.get().map(|rl| view! {
                    <div class="mt-3 text-xs text-slate-400">
                        "Rate limit: "
                        <span class=if rl.can_sample { "text-green-400" } else { "text-red-400" }>
                            {if rl.can_sample { "Available" } else { "Limited" }}
                        </span>
                        " • Max "
                        {rl.config.max_requests_per_minute}
                        " req/min • Max "
                        {rl.config.max_concurrent_per_agent}
                        " concurrent"
                    </div>
                })}
                
                // WebSocket info
                {move || ws_info.get().map(|info| view! {
                    <div class="mt-2 p-2 bg-slate-900 rounded text-xs">
                        <div class="flex items-center space-x-2">
                            <span class="text-slate-400">"WebSocket:"</span>
                            <code class="text-blue-400">{info.websocket_url}</code>
                            <span class="text-slate-500">"•"</span>
                            <span class="text-slate-400">"Protocol:"</span>
                            <code class="text-green-400">{info.protocol}</code>
                        </div>
                    </div>
                })}
            </div>
            
            // Error display
            {move || error.get().map(|e| view! {
                <div class="p-3 bg-red-900/30 border-b border-red-800 text-red-400 text-sm">
                    {e}
                </div>
            })}
            
            // Events display
            <div class="flex-1 overflow-y-auto p-4">
                <div class="flex items-center justify-between mb-3">
                    <h3 class="text-sm font-medium text-slate-300">
                        "Sampled Events "
                        <span class="text-slate-500">
                            "("{move || events.get().len()}")"
                        </span>
                    </h3>
                    <button
                        class="px-3 py-1 text-xs bg-slate-700 hover:bg-slate-600 rounded"
                        on:click=move |_| set_events.set(Vec::new())
                    >
                        "Clear"
                    </button>
                </div>
                
                {move || {
                    let event_list = events.get();
                    if event_list.is_empty() {
                        view! {
                            <div class="text-center py-12 text-slate-500">
                                <div class="text-4xl mb-3">"📡"</div>
                                <p>"No events sampled yet"</p>
                                <p class="text-sm mt-1">"Select an agent and click Sample Events"</p>
                            </div>
                        }.into_view()
                    } else {
                        view! {
                            <div class="space-y-2">
                                {event_list.into_iter().rev().map(|event| {
                                    view! { <EventCard event=event /> }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_view()
                    }
                }}
            </div>
        </div>
    }
}

/// Individual event card component
#[component]
fn EventCard(event: SampledEvent) -> impl IntoView {
    let (expanded, set_expanded) = create_signal(false);
    let event_json = serde_json::to_string_pretty(&event.event).unwrap_or_default();
    
    view! {
        <div class="bg-slate-800 rounded-lg overflow-hidden border border-slate-700">
            // Header
            <div
                class="flex items-center justify-between p-3 cursor-pointer hover:bg-slate-700/50 transition-colors"
                on:click=move |_| set_expanded.update(|e| *e = !*e)
            >
                <div class="flex items-center space-x-3">
                    <span class=format!(
                        "px-2 py-0.5 text-xs rounded {}",
                        match event.component_kind.as_str() {
                            "source" => "bg-green-900/50 text-green-400",
                            "transform" => "bg-blue-900/50 text-blue-400",
                            "sink" => "bg-purple-900/50 text-purple-400",
                            _ => "bg-slate-700 text-slate-400",
                        }
                    )>
                        {event.component_kind.clone()}
                    </span>
                    <code class="text-sm text-blue-400">{event.component_id.clone()}</code>
                </div>
                <div class="flex items-center space-x-3">
                    <span class="text-xs text-slate-500">{event.sampled_at.clone()}</span>
                    <span class="text-slate-400">
                        {move || if expanded.get() { "▼" } else { "▶" }}
                    </span>
                </div>
            </div>
            
            // Expanded content
            {move || expanded.get().then(|| view! {
                <div class="border-t border-slate-700 p-3 bg-slate-900/50">
                    <pre class="text-xs text-slate-300 whitespace-pre-wrap font-mono overflow-x-auto">
                        {event_json.clone()}
                    </pre>
                </div>
            })}
        </div>
    }
}

/// Compact event row for streaming view
#[component]
pub fn EventRow(event: SampledEvent) -> impl IntoView {
    let message = event.event.get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .chars()
        .take(100)
        .collect::<String>();
    
    view! {
        <div class="flex items-center space-x-3 py-1.5 px-3 hover:bg-slate-800/50 text-sm border-b border-slate-800">
            <span class="text-xs text-slate-500 w-20 shrink-0">{event.sampled_at}</span>
            <span class=format!(
                "px-1.5 py-0.5 text-xs rounded shrink-0 {}",
                match event.component_kind.as_str() {
                    "source" => "bg-green-900/30 text-green-500",
                    "transform" => "bg-blue-900/30 text-blue-500",
                    _ => "bg-slate-700 text-slate-400",
                }
            )>
                {event.component_id}
            </span>
            <span class="text-slate-300 truncate">{message}</span>
        </div>
    }
}

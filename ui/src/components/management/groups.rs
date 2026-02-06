//! Worker Group management component

use leptos::*;
use leptos_router::use_navigate;
use serde::{Deserialize, Serialize};

/// Tabs for the worker group detail view
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum GroupDetailTab {
    #[default]
    Overview,
    Agents,
    Pipeline,
    Config,
    Deployments,
    Settings,
}

/// Worker group data from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerGroup {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub deployment_strategy: String,
    pub requires_approval: bool,
    pub agent_count: Option<i64>,
    pub healthy_count: Option<i64>,
    pub unhealthy_count: Option<i64>,
    pub current_config_version: Option<String>,
    pub created_at: String,
}

/// Agent data from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub url: String,
    pub group_id: Option<String>,
    pub status: String,
    pub vector_version: Option<String>,
    pub last_seen: Option<String>,
}

/// Get health status class based on agent counts
fn get_group_health_status(group: &WorkerGroup) -> &'static str {
    let total = group.agent_count.unwrap_or(0);
    let healthy = group.healthy_count.unwrap_or(0);
    let unhealthy = group.unhealthy_count.unwrap_or(0);
    
    // If no agents, show unknown
    if total == 0 {
        return "unknown";
    }
    
    // Use actual health data from the backend
    if unhealthy > 0 {
        "unhealthy"
    } else if healthy == total {
        "healthy"
    } else if healthy > 0 {
        // Some healthy, some unknown status
        "degraded"
    } else {
        // No health data yet (all agents in unknown/other status)
        "unknown"
    }
}

/// Get deployment strategy badge color and display name
fn get_strategy_badge_class(strategy: &str) -> &'static str {
    match strategy.to_lowercase().as_str() {
        "rolling" => "bg-blue-500/20 text-blue-400 border-blue-500/30",
        "canary" => "bg-amber-500/20 text-amber-400 border-amber-500/30",
        "blue_green" | "blue-green" => "bg-green-500/20 text-green-400 border-green-500/30",
        "all_at_once" | "all-at-once" => "bg-violet-500/20 text-violet-400 border-violet-500/30",
        // Legacy "basic" - treat as rolling
        "basic" => "bg-blue-500/20 text-blue-400 border-blue-500/30",
        _ => "bg-theme-surface-hover text-theme-secondary border-theme-border",
    }
}

/// Get human-readable deployment strategy name
fn get_strategy_display_name(strategy: &str) -> &'static str {
    match strategy.to_lowercase().as_str() {
        "rolling" => "Rolling",
        "canary" => "Canary",
        "blue_green" | "blue-green" => "Blue/Green",
        "all_at_once" | "all-at-once" => "All at Once",
        "basic" => "Rolling",  // Map legacy "basic" to "Rolling"
        _ => "Unknown",
    }
}

/// Validation response from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResponse {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub component: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub code: String,
    pub message: String,
    pub component: Option<String>,
}

/// Validate config via API
async fn validate_config(config: &str) -> Result<ValidationResponse, String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let body = serde_json::json!({
        "config": config,
        "use_vector": false
    });
    
    let response = gloo_net::http::Request::post(&format!("{}/api/v1/validate", origin))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .map_err(|e| format!("Body error: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    response.json().await.map_err(|e| format!("Parse failed: {}", e))
}

/// Fetch worker groups from API
async fn fetch_groups() -> Result<Vec<WorkerGroup>, String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let response = gloo_net::http::Request::get(&format!("{}/api/v1/groups", origin))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json().await.map_err(|e| format!("Parse failed: {}", e))
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

/// Fetch unassigned agents from API
async fn fetch_unassigned_agents() -> Result<Vec<Agent>, String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let response = gloo_net::http::Request::get(&format!("{}/api/v1/agents/unassigned", origin))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json().await.map_err(|e| format!("Parse failed: {}", e))
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

/// Assign an agent to a group
async fn assign_agent_to_group(agent_id: &str, group_id: &str) -> Result<Agent, String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let body = serde_json::json!({ "group_id": group_id });
    
    let response = gloo_net::http::Request::post(&format!("{}/api/v1/agents/{}/assign", origin, agent_id))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .map_err(|e| format!("Body error: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json().await.map_err(|e| format!("Parse failed: {}", e))
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

/// Create a new worker group
async fn create_group(name: &str, description: &str) -> Result<WorkerGroup, String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let body = serde_json::json!({
        "name": name,
        "description": description
    });
    
    let response = gloo_net::http::Request::post(&format!("{}/api/v1/groups", origin))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .map_err(|e| format!("Body error: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json().await.map_err(|e| format!("Parse failed: {}", e))
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

/// Fetch agents for a group
async fn fetch_group_agents(group_id: &str) -> Result<Vec<Agent>, String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let response = gloo_net::http::Request::get(&format!("{}/api/v1/groups/{}/agents", origin, group_id))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json().await.map_err(|e| format!("Parse failed: {}", e))
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

/// Worker groups list component
#[component]
pub fn WorkerGroupsList(
    #[prop(into)] on_select: Callback<String>,
) -> impl IntoView {
    let (groups, set_groups) = create_signal(Vec::<WorkerGroup>::new());
    let (unassigned_agents, set_unassigned_agents) = create_signal(Vec::<Agent>::new());
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal(Option::<String>::None);
    let (show_create_modal, set_show_create_modal) = create_signal(false);
    let (show_assign_modal, set_show_assign_modal) = create_signal(Option::<Agent>::None);
    let (refresh_trigger, set_refresh_trigger) = create_signal(0);
    
    // Fetch groups and unassigned agents
    create_effect(move |_| {
        let _ = refresh_trigger.get(); // Subscribe to refresh trigger
        spawn_local(async move {
            set_loading.set(true);
            
            // Fetch both in parallel
            let groups_result = fetch_groups().await;
            let unassigned_result = fetch_unassigned_agents().await;
            
            match groups_result {
                Ok(g) => {
                    set_groups.set(g);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
            
            if let Ok(agents) = unassigned_result {
                set_unassigned_agents.set(agents);
            }
            
            set_loading.set(false);
        });
    });
    
    // Refresh function
    let refresh = move || set_refresh_trigger.update(|n| *n += 1);
    
    view! {
        <div class="flex flex-col h-full">
            <div class="flex items-center justify-between p-4 border-b border-theme-border">
                <h2 class="text-lg font-semibold text-theme">"Worker Groups"</h2>
                <button
                    class="btn-primary text-sm"
                    on:click=move |_| set_show_create_modal.set(true)
                >
                    "+ New Group"
                </button>
            </div>
            
            <div class="flex-1 overflow-y-auto p-4 custom-scrollbar">
                {move || {
                    if loading.get() {
                        view! { <div class="text-theme-secondary">"Loading..."</div> }.into_view()
                    } else if let Some(err) = error.get() {
                        view! { <div class="text-red-400">{err}</div> }.into_view()
                    } else {
                        let groups_list = groups.get();
                        let unassigned_list = unassigned_agents.get();
                        let has_unassigned = !unassigned_list.is_empty();
                        let unassigned_count = unassigned_list.len();
                        let has_groups = !groups_list.is_empty();
                        
                        view! {
                            <div class="space-y-6">
                                // Unassigned Agents Section
                                {if has_unassigned {
                                    view! {
                                        <div class="space-y-3">
                                            <div class="flex items-center gap-2">
                                                <div class="w-2 h-2 rounded-full bg-amber-500 animate-pulse"></div>
                                                <h3 class="text-sm font-medium text-amber-400">
                                                    "Unassigned Agents ("{unassigned_count}")"
                                                </h3>
                                            </div>
                                            <div class="p-4 bg-amber-500/10 border border-amber-500/30 rounded-lg">
                                                <p class="text-sm text-amber-200 mb-3">
                                                    "These agents have connected but are not assigned to any worker group."
                                                </p>
                                                <div class="space-y-2">
                                                    {unassigned_list.into_iter().map(|agent| {
                                                        let agent_for_modal = agent.clone();
                                                        let status_class = match agent.status.as_str() {
                                                            "healthy" => "text-green-400",
                                                            "unhealthy" => "text-red-400",
                                                            _ => "text-amber-400",
                                                        };
                                                        view! {
                                                            <div class="flex items-center justify-between p-3 bg-theme-surface rounded-lg border border-theme-border">
                                                                <div class="flex items-center gap-3">
                                                                    <div class="w-8 h-8 rounded-full bg-theme-surface-hover flex items-center justify-center">
                                                                        <svg class="w-4 h-4 text-theme-secondary" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                                            <rect x="4" y="4" width="6" height="6" rx="1" />
                                                                            <rect x="14" y="4" width="6" height="6" rx="1" />
                                                                            <rect x="4" y="14" width="6" height="6" rx="1" />
                                                                            <rect x="14" y="14" width="6" height="6" rx="1" />
                                                                        </svg>
                                                                    </div>
                                                                    <div>
                                                                        <div class="font-medium text-theme">{&agent.name}</div>
                                                                        <div class="text-xs text-theme-muted flex items-center gap-2">
                                                                            <span class=status_class>{&agent.status}</span>
                                                                            {agent.vector_version.as_ref().map(|v| view! {
                                                                                <span>" · "{v}</span>
                                                                            })}
                                                                        </div>
                                                                    </div>
                                                                </div>
                                                                <button
                                                                    class="btn-primary text-xs px-3 py-1.5"
                                                                    on:click=move |_| set_show_assign_modal.set(Some(agent_for_modal.clone()))
                                                                >
                                                                    "Assign to Group"
                                                                </button>
                                                            </div>
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </div>
                                            </div>
                                        </div>
                                    }.into_view()
                                } else {
                                    view! {}.into_view()
                                }}
                                
                                // Worker Groups Section
                                <div class="space-y-3">
                                    {if has_unassigned || has_groups {
                                        view! {
                                            <h3 class="text-sm font-medium text-theme-secondary">"Worker Groups"</h3>
                                        }.into_view()
                                    } else {
                                        view! {}.into_view()
                                    }}
                                    
                                    {if !has_groups {
                                        view! {
                                            <div class="text-theme-secondary text-center py-8 bg-theme-surface rounded-lg border border-theme-border">
                                                <p>"No worker groups yet"</p>
                                                <p class="text-sm mt-2 text-theme-muted">"Create a group to organize your Vector agents"</p>
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <div class="space-y-3">
                                                {groups_list.into_iter().map(|group| {
                                                    let group_id = group.id.clone();
                                                    let group_name = group.name.clone();
                                                    let group_desc = group.description.clone();
                                                    let agent_count = group.agent_count.unwrap_or(0);
                                                    let strategy = group.deployment_strategy.clone();
                                                    let strategy_badge_class = get_strategy_badge_class(&strategy);
                                                    let strategy_display = get_strategy_display_name(&strategy);
                                                    let config_version = group.current_config_version.clone();
                                                    let health_status = get_group_health_status(&group);
                                                    let healthy_count = group.healthy_count.unwrap_or(0);
                                                    let unhealthy_count = group.unhealthy_count.unwrap_or(0);
                                                    
                                                    // Determine health text color
                                                    let health_text_class = if unhealthy_count > 0 {
                                                        "text-red-400"
                                                    } else if healthy_count == agent_count && agent_count > 0 {
                                                        "text-green-400"
                                                    } else if agent_count == 0 {
                                                        "text-theme-muted"
                                                    } else {
                                                        "text-amber-400"
                                                    };
                                                    
                                                    view! {
                                                        <div
                                                            class="group p-4 bg-theme-surface hover:bg-theme-surface-hover rounded-lg cursor-pointer border border-theme-border hover:border-accent transition-all duration-200"
                                                            on:click=move |_| on_select.call(group_id.clone())
                                                        >
                                                            <div class="flex items-start justify-between gap-4">
                                                                <div class="flex-1 min-w-0">
                                                                    <div class="flex items-center gap-2">
                                                                        <span class=format!("status-dot {}", health_status)></span>
                                                                        <h3 class="font-semibold text-theme truncate">{group_name.clone()}</h3>
                                                                    </div>
                                                                    {group_desc.as_ref().map(|d| {
                                                                        let desc = d.clone();
                                                                        view! { <p class="text-sm text-theme-secondary mt-1 truncate">{desc}</p> }
                                                                    })}
                                                                </div>
                                                                <div class="flex flex-col items-end gap-2 flex-shrink-0">
                                                                    <span class=format!("px-2 py-0.5 text-xs font-medium rounded border {}", strategy_badge_class)>
                                                                        {strategy_display}
                                                                    </span>
                                                                    <div class="text-sm">
                                                                        <span class=health_text_class>
                                                                            {healthy_count}"/"{agent_count}" healthy"
                                                                        </span>
                                                                    </div>
                                                                </div>
                                                            </div>
                                                            {config_version.as_ref().map(|v| {
                                                                let version_short = v[..8.min(v.len())].to_string();
                                                                view! {
                                                                    <div class="mt-3 pt-3 border-t border-theme-border text-xs text-theme-muted flex items-center gap-2">
                                                                        <span>"Config:"</span>
                                                                        <code class="bg-theme-bg px-1.5 py-0.5 rounded font-mono">{version_short}</code>
                                                                    </div>
                                                                }
                                                            })}
                                                        </div>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        }.into_view()
                                    }}
                                </div>
                            </div>
                        }.into_view()
                    }
                }}
            </div>
            
            // Create Group Modal
            {move || {
                if show_create_modal.get() {
                    view! {
                        <CreateGroupModal
                            on_close=move |_| set_show_create_modal.set(false)
                            on_created=move |_| {
                                set_show_create_modal.set(false);
                                refresh();
                            }
                        />
                    }.into_view()
                } else {
                    view! {}.into_view()
                }
            }}
            
            // Assign Agent Modal
            {move || {
                if let Some(agent) = show_assign_modal.get() {
                    let groups_for_modal = groups.get();
                    view! {
                        <AssignAgentModal
                            agent=agent
                            groups=groups_for_modal
                            on_close=move |_| set_show_assign_modal.set(None)
                            on_assigned=move |_| {
                                set_show_assign_modal.set(None);
                                refresh();
                            }
                            on_create_group=move |_| {
                                set_show_assign_modal.set(None);
                                set_show_create_modal.set(true);
                            }
                        />
                    }.into_view()
                } else {
                    view! {}.into_view()
                }
            }}
        </div>
    }
}

/// Modal to create a new group
#[component]
fn CreateGroupModal(
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_created: Callback<WorkerGroup>,
) -> impl IntoView {
    let (name, set_name) = create_signal(String::new());
    let (description, set_description) = create_signal(String::new());
    let (saving, set_saving) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);
    
    view! {
        <div class="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50">
            <div class="bg-theme-surface rounded-xl w-[450px] shadow-xl border border-theme-border">
                <div class="flex items-center justify-between p-4 border-b border-theme-border">
                    <h2 class="text-lg font-semibold text-theme">"Create Worker Group"</h2>
                    <button
                        class="p-1.5 hover:bg-theme-surface-hover rounded-lg text-theme-secondary hover:text-theme transition-colors"
                        on:click=move |_| on_close.call(())
                    >
                        "✕"
                    </button>
                </div>
                
                <div class="p-4 space-y-4">
                    <div class="space-y-1">
                        <label class="text-sm text-theme-secondary">"Group Name"</label>
                        <input
                            type="text"
                            class="w-full px-3 py-2 rounded-lg bg-theme-bg border border-theme-border text-sm text-theme focus:outline-none focus:ring-2 focus:ring-accent focus:border-transparent"
                            placeholder="e.g., Production, Staging"
                            prop:value=move || name.get()
                            on:input=move |e| set_name.set(event_target_value(&e))
                        />
                    </div>
                    
                    <div class="space-y-1">
                        <label class="text-sm text-theme-secondary">"Description"</label>
                        <textarea
                            class="w-full px-3 py-2 rounded-lg bg-theme-bg border border-theme-border text-sm text-theme resize-none focus:outline-none focus:ring-2 focus:ring-accent focus:border-transparent"
                            rows="3"
                            placeholder="Optional description..."
                            prop:value=move || description.get()
                            on:input=move |e| set_description.set(event_target_value(&e))
                        />
                    </div>
                    
                    {move || error.get().map(|e| view! {
                        <div class="p-3 bg-red-500/10 border border-red-500/30 rounded-lg text-red-400 text-sm">
                            {e}
                        </div>
                    })}
                </div>
                
                <div class="p-4 border-t border-theme-border flex justify-end gap-3">
                    <button
                        class="btn-secondary"
                        on:click=move |_| on_close.call(())
                    >
                        "Cancel"
                    </button>
                    <button
                        class="btn-primary disabled:opacity-50"
                        disabled=move || saving.get() || name.get().trim().is_empty()
                        on:click=move |_| {
                            let n = name.get();
                            let d = description.get();
                            set_saving.set(true);
                            set_error.set(None);
                            
                            spawn_local(async move {
                                match create_group(&n, &d).await {
                                    Ok(group) => on_created.call(group),
                                    Err(e) => set_error.set(Some(e)),
                                }
                                set_saving.set(false);
                            });
                        }
                    >
                        {move || if saving.get() { "Creating..." } else { "Create Group" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Modal to assign an agent to a group
#[component]
fn AssignAgentModal(
    #[prop(into)] agent: Agent,
    #[prop(into)] groups: Vec<WorkerGroup>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_assigned: Callback<Agent>,
    #[prop(into)] on_create_group: Callback<()>,
) -> impl IntoView {
    let (selected_group, set_selected_group) = create_signal(Option::<String>::None);
    let (saving, set_saving) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);
    
    let agent_id = agent.id.clone();
    let agent_name = agent.name.clone();
    
    view! {
        <div class="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50">
            <div class="bg-theme-surface rounded-xl w-[450px] shadow-xl border border-theme-border">
                <div class="flex items-center justify-between p-4 border-b border-theme-border">
                    <h2 class="text-lg font-semibold text-theme">"Assign Agent to Group"</h2>
                    <button
                        class="p-1.5 hover:bg-theme-surface-hover rounded-lg text-theme-secondary hover:text-theme transition-colors"
                        on:click=move |_| on_close.call(())
                    >
                        "✕"
                    </button>
                </div>
                
                <div class="p-4 space-y-4">
                    <div class="p-3 bg-theme-bg rounded-lg border border-theme-border">
                        <div class="text-sm text-theme-secondary">"Agent"</div>
                        <div class="font-medium text-theme">{agent_name}</div>
                    </div>
                    
                    <div class="space-y-2">
                        <label class="text-sm text-theme-secondary">"Select Worker Group"</label>
                        {if groups.is_empty() {
                            view! {
                                <div class="p-4 bg-theme-bg rounded-lg border border-theme-border text-center">
                                    <p class="text-theme-muted text-sm mb-3">"No groups available"</p>
                                    <button
                                        class="btn-primary text-sm"
                                        on:click=move |_| on_create_group.call(())
                                    >
                                        "Create First Group"
                                    </button>
                                </div>
                            }.into_view()
                        } else {
                            view! {
                                <div class="space-y-2">
                                    {groups.into_iter().map(|group| {
                                        let group_id = group.id.clone();
                                        let group_id_for_select = group.id.clone();
                                        let is_selected = move || selected_group.get() == Some(group_id.clone());
                                        view! {
                                            <div
                                                class=move || format!(
                                                    "p-3 rounded-lg border cursor-pointer transition-all {}",
                                                    if is_selected() {
                                                        "bg-accent/10 border-accent"
                                                    } else {
                                                        "bg-theme-bg border-theme-border hover:border-theme-secondary"
                                                    }
                                                )
                                                on:click=move |_| set_selected_group.set(Some(group_id_for_select.clone()))
                                            >
                                                <div class="flex items-center justify-between">
                                                    <div>
                                                        <div class="font-medium text-theme">{&group.name}</div>
                                                        {group.description.as_ref().map(|d| view! {
                                                            <div class="text-xs text-theme-muted mt-0.5">{d}</div>
                                                        })}
                                                    </div>
                                                    <div class="text-xs text-theme-secondary">
                                                        {group.agent_count.unwrap_or(0)}" agents"
                                                    </div>
                                                </div>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                    
                                    <button
                                        class="w-full p-3 rounded-lg border border-dashed border-theme-border text-theme-secondary hover:text-theme hover:border-theme-secondary text-sm transition-colors"
                                        on:click=move |_| on_create_group.call(())
                                    >
                                        "+ Create New Group"
                                    </button>
                                </div>
                            }.into_view()
                        }}
                    </div>
                    
                    {move || error.get().map(|e| view! {
                        <div class="p-3 bg-red-500/10 border border-red-500/30 rounded-lg text-red-400 text-sm">
                            {e}
                        </div>
                    })}
                </div>
                
                <div class="p-4 border-t border-theme-border flex justify-end gap-3">
                    <button
                        class="btn-secondary"
                        on:click=move |_| on_close.call(())
                    >
                        "Cancel"
                    </button>
                    <button
                        class="btn-primary disabled:opacity-50"
                        disabled=move || saving.get() || selected_group.get().is_none()
                        on:click={
                            let aid = agent_id.clone();
                            move |_| {
                                if let Some(gid) = selected_group.get() {
                                    let aid = aid.clone();
                                    set_saving.set(true);
                                    set_error.set(None);
                                    
                                    spawn_local(async move {
                                        match assign_agent_to_group(&aid, &gid).await {
                                            Ok(agent) => on_assigned.call(agent),
                                            Err(e) => set_error.set(Some(e)),
                                        }
                                        set_saving.set(false);
                                    });
                                }
                            }
                        }
                    >
                        {move || if saving.get() { "Assigning..." } else { "Assign" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Worker group detail panel for split-view layout (no back button)
#[component]
pub fn WorkerGroupDetailPanel(
    #[prop(into)] group_id: String,
) -> impl IntoView {
    // Dummy callback that won't be used
    let dummy_back: Callback<()> = Callback::new(|_| {});
    view! {
        <WorkerGroupDetailInner group_id=group_id show_back=false on_back=dummy_back />
    }
}

/// Worker group detail view with tabbed interface
#[component]
pub fn WorkerGroupDetail(
    #[prop(into)] group_id: String,
    #[prop(into)] on_back: Callback<()>,
) -> impl IntoView {
    view! {
        <WorkerGroupDetailInner group_id=group_id show_back=true on_back=on_back />
    }
}

/// Inner implementation of worker group detail
#[component]
fn WorkerGroupDetailInner(
    #[prop(into)] group_id: String,
    #[prop(default = true)] show_back: bool,
    #[prop(into)] on_back: Callback<()>,
) -> impl IntoView {
    let (group, set_group) = create_signal(Option::<WorkerGroup>::None);
    let (agents, set_agents) = create_signal(Vec::<Agent>::new());
    let (config_content, set_config_content) = create_signal(String::new());
    let (loading, set_loading) = create_signal(true);
    let (active_tab, set_active_tab) = create_signal(GroupDetailTab::Overview);
    let (show_editor, set_show_editor) = create_signal(false);
    let (show_history_modal, set_show_history_modal) = create_signal(false);
    
    let group_id_clone = group_id.clone();
    let group_id_for_config = group_id.clone();
    
    // Fetch group details
    create_effect(move |_| {
        let gid = group_id_clone.clone();
        let gid_config = group_id_for_config.clone();
        spawn_local(async move {
            set_loading.set(true);
            
            // Fetch group, agents, and config in parallel
            let groups_result = fetch_groups().await;
            let agents_result = fetch_group_agents(&gid).await;
            let config_result = fetch_group_config(&gid_config).await;
            
            if let Ok(groups) = groups_result {
                if let Some(g) = groups.into_iter().find(|g| g.id == gid) {
                    set_group.set(Some(g));
                }
            }
            
            if let Ok(a) = agents_result {
                set_agents.set(a);
            }
            
            if let Ok(cfg) = config_result {
                set_config_content.set(cfg);
            }
            
            set_loading.set(false);
        });
    });
    
    let group_id_for_history = group_id.clone();
    let group_id_for_pipeline = group_id.clone();
    let navigate = use_navigate();
    
    // Tab button helper
    let tab_button = move |tab: GroupDetailTab, label: &'static str| {
        let is_active = move || active_tab.get() == tab;
        view! {
            <button
                class=move || format!("px-4 py-2.5 text-sm font-medium transition-colors border-b-2 -mb-px {}",
                    if is_active() { "text-theme bg-accent/10 border-accent" } else { "text-theme-secondary border-transparent hover:text-theme hover:bg-theme-surface-hover" }
                )
                on:click=move |_| set_active_tab.set(tab)
            >
                {label}
            </button>
        }
    };
    
    let on_back_clone = on_back.clone();
    
    view! {
        <div class="flex flex-col h-full">
            // Header
            <div class="flex items-center p-4 border-b border-theme-border bg-theme-surface">
                {move || {
                    if show_back {
                        let cb = on_back_clone.clone();
                        view! {
                            <button
                                class="btn-ghost mr-4 px-2 py-1"
                                on:click=move |_| cb.call(())
                            >
                                "← Back"
                            </button>
                        }.into_view()
                    } else {
                        view! {}.into_view()
                    }
                }}
                {move || group.get().map(|g| {
                    let health_status = get_group_health_status(&g);
                    let strategy_badge_class = get_strategy_badge_class(&g.deployment_strategy);
                    let strategy_display = get_strategy_display_name(&g.deployment_strategy);
                    let healthy_count = g.healthy_count.unwrap_or(0);
                    let agent_count = g.agent_count.unwrap_or(0);
                    view! {
                        <div class="flex-1">
                            <div class="flex items-center gap-3">
                                <span class=format!("status-dot {}", health_status)></span>
                                <h2 class="text-lg font-semibold text-theme">{&g.name}</h2>
                                <span class=format!("px-2 py-0.5 text-xs font-medium rounded border {}", strategy_badge_class)>
                                    {strategy_display}
                                </span>
                                <span class="text-sm text-theme-secondary">
                                    {healthy_count}"/"{agent_count}" healthy"
                                </span>
                            </div>
                            {g.description.as_ref().map(|d| view! {
                                <p class="text-sm text-theme-secondary mt-1">{d}</p>
                            })}
                        </div>
                    }
                })}
            </div>
            
            // Tabs
            <div class="flex border-b border-theme-border bg-theme-surface">
                {tab_button(GroupDetailTab::Overview, "Overview")}
                {tab_button(GroupDetailTab::Agents, "Agents")}
                {tab_button(GroupDetailTab::Pipeline, "Pipeline")}
                {tab_button(GroupDetailTab::Config, "Config")}
                {tab_button(GroupDetailTab::Deployments, "Deployments")}
                {tab_button(GroupDetailTab::Settings, "Settings")}
            </div>
            
            // Content
            <div class="flex-1 overflow-y-auto p-4 custom-scrollbar">
                {move || {
                    if loading.get() {
                        view! { <div class="text-theme-secondary">"Loading..."</div> }.into_view()
                    } else {
                        match active_tab.get() {
                            GroupDetailTab::Overview => {
                                // Overview Tab
                                let g = group.get();
                                let agent_count = g.as_ref().map(|g| g.agent_count.unwrap_or(0)).unwrap_or(0);
                                let healthy_count = g.as_ref().map(|g| g.healthy_count.unwrap_or(0)).unwrap_or(0);
                                let unhealthy_count = g.as_ref().map(|g| g.unhealthy_count.unwrap_or(0)).unwrap_or(0);
                                let config_version = g.as_ref().and_then(|g| g.current_config_version.clone());
                                let strategy = g.as_ref().map(|g| g.deployment_strategy.clone()).unwrap_or_default();
                                let strategy_display = get_strategy_display_name(&strategy);
                                let health_status = g.as_ref().map(|g| get_group_health_status(g)).unwrap_or("unknown");
                                
                                view! {
                                    <div class="space-y-6">
                                        // Health Summary Card
                                        <div class="p-6 bg-theme-surface rounded-xl border border-theme-border">
                                            <h3 class="text-sm font-medium text-theme-secondary mb-4">"Health Summary"</h3>
                                            <div class="flex items-center gap-4">
                                                <div class=format!("w-16 h-16 rounded-full flex items-center justify-center {}",
                                                    match health_status {
                                                        "healthy" => "bg-green-500/20 border-2 border-green-500",
                                                        "unhealthy" => "bg-red-500/20 border-2 border-red-500",
                                                        "degraded" => "bg-amber-500/20 border-2 border-amber-500",
                                                        _ => "bg-slate-500/20 border-2 border-slate-500",
                                                    }
                                                )>
                                                    <span class=format!("text-2xl font-bold {}",
                                                        match health_status {
                                                            "healthy" => "text-green-400",
                                                            "unhealthy" => "text-red-400",
                                                            "degraded" => "text-amber-400",
                                                            _ => "text-slate-400",
                                                        }
                                                    )>
                                                        {healthy_count}"/"{ agent_count}
                                                    </span>
                                                </div>
                                                <div>
                                                    <div class=format!("text-lg font-semibold capitalize {}",
                                                        match health_status {
                                                            "healthy" => "text-green-400",
                                                            "unhealthy" => "text-red-400",
                                                            "degraded" => "text-amber-400",
                                                            _ => "text-slate-400",
                                                        }
                                                    )>
                                                        {health_status}
                                                    </div>
                                                    <div class="text-sm text-theme-secondary">
                                                        {healthy_count}" healthy, "{unhealthy_count}" unhealthy"
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                        
                                        // Quick Stats
                                        <div class="grid grid-cols-3 gap-4">
                                            <div class="p-4 bg-theme-surface rounded-lg border border-theme-border">
                                                <div class="text-xs text-theme-secondary uppercase tracking-wide">"Agents"</div>
                                                <div class="text-2xl font-bold text-theme mt-1">{agent_count}</div>
                                            </div>
                                            <div class="p-4 bg-theme-surface rounded-lg border border-theme-border">
                                                <div class="text-xs text-theme-secondary uppercase tracking-wide">"Config Version"</div>
                                                <div class="text-sm font-mono text-theme mt-2">
                                                    {config_version.map(|v| v[..8.min(v.len())].to_string()).unwrap_or_else(|| "None".to_string())}
                                                </div>
                                            </div>
                                            <div class="p-4 bg-theme-surface rounded-lg border border-theme-border">
                                                <div class="text-xs text-theme-secondary uppercase tracking-wide">"Deployment Strategy"</div>
                                                <div class="text-sm font-medium text-theme mt-2">{strategy_display}</div>
                                            </div>
                                        </div>
                                        
                                        // Recent Activity
                                        <div class="p-6 bg-theme-surface rounded-xl border border-theme-border">
                                            <h3 class="text-sm font-medium text-theme-secondary mb-4">"Recent Activity"</h3>
                                            <div class="text-theme-muted text-sm text-center py-8">
                                                <p>"No recent activity to display"</p>
                                                <p class="text-xs mt-2">"Activity will appear here as agents report status changes"</p>
                                            </div>
                                        </div>
                                    </div>
                                }.into_view()
                            },
                            GroupDetailTab::Agents => {
                                // Agents Tab - Enhanced table format
                                let agents_list = agents.get();
                                view! {
                                    <div class="space-y-4">
                                        <div class="flex items-center justify-between">
                                            <h3 class="text-sm font-medium text-theme-secondary">
                                                {agents_list.len()}" agent(s) registered"
                                            </h3>
                                            <button class="btn-primary text-sm">
                                                "+ Register Agent"
                                            </button>
                                        </div>
                                        
                                        {if agents_list.is_empty() {
                                            view! {
                                                <div class="text-theme-secondary text-center py-8 bg-theme-surface rounded-lg border border-theme-border">
                                                    <p>"No agents in this group"</p>
                                                    <p class="text-sm mt-2 text-theme-muted">"Agents will appear here when they register"</p>
                                                </div>
                                            }.into_view()
                                        } else {
                                            view! {
                                                <div class="bg-theme-surface rounded-lg border border-theme-border overflow-hidden">
                                                    <table class="w-full">
                                                        <thead class="bg-theme-bg">
                                                            <tr class="text-left text-xs text-theme-secondary uppercase tracking-wide">
                                                                <th class="px-4 py-3 font-medium">"Name"</th>
                                                                <th class="px-4 py-3 font-medium">"URL"</th>
                                                                <th class="px-4 py-3 font-medium">"Status"</th>
                                                                <th class="px-4 py-3 font-medium">"Version"</th>
                                                                <th class="px-4 py-3 font-medium">"Last Seen"</th>
                                                                <th class="px-4 py-3 font-medium text-right">"Actions"</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody class="divide-y divide-theme-border">
                                                            {agents_list.into_iter().map(|agent| {
                                                                let status_class = match agent.status.as_str() {
                                                                    "healthy" => "healthy",
                                                                    "unhealthy" => "unhealthy",
                                                                    _ => "degraded",
                                                                };
                                                                let status_text_class = match agent.status.as_str() {
                                                                    "healthy" => "text-green-400",
                                                                    "unhealthy" => "text-red-400",
                                                                    _ => "text-amber-400",
                                                                };
                                                                view! {
                                                                    <tr class="hover:bg-theme-surface-hover transition-colors">
                                                                        <td class="px-4 py-3">
                                                                            <div class="flex items-center gap-2">
                                                                                <span class=format!("status-dot {}", status_class)></span>
                                                                                <span class="font-medium text-theme">{&agent.name}</span>
                                                                            </div>
                                                                        </td>
                                                                        <td class="px-4 py-3 text-sm text-theme-secondary font-mono truncate max-w-[200px]">
                                                                            {&agent.url}
                                                                        </td>
                                                                        <td class="px-4 py-3">
                                                                            <span class=format!("text-sm font-medium capitalize {}", status_text_class)>
                                                                                {&agent.status}
                                                                            </span>
                                                                        </td>
                                                                        <td class="px-4 py-3 text-sm text-theme-muted">
                                                                            {agent.vector_version.clone().unwrap_or_else(|| "-".to_string())}
                                                                        </td>
                                                                        <td class="px-4 py-3 text-sm text-theme-muted">
                                                                            {agent.last_seen.clone().unwrap_or_else(|| "-".to_string())}
                                                                        </td>
                                                                        <td class="px-4 py-3 text-right">
                                                                            <button class="btn-ghost text-xs px-2 py-1">
                                                                                "View"
                                                                            </button>
                                                                        </td>
                                                                    </tr>
                                                                }
                                                            }).collect::<Vec<_>>()}
                                                        </tbody>
                                                    </table>
                                                </div>
                                            }.into_view()
                                        }}
                                    </div>
                                }.into_view()
                            },
                            GroupDetailTab::Pipeline => {
                                // Pipeline Tab
                                let gid = group_id_for_pipeline.clone();
                                let nav = navigate.clone();
                                view! {
                                    <div class="flex flex-col items-center justify-center py-16 bg-theme-surface rounded-xl border border-theme-border">
                                        <div class="w-16 h-16 rounded-full bg-violet-500/20 flex items-center justify-center mb-4">
                                            <svg class="w-8 h-8 text-violet-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z" />
                                            </svg>
                                        </div>
                                        <h3 class="text-lg font-semibold text-theme mb-2">"Pipeline Builder"</h3>
                                        <p class="text-theme-secondary text-sm mb-6 text-center max-w-sm">
                                            "Design and configure your data pipeline visually using the pipeline builder."
                                        </p>
                                        <button
                                            class="btn-primary"
                                            on:click=move |_| {
                                                let path = format!("/fleets/{}/pipeline", gid);
                                                nav(&path, Default::default());
                                            }
                                        >
                                            "Open Pipeline Builder"
                                        </button>
                                    </div>
                                }.into_view()
                            },
                            GroupDetailTab::Config => {
                                // Config Tab
                                let gid = group_id_for_history.clone();
                                let cfg = config_content.get();
                                view! {
                                    <div class="space-y-4">
                                        <div class="flex items-center justify-between">
                                            <h3 class="text-sm font-medium text-theme-secondary">"Current Configuration"</h3>
                                            <div class="flex gap-2">
                                                <button
                                                    class="btn-ghost text-sm"
                                                    on:click=move |_| set_show_history_modal.set(true)
                                                >
                                                    "View History"
                                                </button>
                                                <button
                                                    class="btn-primary text-sm"
                                                    on:click=move |_| set_show_editor.set(true)
                                                >
                                                    "Edit Config"
                                                </button>
                                            </div>
                                        </div>
                                        
                                        {if cfg.is_empty() {
                                            view! {
                                                <div class="text-theme-secondary text-center py-8 bg-theme-surface rounded-lg border border-theme-border">
                                                    <p>"No configuration set"</p>
                                                    <p class="text-sm mt-2 text-theme-muted">"Click 'Edit Config' to add a configuration"</p>
                                                </div>
                                            }.into_view()
                                        } else {
                                            view! {
                                                <div class="bg-theme-surface rounded-lg border border-theme-border overflow-hidden">
                                                    <div class="px-4 py-2 bg-theme-bg border-b border-theme-border flex items-center justify-between">
                                                        <span class="text-xs text-theme-muted font-mono">"vector.toml"</span>
                                                        <span class="text-xs text-theme-secondary">{cfg.lines().count()}" lines"</span>
                                                    </div>
                                                    <pre class="p-4 overflow-x-auto text-sm font-mono text-theme whitespace-pre custom-scrollbar max-h-[500px] overflow-y-auto">
                                                        {cfg}
                                                    </pre>
                                                </div>
                                            }.into_view()
                                        }}
                                        
                                        // History modal
                                        {move || {
                                            if show_history_modal.get() {
                                                let gid = gid.clone();
                                                view! {
                                                    <div class="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50">
                                                        <div class="bg-theme-surface rounded-xl w-[700px] max-h-[80vh] flex flex-col shadow-xl border border-theme-border">
                                                            <div class="flex items-center justify-between p-4 border-b border-theme-border">
                                                                <h2 class="text-lg font-semibold text-theme">"Configuration History"</h2>
                                                                <button
                                                                    class="p-1.5 hover:bg-theme-surface-hover rounded-lg text-theme-secondary hover:text-theme transition-colors"
                                                                    on:click=move |_| set_show_history_modal.set(false)
                                                                >
                                                                    "✕"
                                                                </button>
                                                            </div>
                                                            <div class="flex-1 overflow-y-auto p-4 custom-scrollbar">
                                                                <ConfigHistory group_id=gid />
                                                            </div>
                                                        </div>
                                                    </div>
                                                }.into_view()
                                            } else {
                                                view! {}.into_view()
                                            }
                                        }}
                                    </div>
                                }.into_view()
                            },
                            GroupDetailTab::Deployments => {
                                // Deployments Tab
                                view! {
                                    <div class="space-y-4">
                                        <div class="flex items-center justify-between">
                                            <h3 class="text-sm font-medium text-theme-secondary">"Deployment History"</h3>
                                            <button class="btn-primary text-sm">
                                                "+ New Deployment"
                                            </button>
                                        </div>
                                        
                                        <div class="text-theme-muted text-center py-12 bg-theme-surface rounded-lg border border-theme-border">
                                            <div class="w-12 h-12 rounded-full bg-blue-500/20 flex items-center justify-center mx-auto mb-4">
                                                <svg class="w-6 h-6 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                                                </svg>
                                            </div>
                                            <p class="font-medium text-theme">"No deployments yet"</p>
                                            <p class="text-sm mt-2">"Deployments will appear here when you push configurations to agents"</p>
                                        </div>
                                    </div>
                                }.into_view()
                            },
                            GroupDetailTab::Settings => {
                                // Settings Tab
                                let g = group.get();
                                view! {
                                    <div class="space-y-6 max-w-2xl">
                                        <div>
                                            <h3 class="text-sm font-medium text-theme-secondary mb-4">"Group Settings"</h3>
                                            
                                            <div class="space-y-4 bg-theme-surface rounded-lg border border-theme-border p-4">
                                                // Name
                                                <div class="space-y-1">
                                                    <label class="text-xs text-theme-secondary">"Group Name"</label>
                                                    <input
                                                        type="text"
                                                        class="w-full px-3 py-2 rounded-lg bg-theme-bg border border-theme-border text-sm text-theme focus:outline-none focus:ring-2 focus:ring-accent focus:border-transparent"
                                                        value=g.as_ref().map(|g| g.name.clone()).unwrap_or_default()
                                                        disabled=true
                                                    />
                                                </div>
                                                
                                                // Description
                                                <div class="space-y-1">
                                                    <label class="text-xs text-theme-secondary">"Description"</label>
                                                    <textarea
                                                        class="w-full px-3 py-2 rounded-lg bg-theme-bg border border-theme-border text-sm text-theme resize-none focus:outline-none focus:ring-2 focus:ring-accent focus:border-transparent"
                                                        rows="3"
                                                        disabled=true
                                                    >
                                                        {g.as_ref().and_then(|g| g.description.clone()).unwrap_or_default()}
                                                    </textarea>
                                                </div>
                                                
                                                // Deployment Strategy
                                                <div class="space-y-1">
                                                    <label class="text-xs text-theme-secondary">"Deployment Strategy"</label>
                                                    <select
                                                        class="w-full px-3 py-2 rounded-lg bg-theme-bg border border-theme-border text-sm text-theme focus:outline-none focus:ring-2 focus:ring-accent focus:border-transparent"
                                                        disabled=true
                                                    >
                                                        <option selected=g.as_ref().map(|g| g.deployment_strategy == "rolling").unwrap_or(false)>"Rolling"</option>
                                                        <option selected=g.as_ref().map(|g| g.deployment_strategy == "canary").unwrap_or(false)>"Canary"</option>
                                                        <option selected=g.as_ref().map(|g| g.deployment_strategy == "blue_green").unwrap_or(false)>"Blue/Green"</option>
                                                        <option selected=g.as_ref().map(|g| g.deployment_strategy == "all_at_once").unwrap_or(false)>"All at Once"</option>
                                                    </select>
                                                </div>
                                                
                                                // Requires Approval
                                                <div class="flex items-center justify-between py-2">
                                                    <div>
                                                        <div class="text-sm text-theme">"Require Approval"</div>
                                                        <div class="text-xs text-theme-muted">"Deployments must be approved before executing"</div>
                                                    </div>
                                                    <div class=format!("w-10 h-6 rounded-full transition-colors {}",
                                                        if g.as_ref().map(|g| g.requires_approval).unwrap_or(false) { "bg-accent" } else { "bg-theme-border" }
                                                    )>
                                                        <div class=format!("w-4 h-4 mt-1 rounded-full bg-white transition-transform {}",
                                                            if g.as_ref().map(|g| g.requires_approval).unwrap_or(false) { "translate-x-5 ml-0" } else { "translate-x-1" }
                                                        )></div>
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                        
                                        <div class="text-xs text-theme-muted">
                                            "Settings are currently read-only. Editing will be available in a future update."
                                        </div>
                                    </div>
                                }.into_view()
                            },
                        }
                    }
                }}
            </div>
            
            // Actions footer (show on certain tabs)
            {move || {
                let tab = active_tab.get();
                let g = group.get();
                
                // Only show actions on Overview, Agents, Config tabs
                if matches!(tab, GroupDetailTab::Overview | GroupDetailTab::Agents | GroupDetailTab::Config) {
                    g.map(|g| {
                        let gid = g.id.clone();
                        view! {
                            <div class="p-4 border-t border-theme-border bg-theme-surface flex justify-end gap-3">
                                <button
                                    class="btn-secondary"
                                    on:click=move |_| set_show_editor.set(true)
                                >
                                    "Edit Config"
                                </button>
                                <button
                                    class="btn-primary"
                                    on:click=move |_| {
                                        let group_id = gid.clone();
                                        spawn_local(async move {
                                            let _ = deploy_to_group(&group_id).await;
                                        });
                                    }
                                >
                                    "Deploy"
                                </button>
                            </div>
                        }
                    })
                } else {
                    None
                }
            }}
            
            // Config editor modal
            {
                let group_id_for_editor = group_id.clone();
                let group_id_for_refresh = group_id.clone();
                move || {
                    if show_editor.get() {
                        let gid = group_id_for_editor.clone();
                        let gid_refresh = group_id_for_refresh.clone();
                        view! {
                            <ConfigEditor
                                group_id=gid
                                on_close=move |_| set_show_editor.set(false)
                                on_save=move |_version| {
                                    set_show_editor.set(false);
                                    // Refresh group data and config
                                    let gid = gid_refresh.clone();
                                    spawn_local(async move {
                                        if let Ok(groups) = fetch_groups().await {
                                            if let Some(g) = groups.into_iter().find(|g| g.id == gid) {
                                                set_group.set(Some(g));
                                            }
                                        }
                                        if let Ok(cfg) = fetch_group_config(&gid).await {
                                            set_config_content.set(cfg);
                                        }
                                    });
                                }
                            />
                        }.into_view()
                    } else {
                        view! {}.into_view()
                    }
                }
            }
        </div>
    }
}

/// Deploy to a worker group
async fn deploy_to_group(group_id: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let response = gloo_net::http::Request::post(&format!("{}/api/v1/groups/{}/deploy", origin, group_id))
        .header("Content-Type", "application/json")
        .body("{}")
        .map_err(|e| format!("Body error: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        Ok(())
    } else {
        Err(format!("Deploy failed: {}", response.status()))
    }
}

/// Update group config via API
async fn update_group_config(group_id: &str, config: &str) -> Result<String, String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let body = serde_json::json!({ "config": config });
    
    let response = gloo_net::http::Request::put(&format!("{}/api/v1/groups/{}/config", origin, group_id))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .map_err(|e| format!("Body error: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        let result: serde_json::Value = response.json().await.map_err(|e| format!("Parse: {}", e))?;
        Ok(result.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string())
    } else {
        let result: serde_json::Value = response.json().await.map_err(|e| format!("Parse: {}", e))?;
        Err(result.get("message").and_then(|v| v.as_str()).unwrap_or("Update failed").to_string())
    }
}

/// Fetch current group config
async fn fetch_group_config(group_id: &str) -> Result<String, String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let response = gloo_net::http::Request::get(&format!("{}/api/v1/groups/{}/config", origin, group_id))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        let result: serde_json::Value = response.json().await.map_err(|e| format!("Parse: {}", e))?;
        Ok(result.get("config").and_then(|v| v.as_str()).unwrap_or("").to_string())
    } else {
        Err("Failed to fetch config".to_string())
    }
}

/// Config editor component with validation
#[component]
pub fn ConfigEditor(
    #[prop(into)] group_id: String,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_save: Callback<String>,
) -> impl IntoView {
    let (config, set_config) = create_signal(String::new());
    let (validation, set_validation) = create_signal(Option::<ValidationResponse>::None);
    let (loading, set_loading) = create_signal(true);
    let (saving, set_saving) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);
    
    let group_id_clone = group_id.clone();
    
    // Load config on mount
    create_effect(move |_| {
        let gid = group_id_clone.clone();
        spawn_local(async move {
            set_loading.set(true);
            match fetch_group_config(&gid).await {
                Ok(cfg) => set_config.set(cfg),
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    });
    
    // Validate on config change (debounced)
    let validate_config_fn = {
        move |cfg: String| {
            spawn_local(async move {
                match validate_config(&cfg).await {
                    Ok(result) => set_validation.set(Some(result)),
                    Err(_) => set_validation.set(None),
                }
            });
        }
    };
    
    let group_id_for_save = group_id.clone();
    
    view! {
        <div class="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50">
            <div class="bg-theme-surface rounded-xl w-[800px] max-h-[80vh] flex flex-col shadow-xl border border-theme-border">
                // Header
                <div class="flex items-center justify-between p-4 border-b border-theme-border">
                    <h2 class="text-lg font-semibold text-theme">"Edit Configuration"</h2>
                    <button
                        class="p-1.5 hover:bg-theme-surface-hover rounded-lg text-theme-secondary hover:text-theme transition-colors"
                        on:click=move |_| on_close.call(())
                    >
                        "✕"
                    </button>
                </div>
                
                // Content
                <div class="flex-1 overflow-hidden flex flex-col p-4 min-h-0">
                    {move || {
                        if loading.get() {
                            view! { <div class="text-theme-secondary">"Loading..."</div> }.into_view()
                        } else {
                            view! {
                                <div class="flex-1 flex flex-col min-h-0">
                                    // Editor
                                    <textarea
                                        class="flex-1 w-full p-3 bg-theme-bg border border-theme-border rounded-lg font-mono text-sm resize-none text-theme focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all"
                                        prop:value=move || config.get()
                                        on:input=move |e| {
                                            let value = event_target_value(&e);
                                            set_config.set(value.clone());
                                            validate_config_fn(value);
                                        }
                                    />
                                    
                                    // Validation feedback
                                    {move || validation.get().map(|v| {
                                        let has_errors = !v.errors.is_empty();
                                        let has_warnings = !v.warnings.is_empty();
                                        
                                        view! {
                                            <div class="mt-4 space-y-2">
                                                {if v.valid && !has_warnings {
                                                    view! {
                                                        <div class="p-3 bg-green-500/10 border border-green-500/30 rounded-lg text-green-400 text-sm flex items-center gap-2">
                                                            <span class="status-dot healthy"></span>
                                                            "Configuration is valid"
                                                        </div>
                                                    }.into_view()
                                                } else if has_errors {
                                                    view! {
                                                        <div class="p-3 bg-red-500/10 border border-red-500/30 rounded-lg text-red-400 text-sm">
                                                            <div class="font-medium flex items-center gap-2">
                                                                <span class="status-dot unhealthy"></span>
                                                                "Validation errors:"
                                                            </div>
                                                            <ul class="mt-2 list-disc list-inside space-y-1 ml-4">
                                                                {v.errors.iter().map(|e| {
                                                                    let msg = e.message.clone();
                                                                    let loc = match (e.line, e.column) {
                                                                        (Some(l), Some(c)) => format!(" (line {}, col {})", l, c),
                                                                        (Some(l), None) => format!(" (line {})", l),
                                                                        _ => String::new(),
                                                                    };
                                                                    view! { <li>{msg}{loc}</li> }
                                                                }).collect::<Vec<_>>()}
                                                            </ul>
                                                        </div>
                                                    }.into_view()
                                                } else {
                                                    view! {}.into_view()
                                                }}
                                                
                                                {if has_warnings {
                                                    view! {
                                                        <div class="p-3 bg-amber-500/10 border border-amber-500/30 rounded-lg text-amber-400 text-sm">
                                                            <div class="font-medium flex items-center gap-2">
                                                                <span class="status-dot degraded"></span>
                                                                "Warnings:"
                                                            </div>
                                                            <ul class="mt-2 list-disc list-inside space-y-1 ml-4">
                                                                {v.warnings.iter().map(|w| {
                                                                    let msg = w.message.clone();
                                                                    view! { <li>{msg}</li> }
                                                                }).collect::<Vec<_>>()}
                                                            </ul>
                                                        </div>
                                                    }.into_view()
                                                } else {
                                                    view! {}.into_view()
                                                }}
                                            </div>
                                        }
                                    })}
                                    
                                    // Error message
                                    {move || error.get().map(|e| view! {
                                        <div class="mt-4 p-3 bg-red-500/10 border border-red-500/30 rounded-lg text-red-400 text-sm flex items-center gap-2">
                                            <span class="status-dot unhealthy"></span>
                                            {e}
                                        </div>
                                    })}
                                </div>
                            }.into_view()
                        }
                    }}
                </div>
                
                // Footer
                <div class="p-4 border-t border-theme-border flex justify-end gap-3">
                    <button
                        class="btn-secondary"
                        on:click=move |_| on_close.call(())
                    >
                        "Cancel"
                    </button>
                    <button
                        class="btn-primary disabled:opacity-50 disabled:cursor-not-allowed"
                        disabled=move || {
                            saving.get() || validation.get().map(|v| !v.valid).unwrap_or(true)
                        }
                        on:click={
                            let gid = group_id_for_save.clone();
                            move |_| {
                                let gid = gid.clone();
                                let cfg = config.get();
                                set_saving.set(true);
                                set_error.set(None);
                                
                                spawn_local(async move {
                                    match update_group_config(&gid, &cfg).await {
                                        Ok(version) => {
                                            on_save.call(version);
                                        }
                                        Err(e) => {
                                            set_error.set(Some(e));
                                        }
                                    }
                                    set_saving.set(false);
                                });
                            }
                        }
                    >
                        {move || if saving.get() { "Saving..." } else { "Save" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

use crate::components::management::history::ConfigHistory;

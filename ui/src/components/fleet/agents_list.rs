//! Agents List Component
//!
//! A comprehensive table view of all Vector agents with status, version,
//! health history, and management actions.

use leptos::*;
use leptos_router::*;

use super::{AgentInfo, AgentStatus, fetch_agents, delete_agent};

/// Agents list with table view
#[component]
pub fn AgentsList() -> impl IntoView {
    let (agents, set_agents) = create_signal(Vec::<AgentInfo>::new());
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal(Option::<String>::None);
    let (search, set_search) = create_signal(String::new());
    let (status_filter, set_status_filter) = create_signal(Option::<AgentStatus>::None);
    
    // Fetch agents on mount
    create_effect(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            match fetch_agents().await {
                Ok(a) => {
                    set_agents.set(a);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    });
    
    // Filtered agents
    let filtered_agents = move || {
        let agents_list = agents.get();
        let search_term = search.get().to_lowercase();
        let status = status_filter.get();
        
        agents_list
            .into_iter()
            .filter(|a| {
                let matches_search = search_term.is_empty() 
                    || a.name.to_lowercase().contains(&search_term)
                    || a.url.to_lowercase().contains(&search_term)
                    || a.group_name.as_ref().map(|g| g.to_lowercase().contains(&search_term)).unwrap_or(false);
                
                let matches_status = status.as_ref()
                    .map(|s| &a.status == s)
                    .unwrap_or(true);
                
                matches_search && matches_status
            })
            .collect::<Vec<_>>()
    };
    
    // Stats
    let stats = move || {
        let all = agents.get();
        let healthy = all.iter().filter(|a| a.status == AgentStatus::Healthy).count();
        let unhealthy = all.iter().filter(|a| a.status == AgentStatus::Unhealthy).count();
        let unknown = all.iter().filter(|a| a.status == AgentStatus::Unknown).count();
        (all.len(), healthy, unhealthy, unknown)
    };
    
    // Delete handler
    let handle_delete = move |id: String| {
        let id_clone = id.clone();
        spawn_local(async move {
            if let Ok(()) = delete_agent(&id_clone).await {
                set_agents.update(|agents| {
                    agents.retain(|a| a.id != id_clone);
                });
            }
        });
    };
    
    view! {
        <div class="flex-1 overflow-auto p-6">
            <div class="max-w-7xl mx-auto">
                // Header with stats
                <div class="flex items-center justify-between mb-6">
                    <div>
                        <h1 class="text-2xl font-bold text-theme">"All Agents"</h1>
                        <div class="flex items-center gap-4 mt-2 text-sm">
                            <span class="text-theme-secondary">{move || stats().0}" total"</span>
                            <span class="flex items-center gap-1.5">
                                <span class="status-dot healthy"></span>
                                <span class="text-green-400">{move || stats().1}</span>
                            </span>
                            <span class="flex items-center gap-1.5">
                                <span class="status-dot unhealthy"></span>
                                <span class="text-red-400">{move || stats().2}</span>
                            </span>
                            <span class="flex items-center gap-1.5">
                                <span class="status-dot unknown"></span>
                                <span class="text-theme-muted">{move || stats().3}</span>
                            </span>
                        </div>
                    </div>
                    <button class="btn-primary">"Register Agent"</button>
                </div>
                
                // Filters
                <div class="flex items-center gap-4 mb-4">
                    <input
                        type="text"
                        class="input max-w-xs"
                        placeholder="Search agents..."
                        prop:value=move || search.get()
                        on:input=move |e| set_search.set(event_target_value(&e))
                    />
                    <select
                        class="input w-40"
                        on:change=move |e| {
                            let value = event_target_value(&e);
                            set_status_filter.set(match value.as_str() {
                                "healthy" => Some(AgentStatus::Healthy),
                                "unhealthy" => Some(AgentStatus::Unhealthy),
                                "unknown" => Some(AgentStatus::Unknown),
                                _ => None,
                            });
                        }
                    >
                        <option value="">"All statuses"</option>
                        <option value="healthy">"Healthy"</option>
                        <option value="unhealthy">"Unhealthy"</option>
                        <option value="unknown">"Unknown"</option>
                    </select>
                </div>
                
                // Table
                <div class="bg-theme-surface rounded-xl border border-theme-border overflow-hidden">
                    {move || {
                        if loading.get() {
                            view! {
                                <div class="p-8 text-center text-theme-secondary">
                                    "Loading agents..."
                                </div>
                            }.into_view()
                        } else if let Some(err) = error.get() {
                            view! {
                                <div class="p-8 text-center text-error">
                                    {err}
                                </div>
                            }.into_view()
                        } else {
                            let agents_list = filtered_agents();
                            if agents_list.is_empty() {
                                view! {
                                    <div class="p-8 text-center">
                                        <div class="w-16 h-16 rounded-full bg-theme-surface-hover flex items-center justify-center mx-auto mb-4">
                                            <svg class="w-8 h-8 text-theme-muted" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                <rect x="4" y="4" width="6" height="6" rx="1" />
                                                <rect x="14" y="4" width="6" height="6" rx="1" />
                                                <rect x="4" y="14" width="6" height="6" rx="1" />
                                                <rect x="14" y="14" width="6" height="6" rx="1" />
                                            </svg>
                                        </div>
                                        <p class="text-theme-secondary">"No agents found"</p>
                                        <p class="text-sm text-theme-muted mt-1">
                                            {if search.get().is_empty() && status_filter.get().is_none() {
                                                "Register your first agent to get started"
                                            } else {
                                                "Try adjusting your search or filters"
                                            }}
                                        </p>
                                    </div>
                                }.into_view()
                            } else {
                                view! {
                                    <table class="w-full">
                                        <thead>
                                            <tr class="border-b border-theme-border bg-theme-bg">
                                                <th class="px-4 py-3 text-left text-xs font-medium text-theme-muted uppercase tracking-wider">"Status"</th>
                                                <th class="px-4 py-3 text-left text-xs font-medium text-theme-muted uppercase tracking-wider">"Name"</th>
                                                <th class="px-4 py-3 text-left text-xs font-medium text-theme-muted uppercase tracking-wider">"URL"</th>
                                                <th class="px-4 py-3 text-left text-xs font-medium text-theme-muted uppercase tracking-wider">"Group"</th>
                                                <th class="px-4 py-3 text-left text-xs font-medium text-theme-muted uppercase tracking-wider">"Version"</th>
                                                <th class="px-4 py-3 text-left text-xs font-medium text-theme-muted uppercase tracking-wider">"Last Seen"</th>
                                                <th class="px-4 py-3 text-left text-xs font-medium text-theme-muted uppercase tracking-wider">"Latency"</th>
                                                <th class="px-4 py-3 text-right text-xs font-medium text-theme-muted uppercase tracking-wider">"Actions"</th>
                                            </tr>
                                        </thead>
                                        <tbody class="divide-y divide-theme-border">
                                            {agents_list.into_iter().map(|agent| {
                                                let agent_id = agent.id.clone();
                                                let agent_id_for_delete = agent.id.clone();
                                                let status_class = match agent.status {
                                                    AgentStatus::Healthy => "healthy",
                                                    AgentStatus::Unhealthy => "unhealthy",
                                                    AgentStatus::Unknown => "unknown",
                                                };
                                                view! {
                                                    <tr class="hover:bg-theme-surface-hover transition-colors">
                                                        <td class="px-4 py-3">
                                                            <div class="flex items-center gap-2">
                                                                <span class=format!("status-dot {}", status_class)></span>
                                                                <span class=agent.status.text_class()>{agent.status.label()}</span>
                                                            </div>
                                                        </td>
                                                        <td class="px-4 py-3">
                                                            <A 
                                                                href=format!("/fleet/{}", agent_id)
                                                                class="text-theme font-medium hover:text-accent transition-colors"
                                                            >
                                                                {&agent.name}
                                                            </A>
                                                        </td>
                                                        <td class="px-4 py-3 text-theme-secondary font-mono text-sm">
                                                            {&agent.url}
                                                        </td>
                                                        <td class="px-4 py-3">
                                                            {agent.group_name.as_ref().map(|g| view! {
                                                                <A 
                                                                    href=format!("/fleets/{}", agent.group_id.as_ref().unwrap_or(&String::new()))
                                                                    class="text-accent hover:underline"
                                                                >
                                                                    {g}
                                                                </A>
                                                            }).unwrap_or_else(|| view! {
                                                                <span class="text-theme-muted">"—"</span>
                                                            }.into_view())}
                                                        </td>
                                                        <td class="px-4 py-3 text-theme-secondary">
                                                            {agent.version.as_ref().unwrap_or(&"—".to_string()).clone()}
                                                        </td>
                                                        <td class="px-4 py-3 text-theme-secondary text-sm">
                                                            {agent.last_seen.as_ref().unwrap_or(&"—".to_string()).clone()}
                                                        </td>
                                                        <td class="px-4 py-3 text-theme-secondary text-sm">
                                                            {agent.latency_ms.map(|l| format!("{}ms", l)).unwrap_or_else(|| "—".to_string())}
                                                        </td>
                                                        <td class="px-4 py-3 text-right">
                                                            <div class="flex items-center justify-end gap-2">
                                                                <A 
                                                                    href=format!("/fleet/{}", agent_id_for_delete.clone())
                                                                    class="btn-ghost px-2 py-1 text-sm"
                                                                >
                                                                    "View"
                                                                </A>
                                                                <button
                                                                    class="btn-ghost px-2 py-1 text-sm text-error hover:bg-error/10"
                                                                    on:click={
                                                                        let id = agent_id_for_delete.clone();
                                                                        move |_| handle_delete(id.clone())
                                                                    }
                                                                >
                                                                    "Remove"
                                                                </button>
                                                            </div>
                                                        </td>
                                                    </tr>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </tbody>
                                    </table>
                                }.into_view()
                            }
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

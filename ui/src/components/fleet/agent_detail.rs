//! Agent Detail Component
//!
//! Detailed view of a single Vector agent with health history,
//! metrics, and configuration.

use leptos::*;

use super::{AgentInfo, AgentStatus, fetch_agents};

/// Agent detail view
#[component]
pub fn AgentDetail(
    #[prop(into)] agent_id: String,
    #[prop(into, optional)] on_back: Option<Callback<()>>,
) -> impl IntoView {
    let (agent, set_agent) = create_signal(Option::<AgentInfo>::None);
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal(Option::<String>::None);
    
    let agent_id_clone = agent_id.clone();
    
    // Fetch agent details
    create_effect(move |_| {
        let aid = agent_id_clone.clone();
        spawn_local(async move {
            set_loading.set(true);
            match fetch_agents().await {
                Ok(agents) => {
                    if let Some(a) = agents.into_iter().find(|a| a.id == aid) {
                        set_agent.set(Some(a));
                        set_error.set(None);
                    } else {
                        set_error.set(Some("Agent not found".to_string()));
                    }
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    });
    
    view! {
        <div class="flex flex-col h-full">
            // Header
            <div class="flex items-center p-4 border-b border-theme-border bg-theme-surface">
                {on_back.map(|cb| view! {
                    <button
                        class="btn-ghost mr-4 px-2 py-1"
                        on:click=move |_| cb.call(())
                    >
                        "← Back"
                    </button>
                })}
                {move || agent.get().map(|a| {
                    let status_class = match a.status {
                        AgentStatus::Healthy => "healthy",
                        AgentStatus::Unhealthy => "unhealthy",
                        AgentStatus::Unknown => "unknown",
                    };
                    view! {
                        <div class="flex-1">
                            <div class="flex items-center gap-3">
                                <span class=format!("status-dot {}", status_class)></span>
                                <h2 class="text-lg font-semibold text-theme">{&a.name}</h2>
                            </div>
                            <p class="text-sm text-theme-secondary mt-1 font-mono">{&a.url}</p>
                        </div>
                    }
                })}
            </div>
            
            // Content
            <div class="flex-1 overflow-y-auto p-6 custom-scrollbar">
                {move || {
                    if loading.get() {
                        view! {
                            <div class="text-theme-secondary text-center py-8">
                                "Loading agent details..."
                            </div>
                        }.into_view()
                    } else if let Some(err) = error.get() {
                        view! {
                            <div class="text-error text-center py-8">
                                {err}
                            </div>
                        }.into_view()
                    } else if let Some(a) = agent.get() {
                        view! {
                            <div class="max-w-4xl mx-auto space-y-6">
                                // Status card
                                <div class="bg-theme-surface rounded-xl border border-theme-border p-6">
                                    <h3 class="text-lg font-medium text-theme mb-4">"Agent Status"</h3>
                                    <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                                        <div>
                                            <div class="text-sm text-theme-muted">"Status"</div>
                                            <div class=format!("text-lg font-medium {}", a.status.text_class())>
                                                {a.status.label()}
                                            </div>
                                        </div>
                                        <div>
                                            <div class="text-sm text-theme-muted">"Version"</div>
                                            <div class="text-lg font-medium text-theme">
                                                {a.version.as_ref().unwrap_or(&"Unknown".to_string()).clone()}
                                            </div>
                                        </div>
                                        <div>
                                            <div class="text-sm text-theme-muted">"Latency"</div>
                                            <div class="text-lg font-medium text-theme">
                                                {a.latency_ms.map(|l| format!("{}ms", l)).unwrap_or_else(|| "—".to_string())}
                                            </div>
                                        </div>
                                        <div>
                                            <div class="text-sm text-theme-muted">"Last Seen"</div>
                                            <div class="text-lg font-medium text-theme">
                                                {a.last_seen.as_ref().unwrap_or(&"—".to_string()).clone()}
                                            </div>
                                        </div>
                                    </div>
                                </div>
                                
                                // Group info
                                {a.group_name.as_ref().map(|group| view! {
                                    <div class="bg-theme-surface rounded-xl border border-theme-border p-6">
                                        <h3 class="text-lg font-medium text-theme mb-4">"Worker Group"</h3>
                                        <div class="flex items-center justify-between">
                                            <div>
                                                <div class="text-theme font-medium">{group}</div>
                                                <div class="text-sm text-theme-muted">
                                                    "ID: "{a.group_id.as_ref().unwrap_or(&"—".to_string()).clone()}
                                                </div>
                                            </div>
                                            <a 
                                                href=format!("/fleets/{}", a.group_id.as_ref().unwrap_or(&String::new()))
                                                class="btn-secondary"
                                            >
                                                "View Group"
                                            </a>
                                        </div>
                                    </div>
                                })}
                                
                                // Health history placeholder
                                <div class="bg-theme-surface rounded-xl border border-theme-border p-6">
                                    <h3 class="text-lg font-medium text-theme mb-4">"Health History"</h3>
                                    <div class="text-center py-8 text-theme-muted">
                                        <svg class="w-12 h-12 mx-auto mb-3 opacity-50" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                                            <line x1="18" y1="20" x2="18" y2="10" />
                                            <line x1="12" y1="20" x2="12" y2="4" />
                                            <line x1="6" y1="20" x2="6" y2="14" />
                                        </svg>
                                        <p>"Health history chart coming soon"</p>
                                    </div>
                                </div>
                                
                                // Actions
                                <div class="flex justify-end gap-3">
                                    <button class="btn-danger">"Remove Agent"</button>
                                </div>
                            </div>
                        }.into_view()
                    } else {
                        view! {
                            <div class="text-theme-muted text-center py-8">
                                "No agent data"
                            </div>
                        }.into_view()
                    }
                }}
            </div>
        </div>
    }
}

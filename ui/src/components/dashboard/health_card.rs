//! Fleet Health Card Component

use leptos::*;

use super::FleetHealth;

/// Fleet health overview card
#[component]
pub fn HealthCard() -> impl IntoView {
    let (health, set_health) = create_signal(Option::<FleetHealth>::None);
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal(Option::<String>::None);
    
    // Fetch health data on mount
    create_effect(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            
            match fetch_fleet_health().await {
                Ok(data) => {
                    set_health.set(Some(data));
                    set_error.set(None);
                }
                Err(e) => {
                    set_error.set(Some(e));
                }
            }
            
            set_loading.set(false);
        });
    });
    
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
            <h2 class="text-lg font-semibold text-white mb-4">"Fleet Health"</h2>
            
            <Show
                when=move || !loading.get()
                fallback=move || view! {
                    <div class="flex items-center justify-center py-8">
                        <div class="animate-spin w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full" />
                    </div>
                }
            >
                {move || {
                    if let Some(err) = error.get() {
                        view! {
                            <div class="text-center py-8">
                                <p class="text-slate-400">{err}</p>
                            </div>
                        }.into_view()
                    } else if let Some(h) = health.get() {
                        view! {
                            <div class="space-y-6">
                                // Agent counts
                                <div class="grid grid-cols-4 gap-4">
                                    <StatBox 
                                        label="Total Agents"
                                        value=h.total_agents
                                        color="text-white"
                                    />
                                    <StatBox 
                                        label="Healthy"
                                        value=h.healthy
                                        color="text-green-400"
                                    />
                                    <StatBox 
                                        label="Unhealthy"
                                        value=h.unhealthy
                                        color="text-red-400"
                                    />
                                    <StatBox 
                                        label="Unknown"
                                        value=h.unknown
                                        color="text-slate-400"
                                    />
                                </div>
                                
                                // Health bar
                                <div>
                                    <div class="flex justify-between text-sm mb-2">
                                        <span class="text-slate-400">"Health Status"</span>
                                        <span class="text-white">
                                            {format!("{}%", if h.total_agents > 0 { (h.healthy * 100) / h.total_agents } else { 0 })}
                                        </span>
                                    </div>
                                    <HealthBar 
                                        healthy=h.healthy
                                        unhealthy=h.unhealthy
                                        unknown=h.unknown
                                    />
                                </div>
                                
                                // Version distribution
                                {
                                    let versions = h.version_distribution.clone();
                                    let versions_check = versions.clone();
                                    let total = h.total_agents;
                                    let has_versions = !versions.is_empty();
                                    view! {
                                        <Show when=move || has_versions>
                                            <div>
                                                <h3 class="text-sm text-slate-400 mb-2">"Version Distribution"</h3>
                                                <div class="space-y-2">
                                                    {versions_check.iter().map(|v| {
                                                        view! {
                                                            <VersionRow 
                                                                version=v.version.clone()
                                                                count=v.count
                                                                total=total
                                                            />
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </div>
                                            </div>
                                        </Show>
                                    }
                                }
                            </div>
                        }.into_view()
                    } else {
                        view! {
                            <div class="text-center py-8">
                                <p class="text-slate-400">"No agents registered"</p>
                                <p class="text-sm text-slate-500 mt-1">"Register your first Vector agent to get started"</p>
                            </div>
                        }.into_view()
                    }
                }}
            </Show>
        </div>
    }
}

#[component]
fn StatBox(
    label: &'static str,
    value: u32,
    color: &'static str,
) -> impl IntoView {
    view! {
        <div class="text-center">
            <div class=format!("text-3xl font-bold {}", color)>{value}</div>
            <div class="text-xs text-slate-500 mt-1">{label}</div>
        </div>
    }
}

#[component]
fn HealthBar(
    healthy: u32,
    unhealthy: u32,
    unknown: u32,
) -> impl IntoView {
    let total = healthy + unhealthy + unknown;
    let healthy_pct = if total > 0 { (healthy * 100) / total } else { 0 };
    let unhealthy_pct = if total > 0 { (unhealthy * 100) / total } else { 0 };
    
    view! {
        <div class="h-2 bg-slate-700 rounded-full overflow-hidden flex">
            <div 
                class="bg-green-500 transition-all"
                style=format!("width: {}%", healthy_pct)
            />
            <div 
                class="bg-red-500 transition-all"
                style=format!("width: {}%", unhealthy_pct)
            />
            <div 
                class="bg-slate-500 transition-all flex-1"
            />
        </div>
    }
}

#[component]
fn VersionRow(
    version: String,
    count: u32,
    total: u32,
) -> impl IntoView {
    let pct = if total > 0 { (count * 100) / total } else { 0 };
    
    view! {
        <div class="flex items-center gap-3">
            <div class="w-20 text-sm text-slate-300 font-mono">{version}</div>
            <div class="flex-1 h-2 bg-slate-700 rounded-full overflow-hidden">
                <div 
                    class="h-full bg-blue-500"
                    style=format!("width: {}%", pct)
                />
            </div>
            <div class="w-12 text-right text-sm text-slate-400">{count}</div>
        </div>
    }
}

/// Fetch fleet health from API
async fn fetch_fleet_health() -> Result<FleetHealth, String> {
    let base_url = web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_else(|| "http://localhost:8080".to_string());
    
    let response = gloo_net::http::Request::get(&format!("{}/api/v1/health/fleet", base_url))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json::<FleetHealth>().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        // Return mock data for now if endpoint doesn't exist
        Ok(FleetHealth {
            total_agents: 0,
            healthy: 0,
            unhealthy: 0,
            unknown: 0,
            version_distribution: vec![],
        })
    }
}

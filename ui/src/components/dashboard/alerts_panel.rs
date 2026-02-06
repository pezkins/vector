//! Alerts Panel Component

use leptos::*;

use super::{AlertSummary, AlertSeverity};

/// Active alerts panel
#[component]
pub fn AlertsPanel() -> impl IntoView {
    let (alerts, set_alerts) = create_signal(Vec::<AlertSummary>::new());
    let (loading, set_loading) = create_signal(true);
    
    // Fetch alerts on mount
    create_effect(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            
            if let Ok(data) = fetch_active_alerts().await {
                set_alerts.set(data);
            }
            
            set_loading.set(false);
        });
    });
    
    let critical_count = move || -> usize {
        alerts.get().iter().filter(|a| a.severity == AlertSeverity::Critical).count()
    };
    
    let warning_count = move || -> usize {
        alerts.get().iter().filter(|a| a.severity == AlertSeverity::Warning).count()
    };
    
    let has_critical = move || critical_count() > 0;
    let has_warning = move || warning_count() > 0;
    
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
            <div class="flex items-center justify-between mb-4">
                <div class="flex items-center gap-3">
                    <h2 class="text-lg font-semibold text-white">"Active Alerts"</h2>
                    <Show when=has_critical>
                        <span class="px-2 py-0.5 bg-red-500/20 text-red-400 text-xs font-medium rounded-full">
                            {critical_count} " critical"
                        </span>
                    </Show>
                    <Show when=has_warning>
                        <span class="px-2 py-0.5 bg-amber-500/20 text-amber-400 text-xs font-medium rounded-full">
                            {warning_count} " warning"
                        </span>
                    </Show>
                </div>
                <a href="/observability" class="text-sm text-blue-400 hover:text-blue-300">
                    "Manage Alerts"
                </a>
            </div>
            
            <Show
                when=move || !loading.get()
                fallback=move || view! {
                    <div class="flex items-center justify-center py-8">
                        <div class="animate-spin w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full" />
                    </div>
                }
            >
                {move || {
                    let items = alerts.get();
                    if items.is_empty() {
                        view! {
                            <div class="text-center py-8">
                                <div class="w-12 h-12 rounded-full bg-green-500/10 flex items-center justify-center mx-auto mb-3">
                                    <svg class="w-6 h-6 text-green-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                        <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
                                        <polyline points="22 4 12 14.01 9 11.01" />
                                    </svg>
                                </div>
                                <p class="text-green-400 font-medium">"All Clear"</p>
                                <p class="text-sm text-slate-500 mt-1">"No active alerts"</p>
                            </div>
                        }.into_view()
                    } else {
                        view! {
                            <div class="space-y-2">
                                {items.into_iter().map(|alert| {
                                    view! { <AlertRow alert=alert /> }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_view()
                    }
                }}
            </Show>
        </div>
    }
}

#[component]
fn AlertRow(alert: AlertSummary) -> impl IntoView {
    let severity_class = alert.severity.class();
    let text_class = alert.severity.text_class();
    
    view! {
        <div class="flex items-start gap-3 p-3 rounded-lg bg-slate-700/30 hover:bg-slate-700/50 transition-colors">
            // Severity indicator
            <div class=format!("w-2 h-2 rounded-full mt-1.5 {}", severity_class) />
            
            // Content
            <div class="flex-1 min-w-0">
                <p class=format!("text-sm font-medium {}", text_class)>{alert.title}</p>
                <div class="flex items-center gap-2 mt-1">
                    <span class="text-xs text-slate-500">{alert.source}</span>
                    <span class="text-xs text-slate-600">"·"</span>
                    <span class="text-xs text-slate-500">{alert.timestamp}</span>
                </div>
            </div>
            
            // Action
            <a 
                href=format!("/observability?alert={}", alert.id)
                class="text-xs text-slate-400 hover:text-white transition-colors"
            >
                "View"
            </a>
        </div>
    }
}

/// Fetch active alerts from API
async fn fetch_active_alerts() -> Result<Vec<AlertSummary>, String> {
    let base_url = web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_else(|| "http://localhost:8080".to_string());
    
    let response = gloo_net::http::Request::get(&format!("{}/api/v1/alerts?status=active&limit=5", base_url))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json::<Vec<AlertSummary>>().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        // Return empty for now if endpoint doesn't exist
        Ok(vec![])
    }
}

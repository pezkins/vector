//! Activity Feed Component

use leptos::*;

use super::{ActivityItem, ActivityType};

/// Recent activity feed
#[component]
pub fn ActivityFeed() -> impl IntoView {
    let (activities, set_activities) = create_signal(Vec::<ActivityItem>::new());
    let (loading, set_loading) = create_signal(true);
    
    // Fetch activities on mount
    create_effect(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            
            if let Ok(data) = fetch_recent_activity().await {
                set_activities.set(data);
            }
            
            set_loading.set(false);
        });
    });
    
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
            <div class="flex items-center justify-between mb-4">
                <h2 class="text-lg font-semibold text-white">"Recent Activity"</h2>
                <a href="/observability/audit" class="text-sm text-blue-400 hover:text-blue-300">
                    "View All"
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
                    let items = activities.get();
                    if items.is_empty() {
                        view! {
                            <div class="text-center py-8">
                                <p class="text-slate-400">"No recent activity"</p>
                                <p class="text-sm text-slate-500 mt-1">"Activity will appear here as you use Vectorize"</p>
                            </div>
                        }.into_view()
                    } else {
                        view! {
                            <div class="space-y-3">
                                {items.into_iter().map(|item| {
                                    view! { <ActivityRow item=item /> }
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
fn ActivityRow(item: ActivityItem) -> impl IntoView {
    let icon_class = item.activity_type.icon_class();
    
    view! {
        <div class="flex items-start gap-3 py-2">
            // Icon
            <div class=format!("mt-0.5 {}", icon_class)>
                <ActivityIcon activity_type=item.activity_type.clone() />
            </div>
            
            // Content
            <div class="flex-1 min-w-0">
                <p class="text-sm text-slate-200 truncate">{item.description}</p>
                <div class="flex items-center gap-2 mt-0.5">
                    <span class="text-xs text-slate-500">{item.timestamp}</span>
                    {item.user.map(|user| view! {
                        <span class="text-xs text-slate-500">
                            "by "
                            <span class="text-slate-400">{user}</span>
                        </span>
                    })}
                </div>
            </div>
        </div>
    }
}

#[component]
fn ActivityIcon(activity_type: ActivityType) -> impl IntoView {
    match activity_type {
        ActivityType::Deployment => view! {
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
            </svg>
        }.into_view(),
        ActivityType::ConfigChange => view! {
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                <polyline points="14 2 14 8 20 8" />
            </svg>
        }.into_view(),
        ActivityType::AgentRegistered => view! {
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10" />
                <line x1="12" y1="8" x2="12" y2="16" />
                <line x1="8" y1="12" x2="16" y2="12" />
            </svg>
        }.into_view(),
        ActivityType::AgentRemoved => view! {
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10" />
                <line x1="8" y1="12" x2="16" y2="12" />
            </svg>
        }.into_view(),
        ActivityType::AlertTriggered => view! {
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
                <line x1="12" y1="9" x2="12" y2="13" />
                <line x1="12" y1="17" x2="12.01" y2="17" />
            </svg>
        }.into_view(),
        ActivityType::AlertResolved => view! {
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
                <polyline points="22 4 12 14.01 9 11.01" />
            </svg>
        }.into_view(),
    }
}

/// Fetch recent activity from API
async fn fetch_recent_activity() -> Result<Vec<ActivityItem>, String> {
    let base_url = web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_else(|| "http://localhost:8080".to_string());
    
    let response = gloo_net::http::Request::get(&format!("{}/api/v1/audit?limit=10", base_url))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json::<Vec<ActivityItem>>().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        // Return empty for now if endpoint doesn't exist or fails
        Ok(vec![])
    }
}

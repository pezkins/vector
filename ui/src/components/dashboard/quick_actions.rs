//! Quick Actions Component

use leptos::*;
use leptos_router::*;

/// Quick action buttons
#[component]
pub fn QuickActions() -> impl IntoView {
    let _navigate = use_navigate();
    
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 p-6 h-full">
            <h2 class="text-lg font-semibold text-white mb-4">"Quick Actions"</h2>
            
            <div class="space-y-3">
                <ActionButton 
                    icon=ActionIcon::Pipeline
                    label="New Pipeline"
                    description="Create a new data pipeline"
                    href="/pipelines/new"
                />
                <ActionButton 
                    icon=ActionIcon::Agent
                    label="Register Agent"
                    description="Add a Vector instance"
                    href="/fleet?action=register"
                />
                <ActionButton 
                    icon=ActionIcon::Deploy
                    label="Deploy Config"
                    description="Deploy to a worker group"
                    href="/deployments?action=deploy"
                />
                <ActionButton 
                    icon=ActionIcon::Alert
                    label="Create Alert Rule"
                    description="Set up monitoring"
                    href="/observability/rules?action=create"
                />
            </div>
        </div>
    }
}

#[derive(Clone, Copy)]
enum ActionIcon {
    Pipeline,
    Agent,
    Deploy,
    Alert,
}

#[component]
fn ActionButton(
    icon: ActionIcon,
    label: &'static str,
    description: &'static str,
    href: &'static str,
) -> impl IntoView {
    view! {
        <a 
            href=href
            class="flex items-center gap-3 p-3 rounded-lg bg-slate-700/30 hover:bg-slate-700/50 border border-transparent hover:border-slate-600 transition-all group"
        >
            <div class="w-10 h-10 rounded-lg bg-slate-700 flex items-center justify-center text-slate-400 group-hover:text-blue-400 group-hover:bg-blue-500/10 transition-colors">
                {match icon {
                    ActionIcon::Pipeline => view! {
                        <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M12 2L2 7l10 5 10-5-10-5z" />
                            <path d="M2 17l10 5 10-5" />
                            <path d="M2 12l10 5 10-5" />
                        </svg>
                    }.into_view(),
                    ActionIcon::Agent => view! {
                        <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <rect x="4" y="4" width="16" height="16" rx="2" />
                            <line x1="12" y1="8" x2="12" y2="16" />
                            <line x1="8" y1="12" x2="16" y2="12" />
                        </svg>
                    }.into_view(),
                    ActionIcon::Deploy => view! {
                        <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
                            <polyline points="3.27 6.96 12 12.01 20.73 6.96" />
                            <line x1="12" y1="22.08" x2="12" y2="12" />
                        </svg>
                    }.into_view(),
                    ActionIcon::Alert => view! {
                        <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" />
                            <path d="M13.73 21a2 2 0 0 1-3.46 0" />
                        </svg>
                    }.into_view(),
                }}
            </div>
            <div class="flex-1">
                <div class="text-sm font-medium text-white">{label}</div>
                <div class="text-xs text-slate-500">{description}</div>
            </div>
            <svg class="w-4 h-4 text-slate-500 group-hover:text-slate-400 transition-colors" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="9 18 15 12 9 6" />
            </svg>
        </a>
    }
}

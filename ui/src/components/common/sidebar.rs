//! Sidebar Navigation Component
//!
//! Collapsible sidebar navigation inspired by Cribl's UI.

use leptos::*;
use leptos_router::*;

use crate::state::AppState;

/// Main sidebar navigation component
#[component]
pub fn Sidebar() -> impl IntoView {
    let app_state = expect_context::<AppState>();
    let (collapsed, set_collapsed) = create_signal(false);
    
    view! {
        <aside 
            class=move || {
                let base = "h-full bg-slate-800 border-r border-slate-700 flex flex-col transition-all duration-200";
                if collapsed.get() {
                    format!("{} w-16", base)
                } else {
                    format!("{} w-56", base)
                }
            }
        >
            // Logo header
            <div class="h-14 flex items-center px-4 border-b border-slate-700">
                <A href="/" class="flex items-center gap-2 text-white font-bold overflow-hidden">
                    <div class="w-8 h-8 bg-gradient-to-br from-blue-500 to-purple-600 rounded-lg flex items-center justify-center flex-shrink-0">
                        <span class="text-white text-sm font-bold">"V"</span>
                    </div>
                    <Show when=move || !collapsed.get()>
                        <span class="text-lg whitespace-nowrap">"Vectorize"</span>
                    </Show>
                </A>
            </div>
            
            // Navigation items
            <nav class="flex-1 py-4 overflow-y-auto">
                <div class="space-y-1 px-2">
                    <SidebarLink 
                        href="/" 
                        icon=SidebarIcon::Dashboard 
                        label="Dashboard" 
                        collapsed=collapsed
                        exact=true
                    />
                    <SidebarLink 
                        href="/fleet" 
                        icon=SidebarIcon::Fleet 
                        label="Fleet" 
                        collapsed=collapsed
                        exact=false
                    />
                    <SidebarLink 
                        href="/pipelines" 
                        icon=SidebarIcon::Pipelines 
                        label="Pipelines" 
                        collapsed=collapsed
                        exact=false
                    />
                    <SidebarLink 
                        href="/deployments" 
                        icon=SidebarIcon::Deployments 
                        label="Deployments" 
                        collapsed=collapsed
                        exact=false
                    />
                    <SidebarLink 
                        href="/observability" 
                        icon=SidebarIcon::Observability 
                        label="Observability" 
                        collapsed=collapsed
                        exact=false
                    />
                </div>
            </nav>
            
            // Bottom section - Settings and User
            <div class="border-t border-slate-700 p-2">
                <SidebarLink 
                    href="/settings" 
                    icon=SidebarIcon::Settings 
                    label="Settings" 
                    collapsed=collapsed
                    exact=false
                />
                
                // Collapse toggle
                <button
                    class="w-full mt-2 flex items-center gap-3 px-3 py-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-700/50 transition-colors"
                    on:click=move |_| set_collapsed.update(|c| *c = !*c)
                >
                    <div class="w-5 h-5 flex items-center justify-center flex-shrink-0">
                        {move || if collapsed.get() {
                            view! { <ChevronRightIcon class="w-5 h-5" /> }
                        } else {
                            view! { <ChevronLeftIcon class="w-5 h-5" /> }
                        }}
                    </div>
                    <Show when=move || !collapsed.get()>
                        <span class="text-sm whitespace-nowrap">"Collapse"</span>
                    </Show>
                </button>
                
                // Connection status
                {
                    let app_state = app_state.clone();
                    view! {
                        <Show when=move || app_state.connected.get()>
                            <div class=move || {
                                let base = "mt-2 px-3 py-2 rounded-lg bg-slate-700/30";
                                if collapsed.get() {
                                    format!("{} flex justify-center", base)
                                } else {
                                    base.to_string()
                                }
                            }>
                                <Show
                                    when=move || !collapsed.get()
                                    fallback=move || view! {
                                        <div class="w-2 h-2 rounded-full bg-green-500" title="Connected" />
                                    }
                                >
                                    <div class="flex items-center gap-2 text-xs text-slate-400">
                                        <div class="w-2 h-2 rounded-full bg-green-500 flex-shrink-0" />
                                        <span class="truncate">"Connected"</span>
                                    </div>
                                </Show>
                            </div>
                        </Show>
                    }
                }
            </div>
        </aside>
    }
}

/// Sidebar navigation link
#[component]
fn SidebarLink(
    href: &'static str,
    icon: SidebarIcon,
    label: &'static str,
    collapsed: ReadSignal<bool>,
    #[prop(default = false)] exact: bool,
) -> impl IntoView {
    let location = use_location();
    
    view! {
        <A
            href=href
            class=move || {
                let pathname = location.pathname.get();
                let is_active = if exact {
                    pathname == href
                } else {
                    pathname == href || pathname.starts_with(&format!("{}/", href))
                };
                
                let base = "flex items-center gap-3 px-3 py-2 rounded-lg transition-colors";
                if is_active {
                    format!("{} bg-blue-600 text-white", base)
                } else {
                    format!("{} text-slate-400 hover:text-white hover:bg-slate-700/50", base)
                }
            }
        >
            <div class="w-5 h-5 flex items-center justify-center flex-shrink-0">
                {icon.render()}
            </div>
            <Show when=move || !collapsed.get()>
                <span class="text-sm font-medium whitespace-nowrap">{label}</span>
            </Show>
        </A>
    }
}

/// Sidebar icon variants
#[derive(Clone, Copy)]
pub enum SidebarIcon {
    Dashboard,
    Fleet,
    Pipelines,
    Deployments,
    Observability,
    Settings,
}

impl SidebarIcon {
    fn render(self) -> impl IntoView {
        match self {
            SidebarIcon::Dashboard => view! { <DashboardIcon class="w-5 h-5" /> }.into_view(),
            SidebarIcon::Fleet => view! { <FleetIcon class="w-5 h-5" /> }.into_view(),
            SidebarIcon::Pipelines => view! { <PipelinesIcon class="w-5 h-5" /> }.into_view(),
            SidebarIcon::Deployments => view! { <DeploymentsIcon class="w-5 h-5" /> }.into_view(),
            SidebarIcon::Observability => view! { <ObservabilityIcon class="w-5 h-5" /> }.into_view(),
            SidebarIcon::Settings => view! { <SettingsNavIcon class="w-5 h-5" /> }.into_view(),
        }
    }
}

// Navigation Icons

#[component]
fn DashboardIcon(#[prop(default = "w-5 h-5")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="7" height="9" rx="1" />
            <rect x="14" y="3" width="7" height="5" rx="1" />
            <rect x="14" y="12" width="7" height="9" rx="1" />
            <rect x="3" y="16" width="7" height="5" rx="1" />
        </svg>
    }
}

#[component]
fn FleetIcon(#[prop(default = "w-5 h-5")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="4" y="4" width="6" height="6" rx="1" />
            <rect x="14" y="4" width="6" height="6" rx="1" />
            <rect x="4" y="14" width="6" height="6" rx="1" />
            <rect x="14" y="14" width="6" height="6" rx="1" />
        </svg>
    }
}

#[component]
fn PipelinesIcon(#[prop(default = "w-5 h-5")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 2L2 7l10 5 10-5-10-5z" />
            <path d="M2 17l10 5 10-5" />
            <path d="M2 12l10 5 10-5" />
        </svg>
    }
}

#[component]
fn DeploymentsIcon(#[prop(default = "w-5 h-5")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
            <polyline points="3.27 6.96 12 12.01 20.73 6.96" />
            <line x1="12" y1="22.08" x2="12" y2="12" />
        </svg>
    }
}

#[component]
fn ObservabilityIcon(#[prop(default = "w-5 h-5")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
            <circle cx="12" cy="12" r="3" />
        </svg>
    }
}

#[component]
fn SettingsNavIcon(#[prop(default = "w-5 h-5")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="3" />
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
    }
}

#[component]
fn ChevronLeftIcon(#[prop(default = "w-5 h-5")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="15 18 9 12 15 6" />
        </svg>
    }
}

#[component]
fn ChevronRightIcon(#[prop(default = "w-5 h-5")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="9 18 15 12 9 6" />
        </svg>
    }
}

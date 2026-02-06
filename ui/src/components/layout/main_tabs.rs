//! Main Navigation Tabs Component
//!
//! Horizontal tabs for top-level navigation, similar to browser tabs.
//! Replaces the sidebar for main navigation.

use leptos::*;
use leptos_router::*;

use crate::state::{AppState, Theme};

/// Main navigation tabs at the top of the app
#[component]
pub fn MainTabs() -> impl IntoView {
    let _app_state = expect_context::<AppState>();
    let location = use_location();
    
    view! {
        <header class="bg-theme-surface border-b border-theme-border flex-shrink-0">
            // Top row: Logo, Tabs, and Actions
            <div class="h-14 flex items-center px-4 gap-4">
                // Logo
                <A href="/" class="flex items-center gap-2 text-theme flex-shrink-0">
                    <div class="w-8 h-8 bg-gradient-to-br from-blue-500 to-purple-600 rounded-lg flex items-center justify-center">
                        <span class="text-white text-sm font-bold">"V"</span>
                    </div>
                    <span class="text-lg font-bold">"Vectorize"</span>
                </A>
                
                // Main tabs
                <nav class="flex items-center gap-1 ml-6">
                    <MainTab 
                        href="/" 
                        label="Dashboard" 
                        exact=true 
                    />
                    <MainTab 
                        href="/fleets" 
                        label="Worker Groups" 
                        exact=false 
                    />
                    <MainTab 
                        href="/pipelines" 
                        label="Pipelines" 
                        exact=false 
                    />
                    <MainTab 
                        href="/observe" 
                        label="Observability" 
                        exact=false 
                    />
                    <MainTab 
                        href="/settings" 
                        label="Settings" 
                        exact=false 
                    />
                </nav>
                
                // Right side: Theme toggle and connection status
                <div class="flex items-center gap-3 ml-auto">
                    // Connection status
                    <ConnectionBadge />
                    
                    // Theme toggle
                    <ThemeToggleButton />
                </div>
            </div>
            
            // Sub-tabs row (shown for routes with sub-navigation)
            {move || {
                let pathname = location.pathname.get();
                
                // Observability sub-tabs
                if pathname.starts_with("/observe") {
                    Some(view! {
                        <SubTabsRow>
                            <SubTab href="/observe" label="Data Explorer" exact=true />
                            <SubTab href="/observe/metrics" label="Metrics" exact=true />
                            <SubTab href="/observe/alerts" label="Alerts" exact=true />
                            <SubTab href="/observe/audit" label="Audit Logs" exact=true />
                        </SubTabsRow>
                    }.into_view())
                }
                // Settings sub-tabs
                else if pathname.starts_with("/settings") {
                    Some(view! {
                        <SubTabsRow>
                            <SubTab href="/settings" label="General" exact=true />
                            <SubTab href="/settings/users" label="Users" exact=true />
                            <SubTab href="/settings/roles" label="Roles" exact=true />
                            <SubTab href="/settings/api-keys" label="API Keys" exact=true />
                            <SubTab href="/settings/sso" label="SSO" exact=true />
                            <SubTab href="/settings/git" label="Git Sync" exact=true />
                            <SubTab href="/settings/system" label="System" exact=true />
                        </SubTabsRow>
                    }.into_view())
                }
                else {
                    None
                }
            }}
        </header>
    }
}

/// Individual main tab
#[component]
fn MainTab(
    href: &'static str,
    label: &'static str,
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
                
                let base = "px-4 py-2 rounded-lg text-sm font-medium transition-colors";
                if is_active {
                    format!("{} bg-accent text-white", base)
                } else {
                    format!("{} text-theme-secondary hover:text-theme hover:bg-theme-surface-hover", base)
                }
            }
        >
            {label}
        </A>
    }
}

/// Sub-tabs row container
#[component]
fn SubTabsRow(children: Children) -> impl IntoView {
    view! {
        <div class="h-10 flex items-center px-4 gap-1 bg-theme-bg border-t border-theme-border">
            {children()}
        </div>
    }
}

/// Individual sub-tab
#[component]
fn SubTab(
    href: &'static str,
    label: &'static str,
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
                
                let base = "px-3 py-1.5 rounded text-xs font-medium transition-colors";
                if is_active {
                    format!("{} bg-theme-surface text-theme border border-theme-border", base)
                } else {
                    format!("{} text-theme-muted hover:text-theme hover:bg-theme-surface-hover", base)
                }
            }
        >
            {label}
        </A>
    }
}

/// Connection status badge
#[component]
fn ConnectionBadge() -> impl IntoView {
    let app_state = expect_context::<AppState>();
    
    view! {
        <div class=move || {
            let base = "flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium";
            if app_state.connected.get() {
                format!("{} bg-success/10 text-success", base)
            } else {
                format!("{} bg-theme-surface text-theme-muted", base)
            }
        }>
            <div class=move || {
                let base = "w-2 h-2 rounded-full";
                if app_state.connected.get() {
                    format!("{} bg-success", base)
                } else {
                    format!("{} bg-theme-muted", base)
                }
            } />
            {move || if app_state.connected.get() { "Connected" } else { "Disconnected" }}
        </div>
    }
}

/// Theme toggle button
#[component]
fn ThemeToggleButton() -> impl IntoView {
    let app_state = expect_context::<AppState>();
    
    view! {
        <button
            class="p-2 rounded-lg text-theme-secondary hover:text-theme hover:bg-theme-surface-hover transition-colors"
            on:click=move |_| {
                app_state.theme.update(|t| {
                    *t = match t {
                        Theme::Dark => Theme::Light,
                        Theme::Light => Theme::System,
                        Theme::System => Theme::Dark,
                    };
                });
            }
            title=move || format!("Theme: {:?}", app_state.theme.get())
        >
            {move || match app_state.theme.get() {
                Theme::Dark => view! {
                    <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
                    </svg>
                }.into_view(),
                Theme::Light => view! {
                    <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <circle cx="12" cy="12" r="5" />
                        <line x1="12" y1="1" x2="12" y2="3" />
                        <line x1="12" y1="21" x2="12" y2="23" />
                        <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
                        <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
                        <line x1="1" y1="12" x2="3" y2="12" />
                        <line x1="21" y1="12" x2="23" y2="12" />
                        <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
                        <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
                    </svg>
                }.into_view(),
                Theme::System => view! {
                    <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
                        <line x1="8" y1="21" x2="16" y2="21" />
                        <line x1="12" y1="17" x2="12" y2="21" />
                    </svg>
                }.into_view(),
            }}
        </button>
    }
}

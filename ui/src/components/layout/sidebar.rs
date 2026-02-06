//! Sidebar Navigation Components
//!
//! Contains sidebar components for different sections:
//! - `Sidebar` - Main sidebar (legacy, kept for reference)
//! - `PipelineSidebar` - Sidebar for pipeline editing pages

use leptos::*;
use leptos_router::*;

use crate::state::{AppState, Theme};

// ============================================================================
// Pipeline Sidebar - For Pipeline Editor Pages
// ============================================================================

/// Sidebar for pipeline editing pages with component palette
#[component]
pub fn PipelineSidebar() -> impl IntoView {
    let _app_state = expect_context::<AppState>();
    
    view! {
        <aside class="w-64 bg-theme-surface border-r border-theme-border flex flex-col overflow-hidden flex-shrink-0">
            // Component Palette Header
            <div class="h-12 flex items-center px-4 border-b border-theme-border">
                <span class="text-sm font-semibold text-theme">"Components"</span>
            </div>
            
            // Component categories
            <div class="flex-1 overflow-y-auto custom-scrollbar p-2">
                <PaletteSection title="Sources">
                    <PaletteItem label="Demo Logs" icon="demo" />
                    <PaletteItem label="File" icon="file" />
                    <PaletteItem label="HTTP" icon="http" />
                    <PaletteItem label="Kafka" icon="kafka" />
                    <PaletteItem label="Syslog" icon="syslog" />
                    <PaletteItem label="Socket" icon="socket" />
                </PaletteSection>
                
                <PaletteSection title="Transforms">
                    <PaletteItem label="Filter" icon="filter" />
                    <PaletteItem label="Remap" icon="remap" />
                    <PaletteItem label="Route" icon="route" />
                    <PaletteItem label="Sample" icon="sample" />
                    <PaletteItem label="Aggregate" icon="aggregate" />
                    <PaletteItem label="Dedupe" icon="dedupe" />
                </PaletteSection>
                
                <PaletteSection title="Sinks">
                    <PaletteItem label="Console" icon="console" />
                    <PaletteItem label="File" icon="file" />
                    <PaletteItem label="HTTP" icon="http" />
                    <PaletteItem label="Elasticsearch" icon="elastic" />
                    <PaletteItem label="S3" icon="s3" />
                    <PaletteItem label="Kafka" icon="kafka" />
                </PaletteSection>
            </div>
            
            // Bottom actions
            <div class="p-2 border-t border-theme-border">
                <button class="w-full px-3 py-2 text-sm text-theme-secondary hover:text-theme hover:bg-theme-surface-hover rounded-lg transition-colors flex items-center gap-2">
                    <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="11" cy="11" r="8" />
                        <line x1="21" y1="21" x2="16.65" y2="16.65" />
                    </svg>
                    "Search components..."
                </button>
            </div>
        </aside>
    }
}

/// Palette section with collapsible header
#[component]
fn PaletteSection(
    title: &'static str,
    children: Children,
) -> impl IntoView {
    let (expanded, set_expanded) = create_signal(true);
    let children_view = children();  // Call children once and store
    
    view! {
        <div class="mb-2">
            <button 
                class="w-full flex items-center gap-2 px-2 py-1.5 text-xs font-medium text-theme-muted uppercase tracking-wider hover:text-theme transition-colors"
                on:click=move |_| set_expanded.update(|e| *e = !*e)
            >
                <svg 
                    class=move || format!("w-3 h-3 transition-transform {}", if expanded.get() { "rotate-90" } else { "" })
                    viewBox="0 0 24 24" 
                    fill="none" 
                    stroke="currentColor" 
                    stroke-width="2"
                >
                    <polyline points="9 18 15 12 9 6" />
                </svg>
                {title}
            </button>
            <Show when=move || expanded.get()>
                <div class="space-y-0.5 mt-1">
                    {children_view.clone()}
                </div>
            </Show>
        </div>
    }
}

#[allow(unused)]
/// Draggable component item in palette
#[component]
fn PaletteItem(
    label: &'static str,
    #[prop(default = "")] icon: &'static str,
) -> impl IntoView {
    view! {
        <div 
            class="flex items-center gap-2 px-2 py-1.5 rounded cursor-grab text-sm text-theme-secondary hover:text-theme hover:bg-theme-surface-hover transition-colors"
            draggable="true"
        >
            <div class="w-5 h-5 rounded bg-theme-surface-hover flex items-center justify-center text-xs">
                {&label[0..1]}
            </div>
            {label}
        </div>
    }
}

// ============================================================================
// Legacy Main Sidebar - Kept for Reference
// ============================================================================

/// Main sidebar navigation component
#[component]
pub fn Sidebar() -> impl IntoView {
    let app_state = expect_context::<AppState>();
    
    view! {
        <aside 
            class=move || {
                let base = "h-full bg-theme-surface border-r border-theme-border flex flex-col transition-all duration-200";
                if app_state.sidebar_collapsed.get() {
                    format!("{} w-14", base)
                } else {
                    format!("{} w-60", base)
                }
            }
        >
            // Logo header
            <div class="h-14 flex items-center px-3 border-b border-theme-border">
                <A href="/" class="flex items-center gap-3 text-theme overflow-hidden">
                    <div class="w-8 h-8 bg-gradient-to-br from-blue-500 to-purple-600 rounded-lg flex items-center justify-center flex-shrink-0">
                        <span class="text-white text-sm font-bold">"V"</span>
                    </div>
                    <Show when=move || !app_state.sidebar_collapsed.get()>
                        <span class="text-lg font-bold whitespace-nowrap">"Vectorize"</span>
                    </Show>
                </A>
            </div>
            
            // Main navigation
            <nav class="flex-1 py-2 overflow-y-auto custom-scrollbar">
                <NavSection 
                    label="Main"
                    collapsed=app_state.sidebar_collapsed
                >
                    <NavItem 
                        href="/" 
                        icon=NavIcon::Dashboard 
                        label="Dashboard" 
                        collapsed=app_state.sidebar_collapsed
                        exact=true
                    />
                </NavSection>
                
                <NavSection 
                    label="Fleet"
                    collapsed=app_state.sidebar_collapsed
                >
                    <NavItem 
                        href="/fleets" 
                        icon=NavIcon::Fleet 
                        label="Worker Groups" 
                        collapsed=app_state.sidebar_collapsed
                        exact=false
                    />
                </NavSection>
                
                <NavSection 
                    label="Pipelines"
                    collapsed=app_state.sidebar_collapsed
                >
                    <NavItem 
                        href="/pipelines" 
                        icon=NavIcon::Pipelines 
                        label="Pipelines" 
                        collapsed=app_state.sidebar_collapsed
                        exact=false
                    />
                </NavSection>
                
                <NavSection 
                    label="Observability"
                    collapsed=app_state.sidebar_collapsed
                >
                    <NavItem 
                        href="/observe" 
                        icon=NavIcon::DataExplorer 
                        label="Data Explorer" 
                        collapsed=app_state.sidebar_collapsed
                        exact=true
                    />
                    <NavItem 
                        href="/observe/metrics" 
                        icon=NavIcon::Metrics 
                        label="Metrics" 
                        collapsed=app_state.sidebar_collapsed
                        exact=true
                    />
                    <NavItem 
                        href="/observe/alerts" 
                        icon=NavIcon::Alerts 
                        label="Alerts" 
                        collapsed=app_state.sidebar_collapsed
                        exact=true
                    />
                    <NavItem 
                        href="/observe/audit" 
                        icon=NavIcon::Audit 
                        label="Audit Logs" 
                        collapsed=app_state.sidebar_collapsed
                        exact=true
                    />
                </NavSection>
            </nav>
            
            // Bottom section - Settings, Theme, Collapse
            <div class="border-t border-theme-border p-2 space-y-1">
                <NavItem 
                    href="/settings" 
                    icon=NavIcon::Settings 
                    label="Settings" 
                    collapsed=app_state.sidebar_collapsed
                    exact=false
                />
                
                // Theme toggle
                <ThemeToggle collapsed=app_state.sidebar_collapsed />
                
                // Collapse toggle
                <CollapseToggle collapsed=app_state.sidebar_collapsed />
                
                // Connection status
                <ConnectionStatus collapsed=app_state.sidebar_collapsed />
            </div>
        </aside>
    }
}

/// Navigation section with optional label
#[component]
fn NavSection(
    label: &'static str,
    collapsed: RwSignal<bool>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="px-2 mb-2">
            <Show when=move || !collapsed.get()>
                <div class="px-2 py-1 text-xs font-medium text-theme-muted uppercase tracking-wider">
                    {label}
                </div>
            </Show>
            <div class="space-y-0.5">
                {children()}
            </div>
        </div>
    }
}

/// Navigation item with icon and label
#[component]
fn NavItem(
    href: &'static str,
    icon: NavIcon,
    label: &'static str,
    collapsed: RwSignal<bool>,
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
                
                let base = "flex items-center gap-3 px-2 py-2 rounded-lg transition-colors relative group";
                if is_active {
                    format!("{} bg-accent text-white", base)
                } else {
                    format!("{} text-theme-secondary hover:text-theme hover:bg-theme-surface-hover", base)
                }
            }
        >
            <div class="w-5 h-5 flex items-center justify-center flex-shrink-0">
                {icon.render()}
            </div>
            <Show when=move || !collapsed.get()>
                <span class="text-sm font-medium whitespace-nowrap">{label}</span>
            </Show>
            
            // Tooltip for collapsed state
            <Show when=move || collapsed.get()>
                <div class="absolute left-full ml-2 px-2 py-1 bg-theme-surface border border-theme-border rounded text-sm text-theme whitespace-nowrap opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none z-50 shadow-lg">
                    {label}
                </div>
            </Show>
        </A>
    }
}

/// Theme toggle button
#[component]
fn ThemeToggle(collapsed: RwSignal<bool>) -> impl IntoView {
    let app_state = expect_context::<AppState>();
    
    view! {
        <button
            class="w-full flex items-center gap-3 px-2 py-2 rounded-lg text-theme-secondary hover:text-theme hover:bg-theme-surface-hover transition-colors group relative"
            on:click=move |_| {
                app_state.theme.update(|t| {
                    *t = match t {
                        Theme::Dark => Theme::Light,
                        Theme::Light => Theme::System,
                        Theme::System => Theme::Dark,
                    };
                });
            }
            title=move || if collapsed.get() { Some("Toggle theme") } else { None }
        >
            <div class="w-5 h-5 flex items-center justify-center flex-shrink-0">
                {move || match app_state.theme.get() {
                    Theme::Dark => view! { <MoonIcon class="w-5 h-5" /> }.into_view(),
                    Theme::Light => view! { <SunIcon class="w-5 h-5" /> }.into_view(),
                    Theme::System => view! { <ComputerIcon class="w-5 h-5" /> }.into_view(),
                }}
            </div>
            <Show when=move || !collapsed.get()>
                <span class="text-sm whitespace-nowrap">
                    {move || match app_state.theme.get() {
                        Theme::Dark => "Dark",
                        Theme::Light => "Light",
                        Theme::System => "System",
                    }}
                </span>
            </Show>
            
            // Tooltip for collapsed state
            <Show when=move || collapsed.get()>
                <div class="absolute left-full ml-2 px-2 py-1 bg-theme-surface border border-theme-border rounded text-sm text-theme whitespace-nowrap opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none z-50 shadow-lg">
                    "Theme: " {move || match app_state.theme.get() {
                        Theme::Dark => "Dark",
                        Theme::Light => "Light",
                        Theme::System => "System",
                    }}
                </div>
            </Show>
        </button>
    }
}

/// Collapse toggle button
#[component]
fn CollapseToggle(collapsed: RwSignal<bool>) -> impl IntoView {
    let app_state = expect_context::<AppState>();
    
    view! {
        <button
            class="w-full flex items-center gap-3 px-2 py-2 rounded-lg text-theme-secondary hover:text-theme hover:bg-theme-surface-hover transition-colors"
            on:click=move |_| app_state.sidebar_collapsed.update(|c| *c = !*c)
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
    }
}

/// Connection status indicator
#[component]
fn ConnectionStatus(collapsed: RwSignal<bool>) -> impl IntoView {
    let app_state = expect_context::<AppState>();
    
    view! {
        <Show when=move || app_state.connected.get()>
            <div class=move || {
                let base = "px-2 py-2 rounded-lg bg-theme-surface-hover";
                if collapsed.get() {
                    format!("{} flex justify-center", base)
                } else {
                    base.to_string()
                }
            }>
                <Show
                    when=move || !collapsed.get()
                    fallback=move || view! {
                        <div class="w-2 h-2 rounded-full bg-success" title="Connected" />
                    }
                >
                    <div class="flex items-center gap-2 text-xs text-theme-secondary">
                        <div class="w-2 h-2 rounded-full bg-success flex-shrink-0" />
                        <span class="truncate">"Connected"</span>
                    </div>
                </Show>
            </div>
        </Show>
    }
}

// ============================================================================
// Navigation Icons
// ============================================================================

#[derive(Clone, Copy)]
pub enum NavIcon {
    Dashboard,
    Fleet,
    Pipelines,
    DataExplorer,
    Metrics,
    Alerts,
    Audit,
    Settings,
}

impl NavIcon {
    fn render(self) -> impl IntoView {
        match self {
            NavIcon::Dashboard => view! { <DashboardIcon class="w-5 h-5" /> }.into_view(),
            NavIcon::Fleet => view! { <FleetIcon class="w-5 h-5" /> }.into_view(),
            NavIcon::Pipelines => view! { <PipelinesIcon class="w-5 h-5" /> }.into_view(),
            NavIcon::DataExplorer => view! { <DataExplorerIcon class="w-5 h-5" /> }.into_view(),
            NavIcon::Metrics => view! { <MetricsIcon class="w-5 h-5" /> }.into_view(),
            NavIcon::Alerts => view! { <AlertsIcon class="w-5 h-5" /> }.into_view(),
            NavIcon::Audit => view! { <AuditIcon class="w-5 h-5" /> }.into_view(),
            NavIcon::Settings => view! { <SettingsIcon class="w-5 h-5" /> }.into_view(),
        }
    }
}

// Icon Components

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
fn DataExplorerIcon(#[prop(default = "w-5 h-5")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <ellipse cx="12" cy="5" rx="9" ry="3" />
            <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3" />
            <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
        </svg>
    }
}

#[component]
fn MetricsIcon(#[prop(default = "w-5 h-5")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="20" x2="18" y2="10" />
            <line x1="12" y1="20" x2="12" y2="4" />
            <line x1="6" y1="20" x2="6" y2="14" />
        </svg>
    }
}

#[component]
fn AlertsIcon(#[prop(default = "w-5 h-5")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" />
            <path d="M13.73 21a2 2 0 0 1-3.46 0" />
        </svg>
    }
}

#[component]
fn AuditIcon(#[prop(default = "w-5 h-5")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
            <line x1="16" y1="13" x2="8" y2="13" />
            <line x1="16" y1="17" x2="8" y2="17" />
            <polyline points="10 9 9 9 8 9" />
        </svg>
    }
}

#[component]
fn SettingsIcon(#[prop(default = "w-5 h-5")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="3" />
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
    }
}

#[component]
fn SunIcon(#[prop(default = "w-5 h-5")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
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
    }
}

#[component]
fn MoonIcon(#[prop(default = "w-5 h-5")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
        </svg>
    }
}

#[component]
fn ComputerIcon(#[prop(default = "w-5 h-5")] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
            <line x1="8" y1="21" x2="16" y2="21" />
            <line x1="12" y1="17" x2="12" y2="21" />
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

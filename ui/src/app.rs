//! Root Application Component
//!
//! This module contains the main App component that sets up:
//! - Routing with new information architecture
//! - Global state providers
//! - Layout structure with new AppShell

use leptos::*;
use leptos_router::*;

use crate::components::layout::AppShell;
use crate::components::dashboard::Dashboard;
use crate::components::management::{WorkerGroupsList, WorkerGroupDetail, WorkerGroupDetailPanel};
use crate::components::pipeline::PipelineView;
use crate::components::setup::SetupWizard;
use crate::components::tap::TapViewer;
use crate::state::AppState;

/// Root application component
#[component]
pub fn App() -> impl IntoView {
    // Initialize global state
    let app_state = AppState::new();
    provide_context(app_state.clone());
    
    // Auto-connect when running inside Vectorize (works on any route)
    {
        let app_state = app_state.clone();
        create_effect(move |prev_run: Option<bool>| {
            // Only run once
            if prev_run.is_some() {
                return true;
            }
            
            // Skip if already connected
            if app_state.connected.get() {
                return true;
            }
            
            let app_state = app_state.clone();
            spawn_local(async move {
                // Try to detect if we're running inside Vectorize
                match gloo_net::http::Request::get("/api/info")
                    .send()
                    .await
                {
                    Ok(response) if response.ok() => {
                        // We're inside Vectorize - auto-connect via proxy
                        let proxy_url = format!("{}/api", 
                            web_sys::window()
                                .and_then(|w| w.location().origin().ok())
                                .unwrap_or_else(|| "http://localhost:8080".to_string())
                        );
                        
                        if let Err(e) = app_state.connect_direct(&proxy_url).await {
                            web_sys::console::error_1(&format!("Auto-connect failed: {}", e).into());
                        } else {
                            web_sys::console::log_1(&"Auto-connected to Vectorize".into());
                        }
                    }
                    _ => {
                        // Not running inside Vectorize - that's fine, user can connect manually
                        web_sys::console::log_1(&"Not running inside Vectorize, manual connection required".into());
                    }
                }
            });
            
            true
        });
    }
    
    // Save UI preferences when they change
    {
        let app_state = app_state.clone();
        create_effect(move |_| {
            // Track changes to UI state
            let _ = app_state.theme.get();
            let _ = app_state.sidebar_collapsed.get();
            let _ = app_state.bottom_panel_height.get();
            
            // Save to localStorage
            app_state.save_ui_preferences();
        });
    }
    
    view! {
        <Router>
            <Routes>
                // Setup page - no shell
                <Route path="/setup" view=SetupPage />
                
                // Dashboard (home) - with shell
                <Route path="/" view=|| view! { <MainLayout><Dashboard /></MainLayout> } />
                
                // ====================================================================
                // Fleet Management (Worker Groups)
                // ====================================================================
                <Route path="/fleets" view=|| view! { <MainLayout><FleetsPage /></MainLayout> } />
                <Route path="/fleets/:group_id" view=|| view! { <MainLayout><FleetDetailPage /></MainLayout> } />
                <Route path="/fleets/:group_id/pipeline" view=|| view! { <PipelineLayout><FleetPipelinePage /></PipelineLayout> } />
                <Route path="/fleets/:group_id/history" view=|| view! { <MainLayout><FleetHistoryPage /></MainLayout> } />
                
                // ====================================================================
                // Pipelines - with sidebar and bottom panel
                // ====================================================================
                <Route path="/pipelines" view=|| view! { <MainLayout><PipelineLibraryPage /></MainLayout> } />
                <Route path="/pipelines/new" view=|| view! { <PipelineLayout><PipelineBuilderPage /></PipelineLayout> } />
                <Route path="/pipelines/:id" view=|| view! { <PipelineLayout><PipelineBuilderPage /></PipelineLayout> } />
                
                // ====================================================================
                // Observability - with bottom panel for data preview
                // ====================================================================
                <Route path="/observe" view=|| view! { <ObserveLayout><DataExplorerPage /></ObserveLayout> } />
                <Route path="/observe/metrics" view=|| view! { <MainLayout><MetricsPage /></MainLayout> } />
                <Route path="/observe/alerts" view=|| view! { <MainLayout><AlertsPage /></MainLayout> } />
                <Route path="/observe/audit" view=|| view! { <MainLayout><AuditPage /></MainLayout> } />
                
                // ====================================================================
                // Settings
                // ====================================================================
                <Route path="/settings" view=|| view! { <MainLayout><SettingsPage /></MainLayout> } />
                <Route path="/settings/users" view=|| view! { <MainLayout><UsersPage /></MainLayout> } />
                <Route path="/settings/roles" view=|| view! { <MainLayout><RolesPage /></MainLayout> } />
                <Route path="/settings/api-keys" view=|| view! { <MainLayout><ApiKeysPage /></MainLayout> } />
                <Route path="/settings/sso" view=|| view! { <MainLayout><SsoPage /></MainLayout> } />
                <Route path="/settings/git" view=|| view! { <MainLayout><GitSyncPage /></MainLayout> } />
                <Route path="/settings/system" view=|| view! { <MainLayout><SystemPage /></MainLayout> } />
                
                // Catch-all for 404
                <Route path="/*" view=|| view! { <MainLayout><NotFoundPage /></MainLayout> } />
            </Routes>
        </Router>
    }
}

/// Main layout wrapper - no sidebar, no bottom panel (Dashboard, Fleet, Settings)
#[component]
fn MainLayout(children: Children) -> impl IntoView {
    view! {
        <AppShell show_sidebar=false show_bottom_panel=false>
            {children()}
        </AppShell>
    }
}

/// Pipeline layout - with bottom panel (sidebar is built into PipelineView)
#[component]
fn PipelineLayout(children: Children) -> impl IntoView {
    view! {
        <AppShell show_sidebar=false show_bottom_panel=true>
            {children()}
        </AppShell>
    }
}

/// Observability layout - with bottom panel, no sidebar
#[component]
fn ObserveLayout(children: Children) -> impl IntoView {
    view! {
        <AppShell show_sidebar=false show_bottom_panel=true>
            {children()}
        </AppShell>
    }
}

// ============================================================================
// Fleet Pages
// ============================================================================

/// Fleet management - Worker Groups with split panel layout
#[component]
fn FleetsPage() -> impl IntoView {
    let (selected_group, set_selected_group) = create_signal(Option::<String>::None);
    
    view! {
        <div class="flex-1 flex overflow-hidden">
            // Left panel - Groups list (fixed width)
            <div class="w-80 flex-shrink-0 border-r border-theme-border overflow-hidden">
                <WorkerGroupsList
                    on_select=move |id| set_selected_group.set(Some(id))
                />
            </div>
            
            // Right panel - Group details (flex grow)
            <div class="flex-1 overflow-hidden">
                {move || {
                    if let Some(group_id) = selected_group.get() {
                        view! {
                            <WorkerGroupDetailPanel group_id=group_id />
                        }.into_view()
                    } else {
                        // Placeholder when no group selected
                        view! {
                            <div class="h-full flex items-center justify-center bg-theme-bg">
                                <div class="text-center">
                                    <div class="w-16 h-16 rounded-full bg-theme-surface flex items-center justify-center mx-auto mb-4">
                                        <svg class="w-8 h-8 text-theme-secondary" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                                            <rect x="3" y="3" width="7" height="7" rx="1" />
                                            <rect x="14" y="3" width="7" height="7" rx="1" />
                                            <rect x="3" y="14" width="7" height="7" rx="1" />
                                            <rect x="14" y="14" width="7" height="7" rx="1" />
                                        </svg>
                                    </div>
                                    <h3 class="text-lg font-medium text-theme mb-2">"Select a Worker Group"</h3>
                                    <p class="text-sm text-theme-secondary max-w-xs">
                                        "Choose a worker group from the list to view its agents, configuration, and deployment status."
                                    </p>
                                </div>
                            </div>
                        }.into_view()
                    }
                }}
            </div>
        </div>
    }
}

/// Fleet detail page
#[component]
fn FleetDetailPage() -> impl IntoView {
    let params = use_params_map();
    let group_id = move || params.get().get("group_id").cloned().unwrap_or_default();
    
    view! {
        <WorkerGroupDetail
            group_id=group_id()
            on_back=move |_| {
                let navigate = use_navigate();
                navigate("/fleets", Default::default());
            }
        />
    }
}

/// Fleet pipeline page
#[component]
fn FleetPipelinePage() -> impl IntoView {
    view! {
        <PipelineView />
    }
}

/// Fleet history page
#[component]
fn FleetHistoryPage() -> impl IntoView {
    view! {
        <PlaceholderPage 
            title="Configuration History"
            description="View and compare configuration versions"
            phase="Phase 2"
        />
    }
}

// ============================================================================
// Pipeline Pages
// ============================================================================

/// Pipeline library
#[component]
fn PipelineLibraryPage() -> impl IntoView {
    view! {
        <div class="flex-1 overflow-auto p-6">
            <div class="max-w-7xl mx-auto">
                <div class="flex items-center justify-between mb-6">
                    <div>
                        <h1 class="text-2xl font-bold text-theme">"Pipelines"</h1>
                        <p class="text-theme-secondary mt-1">"Manage your data pipeline configurations"</p>
                    </div>
                    <a href="/pipelines/new" class="btn-primary">"New Pipeline"</a>
                </div>
                
                <div class="bg-theme-surface rounded-xl border border-theme-border p-8 text-center">
                    <div class="w-16 h-16 rounded-full bg-theme-surface-hover flex items-center justify-center mx-auto mb-4">
                        <svg class="w-8 h-8 text-theme-muted" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M12 2L2 7l10 5 10-5-10-5z" />
                            <path d="M2 17l10 5 10-5" />
                            <path d="M2 12l10 5 10-5" />
                        </svg>
                    </div>
                    <p class="text-theme-secondary">"Pipeline library coming in Phase 3"</p>
                    <a href="/pipelines/new" class="text-accent hover:text-accent/80 mt-2 inline-block">
                        "Create your first pipeline →"
                    </a>
                </div>
            </div>
        </div>
    }
}

/// Pipeline builder (new or edit)
#[component]
fn PipelineBuilderPage() -> impl IntoView {
    view! {
        <PipelineView />
    }
}

// ============================================================================
// Observability Pages
// ============================================================================

/// Data explorer
#[component]
fn DataExplorerPage() -> impl IntoView {
    view! {
        <TapViewer />
    }
}

/// Metrics dashboard
#[component]
fn MetricsPage() -> impl IntoView {
    view! {
        <PlaceholderPage 
            title="Metrics"
            description="Aggregated metrics across your fleet"
            phase="Phase 5"
        />
    }
}

/// Alerts management
#[component]
fn AlertsPage() -> impl IntoView {
    view! {
        <crate::components::observe::AlertsManagement />
    }
}

/// Audit logs
#[component]
fn AuditPage() -> impl IntoView {
    view! {
        <PlaceholderPage 
            title="Audit Logs"
            description="View all administrative actions"
            phase="Phase 5"
        />
    }
}

// ============================================================================
// Settings Pages
// ============================================================================

/// Settings main - User profile
#[component]
fn SettingsPage() -> impl IntoView {
    view! {
        <div class="flex-1 overflow-auto p-6">
            <div class="max-w-7xl mx-auto">
                <div class="mb-6">
                    <h1 class="text-2xl font-bold text-theme">"Settings"</h1>
                    <p class="text-theme-secondary mt-1">"Manage users, roles, and system configuration"</p>
                </div>
                
                // Settings navigation cards
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                    <SettingsCard 
                        href="/settings/users"
                        title="Users"
                        description="Manage user accounts"
                        icon=SettingsIcon::Users
                    />
                    <SettingsCard 
                        href="/settings/roles"
                        title="Roles & Permissions"
                        description="Configure access control"
                        icon=SettingsIcon::Roles
                    />
                    <SettingsCard 
                        href="/settings/api-keys"
                        title="API Keys"
                        description="Manage API keys for automation"
                        icon=SettingsIcon::ApiKeys
                    />
                    <SettingsCard 
                        href="/settings/sso"
                        title="Single Sign-On"
                        description="Configure OIDC and SAML"
                        icon=SettingsIcon::Sso
                    />
                    <SettingsCard 
                        href="/settings/git"
                        title="Git Sync"
                        description="Configure remote Git repositories"
                        icon=SettingsIcon::Git
                    />
                    <SettingsCard 
                        href="/settings/system"
                        title="System"
                        description="General system settings"
                        icon=SettingsIcon::System
                    />
                </div>
            </div>
        </div>
    }
}

/// Users management
#[component]
fn UsersPage() -> impl IntoView {
    view! {
        <PlaceholderPage 
            title="Users"
            description="Manage user accounts and permissions"
            phase="Phase 6"
        />
    }
}

/// Roles management
#[component]
fn RolesPage() -> impl IntoView {
    view! {
        <PlaceholderPage 
            title="Roles & Permissions"
            description="Configure role-based access control"
            phase="Phase 6"
        />
    }
}

/// API keys management
#[component]
fn ApiKeysPage() -> impl IntoView {
    view! {
        <PlaceholderPage 
            title="API Keys"
            description="Create and manage API keys"
            phase="Phase 6"
        />
    }
}

/// SSO configuration
#[component]
fn SsoPage() -> impl IntoView {
    view! {
        <PlaceholderPage 
            title="Single Sign-On"
            description="Configure OIDC and SAML providers"
            phase="Phase 6"
        />
    }
}

/// Git sync configuration
#[component]
fn GitSyncPage() -> impl IntoView {
    view! {
        <PlaceholderPage 
            title="Git Sync"
            description="Configure remote Git repositories for config sync"
            phase="Phase 6"
        />
    }
}

/// System settings
#[component]
fn SystemPage() -> impl IntoView {
    view! {
        <PlaceholderPage 
            title="System Settings"
            description="General system configuration"
            phase="Phase 6"
        />
    }
}

// ============================================================================
// Utility Components
// ============================================================================

/// Setup page - wraps the setup wizard
#[component]
fn SetupPage() -> impl IntoView {
    view! {
        <main class="h-screen flex flex-col bg-theme-bg text-theme">
            <SetupWizard />
        </main>
    }
}

/// 404 Not Found page
#[component]
fn NotFoundPage() -> impl IntoView {
    view! {
        <div class="flex-1 flex items-center justify-center p-6">
            <div class="text-center">
                <h1 class="text-6xl font-bold text-theme-muted mb-4">"404"</h1>
                <p class="text-xl text-theme-secondary mb-6">"Page not found"</p>
                <a href="/" class="btn-primary">"Go to Dashboard"</a>
            </div>
        </div>
    }
}

/// Generic placeholder page for features in development
#[component]
fn PlaceholderPage(
    title: &'static str,
    description: &'static str,
    phase: &'static str,
) -> impl IntoView {
    view! {
        <div class="flex-1 flex items-center justify-center p-6">
            <div class="text-center">
                <div class="w-16 h-16 rounded-full bg-theme-surface-hover flex items-center justify-center mx-auto mb-4">
                    <svg class="w-8 h-8 text-theme-muted" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="12" cy="12" r="10" />
                        <polyline points="12 6 12 12 16 14" />
                    </svg>
                </div>
                <h2 class="text-xl font-semibold text-theme mb-2">{title}</h2>
                <p class="text-theme-secondary mb-4">{description}</p>
                <span class="inline-block px-3 py-1 bg-theme-surface-hover text-theme-muted text-sm rounded-full">
                    "Coming in " {phase}
                </span>
            </div>
        </div>
    }
}

/// Settings card component
#[component]
fn SettingsCard(
    href: &'static str,
    title: &'static str,
    description: &'static str,
    icon: SettingsIcon,
) -> impl IntoView {
    view! {
        <a 
            href=href 
            class="block p-4 bg-theme-surface border border-theme-border rounded-xl hover:border-accent/50 transition-colors group"
        >
            <div class="flex items-start gap-4">
                <div class="w-10 h-10 rounded-lg bg-theme-surface-hover flex items-center justify-center flex-shrink-0 group-hover:bg-accent/10 transition-colors">
                    {icon.render()}
                </div>
                <div>
                    <h3 class="font-medium text-theme group-hover:text-accent transition-colors">{title}</h3>
                    <p class="text-sm text-theme-secondary mt-1">{description}</p>
                </div>
            </div>
        </a>
    }
}

#[derive(Clone, Copy)]
enum SettingsIcon {
    Users,
    Roles,
    ApiKeys,
    Sso,
    Git,
    System,
}

impl SettingsIcon {
    fn render(self) -> impl IntoView {
        match self {
            SettingsIcon::Users => view! {
                <svg class="w-5 h-5 text-theme-secondary group-hover:text-accent transition-colors" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
                    <circle cx="9" cy="7" r="4" />
                    <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
                    <path d="M16 3.13a4 4 0 0 1 0 7.75" />
                </svg>
            }.into_view(),
            SettingsIcon::Roles => view! {
                <svg class="w-5 h-5 text-theme-secondary group-hover:text-accent transition-colors" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
                </svg>
            }.into_view(),
            SettingsIcon::ApiKeys => view! {
                <svg class="w-5 h-5 text-theme-secondary group-hover:text-accent transition-colors" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" />
                </svg>
            }.into_view(),
            SettingsIcon::Sso => view! {
                <svg class="w-5 h-5 text-theme-secondary group-hover:text-accent transition-colors" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
                    <path d="M7 11V7a5 5 0 0 1 10 0v4" />
                </svg>
            }.into_view(),
            SettingsIcon::Git => view! {
                <svg class="w-5 h-5 text-theme-secondary group-hover:text-accent transition-colors" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="4" />
                    <line x1="1.05" y1="12" x2="7" y2="12" />
                    <line x1="17.01" y1="12" x2="22.96" y2="12" />
                </svg>
            }.into_view(),
            SettingsIcon::System => view! {
                <svg class="w-5 h-5 text-theme-secondary group-hover:text-accent transition-colors" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="3" />
                    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
                </svg>
            }.into_view(),
        }
    }
}

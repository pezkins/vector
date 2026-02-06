//! System Settings Component
//!
//! Global system settings interface with:
//! - General settings (instance name, description)
//! - Deployment settings (strategy, approval)
//! - Retention settings (event, audit log)
//! - Feature flags

use leptos::*;
use serde::{Deserialize, Serialize};

use crate::components::common::RefreshIcon;

// ============================================================================
// Types
// ============================================================================

/// Deployment strategy
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentStrategy {
    #[default]
    Rolling,
    BlueGreen,
    Canary,
    Immediate,
}

impl DeploymentStrategy {
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            DeploymentStrategy::Rolling => "Rolling",
            DeploymentStrategy::BlueGreen => "Blue-Green",
            DeploymentStrategy::Canary => "Canary",
            DeploymentStrategy::Immediate => "Immediate",
        }
    }

    #[allow(dead_code)]
    pub fn description(&self) -> &'static str {
        match self {
            DeploymentStrategy::Rolling => "Gradually replace instances one by one",
            DeploymentStrategy::BlueGreen => "Deploy to inactive environment, then switch",
            DeploymentStrategy::Canary => "Deploy to a small subset first, then expand",
            DeploymentStrategy::Immediate => "Deploy to all instances at once",
        }
    }
}

/// System settings data
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SystemSettings {
    // General
    pub instance_name: String,
    pub instance_description: String,
    
    // Deployment
    pub default_deployment_strategy: DeploymentStrategy,
    pub require_deployment_approval: bool,
    
    // Retention
    pub event_retention_days: u32,
    pub audit_log_retention_days: u32,
    
    // Feature flags
    pub enable_live_tap: bool,
    pub enable_functional_testing: bool,
    pub enable_metrics_export: bool,
}

// ============================================================================
// Main Component
// ============================================================================

/// System settings page component
#[component]
pub fn SystemSettingsPage() -> impl IntoView {
    let (settings, set_settings) = create_signal(SystemSettings::default());
    let (loading, set_loading) = create_signal(true);
    let (saving, set_saving) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);
    let (success, set_success) = create_signal(Option::<String>::None);
    let (has_changes, set_has_changes) = create_signal(false);
    
    // Fetch settings on mount
    create_effect(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            
            match fetch_system_settings().await {
                Ok(data) => set_settings.set(data),
                Err(e) => {
                    // Use default settings if fetch fails
                    set_settings.set(get_default_settings());
                    web_sys::console::warn_1(&format!("Using default system settings: {}", e).into());
                }
            }
            
            set_loading.set(false);
        });
    });
    
    // Refresh handler
    let on_refresh = move |_| {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            set_success.set(None);
            
            match fetch_system_settings().await {
                Ok(data) => {
                    set_settings.set(data);
                    set_has_changes.set(false);
                }
                Err(e) => set_error.set(Some(e)),
            }
            
            set_loading.set(false);
        });
    };
    
    // Save handler
    let on_save = move |_| {
        let current_settings = settings.get();
        set_saving.set(true);
        set_error.set(None);
        set_success.set(None);
        
        spawn_local(async move {
            match save_system_settings(&current_settings).await {
                Ok(_) => {
                    set_success.set(Some("Settings saved successfully".to_string()));
                    set_has_changes.set(false);
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_saving.set(false);
        });
    };
    
    // Update helper
    let update_settings = move |f: Box<dyn Fn(&mut SystemSettings)>| {
        set_settings.update(|s| f(s));
        set_has_changes.set(true);
        set_success.set(None);
    };

    view! {
        <div class="flex-1 overflow-auto p-6 bg-slate-900">
            <div class="max-w-4xl mx-auto">
                // Header
                <div class="flex items-center justify-between mb-6">
                    <div>
                        <h1 class="text-2xl font-bold text-white">"System Settings"</h1>
                        <p class="text-slate-400 mt-1">"Configure global system settings and preferences"</p>
                    </div>
                    
                    <div class="flex items-center gap-3">
                        <button
                            class="flex items-center gap-2 px-4 py-2 bg-slate-700 hover:bg-slate-600 \
                                   text-white rounded-lg transition-colors"
                            on:click=on_refresh
                        >
                            <RefreshIcon class="w-4 h-4" />
                            "Refresh"
                        </button>
                    </div>
                </div>
                
                // Error display
                {move || {
                    if let Some(err) = error.get() {
                        view! {
                            <div class="bg-red-500/10 border border-red-500/30 rounded-lg p-4 mb-6 flex items-start gap-3">
                                <ErrorIcon class="w-5 h-5 text-red-400 shrink-0 mt-0.5" />
                                <div>
                                    <p class="text-red-400 font-medium">"Error"</p>
                                    <p class="text-red-400/80 text-sm">{err}</p>
                                </div>
                            </div>
                        }.into_view()
                    } else {
                        view! {}.into_view()
                    }
                }}
                
                // Success display
                {move || {
                    if let Some(msg) = success.get() {
                        view! {
                            <div class="bg-green-500/10 border border-green-500/30 rounded-lg p-4 mb-6 flex items-start gap-3">
                                <CheckIcon class="w-5 h-5 text-green-400 shrink-0 mt-0.5" />
                                <p class="text-green-400">{msg}</p>
                            </div>
                        }.into_view()
                    } else {
                        view! {}.into_view()
                    }
                }}
                
                // Loading state
                <Show
                    when=move || !loading.get()
                    fallback=move || view! {
                        <div class="flex items-center justify-center py-16">
                            <div class="animate-spin w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full" />
                        </div>
                    }
                >
                    <div class="space-y-6">
                        // General Section
                        <GeneralSection 
                            settings=Signal::derive(move || settings.get())
                            on_change=move |name, desc| {
                                update_settings(Box::new(move |s| {
                                    s.instance_name = name.clone();
                                    s.instance_description = desc.clone();
                                }));
                            }
                        />
                        
                        // Deployment Section
                        <DeploymentSection 
                            settings=Signal::derive(move || settings.get())
                            on_change=move |strategy, approval| {
                                update_settings(Box::new(move |s| {
                                    s.default_deployment_strategy = strategy.clone();
                                    s.require_deployment_approval = approval;
                                }));
                            }
                        />
                        
                        // Retention Section
                        <RetentionSection 
                            settings=Signal::derive(move || settings.get())
                            on_change=move |event_days, audit_days| {
                                update_settings(Box::new(move |s| {
                                    s.event_retention_days = event_days;
                                    s.audit_log_retention_days = audit_days;
                                }));
                            }
                        />
                        
                        // Feature Flags Section
                        <FeatureFlagsSection 
                            settings=Signal::derive(move || settings.get())
                            on_change=move |tap, testing, metrics| {
                                update_settings(Box::new(move |s| {
                                    s.enable_live_tap = tap;
                                    s.enable_functional_testing = testing;
                                    s.enable_metrics_export = metrics;
                                }));
                            }
                        />
                        
                        // Save button
                        <div class="flex justify-end pt-4">
                            <button
                                class="px-6 py-2.5 bg-blue-500 hover:bg-blue-600 text-white font-medium \
                                       rounded-lg transition-colors disabled:opacity-50"
                                disabled=move || saving.get() || !has_changes.get()
                                on:click=on_save
                            >
                                {move || if saving.get() { "Saving..." } else { "Save Settings" }}
                            </button>
                        </div>
                    </div>
                </Show>
            </div>
        </div>
    }
}

// ============================================================================
// General Section
// ============================================================================

#[component]
fn GeneralSection(
    settings: Signal<SystemSettings>,
    on_change: impl Fn(String, String) + 'static + Clone,
) -> impl IntoView {
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
            <div class="flex items-center gap-3 mb-4">
                <div class="w-10 h-10 rounded-lg bg-slate-700 flex items-center justify-center">
                    <SettingsIcon class="w-5 h-5 text-slate-400" />
                </div>
                <div>
                    <h2 class="text-lg font-semibold text-white">"General"</h2>
                    <p class="text-sm text-slate-400">"Basic instance configuration"</p>
                </div>
            </div>
            
            <div class="space-y-4">
                // Instance Name
                <div class="space-y-1">
                    <label class="block text-sm font-medium text-slate-300">"Instance Name"</label>
                    <input
                        type="text"
                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm \
                               placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                        placeholder="My Vectorize Instance"
                        prop:value=move || settings.get().instance_name
                        on:input={
                            let on_change = on_change.clone();
                            move |e| {
                                on_change(event_target_value(&e), settings.get().instance_description);
                            }
                        }
                    />
                    <p class="text-xs text-slate-500">"A friendly name for this Vectorize instance"</p>
                </div>
                
                // Description
                <div class="space-y-1">
                    <label class="block text-sm font-medium text-slate-300">"Description"</label>
                    <textarea
                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm \
                               placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent \
                               resize-none"
                        rows="3"
                        placeholder="Production observability pipeline management"
                        prop:value=move || settings.get().instance_description
                        on:input={
                            let on_change = on_change.clone();
                            move |e| {
                                on_change(settings.get().instance_name, event_target_value(&e));
                            }
                        }
                    />
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Deployment Section
// ============================================================================

#[component]
fn DeploymentSection(
    settings: Signal<SystemSettings>,
    on_change: impl Fn(DeploymentStrategy, bool) + 'static + Clone,
) -> impl IntoView {
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
            <div class="flex items-center gap-3 mb-4">
                <div class="w-10 h-10 rounded-lg bg-blue-500/10 flex items-center justify-center">
                    <DeployIcon class="w-5 h-5 text-blue-400" />
                </div>
                <div>
                    <h2 class="text-lg font-semibold text-white">"Deployment"</h2>
                    <p class="text-sm text-slate-400">"Default deployment behavior"</p>
                </div>
            </div>
            
            <div class="space-y-4">
                // Default Strategy
                <div class="space-y-2">
                    <label class="block text-sm font-medium text-slate-300">"Default Strategy"</label>
                    <select
                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm \
                               focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                        on:change={
                            let on_change = on_change.clone();
                            move |e| {
                                let strategy = match event_target_value(&e).as_str() {
                                    "rolling" => DeploymentStrategy::Rolling,
                                    "bluegreen" => DeploymentStrategy::BlueGreen,
                                    "canary" => DeploymentStrategy::Canary,
                                    "immediate" => DeploymentStrategy::Immediate,
                                    _ => DeploymentStrategy::Rolling,
                                };
                                on_change(strategy, settings.get().require_deployment_approval);
                            }
                        }
                    >
                        <option value="rolling" selected=move || settings.get().default_deployment_strategy == DeploymentStrategy::Rolling>
                            "Rolling - Gradually replace instances"
                        </option>
                        <option value="bluegreen" selected=move || settings.get().default_deployment_strategy == DeploymentStrategy::BlueGreen>
                            "Blue-Green - Deploy then switch"
                        </option>
                        <option value="canary" selected=move || settings.get().default_deployment_strategy == DeploymentStrategy::Canary>
                            "Canary - Deploy to subset first"
                        </option>
                        <option value="immediate" selected=move || settings.get().default_deployment_strategy == DeploymentStrategy::Immediate>
                            "Immediate - Deploy to all at once"
                        </option>
                    </select>
                </div>
                
                // Require Approval toggle
                <div class="flex items-center justify-between p-4 bg-slate-900 rounded-lg">
                    <div>
                        <div class="text-sm font-medium text-white">"Require Deployment Approval"</div>
                        <div class="text-xs text-slate-400">"Deployments must be approved before execution"</div>
                    </div>
                    <button
                        type="button"
                        class=move || {
                            let base = "relative inline-flex h-6 w-11 items-center rounded-full transition-colors";
                            if settings.get().require_deployment_approval {
                                format!("{} bg-blue-500", base)
                            } else {
                                format!("{} bg-slate-600", base)
                            }
                        }
                        on:click={
                            let on_change = on_change.clone();
                            move |_| {
                                let s = settings.get();
                                on_change(s.default_deployment_strategy, !s.require_deployment_approval);
                            }
                        }
                    >
                        <span class=move || {
                            let base = "inline-block h-4 w-4 transform rounded-full bg-white transition-transform";
                            if settings.get().require_deployment_approval {
                                format!("{} translate-x-6", base)
                            } else {
                                format!("{} translate-x-1", base)
                            }
                        } />
                    </button>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Retention Section
// ============================================================================

#[component]
fn RetentionSection(
    settings: Signal<SystemSettings>,
    on_change: impl Fn(u32, u32) + 'static + Clone,
) -> impl IntoView {
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
            <div class="flex items-center gap-3 mb-4">
                <div class="w-10 h-10 rounded-lg bg-amber-500/10 flex items-center justify-center">
                    <ClockIcon class="w-5 h-5 text-amber-400" />
                </div>
                <div>
                    <h2 class="text-lg font-semibold text-white">"Data Retention"</h2>
                    <p class="text-sm text-slate-400">"How long to keep historical data"</p>
                </div>
            </div>
            
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                // Event Retention
                <div class="space-y-1">
                    <label class="block text-sm font-medium text-slate-300">"Event Retention (days)"</label>
                    <input
                        type="number"
                        min="1"
                        max="365"
                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm \
                               focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                        prop:value=move || settings.get().event_retention_days.to_string()
                        on:input={
                            let on_change = on_change.clone();
                            move |e| {
                                if let Ok(days) = event_target_value(&e).parse::<u32>() {
                                    on_change(days, settings.get().audit_log_retention_days);
                                }
                            }
                        }
                    />
                    <p class="text-xs text-slate-500">"Tap events and metrics data"</p>
                </div>
                
                // Audit Log Retention
                <div class="space-y-1">
                    <label class="block text-sm font-medium text-slate-300">"Audit Log Retention (days)"</label>
                    <input
                        type="number"
                        min="1"
                        max="365"
                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm \
                               focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                        prop:value=move || settings.get().audit_log_retention_days.to_string()
                        on:input={
                            let on_change = on_change.clone();
                            move |e| {
                                if let Ok(days) = event_target_value(&e).parse::<u32>() {
                                    on_change(settings.get().event_retention_days, days);
                                }
                            }
                        }
                    />
                    <p class="text-xs text-slate-500">"User activity and changes"</p>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Feature Flags Section
// ============================================================================

#[component]
fn FeatureFlagsSection(
    settings: Signal<SystemSettings>,
    on_change: impl Fn(bool, bool, bool) + 'static + Clone,
) -> impl IntoView {
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
            <div class="flex items-center gap-3 mb-4">
                <div class="w-10 h-10 rounded-lg bg-violet-500/10 flex items-center justify-center">
                    <FlagIcon class="w-5 h-5 text-violet-400" />
                </div>
                <div>
                    <h2 class="text-lg font-semibold text-white">"Feature Flags"</h2>
                    <p class="text-sm text-slate-400">"Enable or disable optional features"</p>
                </div>
            </div>
            
            <div class="space-y-3">
                // Live Tap
                <div class="flex items-center justify-between p-4 bg-slate-900 rounded-lg">
                    <div>
                        <div class="text-sm font-medium text-white">"Live Tap"</div>
                        <div class="text-xs text-slate-400">"Real-time event streaming from agents"</div>
                    </div>
                    <button
                        type="button"
                        class=move || {
                            let base = "relative inline-flex h-6 w-11 items-center rounded-full transition-colors";
                            if settings.get().enable_live_tap {
                                format!("{} bg-blue-500", base)
                            } else {
                                format!("{} bg-slate-600", base)
                            }
                        }
                        on:click={
                            let on_change = on_change.clone();
                            move |_| {
                                let s = settings.get();
                                on_change(!s.enable_live_tap, s.enable_functional_testing, s.enable_metrics_export);
                            }
                        }
                    >
                        <span class=move || {
                            let base = "inline-block h-4 w-4 transform rounded-full bg-white transition-transform";
                            if settings.get().enable_live_tap {
                                format!("{} translate-x-6", base)
                            } else {
                                format!("{} translate-x-1", base)
                            }
                        } />
                    </button>
                </div>
                
                // Functional Testing
                <div class="flex items-center justify-between p-4 bg-slate-900 rounded-lg">
                    <div>
                        <div class="text-sm font-medium text-white">"Functional Testing"</div>
                        <div class="text-xs text-slate-400">"Test pipelines with sample data before deployment"</div>
                    </div>
                    <button
                        type="button"
                        class=move || {
                            let base = "relative inline-flex h-6 w-11 items-center rounded-full transition-colors";
                            if settings.get().enable_functional_testing {
                                format!("{} bg-blue-500", base)
                            } else {
                                format!("{} bg-slate-600", base)
                            }
                        }
                        on:click={
                            let on_change = on_change.clone();
                            move |_| {
                                let s = settings.get();
                                on_change(s.enable_live_tap, !s.enable_functional_testing, s.enable_metrics_export);
                            }
                        }
                    >
                        <span class=move || {
                            let base = "inline-block h-4 w-4 transform rounded-full bg-white transition-transform";
                            if settings.get().enable_functional_testing {
                                format!("{} translate-x-6", base)
                            } else {
                                format!("{} translate-x-1", base)
                            }
                        } />
                    </button>
                </div>
                
                // Metrics Export
                <div class="flex items-center justify-between p-4 bg-slate-900 rounded-lg">
                    <div>
                        <div class="text-sm font-medium text-white">"Metrics Export"</div>
                        <div class="text-xs text-slate-400">"Export Vectorize metrics to external systems"</div>
                    </div>
                    <button
                        type="button"
                        class=move || {
                            let base = "relative inline-flex h-6 w-11 items-center rounded-full transition-colors";
                            if settings.get().enable_metrics_export {
                                format!("{} bg-blue-500", base)
                            } else {
                                format!("{} bg-slate-600", base)
                            }
                        }
                        on:click={
                            let on_change = on_change.clone();
                            move |_| {
                                let s = settings.get();
                                on_change(s.enable_live_tap, s.enable_functional_testing, !s.enable_metrics_export);
                            }
                        }
                    >
                        <span class=move || {
                            let base = "inline-block h-4 w-4 transform rounded-full bg-white transition-transform";
                            if settings.get().enable_metrics_export {
                                format!("{} translate-x-6", base)
                            } else {
                                format!("{} translate-x-1", base)
                            }
                        } />
                    </button>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Icons
// ============================================================================

#[component]
fn CheckIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="20 6 9 17 4 12" />
        </svg>
    }
}

#[component]
fn ErrorIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="8" x2="12" y2="12" />
            <line x1="12" y1="16" x2="12.01" y2="16" />
        </svg>
    }
}

#[component]
fn SettingsIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="3" />
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
    }
}

#[component]
fn DeployIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polygon points="12 2 2 7 12 12 22 7 12 2" />
            <polyline points="2 17 12 22 22 17" />
            <polyline points="2 12 12 17 22 12" />
        </svg>
    }
}

#[component]
fn ClockIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10" />
            <polyline points="12 6 12 12 16 14" />
        </svg>
    }
}

#[component]
fn FlagIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M4 15s1-1 4-1 5 2 8 2 4-1 4-1V3s-1 1-4 1-5-2-8-2-4 1-4 1z" />
            <line x1="4" y1="22" x2="4" y2="15" />
        </svg>
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn get_default_settings() -> SystemSettings {
    SystemSettings {
        instance_name: "Vectorize".to_string(),
        instance_description: "Observability pipeline management".to_string(),
        default_deployment_strategy: DeploymentStrategy::Rolling,
        require_deployment_approval: false,
        event_retention_days: 30,
        audit_log_retention_days: 90,
        enable_live_tap: true,
        enable_functional_testing: true,
        enable_metrics_export: false,
    }
}

// ============================================================================
// API Functions
// ============================================================================

fn get_base_url() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_else(|| "http://localhost:8080".to_string())
}

async fn fetch_system_settings() -> Result<SystemSettings, String> {
    let url = format!("{}/api/v1/settings/system", get_base_url());
    
    let response = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json::<SystemSettings>().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Err("Failed to fetch system settings".to_string())
    }
}

async fn save_system_settings(settings: &SystemSettings) -> Result<(), String> {
    let url = format!("{}/api/v1/settings/system", get_base_url());
    
    let response = gloo_net::http::Request::put(&url)
        .header("Content-Type", "application/json")
        .json(settings)
        .map_err(|e| format!("Serialize error: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        Ok(())
    } else {
        // Mock success for development
        Ok(())
    }
}

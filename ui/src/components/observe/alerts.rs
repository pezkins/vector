//! Alerts Management Component
//!
//! Full alerts management interface with:
//! - Alert Rules tab: Create, edit, delete, and toggle alert rules
//! - Notification Channels tab: Configure webhook, Slack, PagerDuty, email channels

use leptos::*;
use serde::{Deserialize, Serialize};

use crate::components::common::{PlusIcon, TrashIcon, RefreshIcon};

// ============================================================================
// Types
// ============================================================================

/// Alert severity levels
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Critical,
    Warning,
    #[default]
    Info,
}

impl AlertSeverity {
    pub fn label(&self) -> &'static str {
        match self {
            AlertSeverity::Critical => "Critical",
            AlertSeverity::Warning => "Warning",
            AlertSeverity::Info => "Info",
        }
    }

    pub fn badge_class(&self) -> &'static str {
        match self {
            AlertSeverity::Critical => "bg-red-500/20 text-red-400",
            AlertSeverity::Warning => "bg-amber-500/20 text-amber-400",
            AlertSeverity::Info => "bg-blue-500/20 text-blue-400",
        }
    }

    #[allow(dead_code)]
    pub fn all() -> &'static [AlertSeverity] {
        &[AlertSeverity::Critical, AlertSeverity::Warning, AlertSeverity::Info]
    }
}

/// Alert rule definition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub severity: AlertSeverity,
    pub enabled: bool,
    pub channels: Vec<String>,
}

/// Notification channel types
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelType {
    #[default]
    Webhook,
    Slack,
    PagerDuty,
    Email,
}

impl ChannelType {
    pub fn label(&self) -> &'static str {
        match self {
            ChannelType::Webhook => "Webhook",
            ChannelType::Slack => "Slack",
            ChannelType::PagerDuty => "PagerDuty",
            ChannelType::Email => "Email",
        }
    }

    pub fn all() -> &'static [ChannelType] {
        &[ChannelType::Webhook, ChannelType::Slack, ChannelType::PagerDuty, ChannelType::Email]
    }
}

/// Notification channel definition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationChannel {
    pub id: String,
    pub name: String,
    pub channel_type: ChannelType,
    pub config: ChannelConfig,
}

/// Channel-specific configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChannelConfig {
    Webhook { url: String },
    Slack { webhook_url: String, channel: String },
    PagerDuty { routing_key: String },
    Email { recipients: Vec<String> },
}

impl Default for ChannelConfig {
    fn default() -> Self {
        ChannelConfig::Webhook { url: String::new() }
    }
}

/// Tabs for the alerts management interface
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlertsTab {
    #[default]
    Rules,
    Channels,
}

// ============================================================================
// Main Component
// ============================================================================

/// Main alerts management component
#[component]
pub fn AlertsManagement() -> impl IntoView {
    let (active_tab, set_active_tab) = create_signal(AlertsTab::Rules);
    let (rules, set_rules) = create_signal(Vec::<AlertRule>::new());
    let (channels, set_channels) = create_signal(Vec::<NotificationChannel>::new());
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal(Option::<String>::None);
    
    // Modal state
    let (show_rule_modal, set_show_rule_modal) = create_signal(false);
    let (show_channel_modal, set_show_channel_modal) = create_signal(false);
    
    // Fetch data on mount
    create_effect(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            
            // Fetch rules
            match fetch_rules().await {
                Ok(data) => set_rules.set(data),
                Err(e) => set_error.set(Some(e)),
            }
            
            // Fetch channels (ignore errors)
            if let Ok(data) = fetch_channels().await {
                set_channels.set(data);
            }
            
            set_loading.set(false);
        });
    });
    
    // Refresh handler
    let on_refresh = move |_| {
        spawn_local(async move {
            set_loading.set(true);
            
            if let Ok(data) = fetch_rules().await {
                set_rules.set(data);
            }
            if let Ok(data) = fetch_channels().await {
                set_channels.set(data);
            }
            
            set_loading.set(false);
        });
    };
    
    // Rule toggle handler
    let toggle_rule = move |rule_id: String, enabled: bool| {
        spawn_local(async move {
            if toggle_rule_enabled(&rule_id, enabled).await.is_ok() {
                set_rules.update(|rules| {
                    if let Some(rule) = rules.iter_mut().find(|r| r.id == rule_id) {
                        rule.enabled = enabled;
                    }
                });
            }
        });
    };
    
    // Delete rule handler
    let delete_rule = move |rule_id: String| {
        spawn_local(async move {
            if delete_rule_api(&rule_id).await.is_ok() {
                set_rules.update(|rules| {
                    rules.retain(|r| r.id != rule_id);
                });
            }
        });
    };
    
    // Delete channel handler
    let delete_channel = move |channel_id: String| {
        spawn_local(async move {
            if delete_channel_api(&channel_id).await.is_ok() {
                set_channels.update(|channels| {
                    channels.retain(|c| c.id != channel_id);
                });
            }
        });
    };
    
    // Test channel handler
    let test_channel = move |channel_id: String| {
        spawn_local(async move {
            let _ = test_channel_api(&channel_id).await;
        });
    };
    
    // Create rule handler
    let on_create_rule = move |rule: AlertRule| {
        set_rules.update(|rules| rules.push(rule));
        set_show_rule_modal.set(false);
    };
    
    // Create channel handler
    let on_create_channel = move |channel: NotificationChannel| {
        set_channels.update(|channels| channels.push(channel));
        set_show_channel_modal.set(false);
    };

    view! {
        <div class="flex-1 overflow-auto p-6 bg-slate-900">
            <div class="max-w-7xl mx-auto">
                // Header
                <div class="flex items-center justify-between mb-6">
                    <div>
                        <h1 class="text-2xl font-bold text-white">"Alerts Management"</h1>
                        <p class="text-slate-400 mt-1">"Configure alert rules and notification channels"</p>
                    </div>
                    
                    <button
                        class="flex items-center gap-2 px-4 py-2 bg-slate-700 hover:bg-slate-600 \
                               text-white rounded-lg transition-colors"
                        on:click=on_refresh
                    >
                        <RefreshIcon class="w-4 h-4" />
                        "Refresh"
                    </button>
                </div>
                
                // Tabs
                <div class="flex gap-1 mb-6 bg-slate-800 rounded-lg p-1 w-fit border border-slate-700">
                    <TabButton
                        label="Alert Rules"
                        active=move || active_tab.get() == AlertsTab::Rules
                        on_click=move |_| set_active_tab.set(AlertsTab::Rules)
                    />
                    <TabButton
                        label="Notification Channels"
                        active=move || active_tab.get() == AlertsTab::Channels
                        on_click=move |_| set_active_tab.set(AlertsTab::Channels)
                    />
                </div>
                
                // Error display
                {move || {
                    if let Some(err) = error.get() {
                        view! {
                            <div class="bg-red-500/10 border border-red-500/30 rounded-lg p-4 mb-6">
                                <p class="text-red-400">{err}</p>
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
                    // Tab content
                    {move || {
                        match active_tab.get() {
                            AlertsTab::Rules => view! {
                                <RulesTab 
                                    rules=rules 
                                    channels=channels
                                    on_create=move |_| set_show_rule_modal.set(true)
                                    on_toggle=toggle_rule
                                    on_delete=delete_rule
                                />
                            }.into_view(),
                            AlertsTab::Channels => view! {
                                <ChannelsTab 
                                    channels=channels 
                                    on_create=move |_| set_show_channel_modal.set(true)
                                    on_test=test_channel
                                    on_delete=delete_channel
                                />
                            }.into_view(),
                        }
                    }}
                </Show>
                
                // Create Rule Modal
                <Show when=move || show_rule_modal.get()>
                    <CreateRuleModal
                        channels=channels
                        on_close=move || set_show_rule_modal.set(false)
                        on_save=on_create_rule
                    />
                </Show>
                
                // Create Channel Modal
                <Show when=move || show_channel_modal.get()>
                    <CreateChannelModal
                        on_close=move || set_show_channel_modal.set(false)
                        on_save=on_create_channel
                    />
                </Show>
            </div>
        </div>
    }
}

// ============================================================================
// Tab Button Component
// ============================================================================

#[component]
fn TabButton<F>(
    label: &'static str,
    active: F,
    on_click: impl Fn(ev::MouseEvent) + 'static,
) -> impl IntoView
where
    F: Fn() -> bool + 'static,
{
    view! {
        <button
            class=move || {
                if active() {
                    "px-4 py-2 text-sm font-medium rounded-md bg-blue-500 text-white transition-colors"
                } else {
                    "px-4 py-2 text-sm font-medium rounded-md text-slate-400 hover:text-white transition-colors"
                }
            }
            on:click=on_click
        >
            {label}
        </button>
    }
}

// ============================================================================
// Rules Tab
// ============================================================================

#[component]
fn RulesTab(
    rules: ReadSignal<Vec<AlertRule>>,
    channels: ReadSignal<Vec<NotificationChannel>>,
    on_create: impl Fn(ev::MouseEvent) + 'static,
    on_toggle: impl Fn(String, bool) + Clone + 'static,
    on_delete: impl Fn(String) + Clone + 'static,
) -> impl IntoView {
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 overflow-hidden">
            // Header
            <div class="flex items-center justify-between px-6 py-4 border-b border-slate-700">
                <div>
                    <h2 class="text-lg font-semibold text-white">"Alert Rules"</h2>
                    <p class="text-sm text-slate-400 mt-0.5">"Define conditions that trigger alerts"</p>
                </div>
                <button
                    class="flex items-center gap-2 px-4 py-2 bg-blue-500 hover:bg-blue-600 \
                           text-white text-sm font-medium rounded-lg transition-colors"
                    on:click=on_create
                >
                    <PlusIcon class="w-4 h-4" />
                    "Create Rule"
                </button>
            </div>
            
            // Table or empty state
            {move || {
                let items = rules.get();
                let channel_list = channels.get();
                
                if items.is_empty() {
                    view! {
                        <EmptyState
                            icon="bell"
                            title="No Alert Rules"
                            description="Create your first alert rule to start monitoring your pipeline."
                        />
                    }.into_view()
                } else {
                    let on_toggle = on_toggle.clone();
                    let on_delete = on_delete.clone();
                    
                    view! {
                        <div class="overflow-x-auto">
                            <table class="w-full">
                                <thead class="bg-slate-800/50">
                                    <tr>
                                        <th class="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Name"</th>
                                        <th class="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Severity"</th>
                                        <th class="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Status"</th>
                                        <th class="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Channels"</th>
                                        <th class="px-6 py-3 text-right text-xs font-medium text-slate-400 uppercase tracking-wider">"Actions"</th>
                                    </tr>
                                </thead>
                                <tbody class="divide-y divide-slate-700">
                                    {items.into_iter().map(|rule| {
                                        let rule_id_toggle = rule.id.clone();
                                        let rule_id_delete = rule.id.clone();
                                        let enabled = rule.enabled;
                                        let on_toggle = on_toggle.clone();
                                        let on_delete = on_delete.clone();
                                        
                                        // Get channel names for display
                                        let channel_names: Vec<String> = rule.channels.iter()
                                            .filter_map(|id| channel_list.iter().find(|c| &c.id == id).map(|c| c.name.clone()))
                                            .collect();
                                        
                                        view! {
                                            <tr class="hover:bg-slate-700/30 transition-colors">
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    <div>
                                                        <div class="text-sm font-medium text-white">{rule.name.clone()}</div>
                                                        {rule.description.map(|d| view! {
                                                            <div class="text-xs text-slate-500 mt-0.5">{d}</div>
                                                        })}
                                                    </div>
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    <span class=format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}", rule.severity.badge_class())>
                                                        {rule.severity.label()}
                                                    </span>
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    <ToggleSwitch
                                                        enabled=enabled
                                                        on_toggle=move |new_state| {
                                                            let id = rule_id_toggle.clone();
                                                            on_toggle(id, new_state);
                                                        }
                                                    />
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    {if channel_names.is_empty() {
                                                        view! {
                                                            <span class="text-sm text-slate-500">"None"</span>
                                                        }.into_view()
                                                    } else {
                                                        view! {
                                                            <div class="flex flex-wrap gap-1">
                                                                {channel_names.into_iter().map(|name| view! {
                                                                    <span class="inline-flex items-center px-2 py-0.5 rounded-md text-xs bg-slate-700 text-slate-300">
                                                                        {name}
                                                                    </span>
                                                                }).collect::<Vec<_>>()}
                                                            </div>
                                                        }.into_view()
                                                    }}
                                                </td>
                                                <td class="px-6 py-4 whitespace-nowrap text-right">
                                                    <div class="flex items-center justify-end gap-2">
                                                        <button
                                                            class="p-1.5 text-slate-400 hover:text-white hover:bg-slate-700 rounded transition-colors"
                                                            title="Edit rule"
                                                        >
                                                            <EditIcon class="w-4 h-4" />
                                                        </button>
                                                        <button
                                                            class="p-1.5 text-slate-400 hover:text-red-400 hover:bg-red-500/10 rounded transition-colors"
                                                            title="Delete rule"
                                                            on:click=move |_| {
                                                                let id = rule_id_delete.clone();
                                                                on_delete(id);
                                                            }
                                                        >
                                                            <TrashIcon class="w-4 h-4" />
                                                        </button>
                                                    </div>
                                                </td>
                                            </tr>
                                        }
                                    }).collect::<Vec<_>>()}
                                </tbody>
                            </table>
                        </div>
                    }.into_view()
                }
            }}
        </div>
    }
}

// ============================================================================
// Channels Tab
// ============================================================================

#[component]
fn ChannelsTab(
    channels: ReadSignal<Vec<NotificationChannel>>,
    on_create: impl Fn(ev::MouseEvent) + 'static,
    on_test: impl Fn(String) + Clone + 'static,
    on_delete: impl Fn(String) + Clone + 'static,
) -> impl IntoView {
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 overflow-hidden">
            // Header
            <div class="flex items-center justify-between px-6 py-4 border-b border-slate-700">
                <div>
                    <h2 class="text-lg font-semibold text-white">"Notification Channels"</h2>
                    <p class="text-sm text-slate-400 mt-0.5">"Configure where alerts are sent"</p>
                </div>
                <button
                    class="flex items-center gap-2 px-4 py-2 bg-blue-500 hover:bg-blue-600 \
                           text-white text-sm font-medium rounded-lg transition-colors"
                    on:click=on_create
                >
                    <PlusIcon class="w-4 h-4" />
                    "Add Channel"
                </button>
            </div>
            
            // Grid or empty state
            {move || {
                let items = channels.get();
                
                if items.is_empty() {
                    view! {
                        <EmptyState
                            icon="channel"
                            title="No Notification Channels"
                            description="Add a channel to receive alert notifications via Webhook, Slack, PagerDuty, or Email."
                        />
                    }.into_view()
                } else {
                    let on_test = on_test.clone();
                    let on_delete = on_delete.clone();
                    
                    view! {
                        <div class="p-6 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                            {items.into_iter().map(|channel| {
                                let channel_id_test = channel.id.clone();
                                let channel_id_delete = channel.id.clone();
                                let on_test = on_test.clone();
                                let on_delete = on_delete.clone();
                                
                                view! {
                                    <ChannelCard
                                        channel=channel
                                        on_test=move || {
                                            let id = channel_id_test.clone();
                                            on_test(id);
                                        }
                                        on_delete=move || {
                                            let id = channel_id_delete.clone();
                                            on_delete(id);
                                        }
                                    />
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_view()
                }
            }}
        </div>
    }
}

// ============================================================================
// Channel Card
// ============================================================================

#[component]
fn ChannelCard(
    channel: NotificationChannel,
    on_test: impl Fn() + 'static,
    on_delete: impl Fn() + 'static,
) -> impl IntoView {
    let (testing, set_testing) = create_signal(false);
    
    let handle_test = move |_| {
        set_testing.set(true);
        on_test();
        // Reset after 2 seconds
        set_timeout(move || set_testing.set(false), std::time::Duration::from_secs(2));
    };
    
    view! {
        <div class="bg-slate-700/30 rounded-lg border border-slate-700 p-4 hover:border-slate-600 transition-colors">
            // Header with icon and type
            <div class="flex items-start justify-between mb-3">
                <div class="flex items-center gap-3">
                    <div class="w-10 h-10 rounded-lg bg-slate-700 flex items-center justify-center">
                        <ChannelTypeIcon channel_type=channel.channel_type />
                    </div>
                    <div>
                        <h3 class="text-sm font-medium text-white">{channel.name.clone()}</h3>
                        <p class="text-xs text-slate-400">{channel.channel_type.label()}</p>
                    </div>
                </div>
            </div>
            
            // Actions
            <div class="flex items-center gap-2 mt-4 pt-3 border-t border-slate-700">
                <button
                    class="flex-1 flex items-center justify-center gap-2 px-3 py-1.5 text-sm \
                           bg-slate-700 hover:bg-slate-600 text-slate-300 hover:text-white \
                           rounded-md transition-colors disabled:opacity-50"
                    on:click=handle_test
                    disabled=move || testing.get()
                >
                    {move || if testing.get() { "Sending..." } else { "Test" }}
                </button>
                <button
                    class="p-1.5 text-slate-400 hover:text-red-400 hover:bg-red-500/10 \
                           rounded-md transition-colors"
                    title="Delete channel"
                    on:click=move |_| on_delete()
                >
                    <TrashIcon class="w-4 h-4" />
                </button>
            </div>
        </div>
    }
}

// ============================================================================
// Toggle Switch
// ============================================================================

#[component]
fn ToggleSwitch(
    enabled: bool,
    on_toggle: impl Fn(bool) + 'static,
) -> impl IntoView {
    let (is_enabled, set_is_enabled) = create_signal(enabled);
    
    let handle_click = move |_| {
        let new_state = !is_enabled.get();
        set_is_enabled.set(new_state);
        on_toggle(new_state);
    };
    
    view! {
        <button
            class=move || {
                let base = "relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-slate-800";
                if is_enabled.get() {
                    format!("{} bg-blue-500", base)
                } else {
                    format!("{} bg-slate-600", base)
                }
            }
            on:click=handle_click
            role="switch"
            aria-checked=move || is_enabled.get().to_string()
        >
            <span
                class=move || {
                    let base = "inline-block h-4 w-4 transform rounded-full bg-white transition-transform";
                    if is_enabled.get() {
                        format!("{} translate-x-6", base)
                    } else {
                        format!("{} translate-x-1", base)
                    }
                }
            />
        </button>
    }
}

// ============================================================================
// Empty State
// ============================================================================

#[component]
fn EmptyState(
    icon: &'static str,
    title: &'static str,
    description: &'static str,
) -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center py-16">
            <div class="w-16 h-16 rounded-full bg-slate-700 flex items-center justify-center mb-6">
                {match icon {
                    "bell" => view! {
                        <svg class="w-8 h-8 text-amber-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" />
                            <path d="M13.73 21a2 2 0 0 1-3.46 0" />
                        </svg>
                    }.into_view(),
                    "channel" => view! {
                        <svg class="w-8 h-8 text-blue-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M22 17H2a3 3 0 0 0 3-3V9a7 7 0 0 1 14 0v5a3 3 0 0 0 3 3Z" />
                            <path d="M8 21h8" />
                            <path d="M12 17v4" />
                        </svg>
                    }.into_view(),
                    _ => view! {}.into_view(),
                }}
            </div>
            <h2 class="text-xl font-semibold text-white mb-2">{title}</h2>
            <p class="text-slate-400 text-center max-w-md">{description}</p>
        </div>
    }
}

// ============================================================================
// Icons
// ============================================================================

#[component]
fn EditIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
            <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
        </svg>
    }
}

#[component]
fn ChannelTypeIcon(channel_type: ChannelType) -> impl IntoView {
    match channel_type {
        ChannelType::Webhook => view! {
            <svg class="w-5 h-5 text-violet-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10" />
                <line x1="2" y1="12" x2="22" y2="12" />
                <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
            </svg>
        }.into_view(),
        ChannelType::Slack => view! {
            <svg class="w-5 h-5 text-green-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M14.5 2c1.38 0 2.5 1.12 2.5 2.5v3c0 1.38-1.12 2.5-2.5 2.5H11V4.5C11 3.12 12.12 2 13.5 2h1z" />
                <path d="M9.5 22c-1.38 0-2.5-1.12-2.5-2.5v-3c0-1.38 1.12-2.5 2.5-2.5H13v5.5c0 1.38-1.12 2.5-2.5 2.5h-1z" />
                <path d="M22 14.5c0 1.38-1.12 2.5-2.5 2.5h-3c-1.38 0-2.5-1.12-2.5-2.5V11h5.5c1.38 0 2.5 1.12 2.5 2.5v1z" />
                <path d="M2 9.5c0-1.38 1.12-2.5 2.5-2.5h3C8.88 7 10 8.12 10 9.5V13H4.5C3.12 13 2 11.88 2 10.5v-1z" />
            </svg>
        }.into_view(),
        ChannelType::PagerDuty => view! {
            <svg class="w-5 h-5 text-orange-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M22 12h-4l-3 9L9 3l-3 9H2" />
            </svg>
        }.into_view(),
        ChannelType::Email => view! {
            <svg class="w-5 h-5 text-cyan-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" />
                <polyline points="22,6 12,13 2,6" />
            </svg>
        }.into_view(),
    }
}

// ============================================================================
// Create Rule Modal
// ============================================================================

#[component]
fn CreateRuleModal(
    channels: ReadSignal<Vec<NotificationChannel>>,
    on_close: impl Fn() + 'static + Clone,
    on_save: impl Fn(AlertRule) + 'static + Clone,
) -> impl IntoView {
    let (name, set_name) = create_signal(String::new());
    let (description, set_description) = create_signal(String::new());
    let (severity, set_severity) = create_signal(AlertSeverity::Warning);
    let (selected_channels, set_selected_channels) = create_signal(Vec::<String>::new());
    let (saving, set_saving) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);
    
    let on_close_backdrop = on_close.clone();
    let on_close_header = on_close.clone();
    let on_close_cancel = on_close;
    let on_save_clone = on_save;
    
    let handle_save = move |_| {
        let rule_name = name.get();
        if rule_name.is_empty() {
            set_error.set(Some("Name is required".to_string()));
            return;
        }
        
        set_saving.set(true);
        set_error.set(None);
        
        let rule = AlertRule {
            id: uuid::Uuid::new_v4().to_string(),
            name: rule_name,
            description: {
                let d = description.get();
                if d.is_empty() { None } else { Some(d) }
            },
            severity: severity.get(),
            enabled: true,
            channels: selected_channels.get(),
        };
        
        let rule_clone = rule.clone();
        let on_save = on_save_clone.clone();
        spawn_local(async move {
            match create_rule_api(&rule).await {
                Ok(_) => {
                    on_save(rule_clone);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_saving.set(false);
                }
            }
        });
    };
    
    let toggle_channel = move |channel_id: String| {
        set_selected_channels.update(|channels| {
            if channels.contains(&channel_id) {
                channels.retain(|id| id != &channel_id);
            } else {
                channels.push(channel_id);
            }
        });
    };
    
    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center">
            // Backdrop
            <div 
                class="absolute inset-0 bg-black/60 backdrop-blur-sm"
                on:click=move |_| on_close_backdrop()
            />
            
            // Modal
            <div class="relative bg-slate-800 rounded-xl border border-slate-700 shadow-2xl w-full max-w-lg mx-4">
                // Header
                <div class="flex items-center justify-between px-6 py-4 border-b border-slate-700">
                    <h2 class="text-lg font-semibold text-white">"Create Alert Rule"</h2>
                    <button
                        class="p-1 text-slate-400 hover:text-white rounded transition-colors"
                        on:click=move |_| on_close_header()
                    >
                        <CloseIcon class="w-5 h-5" />
                    </button>
                </div>
                
                // Body
                <div class="p-6 space-y-4">
                    // Error display
                    {move || {
                        if let Some(err) = error.get() {
                            view! {
                                <div class="bg-red-500/10 border border-red-500/30 rounded-lg p-3 mb-4">
                                    <p class="text-sm text-red-400">{err}</p>
                                </div>
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                    
                    // Name field
                    <div class="space-y-1">
                        <label class="block text-sm font-medium text-slate-300">"Name"</label>
                        <input
                            type="text"
                            class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 \
                                   text-white text-sm placeholder-slate-500 \
                                   focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                            placeholder="e.g., High Error Rate Alert"
                            prop:value=move || name.get()
                            on:input=move |e| set_name.set(event_target_value(&e))
                        />
                    </div>
                    
                    // Description field
                    <div class="space-y-1">
                        <label class="block text-sm font-medium text-slate-300">
                            "Description "
                            <span class="text-slate-500 font-normal">"(optional)"</span>
                        </label>
                        <textarea
                            class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 \
                                   text-white text-sm placeholder-slate-500 resize-none \
                                   focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                            rows=2
                            placeholder="Brief description of when this alert triggers"
                            prop:value=move || description.get()
                            on:input=move |e| set_description.set(event_target_value(&e))
                        />
                    </div>
                    
                    // Severity field
                    <div class="space-y-1">
                        <label class="block text-sm font-medium text-slate-300">"Severity"</label>
                        <select
                            class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 \
                                   text-white text-sm \
                                   focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                            on:change=move |e| {
                                let value = event_target_value(&e);
                                let sev = match value.as_str() {
                                    "critical" => AlertSeverity::Critical,
                                    "warning" => AlertSeverity::Warning,
                                    _ => AlertSeverity::Info,
                                };
                                set_severity.set(sev);
                            }
                        >
                            <option value="warning" selected=move || severity.get() == AlertSeverity::Warning>"Warning"</option>
                            <option value="critical" selected=move || severity.get() == AlertSeverity::Critical>"Critical"</option>
                            <option value="info" selected=move || severity.get() == AlertSeverity::Info>"Info"</option>
                        </select>
                    </div>
                    
                    // Channels multi-select
                    <div class="space-y-1">
                        <label class="block text-sm font-medium text-slate-300">"Notification Channels"</label>
                        {move || {
                            let channel_list = channels.get();
                            if channel_list.is_empty() {
                                view! {
                                    <p class="text-sm text-slate-500 py-2">"No channels configured. Add a channel first."</p>
                                }.into_view()
                            } else {
                                view! {
                                    <div class="space-y-2 max-h-32 overflow-y-auto">
                                        {channel_list.into_iter().map(|channel| {
                                            let channel_id = channel.id.clone();
                                            let channel_id_check = channel.id.clone();
                                            let is_selected = move || selected_channels.get().contains(&channel_id_check);
                                            
                                            view! {
                                                <label class="flex items-center gap-3 p-2 rounded-lg hover:bg-slate-700/50 cursor-pointer">
                                                    <input
                                                        type="checkbox"
                                                        class="w-4 h-4 rounded border-slate-600 bg-slate-900 text-blue-500 \
                                                               focus:ring-blue-500 focus:ring-offset-slate-800"
                                                        checked=is_selected
                                                        on:change=move |_| toggle_channel(channel_id.clone())
                                                    />
                                                    <div class="flex items-center gap-2">
                                                        <ChannelTypeIcon channel_type=channel.channel_type />
                                                        <span class="text-sm text-white">{channel.name}</span>
                                                    </div>
                                                </label>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                }.into_view()
                            }
                        }}
                    </div>
                </div>
                
                // Footer
                <div class="flex items-center justify-end gap-3 px-6 py-4 border-t border-slate-700">
                    <button
                        class="px-4 py-2 text-sm font-medium text-slate-400 hover:text-white \
                               rounded-lg transition-colors"
                        on:click=move |_| on_close_cancel()
                    >
                        "Cancel"
                    </button>
                    <button
                        class="px-4 py-2 text-sm font-medium bg-blue-500 hover:bg-blue-600 \
                               text-white rounded-lg transition-colors disabled:opacity-50"
                        disabled=move || saving.get() || name.get().is_empty()
                        on:click=handle_save
                    >
                        {move || if saving.get() { "Saving..." } else { "Save Rule" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Create Channel Modal
// ============================================================================

#[component]
fn CreateChannelModal(
    on_close: impl Fn() + 'static + Clone,
    on_save: impl Fn(NotificationChannel) + 'static + Clone,
) -> impl IntoView {
    let (channel_type, set_channel_type) = create_signal(ChannelType::Webhook);
    let (name, set_name) = create_signal(String::new());
    let (url, set_url) = create_signal(String::new());
    let (webhook_url, set_webhook_url) = create_signal(String::new());
    let (slack_channel, set_slack_channel) = create_signal(String::new());
    let (routing_key, set_routing_key) = create_signal(String::new());
    let (recipients, set_recipients) = create_signal(String::new());
    let (saving, set_saving) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);
    
    let on_close_backdrop = on_close.clone();
    let on_close_header = on_close.clone();
    let on_close_cancel = on_close;
    let on_save_clone = on_save;
    
    let handle_save = move |_| {
        let channel_name = name.get();
        if channel_name.is_empty() {
            set_error.set(Some("Name is required".to_string()));
            return;
        }
        
        let config = match channel_type.get() {
            ChannelType::Webhook => {
                let url_val = url.get();
                if url_val.is_empty() {
                    set_error.set(Some("URL is required".to_string()));
                    return;
                }
                ChannelConfig::Webhook { url: url_val }
            }
            ChannelType::Slack => {
                let wh_url = webhook_url.get();
                let ch = slack_channel.get();
                if wh_url.is_empty() {
                    set_error.set(Some("Webhook URL is required".to_string()));
                    return;
                }
                ChannelConfig::Slack { 
                    webhook_url: wh_url, 
                    channel: ch 
                }
            }
            ChannelType::PagerDuty => {
                let key = routing_key.get();
                if key.is_empty() {
                    set_error.set(Some("Routing Key is required".to_string()));
                    return;
                }
                ChannelConfig::PagerDuty { routing_key: key }
            }
            ChannelType::Email => {
                let recip = recipients.get();
                if recip.is_empty() {
                    set_error.set(Some("At least one recipient is required".to_string()));
                    return;
                }
                let recipient_list: Vec<String> = recip
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                ChannelConfig::Email { recipients: recipient_list }
            }
        };
        
        set_saving.set(true);
        set_error.set(None);
        
        let channel = NotificationChannel {
            id: uuid::Uuid::new_v4().to_string(),
            name: channel_name,
            channel_type: channel_type.get(),
            config,
        };
        
        let channel_clone = channel.clone();
        let on_save = on_save_clone.clone();
        spawn_local(async move {
            match create_channel_api(&channel).await {
                Ok(_) => {
                    on_save(channel_clone);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_saving.set(false);
                }
            }
        });
    };
    
    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center">
            // Backdrop
            <div 
                class="absolute inset-0 bg-black/60 backdrop-blur-sm"
                on:click=move |_| on_close_backdrop()
            />
            
            // Modal
            <div class="relative bg-slate-800 rounded-xl border border-slate-700 shadow-2xl w-full max-w-lg mx-4">
                // Header
                <div class="flex items-center justify-between px-6 py-4 border-b border-slate-700">
                    <h2 class="text-lg font-semibold text-white">"Add Notification Channel"</h2>
                    <button
                        class="p-1 text-slate-400 hover:text-white rounded transition-colors"
                        on:click=move |_| on_close_header()
                    >
                        <CloseIcon class="w-5 h-5" />
                    </button>
                </div>
                
                // Body
                <div class="p-6 space-y-4">
                    // Error display
                    {move || {
                        if let Some(err) = error.get() {
                            view! {
                                <div class="bg-red-500/10 border border-red-500/30 rounded-lg p-3 mb-4">
                                    <p class="text-sm text-red-400">{err}</p>
                                </div>
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                    
                    // Channel type selector
                    <div class="space-y-1">
                        <label class="block text-sm font-medium text-slate-300">"Channel Type"</label>
                        <div class="grid grid-cols-4 gap-2">
                            {ChannelType::all().iter().map(|&ct| {
                                let is_selected = move || channel_type.get() == ct;
                                view! {
                                    <button
                                        class=move || {
                                            let base = "flex flex-col items-center gap-2 p-3 rounded-lg border transition-colors";
                                            if is_selected() {
                                                format!("{} border-blue-500 bg-blue-500/10", base)
                                            } else {
                                                format!("{} border-slate-700 hover:border-slate-600 bg-slate-900", base)
                                            }
                                        }
                                        on:click=move |_| set_channel_type.set(ct)
                                    >
                                        <ChannelTypeIcon channel_type=ct />
                                        <span class="text-xs text-slate-300">{ct.label()}</span>
                                    </button>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                    
                    // Name field
                    <div class="space-y-1">
                        <label class="block text-sm font-medium text-slate-300">"Name"</label>
                        <input
                            type="text"
                            class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 \
                                   text-white text-sm placeholder-slate-500 \
                                   focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                            placeholder="e.g., Production Alerts"
                            prop:value=move || name.get()
                            on:input=move |e| set_name.set(event_target_value(&e))
                        />
                    </div>
                    
                    // Dynamic fields based on channel type
                    {move || {
                        match channel_type.get() {
                            ChannelType::Webhook => view! {
                                <div class="space-y-1">
                                    <label class="block text-sm font-medium text-slate-300">"URL"</label>
                                    <input
                                        type="url"
                                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 \
                                               text-white text-sm placeholder-slate-500 \
                                               focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                                        placeholder="https://example.com/webhook"
                                        prop:value=move || url.get()
                                        on:input=move |e| set_url.set(event_target_value(&e))
                                    />
                                </div>
                            }.into_view(),
                            ChannelType::Slack => view! {
                                <div class="space-y-4">
                                    <div class="space-y-1">
                                        <label class="block text-sm font-medium text-slate-300">"Webhook URL"</label>
                                        <input
                                            type="url"
                                            class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 \
                                                   text-white text-sm placeholder-slate-500 \
                                                   focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                                            placeholder="https://hooks.slack.com/services/..."
                                            prop:value=move || webhook_url.get()
                                            on:input=move |e| set_webhook_url.set(event_target_value(&e))
                                        />
                                    </div>
                                    <div class="space-y-1">
                                        <label class="block text-sm font-medium text-slate-300">
                                            "Channel "
                                            <span class="text-slate-500 font-normal">"(optional)"</span>
                                        </label>
                                        <input
                                            type="text"
                                            class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 \
                                                   text-white text-sm placeholder-slate-500 \
                                                   focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                                            placeholder="#alerts"
                                            prop:value=move || slack_channel.get()
                                            on:input=move |e| set_slack_channel.set(event_target_value(&e))
                                        />
                                    </div>
                                </div>
                            }.into_view(),
                            ChannelType::PagerDuty => view! {
                                <div class="space-y-1">
                                    <label class="block text-sm font-medium text-slate-300">"Routing Key"</label>
                                    <input
                                        type="text"
                                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 \
                                               text-white text-sm placeholder-slate-500 \
                                               focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                                        placeholder="Enter your PagerDuty routing key"
                                        prop:value=move || routing_key.get()
                                        on:input=move |e| set_routing_key.set(event_target_value(&e))
                                    />
                                </div>
                            }.into_view(),
                            ChannelType::Email => view! {
                                <div class="space-y-1">
                                    <label class="block text-sm font-medium text-slate-300">"Recipients"</label>
                                    <input
                                        type="text"
                                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 \
                                               text-white text-sm placeholder-slate-500 \
                                               focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                                        placeholder="email1@example.com, email2@example.com"
                                        prop:value=move || recipients.get()
                                        on:input=move |e| set_recipients.set(event_target_value(&e))
                                    />
                                    <p class="text-xs text-slate-500">"Separate multiple emails with commas"</p>
                                </div>
                            }.into_view(),
                        }
                    }}
                </div>
                
                // Footer
                <div class="flex items-center justify-end gap-3 px-6 py-4 border-t border-slate-700">
                    <button
                        class="px-4 py-2 text-sm font-medium text-slate-400 hover:text-white \
                               rounded-lg transition-colors"
                        on:click=move |_| on_close_cancel()
                    >
                        "Cancel"
                    </button>
                    <button
                        class="px-4 py-2 text-sm font-medium bg-blue-500 hover:bg-blue-600 \
                               text-white rounded-lg transition-colors disabled:opacity-50"
                        disabled=move || saving.get() || name.get().is_empty()
                        on:click=handle_save
                    >
                        {move || if saving.get() { "Saving..." } else { "Save Channel" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

#[component]
fn CloseIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
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

async fn fetch_rules() -> Result<Vec<AlertRule>, String> {
    let url = format!("{}/api/v1/alerts/rules", get_base_url());
    
    let response = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json::<Vec<AlertRule>>().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        // Return empty for now if endpoint doesn't exist
        Ok(vec![])
    }
}

async fn fetch_channels() -> Result<Vec<NotificationChannel>, String> {
    let url = format!("{}/api/v1/alerts/channels", get_base_url());
    
    let response = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json::<Vec<NotificationChannel>>().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Ok(vec![])
    }
}

async fn create_rule_api(rule: &AlertRule) -> Result<(), String> {
    let url = format!("{}/api/v1/alerts/rules", get_base_url());
    
    let response = gloo_net::http::Request::post(&url)
        .header("Content-Type", "application/json")
        .json(rule)
        .map_err(|e| format!("Serialize error: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        Ok(())
    } else {
        Err(format!("Failed to create rule: {}", response.status()))
    }
}

async fn toggle_rule_enabled(rule_id: &str, enabled: bool) -> Result<(), String> {
    let url = format!("{}/api/v1/alerts/rules/{}", get_base_url(), rule_id);
    
    #[derive(Serialize)]
    struct UpdatePayload {
        enabled: bool,
    }
    
    let response = gloo_net::http::Request::patch(&url)
        .header("Content-Type", "application/json")
        .json(&UpdatePayload { enabled })
        .map_err(|e| format!("Serialize error: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        Ok(())
    } else {
        Err(format!("Failed to update rule: {}", response.status()))
    }
}

async fn delete_rule_api(rule_id: &str) -> Result<(), String> {
    let url = format!("{}/api/v1/alerts/rules/{}", get_base_url(), rule_id);
    
    let response = gloo_net::http::Request::delete(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        Ok(())
    } else {
        Err(format!("Failed to delete rule: {}", response.status()))
    }
}

async fn create_channel_api(channel: &NotificationChannel) -> Result<(), String> {
    let url = format!("{}/api/v1/alerts/channels", get_base_url());
    
    let response = gloo_net::http::Request::post(&url)
        .header("Content-Type", "application/json")
        .json(channel)
        .map_err(|e| format!("Serialize error: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        Ok(())
    } else {
        Err(format!("Failed to create channel: {}", response.status()))
    }
}

async fn test_channel_api(channel_id: &str) -> Result<(), String> {
    let url = format!("{}/api/v1/alerts/channels/{}/test", get_base_url(), channel_id);
    
    let response = gloo_net::http::Request::post(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        Ok(())
    } else {
        Err(format!("Test failed: {}", response.status()))
    }
}

async fn delete_channel_api(channel_id: &str) -> Result<(), String> {
    let url = format!("{}/api/v1/alerts/channels/{}", get_base_url(), channel_id);
    
    let response = gloo_net::http::Request::delete(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        Ok(())
    } else {
        Err(format!("Failed to delete channel: {}", response.status()))
    }
}

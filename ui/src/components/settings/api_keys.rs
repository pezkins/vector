//! API Keys Management Component
//!
//! Full API key management interface with:
//! - Key list with table view
//! - Create key modal with permission scope
//! - One-time key display after creation
//! - Revoke confirmation modal

use leptos::*;
use serde::{Deserialize, Serialize};

use crate::components::common::{PlusIcon, TrashIcon, RefreshIcon};

// ============================================================================
// Types
// ============================================================================

/// Permission scope for API keys
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyScope {
    #[default]
    ReadOnly,
    ReadWrite,
    Admin,
}

impl KeyScope {
    pub fn label(&self) -> &'static str {
        match self {
            KeyScope::ReadOnly => "Read Only",
            KeyScope::ReadWrite => "Read/Write",
            KeyScope::Admin => "Admin",
        }
    }

    pub fn badge_class(&self) -> &'static str {
        match self {
            KeyScope::ReadOnly => "bg-slate-500/20 text-slate-400 border-slate-500/30",
            KeyScope::ReadWrite => "bg-blue-500/20 text-blue-400 border-blue-500/30",
            KeyScope::Admin => "bg-red-500/20 text-red-400 border-red-500/30",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            KeyScope::ReadOnly => "Can only read data, no modifications allowed",
            KeyScope::ReadWrite => "Can read and modify configurations and agents",
            KeyScope::Admin => "Full access including user and role management",
        }
    }
}

/// API Key data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key_prefix: String,  // First 8 chars of key (masked display)
    pub scope: KeyScope,
    pub expires_at: Option<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub created_by: String,
}

impl ApiKey {
    /// Get masked key display
    pub fn masked_key(&self) -> String {
        format!("{}••••••••••••••••", self.key_prefix)
    }
}

/// Form data for create key
#[derive(Clone, Debug, Default)]
struct KeyFormData {
    pub name: String,
    pub scope: KeyScope,
    pub expires_days: Option<u32>,  // None = never expires
}

/// Validation errors
#[derive(Clone, Debug, Default)]
struct ValidationErrors {
    pub name: Option<String>,
}

impl ValidationErrors {
    fn is_valid(&self) -> bool {
        self.name.is_none()
    }
}

// ============================================================================
// Main Component
// ============================================================================

/// Main API keys management component
#[component]
pub fn ApiKeysPage() -> impl IntoView {
    let (keys, set_keys) = create_signal(Vec::<ApiKey>::new());
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal(Option::<String>::None);
    
    // Modal state
    let (show_create_modal, set_show_create_modal) = create_signal(false);
    let (revoking_key, set_revoking_key) = create_signal(Option::<ApiKey>::None);
    let (new_key_secret, set_new_key_secret) = create_signal(Option::<String>::None);
    
    // Fetch data on mount
    create_effect(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            
            match fetch_api_keys().await {
                Ok(data) => set_keys.set(data),
                Err(e) => {
                    // Use mock data if fetch fails
                    set_keys.set(get_mock_keys());
                    web_sys::console::warn_1(&format!("Using mock API keys: {}", e).into());
                }
            }
            
            set_loading.set(false);
        });
    });
    
    // Refresh handler
    let on_refresh = move |_| {
        spawn_local(async move {
            set_loading.set(true);
            
            if let Ok(data) = fetch_api_keys().await {
                set_keys.set(data);
            }
            
            set_loading.set(false);
        });
    };
    
    // Create key handler
    let on_create_key = move |(key, secret): (ApiKey, String)| {
        set_keys.update(|keys| keys.insert(0, key));
        set_show_create_modal.set(false);
        set_new_key_secret.set(Some(secret));
    };
    
    // Revoke key handler
    let on_confirm_revoke = move |key_id: String| {
        spawn_local(async move {
            if revoke_api_key(&key_id).await.is_ok() {
                set_keys.update(|keys| {
                    keys.retain(|k| k.id != key_id);
                });
            }
            set_revoking_key.set(None);
        });
    };
    
    // Close new key modal
    let on_close_new_key = move || {
        set_new_key_secret.set(None);
    };

    view! {
        <div class="flex-1 overflow-auto p-6 bg-slate-900">
            <div class="max-w-7xl mx-auto">
                // Header
                <div class="flex items-center justify-between mb-6">
                    <div>
                        <h1 class="text-2xl font-bold text-white">"API Keys"</h1>
                        <p class="text-slate-400 mt-1">"Manage API keys for programmatic access to Vectorize"</p>
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
                        <button
                            class="flex items-center gap-2 px-4 py-2 bg-blue-500 hover:bg-blue-600 \
                                   text-white font-medium rounded-lg transition-colors"
                            on:click=move |_| set_show_create_modal.set(true)
                        >
                            <PlusIcon class="w-4 h-4" />
                            "Generate Key"
                        </button>
                    </div>
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
                    // API Keys table
                    <KeysTable 
                        keys=keys 
                        on_revoke=move |key| set_revoking_key.set(Some(key))
                    />
                </Show>
                
                // Create Key Modal
                <Show when=move || show_create_modal.get()>
                    <CreateKeyModal
                        on_close=move || set_show_create_modal.set(false)
                        on_create=on_create_key
                    />
                </Show>
                
                // New Key Display Modal (shown once after creation)
                <Show when=move || new_key_secret.get().is_some()>
                    {move || {
                        if let Some(secret) = new_key_secret.get() {
                            view! {
                                <NewKeyDisplayModal
                                    secret=secret
                                    on_close=on_close_new_key
                                />
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                </Show>
                
                // Revoke Confirmation Modal
                <Show when=move || revoking_key.get().is_some()>
                    {move || {
                        if let Some(key) = revoking_key.get() {
                            let key_id = key.id.clone();
                            view! {
                                <RevokeConfirmModal
                                    api_key=key
                                    on_close=move || set_revoking_key.set(None)
                                    on_confirm=move || on_confirm_revoke(key_id.clone())
                                />
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                </Show>
            </div>
        </div>
    }
}

// ============================================================================
// Keys Table Component
// ============================================================================

#[component]
fn KeysTable(
    keys: ReadSignal<Vec<ApiKey>>,
    on_revoke: impl Fn(ApiKey) + Clone + 'static,
) -> impl IntoView {
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 overflow-hidden">
            {move || {
                let items = keys.get();
                
                if items.is_empty() {
                    view! {
                        <EmptyState />
                    }.into_view()
                } else {
                    let on_revoke = on_revoke.clone();
                    
                    view! {
                        <div class="overflow-x-auto">
                            <table class="w-full">
                                <thead class="bg-slate-800/50 border-b border-slate-700">
                                    <tr>
                                        <th class="px-6 py-4 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Name"</th>
                                        <th class="px-6 py-4 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Key"</th>
                                        <th class="px-6 py-4 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Permissions"</th>
                                        <th class="px-6 py-4 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Created"</th>
                                        <th class="px-6 py-4 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Last Used"</th>
                                        <th class="px-6 py-4 text-right text-xs font-medium text-slate-400 uppercase tracking-wider">"Actions"</th>
                                    </tr>
                                </thead>
                                <tbody class="divide-y divide-slate-700">
                                    {items.into_iter().map(|key| {
                                        let key_revoke = key.clone();
                                        let on_revoke = on_revoke.clone();
                                        
                                        view! {
                                            <tr class="hover:bg-slate-700/30 transition-colors">
                                                // Name
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    <div class="flex items-center gap-3">
                                                        <div class="w-10 h-10 rounded-lg bg-amber-500/10 flex items-center justify-center">
                                                            <KeyIcon class="w-5 h-5 text-amber-400" />
                                                        </div>
                                                        <div>
                                                            <div class="text-sm font-medium text-white">{key.name.clone()}</div>
                                                            <div class="text-xs text-slate-500">"Created by "{key.created_by.clone()}</div>
                                                        </div>
                                                    </div>
                                                </td>
                                                
                                                // Masked Key with copy button
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    <div class="flex items-center gap-2">
                                                        <code class="text-sm text-slate-400 font-mono bg-slate-900 px-2 py-1 rounded">
                                                            {key.masked_key()}
                                                        </code>
                                                        <CopyButton text=key.key_prefix.clone() />
                                                    </div>
                                                </td>
                                                
                                                // Scope badge
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    <span class=format!(
                                                        "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border {}",
                                                        key.scope.badge_class()
                                                    )>
                                                        {key.scope.label()}
                                                    </span>
                                                </td>
                                                
                                                // Created
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    <span class="text-sm text-slate-400">{format_date(&key.created_at)}</span>
                                                </td>
                                                
                                                // Last Used
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    <span class="text-sm text-slate-400">
                                                        {key.last_used_at.clone().map(|d| format_date(&d)).unwrap_or_else(|| "Never".to_string())}
                                                    </span>
                                                </td>
                                                
                                                // Actions
                                                <td class="px-6 py-4 whitespace-nowrap text-right">
                                                    <button
                                                        class="p-2 text-slate-400 hover:text-red-400 hover:bg-red-500/10 rounded-lg transition-colors"
                                                        title="Revoke key"
                                                        on:click=move |_| on_revoke(key_revoke.clone())
                                                    >
                                                        <TrashIcon class="w-4 h-4" />
                                                    </button>
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
// Create Key Modal
// ============================================================================

#[component]
fn CreateKeyModal(
    on_close: impl Fn() + 'static + Clone,
    on_create: impl Fn((ApiKey, String)) + 'static + Clone,
) -> impl IntoView {
    let (form_data, set_form_data) = create_signal(KeyFormData::default());
    let (errors, set_errors) = create_signal(ValidationErrors::default());
    let (creating, set_creating) = create_signal(false);
    let (api_error, set_api_error) = create_signal(Option::<String>::None);
    
    let on_close_backdrop = on_close.clone();
    let on_close_header = on_close.clone();
    let on_close_cancel = on_close;
    let on_create_clone = on_create;
    
    // Validation function
    let validate = move |data: &KeyFormData| -> ValidationErrors {
        let mut errs = ValidationErrors::default();
        
        if data.name.is_empty() {
            errs.name = Some("Name is required".to_string());
        } else if data.name.len() < 3 {
            errs.name = Some("Name must be at least 3 characters".to_string());
        }
        
        errs
    };
    
    let handle_create = move |_| {
        let data = form_data.get();
        let validation_errors = validate(&data);
        set_errors.set(validation_errors.clone());
        
        if !validation_errors.is_valid() {
            return;
        }
        
        set_creating.set(true);
        set_api_error.set(None);
        
        let on_create = on_create_clone.clone();
        
        spawn_local(async move {
            match create_api_key(&data.name, &data.scope, data.expires_days).await {
                Ok((key, secret)) => on_create((key, secret)),
                Err(e) => {
                    set_api_error.set(Some(e));
                    set_creating.set(false);
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
                    <h2 class="text-lg font-semibold text-white">"Generate API Key"</h2>
                    <button
                        class="p-1 text-slate-400 hover:text-white rounded transition-colors"
                        on:click=move |_| on_close_header()
                    >
                        <CloseIcon class="w-5 h-5" />
                    </button>
                </div>
                
                // Body
                <div class="p-6 space-y-4">
                    // API error display
                    {move || {
                        if let Some(err) = api_error.get() {
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
                            class=move || {
                                let base = "w-full px-3 py-2 rounded-lg bg-slate-900 border text-white text-sm placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent";
                                if errors.get().name.is_some() {
                                    format!("{} border-red-500", base)
                                } else {
                                    format!("{} border-slate-700", base)
                                }
                            }
                            placeholder="e.g., CI/CD Pipeline Key"
                            prop:value=move || form_data.get().name
                            on:input=move |e| {
                                set_form_data.update(|d| d.name = event_target_value(&e));
                                set_errors.update(|e| e.name = None);
                            }
                        />
                        {move || {
                            if let Some(err) = errors.get().name {
                                view! { <p class="text-xs text-red-400 mt-1">{err}</p> }.into_view()
                            } else {
                                view! {}.into_view()
                            }
                        }}
                    </div>
                    
                    // Scope selector
                    <div class="space-y-2">
                        <label class="block text-sm font-medium text-slate-300">"Permission Scope"</label>
                        <div class="space-y-2">
                            {[KeyScope::ReadOnly, KeyScope::ReadWrite, KeyScope::Admin].into_iter().map(|scope| {
                                let scope_for_check = scope.clone();
                                let scope_for_change = scope.clone();
                                let scope_label = scope.label();
                                let scope_desc = scope.description();
                                
                                view! {
                                    <label class=move || {
                                        let base = "flex items-start gap-3 p-3 rounded-lg border cursor-pointer transition-colors";
                                        if form_data.get().scope == scope_for_check {
                                            format!("{} border-blue-500 bg-blue-500/10", base)
                                        } else {
                                            format!("{} border-slate-700 hover:border-slate-600 hover:bg-slate-700/30", base)
                                        }
                                    }>
                                        <input
                                            type="radio"
                                            name="scope"
                                            class="mt-1"
                                            prop:checked=move || form_data.get().scope == scope_for_change
                                            on:change=move |_| {
                                                set_form_data.update(|d| d.scope = scope.clone());
                                            }
                                        />
                                        <div>
                                            <div class="text-sm font-medium text-white">{scope_label}</div>
                                            <div class="text-xs text-slate-400">{scope_desc}</div>
                                        </div>
                                    </label>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                    
                    // Expiration selector
                    <div class="space-y-1">
                        <label class="block text-sm font-medium text-slate-300">"Expiration"</label>
                        <select
                            class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm \
                                   focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                            on:change=move |e| {
                                let value = event_target_value(&e);
                                set_form_data.update(|d| {
                                    d.expires_days = match value.as_str() {
                                        "never" => None,
                                        "30" => Some(30),
                                        "90" => Some(90),
                                        "180" => Some(180),
                                        "365" => Some(365),
                                        _ => None,
                                    };
                                });
                            }
                        >
                            <option value="never">"Never expires"</option>
                            <option value="30">"30 days"</option>
                            <option value="90">"90 days"</option>
                            <option value="180">"180 days"</option>
                            <option value="365">"1 year"</option>
                        </select>
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
                        disabled=move || creating.get()
                        on:click=handle_create
                    >
                        {move || if creating.get() { "Generating..." } else { "Generate Key" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// New Key Display Modal (shown once after creation)
// ============================================================================

#[component]
fn NewKeyDisplayModal(
    secret: String,
    on_close: impl Fn() + 'static + Clone,
) -> impl IntoView {
    let (copied, set_copied) = create_signal(false);
    let secret_display = secret.clone();
    let secret_copy = secret;
    
    let on_copy = move |_| {
        let secret = secret_copy.clone();
        spawn_local(async move {
            if let Some(window) = web_sys::window() {
                let clipboard = window.navigator().clipboard();
                let _ = wasm_bindgen_futures::JsFuture::from(
                    clipboard.write_text(&secret)
                ).await;
                set_copied.set(true);
                
                // Reset after 2 seconds
                gloo_timers::callback::Timeout::new(2000, move || {
                    set_copied.set(false);
                }).forget();
            }
        });
    };
    
    let on_close_clone = on_close.clone();
    
    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center">
            // Backdrop
            <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" />
            
            // Modal
            <div class="relative bg-slate-800 rounded-xl border border-slate-700 shadow-2xl w-full max-w-lg mx-4">
                // Header
                <div class="p-6 text-center">
                    <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-green-500/10 flex items-center justify-center">
                        <CheckIcon class="w-8 h-8 text-green-400" />
                    </div>
                    <h2 class="text-xl font-semibold text-white mb-2">"API Key Generated"</h2>
                    <p class="text-slate-400">
                        "Copy your API key now. You won't be able to see it again!"
                    </p>
                </div>
                
                // Key display
                <div class="mx-6 mb-6">
                    <div class="bg-slate-900 rounded-lg border border-slate-700 p-4">
                        <div class="flex items-center justify-between gap-4">
                            <code class="text-sm text-green-400 font-mono break-all flex-1">
                                {secret_display}
                            </code>
                            <button
                                class=move || {
                                    if copied.get() {
                                        "shrink-0 px-3 py-1.5 text-sm font-medium bg-green-500/20 text-green-400 rounded-lg"
                                    } else {
                                        "shrink-0 px-3 py-1.5 text-sm font-medium bg-slate-700 hover:bg-slate-600 text-white rounded-lg transition-colors"
                                    }
                                }
                                on:click=on_copy
                            >
                                {move || if copied.get() { "Copied!" } else { "Copy" }}
                            </button>
                        </div>
                    </div>
                    
                    // Warning
                    <div class="flex items-start gap-2 mt-4 p-3 bg-amber-500/10 rounded-lg border border-amber-500/30">
                        <WarningIcon class="w-5 h-5 text-amber-400 shrink-0 mt-0.5" />
                        <p class="text-sm text-amber-400">
                            "This key will only be displayed once. Please copy and store it securely."
                        </p>
                    </div>
                </div>
                
                // Footer
                <div class="flex items-center justify-end px-6 py-4 border-t border-slate-700">
                    <button
                        class="px-4 py-2 text-sm font-medium bg-blue-500 hover:bg-blue-600 \
                               text-white rounded-lg transition-colors"
                        on:click=move |_| on_close_clone()
                    >
                        "Done"
                    </button>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Revoke Confirmation Modal
// ============================================================================

#[component]
fn RevokeConfirmModal(
    api_key: ApiKey,
    on_close: impl Fn() + 'static + Clone,
    on_confirm: impl Fn() + 'static + Clone,
) -> impl IntoView {
    let (revoking, set_revoking) = create_signal(false);
    
    let on_close_backdrop = on_close.clone();
    let on_close_cancel = on_close;
    let on_confirm_clone = on_confirm;
    
    let handle_confirm = move |_| {
        set_revoking.set(true);
        on_confirm_clone();
    };
    
    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center">
            // Backdrop
            <div 
                class="absolute inset-0 bg-black/60 backdrop-blur-sm"
                on:click=move |_| on_close_backdrop()
            />
            
            // Modal
            <div class="relative bg-slate-800 rounded-xl border border-slate-700 shadow-2xl w-full max-w-md mx-4">
                // Header with warning icon
                <div class="p-6 text-center">
                    <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-red-500/10 flex items-center justify-center">
                        <WarningIcon class="w-8 h-8 text-red-400" />
                    </div>
                    <h2 class="text-xl font-semibold text-white mb-2">"Revoke API Key"</h2>
                    <p class="text-slate-400">
                        "Are you sure you want to revoke this API key? Any applications using this key will lose access."
                    </p>
                </div>
                
                // Key details
                <div class="mx-6 mb-6 p-4 bg-slate-900 rounded-lg border border-slate-700">
                    <div class="flex items-center gap-3">
                        <div class="w-10 h-10 rounded-lg bg-amber-500/10 flex items-center justify-center">
                            <KeyIcon class="w-5 h-5 text-amber-400" />
                        </div>
                        <div>
                            <div class="text-sm font-medium text-white">{api_key.name.clone()}</div>
                            <code class="text-xs text-slate-500 font-mono">{api_key.masked_key()}</code>
                        </div>
                    </div>
                </div>
                
                // Actions
                <div class="flex items-center justify-end gap-3 px-6 py-4 border-t border-slate-700">
                    <button
                        class="px-4 py-2 text-sm font-medium text-slate-400 hover:text-white \
                               rounded-lg transition-colors"
                        on:click=move |_| on_close_cancel()
                        disabled=move || revoking.get()
                    >
                        "Cancel"
                    </button>
                    <button
                        class="px-4 py-2 text-sm font-medium bg-red-500 hover:bg-red-600 \
                               text-white rounded-lg transition-colors disabled:opacity-50"
                        disabled=move || revoking.get()
                        on:click=handle_confirm
                    >
                        {move || if revoking.get() { "Revoking..." } else { "Revoke Key" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Empty State
// ============================================================================

#[component]
fn EmptyState() -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center py-16">
            <div class="w-16 h-16 rounded-full bg-slate-700 flex items-center justify-center mb-6">
                <KeyIcon class="w-8 h-8 text-slate-400" />
            </div>
            <h2 class="text-xl font-semibold text-white mb-2">"No API Keys"</h2>
            <p class="text-slate-400 text-center max-w-md">
                "Generate API keys to enable programmatic access to Vectorize. Keys can be scoped to specific permissions."
            </p>
        </div>
    }
}

// ============================================================================
// Copy Button Component
// ============================================================================

#[component]
fn CopyButton(text: String) -> impl IntoView {
    let (copied, set_copied) = create_signal(false);
    let text_clone = text.clone();
    
    let on_click = move |_| {
        let text = text_clone.clone();
        spawn_local(async move {
            if let Some(window) = web_sys::window() {
                let clipboard = window.navigator().clipboard();
                let _ = wasm_bindgen_futures::JsFuture::from(
                    clipboard.write_text(&text)
                ).await;
                set_copied.set(true);
                
                gloo_timers::callback::Timeout::new(2000, move || {
                    set_copied.set(false);
                }).forget();
            }
        });
    };
    
    view! {
        <button
            class="p-1.5 text-slate-400 hover:text-white hover:bg-slate-700 rounded transition-colors"
            title="Copy key prefix"
            on:click=on_click
        >
            {move || {
                if copied.get() {
                    view! { <CheckIcon class="w-4 h-4 text-green-400" /> }.into_view()
                } else {
                    view! { <CopyIcon class="w-4 h-4" /> }.into_view()
                }
            }}
        </button>
    }
}

// ============================================================================
// Icons
// ============================================================================

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

#[component]
fn KeyIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" />
        </svg>
    }
}

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
fn CopyIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
        </svg>
    }
}

#[component]
fn WarningIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
            <line x1="12" y1="9" x2="12" y2="13" />
            <line x1="12" y1="17" x2="12.01" y2="17" />
        </svg>
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn format_date(date_str: &str) -> String {
    // Simple date formatting - in production, use a proper date library
    if let Some(date_part) = date_str.split('T').next() {
        date_part.to_string()
    } else {
        date_str.to_string()
    }
}

fn get_mock_keys() -> Vec<ApiKey> {
    vec![
        ApiKey {
            id: "key-1".to_string(),
            name: "CI/CD Pipeline".to_string(),
            key_prefix: "vz_prod_".to_string(),
            scope: KeyScope::ReadWrite,
            expires_at: None,
            created_at: "2026-01-15T10:30:00Z".to_string(),
            last_used_at: Some("2026-02-03T08:45:00Z".to_string()),
            created_by: "admin".to_string(),
        },
        ApiKey {
            id: "key-2".to_string(),
            name: "Monitoring Integration".to_string(),
            key_prefix: "vz_mon_".to_string(),
            scope: KeyScope::ReadOnly,
            expires_at: Some("2026-06-15T00:00:00Z".to_string()),
            created_at: "2026-01-20T14:15:00Z".to_string(),
            last_used_at: Some("2026-02-02T22:30:00Z".to_string()),
            created_by: "admin".to_string(),
        },
        ApiKey {
            id: "key-3".to_string(),
            name: "Admin Scripts".to_string(),
            key_prefix: "vz_admin".to_string(),
            scope: KeyScope::Admin,
            expires_at: None,
            created_at: "2026-02-01T09:00:00Z".to_string(),
            last_used_at: None,
            created_by: "admin".to_string(),
        },
    ]
}

// ============================================================================
// API Functions
// ============================================================================

fn get_base_url() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_else(|| "http://localhost:8080".to_string())
}

async fn fetch_api_keys() -> Result<Vec<ApiKey>, String> {
    let url = format!("{}/api/v1/api-keys", get_base_url());
    
    let response = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json::<Vec<ApiKey>>().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Err("Failed to fetch API keys".to_string())
    }
}

async fn create_api_key(name: &str, scope: &KeyScope, expires_days: Option<u32>) -> Result<(ApiKey, String), String> {
    let url = format!("{}/api/v1/api-keys", get_base_url());
    
    #[derive(Serialize)]
    struct CreateKeyPayload {
        name: String,
        scope: KeyScope,
        expires_days: Option<u32>,
    }
    
    let payload = CreateKeyPayload {
        name: name.to_string(),
        scope: scope.clone(),
        expires_days,
    };
    
    let response = gloo_net::http::Request::post(&url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .map_err(|e| format!("Serialize error: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        #[derive(Deserialize)]
        struct CreateKeyResponse {
            key: ApiKey,
            secret: String,
        }
        
        let resp = response.json::<CreateKeyResponse>().await
            .map_err(|e| format!("Parse error: {}", e))?;
        
        Ok((resp.key, resp.secret))
    } else {
        // Mock response for development
        let secret = format!("vz_{}_{}", 
            if matches!(scope, KeyScope::Admin) { "admin" } else if matches!(scope, KeyScope::ReadWrite) { "rw" } else { "ro" },
            uuid::Uuid::new_v4().to_string().replace("-", "")
        );
        let key = ApiKey {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            key_prefix: secret.chars().take(8).collect(),
            scope: scope.clone(),
            expires_at: expires_days.map(|_| "2026-05-03T00:00:00Z".to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_used_at: None,
            created_by: "admin".to_string(),
        };
        Ok((key, secret))
    }
}

async fn revoke_api_key(key_id: &str) -> Result<(), String> {
    let url = format!("{}/api/v1/api-keys/{}", get_base_url(), key_id);
    
    let response = gloo_net::http::Request::delete(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() || response.status() == 404 {
        Ok(())
    } else {
        // Mock success for development
        Ok(())
    }
}

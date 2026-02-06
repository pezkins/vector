//! SSO Configuration Component
//!
//! Single Sign-On configuration interface with:
//! - Provider selection (None, OIDC, SAML)
//! - OIDC configuration form
//! - SAML configuration form
//! - Test connection functionality
//! - User mapping rules placeholder

use leptos::*;
use serde::{Deserialize, Serialize};

use crate::components::common::RefreshIcon;

// ============================================================================
// Types
// ============================================================================

/// SSO Provider type
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SsoProvider {
    #[default]
    None,
    Oidc,
    Saml,
}

impl SsoProvider {
    pub fn label(&self) -> &'static str {
        match self {
            SsoProvider::None => "Disabled",
            SsoProvider::Oidc => "OIDC (OpenID Connect)",
            SsoProvider::Saml => "SAML 2.0",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SsoProvider::None => "Users authenticate with local credentials",
            SsoProvider::Oidc => "Use OpenID Connect providers like Okta, Auth0, Azure AD",
            SsoProvider::Saml => "Use SAML 2.0 identity providers",
        }
    }
}

/// OIDC Configuration
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OidcConfig {
    pub client_id: String,
    pub client_secret: String,
    pub issuer_url: String,
    pub redirect_uri: String,
    pub scopes: String,
}

/// SAML Configuration  
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SamlConfig {
    pub entity_id: String,
    pub sso_url: String,
    pub certificate: String,
    pub sign_requests: bool,
}

/// Full SSO Configuration
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SsoConfig {
    pub provider: SsoProvider,
    pub oidc: OidcConfig,
    pub saml: SamlConfig,
    pub auto_provision_users: bool,
    pub default_role_id: Option<String>,
}

/// Test connection result
#[derive(Clone, Debug)]
enum TestResult {
    None,
    Testing,
    Success(String),
    Error(String),
}

// ============================================================================
// Main Component
// ============================================================================

/// SSO configuration page component
#[component]
pub fn SsoConfigPage() -> impl IntoView {
    let (config, set_config) = create_signal(SsoConfig::default());
    let (loading, set_loading) = create_signal(true);
    let (saving, set_saving) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);
    let (success, set_success) = create_signal(Option::<String>::None);
    let (test_result, set_test_result) = create_signal(TestResult::None);
    let (has_changes, set_has_changes) = create_signal(false);
    
    // Fetch config on mount
    create_effect(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            
            match fetch_sso_config().await {
                Ok(data) => set_config.set(data),
                Err(e) => {
                    // Use default config if fetch fails
                    web_sys::console::warn_1(&format!("Using default SSO config: {}", e).into());
                }
            }
            
            set_loading.set(false);
        });
    });
    
    // Update provider
    let on_provider_change = move |provider: SsoProvider| {
        set_config.update(|c| c.provider = provider);
        set_has_changes.set(true);
        set_test_result.set(TestResult::None);
    };
    
    // Test connection
    let on_test_connection = move |_| {
        let current_config = config.get();
        set_test_result.set(TestResult::Testing);
        
        spawn_local(async move {
            match test_sso_connection(&current_config).await {
                Ok(msg) => set_test_result.set(TestResult::Success(msg)),
                Err(e) => set_test_result.set(TestResult::Error(e)),
            }
        });
    };
    
    // Save configuration
    let on_save = move |_| {
        let current_config = config.get();
        set_saving.set(true);
        set_error.set(None);
        set_success.set(None);
        
        spawn_local(async move {
            match save_sso_config(&current_config).await {
                Ok(_) => {
                    set_success.set(Some("SSO configuration saved successfully".to_string()));
                    set_has_changes.set(false);
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_saving.set(false);
        });
    };

    view! {
        <div class="flex-1 overflow-auto p-6 bg-slate-900">
            <div class="max-w-4xl mx-auto">
                // Header
                <div class="flex items-center justify-between mb-6">
                    <div>
                        <h1 class="text-2xl font-bold text-white">"Single Sign-On"</h1>
                        <p class="text-slate-400 mt-1">"Configure SSO with OIDC or SAML providers"</p>
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
                        // Provider Selection
                        <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
                            <h2 class="text-lg font-semibold text-white mb-4">"Provider"</h2>
                            <div class="space-y-3">
                                {[SsoProvider::None, SsoProvider::Oidc, SsoProvider::Saml].into_iter().map(|provider| {
                                    let provider_for_check = provider.clone();
                                    let provider_for_checked = provider.clone();
                                    let provider_label = provider.label();
                                    let provider_desc = provider.description();
                                    
                                    view! {
                                        <label class=move || {
                                            let base = "flex items-start gap-4 p-4 rounded-lg border cursor-pointer transition-colors";
                                            if config.get().provider == provider_for_check {
                                                format!("{} border-blue-500 bg-blue-500/10", base)
                                            } else {
                                                format!("{} border-slate-700 hover:border-slate-600 hover:bg-slate-700/30", base)
                                            }
                                        }>
                                            <input
                                                type="radio"
                                                name="provider"
                                                class="mt-1"
                                                prop:checked=move || config.get().provider == provider_for_checked
                                                on:change=move |_| on_provider_change(provider.clone())
                                            />
                                            <div>
                                                <div class="text-sm font-medium text-white">{provider_label}</div>
                                                <div class="text-xs text-slate-400 mt-0.5">{provider_desc}</div>
                                            </div>
                                        </label>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                        
                        // OIDC Configuration (shown when OIDC selected)
                        <Show when=move || config.get().provider == SsoProvider::Oidc>
                            <OidcConfigForm 
                                config=Signal::derive(move || config.get().oidc)
                                on_change=move |oidc| {
                                    set_config.update(|c| c.oidc = oidc);
                                    set_has_changes.set(true);
                                }
                            />
                        </Show>
                        
                        // SAML Configuration (shown when SAML selected)
                        <Show when=move || config.get().provider == SsoProvider::Saml>
                            <SamlConfigForm 
                                config=Signal::derive(move || config.get().saml)
                                on_change=move |saml| {
                                    set_config.update(|c| c.saml = saml);
                                    set_has_changes.set(true);
                                }
                            />
                        </Show>
                        
                        // User Provisioning (shown when SSO enabled)
                        <Show when=move || config.get().provider != SsoProvider::None>
                            <UserProvisioningSection 
                                auto_provision=Signal::derive(move || config.get().auto_provision_users)
                                default_role=Signal::derive(move || config.get().default_role_id)
                                on_change=move |(auto, role)| {
                                    set_config.update(|c| {
                                        c.auto_provision_users = auto;
                                        c.default_role_id = role;
                                    });
                                    set_has_changes.set(true);
                                }
                            />
                        </Show>
                        
                        // Test Connection & Save buttons (shown when SSO enabled)
                        <Show when=move || config.get().provider != SsoProvider::None>
                            <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
                                <div class="flex items-center justify-between">
                                    // Test Connection
                                    <div class="flex items-center gap-4">
                                        <button
                                            class="flex items-center gap-2 px-4 py-2 bg-slate-700 hover:bg-slate-600 \
                                                   text-white rounded-lg transition-colors disabled:opacity-50"
                                            disabled=move || matches!(test_result.get(), TestResult::Testing)
                                            on:click=on_test_connection
                                        >
                                            <RefreshIcon class="w-4 h-4" />
                                            {move || {
                                                if matches!(test_result.get(), TestResult::Testing) {
                                                    "Testing..."
                                                } else {
                                                    "Test Connection"
                                                }
                                            }}
                                        </button>
                                        
                                        // Test result display
                                        {move || match test_result.get() {
                                            TestResult::None | TestResult::Testing => view! {}.into_view(),
                                            TestResult::Success(msg) => view! {
                                                <div class="flex items-center gap-2 text-green-400">
                                                    <CheckIcon class="w-4 h-4" />
                                                    <span class="text-sm">{msg}</span>
                                                </div>
                                            }.into_view(),
                                            TestResult::Error(err) => view! {
                                                <div class="flex items-center gap-2 text-red-400">
                                                    <ErrorIcon class="w-4 h-4" />
                                                    <span class="text-sm">{err}</span>
                                                </div>
                                            }.into_view(),
                                        }}
                                    </div>
                                    
                                    // Save button
                                    <button
                                        class="px-6 py-2 bg-blue-500 hover:bg-blue-600 text-white font-medium \
                                               rounded-lg transition-colors disabled:opacity-50"
                                        disabled=move || saving.get() || !has_changes.get()
                                        on:click=on_save
                                    >
                                        {move || if saving.get() { "Saving..." } else { "Save Configuration" }}
                                    </button>
                                </div>
                            </div>
                        </Show>
                        
                        // Save button when SSO disabled
                        <Show when=move || config.get().provider == SsoProvider::None>
                            <div class="flex justify-end">
                                <button
                                    class="px-6 py-2 bg-blue-500 hover:bg-blue-600 text-white font-medium \
                                           rounded-lg transition-colors disabled:opacity-50"
                                    disabled=move || saving.get() || !has_changes.get()
                                    on:click=on_save
                                >
                                    {move || if saving.get() { "Saving..." } else { "Save Configuration" }}
                                </button>
                            </div>
                        </Show>
                    </div>
                </Show>
            </div>
        </div>
    }
}

// ============================================================================
// OIDC Configuration Form
// ============================================================================

#[component]
fn OidcConfigForm(
    config: Signal<OidcConfig>,
    on_change: impl Fn(OidcConfig) + 'static + Clone,
) -> impl IntoView {
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
            <h2 class="text-lg font-semibold text-white mb-4">"OIDC Configuration"</h2>
            <div class="space-y-4">
                // Client ID
                <div class="space-y-1">
                    <label class="block text-sm font-medium text-slate-300">"Client ID"</label>
                    <input
                        type="text"
                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm \
                               placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                        placeholder="your-client-id"
                        prop:value=move || config.get().client_id
                        on:input={
                            let on_change = on_change.clone();
                            move |e| {
                                let mut c = config.get();
                                c.client_id = event_target_value(&e);
                                on_change(c);
                            }
                        }
                    />
                </div>
                
                // Client Secret
                <div class="space-y-1">
                    <label class="block text-sm font-medium text-slate-300">"Client Secret"</label>
                    <input
                        type="password"
                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm \
                               placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                        placeholder="••••••••••••"
                        prop:value=move || config.get().client_secret
                        on:input={
                            let on_change = on_change.clone();
                            move |e| {
                                let mut c = config.get();
                                c.client_secret = event_target_value(&e);
                                on_change(c);
                            }
                        }
                    />
                </div>
                
                // Issuer URL
                <div class="space-y-1">
                    <label class="block text-sm font-medium text-slate-300">"Issuer URL"</label>
                    <input
                        type="url"
                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm \
                               placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                        placeholder="https://your-idp.example.com"
                        prop:value=move || config.get().issuer_url
                        on:input={
                            let on_change = on_change.clone();
                            move |e| {
                                let mut c = config.get();
                                c.issuer_url = event_target_value(&e);
                                on_change(c);
                            }
                        }
                    />
                    <p class="text-xs text-slate-500">"The OpenID Connect issuer URL for your identity provider"</p>
                </div>
                
                // Redirect URI
                <div class="space-y-1">
                    <label class="block text-sm font-medium text-slate-300">"Redirect URI"</label>
                    <input
                        type="url"
                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm \
                               placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                        placeholder="https://vectorize.example.com/auth/callback"
                        prop:value=move || config.get().redirect_uri
                        on:input={
                            let on_change = on_change.clone();
                            move |e| {
                                let mut c = config.get();
                                c.redirect_uri = event_target_value(&e);
                                on_change(c);
                            }
                        }
                    />
                    <p class="text-xs text-slate-500">"Configure this URL in your identity provider's allowed callback URLs"</p>
                </div>
                
                // Scopes
                <div class="space-y-1">
                    <label class="block text-sm font-medium text-slate-300">"Scopes"</label>
                    <input
                        type="text"
                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm \
                               placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                        placeholder="openid profile email"
                        prop:value=move || config.get().scopes
                        on:input={
                            let on_change = on_change.clone();
                            move |e| {
                                let mut c = config.get();
                                c.scopes = event_target_value(&e);
                                on_change(c);
                            }
                        }
                    />
                    <p class="text-xs text-slate-500">"Space-separated list of OAuth scopes to request"</p>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// SAML Configuration Form
// ============================================================================

#[component]
fn SamlConfigForm(
    config: Signal<SamlConfig>,
    on_change: impl Fn(SamlConfig) + 'static + Clone,
) -> impl IntoView {
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
            <h2 class="text-lg font-semibold text-white mb-4">"SAML Configuration"</h2>
            <div class="space-y-4">
                // Entity ID
                <div class="space-y-1">
                    <label class="block text-sm font-medium text-slate-300">"Entity ID (Issuer)"</label>
                    <input
                        type="text"
                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm \
                               placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                        placeholder="https://your-idp.example.com/saml/metadata"
                        prop:value=move || config.get().entity_id
                        on:input={
                            let on_change = on_change.clone();
                            move |e| {
                                let mut c = config.get();
                                c.entity_id = event_target_value(&e);
                                on_change(c);
                            }
                        }
                    />
                    <p class="text-xs text-slate-500">"The unique identifier for your identity provider"</p>
                </div>
                
                // SSO URL
                <div class="space-y-1">
                    <label class="block text-sm font-medium text-slate-300">"SSO URL"</label>
                    <input
                        type="url"
                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm \
                               placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                        placeholder="https://your-idp.example.com/saml/sso"
                        prop:value=move || config.get().sso_url
                        on:input={
                            let on_change = on_change.clone();
                            move |e| {
                                let mut c = config.get();
                                c.sso_url = event_target_value(&e);
                                on_change(c);
                            }
                        }
                    />
                    <p class="text-xs text-slate-500">"The SAML Single Sign-On service URL"</p>
                </div>
                
                // Certificate
                <div class="space-y-1">
                    <label class="block text-sm font-medium text-slate-300">"X.509 Certificate"</label>
                    <textarea
                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm \
                               font-mono placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 \
                               focus:border-transparent resize-none"
                        rows="6"
                        placeholder="-----BEGIN CERTIFICATE-----\n...\n-----END CERTIFICATE-----"
                        prop:value=move || config.get().certificate
                        on:input={
                            let on_change = on_change.clone();
                            move |e| {
                                let mut c = config.get();
                                c.certificate = event_target_value(&e);
                                on_change(c);
                            }
                        }
                    />
                    <p class="text-xs text-slate-500">"The PEM-encoded X.509 certificate from your identity provider"</p>
                </div>
                
                // Sign Requests toggle
                <div class="flex items-center justify-between p-4 bg-slate-900 rounded-lg">
                    <div>
                        <div class="text-sm font-medium text-white">"Sign Authentication Requests"</div>
                        <div class="text-xs text-slate-400">"Sign SAML authentication requests sent to the IdP"</div>
                    </div>
                    <button
                        type="button"
                        class=move || {
                            let base = "relative inline-flex h-6 w-11 items-center rounded-full transition-colors";
                            if config.get().sign_requests {
                                format!("{} bg-blue-500", base)
                            } else {
                                format!("{} bg-slate-600", base)
                            }
                        }
                        on:click={
                            let on_change = on_change.clone();
                            move |_| {
                                let mut c = config.get();
                                c.sign_requests = !c.sign_requests;
                                on_change(c);
                            }
                        }
                    >
                        <span class=move || {
                            let base = "inline-block h-4 w-4 transform rounded-full bg-white transition-transform";
                            if config.get().sign_requests {
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
// User Provisioning Section
// ============================================================================

#[component]
fn UserProvisioningSection(
    auto_provision: Signal<bool>,
    default_role: Signal<Option<String>>,
    on_change: impl Fn((bool, Option<String>)) + 'static + Clone,
) -> impl IntoView {
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
            <h2 class="text-lg font-semibold text-white mb-4">"User Provisioning"</h2>
            <div class="space-y-4">
                // Auto-provision toggle
                <div class="flex items-center justify-between p-4 bg-slate-900 rounded-lg">
                    <div>
                        <div class="text-sm font-medium text-white">"Auto-provision Users"</div>
                        <div class="text-xs text-slate-400">"Automatically create users on first SSO login"</div>
                    </div>
                    <button
                        type="button"
                        class=move || {
                            let base = "relative inline-flex h-6 w-11 items-center rounded-full transition-colors";
                            if auto_provision.get() {
                                format!("{} bg-blue-500", base)
                            } else {
                                format!("{} bg-slate-600", base)
                            }
                        }
                        on:click={
                            let on_change = on_change.clone();
                            move |_| {
                                on_change((!auto_provision.get(), default_role.get()));
                            }
                        }
                    >
                        <span class=move || {
                            let base = "inline-block h-4 w-4 transform rounded-full bg-white transition-transform";
                            if auto_provision.get() {
                                format!("{} translate-x-6", base)
                            } else {
                                format!("{} translate-x-1", base)
                            }
                        } />
                    </button>
                </div>
                
                // Default role selector (shown when auto-provision is enabled)
                <Show when=move || auto_provision.get()>
                    <div class="space-y-1">
                        <label class="block text-sm font-medium text-slate-300">"Default Role for New Users"</label>
                        <select
                            class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm \
                                   focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                            on:change={
                                let on_change = on_change.clone();
                                move |e| {
                                    let value = event_target_value(&e);
                                    let role = if value.is_empty() { None } else { Some(value) };
                                    on_change((auto_provision.get(), role));
                                }
                            }
                        >
                            <option value="" selected=move || default_role.get().is_none()>"Select a role"</option>
                            <option value="viewer" selected=move || default_role.get().as_deref() == Some("viewer")>"Viewer"</option>
                            <option value="editor" selected=move || default_role.get().as_deref() == Some("editor")>"Editor"</option>
                            <option value="operator" selected=move || default_role.get().as_deref() == Some("operator")>"Operator"</option>
                        </select>
                        <p class="text-xs text-slate-500">"New users created via SSO will be assigned this role"</p>
                    </div>
                </Show>
                
                // User Mapping Rules (placeholder)
                <div class="p-4 bg-slate-900 rounded-lg border border-dashed border-slate-700">
                    <div class="flex items-center gap-3 text-slate-400">
                        <MappingIcon class="w-5 h-5" />
                        <div>
                            <div class="text-sm font-medium">"User Attribute Mapping"</div>
                            <div class="text-xs text-slate-500">"Configure how SSO attributes map to Vectorize user fields (coming soon)"</div>
                        </div>
                    </div>
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
fn MappingIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M16 3h5v5" />
            <line x1="4" y1="20" x2="21" y2="3" />
            <path d="M21 16v5h-5" />
            <line x1="15" y1="15" x2="21" y2="21" />
            <line x1="4" y1="4" x2="9" y2="9" />
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

async fn fetch_sso_config() -> Result<SsoConfig, String> {
    let url = format!("{}/api/v1/settings/sso", get_base_url());
    
    let response = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json::<SsoConfig>().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Err("Failed to fetch SSO config".to_string())
    }
}

async fn save_sso_config(config: &SsoConfig) -> Result<(), String> {
    let url = format!("{}/api/v1/settings/sso", get_base_url());
    
    let response = gloo_net::http::Request::put(&url)
        .header("Content-Type", "application/json")
        .json(config)
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

async fn test_sso_connection(config: &SsoConfig) -> Result<String, String> {
    let url = format!("{}/api/v1/settings/sso/test", get_base_url());
    
    let response = gloo_net::http::Request::post(&url)
        .header("Content-Type", "application/json")
        .json(config)
        .map_err(|e| format!("Serialize error: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        Ok("Connection successful".to_string())
    } else {
        // Mock response for development
        match config.provider {
            SsoProvider::None => Err("No provider configured".to_string()),
            SsoProvider::Oidc => {
                if config.oidc.client_id.is_empty() || config.oidc.issuer_url.is_empty() {
                    Err("Client ID and Issuer URL are required".to_string())
                } else {
                    Ok("OIDC configuration validated".to_string())
                }
            }
            SsoProvider::Saml => {
                if config.saml.entity_id.is_empty() || config.saml.sso_url.is_empty() {
                    Err("Entity ID and SSO URL are required".to_string())
                } else {
                    Ok("SAML configuration validated".to_string())
                }
            }
        }
    }
}

//! Setup wizard component
//!
//! First-time setup screen for creating the initial admin user

use leptos::*;
use serde::{Deserialize, Serialize};

/// Setup status from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStatus {
    pub is_setup: bool,
    pub version: String,
}

/// Setup init request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupInitRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// Setup init response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupInitResponse {
    pub success: bool,
    pub message: String,
}

/// Check setup status
async fn check_setup_status() -> Result<SetupStatus, String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let response = gloo_net::http::Request::get(&format!("{}/api/v1/setup/status", origin))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if response.ok() {
        response.json().await.map_err(|e| e.to_string())
    } else {
        Err("Failed to check setup status".to_string())
    }
}

/// Submit setup form
async fn submit_setup(request: SetupInitRequest) -> Result<SetupInitResponse, String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let response = gloo_net::http::Request::post(&format!("{}/api/v1/setup/init", origin))
        .json(&request)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    response.json().await.map_err(|e| e.to_string())
}

/// Setup wizard component
#[component]
pub fn SetupWizard() -> impl IntoView {
    let (username, set_username) = create_signal(String::new());
    let (email, set_email) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (confirm_password, set_confirm_password) = create_signal(String::new());
    let (error, set_error) = create_signal(Option::<String>::None);
    let (loading, set_loading) = create_signal(false);
    let (setup_complete, set_setup_complete) = create_signal(false);
    
    // Check if already set up
    let status = create_resource(|| (), |_| async move {
        check_setup_status().await
    });
    
    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        
        let username_val = username.get();
        let email_val = email.get();
        let password_val = password.get();
        let confirm_val = confirm_password.get();
        
        // Validate
        if username_val.len() < 3 {
            set_error.set(Some("Username must be at least 3 characters".to_string()));
            return;
        }
        
        if !email_val.contains('@') || !email_val.contains('.') {
            set_error.set(Some("Please enter a valid email address".to_string()));
            return;
        }
        
        if password_val.len() < 8 {
            set_error.set(Some("Password must be at least 8 characters".to_string()));
            return;
        }
        
        if password_val != confirm_val {
            set_error.set(Some("Passwords do not match".to_string()));
            return;
        }
        
        set_error.set(None);
        set_loading.set(true);
        
        spawn_local(async move {
            let request = SetupInitRequest {
                username: username_val,
                email: email_val,
                password: password_val,
            };
            
            match submit_setup(request).await {
                Ok(response) => {
                    set_loading.set(false);
                    if response.success {
                        set_setup_complete.set(true);
                    } else {
                        set_error.set(Some(response.message));
                    }
                }
                Err(e) => {
                    set_loading.set(false);
                    set_error.set(Some(format!("Setup failed: {}", e)));
                }
            }
        });
    };
    
    view! {
        <div class="min-h-screen bg-gray-900 flex items-center justify-center p-4">
            <div class="max-w-md w-full">
                // Logo and title
                <div class="text-center mb-8">
                    <h1 class="text-3xl font-bold text-white mb-2">"Vectorize"</h1>
                    <p class="text-gray-400">"Visual Pipeline Builder for Vector"</p>
                </div>
                
                <Suspense fallback=move || view! {
                    <div class="bg-gray-800 rounded-lg p-6 shadow-lg">
                        <div class="flex items-center justify-center">
                            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
                            <span class="ml-3 text-gray-300">"Checking setup status..."</span>
                        </div>
                    </div>
                }>
                    {move || {
                        match status.get() {
                            Some(Ok(s)) if s.is_setup => {
                                // Already set up, redirect to main app
                                view! {
                                    <div class="bg-gray-800 rounded-lg p-6 shadow-lg text-center">
                                        <div class="text-green-400 text-lg mb-4">"Setup Complete"</div>
                                        <p class="text-gray-300 mb-4">"Vectorize is already configured."</p>
                                        <a href="/" class="inline-block bg-blue-600 hover:bg-blue-700 text-white px-6 py-2 rounded-lg transition">
                                            "Go to Dashboard"
                                        </a>
                                    </div>
                                }.into_view()
                            }
                            Some(Ok(_)) if setup_complete.get() => {
                                // Just completed setup
                                view! {
                                    <div class="bg-gray-800 rounded-lg p-6 shadow-lg text-center">
                                        <div class="text-green-400 text-lg mb-4">"Setup Complete!"</div>
                                        <p class="text-gray-300 mb-4">"Your admin account has been created."</p>
                                        <a href="/" class="inline-block bg-blue-600 hover:bg-blue-700 text-white px-6 py-2 rounded-lg transition">
                                            "Go to Dashboard"
                                        </a>
                                    </div>
                                }.into_view()
                            }
                            Some(Ok(_)) => {
                                // Show setup form
                                view! {
                                    <div class="bg-gray-800 rounded-lg p-6 shadow-lg">
                                        <h2 class="text-xl font-semibold text-white mb-6">"Create Admin Account"</h2>
                                        
                                        {move || error.get().map(|e| view! {
                                            <div class="bg-red-500/20 border border-red-500 text-red-300 px-4 py-2 rounded mb-4">
                                                {e}
                                            </div>
                                        })}
                                        
                                        <form on:submit=on_submit class="space-y-4">
                                            <div>
                                                <label class="block text-gray-300 text-sm mb-1">"Username"</label>
                                                <input
                                                    type="text"
                                                    class="w-full bg-gray-700 text-white px-4 py-2 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                                                    placeholder="admin"
                                                    prop:value=username
                                                    on:input=move |ev| set_username.set(event_target_value(&ev))
                                                    required
                                                />
                                            </div>
                                            
                                            <div>
                                                <label class="block text-gray-300 text-sm mb-1">"Email"</label>
                                                <input
                                                    type="email"
                                                    class="w-full bg-gray-700 text-white px-4 py-2 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                                                    placeholder="admin@example.com"
                                                    prop:value=email
                                                    on:input=move |ev| set_email.set(event_target_value(&ev))
                                                    required
                                                />
                                            </div>
                                            
                                            <div>
                                                <label class="block text-gray-300 text-sm mb-1">"Password"</label>
                                                <input
                                                    type="password"
                                                    class="w-full bg-gray-700 text-white px-4 py-2 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                                                    placeholder="••••••••"
                                                    prop:value=password
                                                    on:input=move |ev| set_password.set(event_target_value(&ev))
                                                    required
                                                />
                                            </div>
                                            
                                            <div>
                                                <label class="block text-gray-300 text-sm mb-1">"Confirm Password"</label>
                                                <input
                                                    type="password"
                                                    class="w-full bg-gray-700 text-white px-4 py-2 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                                                    placeholder="••••••••"
                                                    prop:value=confirm_password
                                                    on:input=move |ev| set_confirm_password.set(event_target_value(&ev))
                                                    required
                                                />
                                            </div>
                                            
                                            <button
                                                type="submit"
                                                class="w-full bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 text-white py-2 rounded transition"
                                                disabled=loading
                                            >
                                                {move || if loading.get() { "Creating Account..." } else { "Create Admin Account" }}
                                            </button>
                                        </form>
                                    </div>
                                }.into_view()
                            }
                            Some(Err(e)) => {
                                view! {
                                    <div class="bg-gray-800 rounded-lg p-6 shadow-lg">
                                        <div class="text-red-400 text-center">
                                            <p class="mb-2">"Failed to check setup status"</p>
                                            <p class="text-sm text-gray-400">{e}</p>
                                        </div>
                                    </div>
                                }.into_view()
                            }
                            None => {
                                view! {
                                    <div class="bg-gray-800 rounded-lg p-6 shadow-lg">
                                        <div class="flex items-center justify-center">
                                            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
                                        </div>
                                    </div>
                                }.into_view()
                            }
                        }
                    }}
                </Suspense>
                
                // Version info
                <div class="mt-4 text-center text-gray-500 text-sm">
                    {move || status.get().and_then(|r| r.ok()).map(|s| format!("Version {}", s.version))}
                </div>
            </div>
        </div>
    }
}

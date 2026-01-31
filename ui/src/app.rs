//! Root Application Component
//!
//! This module contains the main App component that sets up:
//! - Routing
//! - Global state providers
//! - Layout structure

use leptos::*;
use leptos_router::*;
use vectorize_shared::ConnectionMode;

use crate::components::common::*;
use crate::components::pipeline::PipelineView;
use crate::state::AppState;

/// Root application component
#[component]
pub fn App() -> impl IntoView {
    // Initialize global state
    let app_state = AppState::new();
    provide_context(app_state.clone());
    
    view! {
        <Router>
            <main class="h-screen flex flex-col bg-slate-900 text-slate-50">
                <Header />
                <div class="flex-1 flex overflow-hidden">
                    <Routes>
                        <Route path="/" view=HomePage />
                        <Route path="/pipeline" view=PipelineView />
                        <Route path="/data" view=DataView />
                        <Route path="/nodes" view=NodesView />
                        <Route path="/settings" view=SettingsView />
                    </Routes>
                </div>
                <StatusBar />
            </main>
        </Router>
    }
}

/// Home page - auto-connects when running inside Vectorize
#[component]
fn HomePage() -> impl IntoView {
    let app_state = expect_context::<AppState>();
    
    let (url, set_url) = create_signal(String::new());
    let (error, set_error) = create_signal(Option::<String>::None);
    let (connecting, set_connecting) = create_signal(false);
    let (checked_vectorize, set_checked_vectorize) = create_signal(false);
    
    // On mount, check if we're running inside Vectorize and auto-connect
    {
        let app_state = app_state.clone();
        create_effect(move |_| {
            if checked_vectorize.get() {
                return;
            }
            set_checked_vectorize.set(true);
            
            let app_state = app_state.clone();
            spawn_local(async move {
                // Try to fetch /api/info to detect if running inside Vectorize
                match gloo_net::http::Request::get("/api/info")
                    .send()
                    .await
                {
                    Ok(response) if response.ok() => {
                        // We're running inside Vectorize! Auto-connect via proxy
                        set_connecting.set(true);
                        
                        // The proxy URL - use /api as base (DirectClient adds /graphql, /health, etc.)
                        let proxy_url = format!("{}/api", 
                            web_sys::window()
                                .and_then(|w| w.location().origin().ok())
                                .unwrap_or_else(|| "http://localhost:8080".to_string())
                        );
                        set_url.set(proxy_url.clone());
                        
                        // Connect using the proxy
                        match app_state.connect_direct(&proxy_url).await {
                            Ok(_) => {
                                let navigate = use_navigate();
                                navigate("/pipeline", Default::default());
                            }
                            Err(e) => {
                                set_error.set(Some(format!("Auto-connect failed: {}", e)));
                                // Fall back to showing manual connection
                                set_url.set("http://localhost:8686".to_string());
                            }
                        }
                        set_connecting.set(false);
                    }
                    _ => {
                        // Not running inside Vectorize, show manual connection screen
                        set_url.set("http://localhost:8686".to_string());
                    }
                }
            });
        });
    }
    
    let connect = {
        let app_state = app_state.clone();
        move |_| {
            set_connecting.set(true);
            set_error.set(None);
            
            let url_value = url.get();
            let app_state = app_state.clone();
            
            spawn_local(async move {
                match app_state.connect_direct(&url_value).await {
                    Ok(_) => {
                        let navigate = use_navigate();
                        navigate("/pipeline", Default::default());
                    }
                    Err(e) => {
                        set_error.set(Some(format!("Connection failed: {}", e)));
                    }
                }
                set_connecting.set(false);
            });
        }
    };
    
    view! {
        <div class="flex-1 flex items-center justify-center p-8">
            <div class="max-w-md w-full">
                <div class="text-center mb-8">
                    <h1 class="text-4xl font-bold mb-2">
                        "Vectorize"
                    </h1>
                    <p class="text-slate-400">
                        "Visual Pipeline Builder for Vector"
                    </p>
                </div>
                
                // Show loading while checking for Vectorize
                <Show
                    when=move || !connecting.get() || error.get().is_some()
                    fallback=move || view! {
                        <div class="card text-center py-12">
                            <div class="animate-spin w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full mx-auto mb-4"></div>
                            <p class="text-slate-400">"Connecting to Vector..."</p>
                        </div>
                    }
                >
                    <div class="card">
                        <h2 class="text-xl font-semibold mb-4">
                            "Connect to Vector"
                        </h2>
                        
                        // Connection mode selector
                        <div class="mb-4">
                            <label class="input-label">"Connection Mode"</label>
                            <div class="flex gap-2">
                                {
                                    let app_state = app_state.clone();
                                    let app_state2 = app_state.clone();
                                    view! {
                                        <button
                                            class=move || {
                                                let base = "flex-1 py-2 px-4 rounded-lg border-2 transition-colors";
                                                if app_state.connection_mode.get() == ConnectionMode::Direct {
                                                    format!("{} border-blue-500 bg-blue-500/20", base)
                                                } else {
                                                    format!("{} border-slate-700 hover:border-slate-600", base)
                                                }
                                            }
                                            on:click={
                                                let app_state = app_state.clone();
                                                move |_| app_state.connection_mode.set(ConnectionMode::Direct)
                                            }
                                        >
                                            <div class="font-medium">"Direct"</div>
                                            <div class="text-xs text-slate-400">"Single Vector instance"</div>
                                        </button>
                                        <button
                                            class=move || {
                                                let base = "flex-1 py-2 px-4 rounded-lg border-2 transition-colors";
                                                if app_state2.connection_mode.get() == ConnectionMode::ControlPlane {
                                                    format!("{} border-blue-500 bg-blue-500/20", base)
                                                } else {
                                                    format!("{} border-slate-700 hover:border-slate-600", base)
                                                }
                                            }
                                            on:click={
                                                let app_state = app_state.clone();
                                                move |_| app_state.connection_mode.set(ConnectionMode::ControlPlane)
                                            }
                                        >
                                            <div class="font-medium">"Control Plane"</div>
                                            <div class="text-xs text-slate-400">"Multiple nodes"</div>
                                        </button>
                                    }
                                }
                            </div>
                        </div>
                        
                        // URL input
                        <div class="mb-4">
                            <label class="input-label">
                                {
                                    let app_state = app_state.clone();
                                    move || {
                                        if app_state.connection_mode.get() == ConnectionMode::Direct {
                                            "Vector API URL"
                                        } else {
                                            "Control Plane URL"
                                        }
                                    }
                                }
                            </label>
                            <input
                                type="text"
                                class="input"
                                placeholder="http://localhost:8686"
                                prop:value=move || url.get()
                                on:input=move |e| set_url.set(event_target_value(&e))
                            />
                            <p class="text-xs text-slate-500 mt-1">
                                {
                                    let app_state = app_state.clone();
                                    move || {
                                        if app_state.connection_mode.get() == ConnectionMode::Direct {
                                            "Make sure Vector is running with [api] enabled = true"
                                        } else {
                                            "Connect to the Vectorize control plane service"
                                        }
                                    }
                                }
                            </p>
                        </div>
                        
                        // Error message
                        {move || error.get().map(|e| view! {
                            <div class="mb-4 p-3 bg-red-500/20 border border-red-500 rounded-lg text-red-200 text-sm">
                                {e}
                            </div>
                        })}
                        
                        // Connect button
                        <button
                            class="btn-primary w-full"
                            disabled=move || connecting.get()
                            on:click=connect.clone()
                        >
                            {move || {
                                if connecting.get() {
                                    "Connecting..."
                                } else {
                                    "Connect"
                                }
                            }}
                        </button>
                    </div>
                </Show>
                
                // Quick start info
                <div class="mt-6 text-center text-sm text-slate-500">
                    <p>"New to Vector? "</p>
                    <a 
                        href="https://vector.dev/docs/setup/quickstart/"
                        target="_blank"
                        class="text-blue-400 hover:text-blue-300"
                    >
                        "Get started with Vector →"
                    </a>
                </div>
            </div>
        </div>
    }
}

/// Placeholder data view
#[component]
fn DataView() -> impl IntoView {
    view! {
        <div class="flex-1 flex items-center justify-center">
            <div class="text-center text-slate-400">
                <h2 class="text-xl font-semibold mb-2">"Live Data View"</h2>
                <p>"Coming soon - real-time event streaming"</p>
            </div>
        </div>
    }
}

/// Placeholder nodes view
#[component]
fn NodesView() -> impl IntoView {
    view! {
        <div class="flex-1 flex items-center justify-center">
            <div class="text-center text-slate-400">
                <h2 class="text-xl font-semibold mb-2">"Node Management"</h2>
                <p>"Available in Control Plane mode"</p>
            </div>
        </div>
    }
}

/// Placeholder settings view
#[component]
fn SettingsView() -> impl IntoView {
    view! {
        <div class="flex-1 flex items-center justify-center">
            <div class="text-center text-slate-400">
                <h2 class="text-xl font-semibold mb-2">"Settings"</h2>
                <p>"Configuration options coming soon"</p>
            </div>
        </div>
    }
}

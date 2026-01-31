//! Application Header Component

use leptos::*;
use leptos_router::*;

use crate::state::AppState;

/// Main application header with navigation
#[component]
pub fn Header() -> impl IntoView {
    let app_state = expect_context::<AppState>();
    
    view! {
        <header class="h-14 border-b border-slate-700 bg-slate-800/50 backdrop-blur-sm flex items-center px-4 gap-4">
            // Logo
            <A href="/" class="flex items-center gap-2 text-white font-bold text-lg">
                <span class="text-2xl">"🚀"</span>
                <span>"Vectorize"</span>
            </A>
            
            // Navigation (only show when connected)
            {
                let app_state = app_state.clone();
                view! {
                    <Show when=move || app_state.connected.get()>
                        {
                            let app_state = app_state.clone();
                            view! {
                                <nav class="flex items-center gap-1 ml-4">
                                    <NavLink href="/pipeline" label="Pipeline" />
                                    <NavLink href="/data" label="Data" />
                                    <Show when=move || app_state.connection_mode.get() == vectorize_shared::ConnectionMode::ControlPlane>
                                        <NavLink href="/nodes" label="Nodes" />
                                    </Show>
                                    <NavLink href="/settings" label="Settings" />
                                </nav>
                            }
                        }
                    </Show>
                }
            }
            
            // Spacer
            <div class="flex-1" />
            
            // Connection status
            {
                let app_state = app_state.clone();
                view! {
                    <Show when=move || app_state.connected.get()>
                        {
                            let app_state = app_state.clone();
                            let app_state_url = app_state.clone();
                            view! {
                                <div class="flex items-center gap-2 text-sm">
                                    <span class="status-dot healthy" />
                                    <span class="text-slate-400">
                                        "Connected to "
                                        <span class="text-white">{move || app_state_url.url.get()}</span>
                                    </span>
                                    <button
                                        class="btn-ghost text-xs px-2 py-1"
                                        on:click={
                                            let app_state = app_state.clone();
                                            move |_| {
                                                app_state.disconnect();
                                                let navigate = use_navigate();
                                                navigate("/", Default::default());
                                            }
                                        }
                                    >
                                        "Disconnect"
                                    </button>
                                </div>
                            }
                        }
                    </Show>
                }
            }
        </header>
    }
}

/// Navigation link component
#[component]
fn NavLink(
    href: &'static str,
    label: &'static str,
) -> impl IntoView {
    let location = use_location();
    
    view! {
        <A
            href=href
            class=move || {
                let is_active = location.pathname.get().starts_with(href);
                let base = "px-3 py-1.5 rounded-md text-sm font-medium transition-colors";
                if is_active {
                    format!("{} bg-slate-700 text-white", base)
                } else {
                    format!("{} text-slate-400 hover:text-white hover:bg-slate-700/50", base)
                }
            }
        >
            {label}
        </A>
    }
}

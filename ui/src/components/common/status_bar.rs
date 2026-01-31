//! Status Bar Component
//!
//! Bottom status bar showing connection info and metrics.

use leptos::*;

use crate::state::AppState;

/// Application status bar
#[component]
pub fn StatusBar() -> impl IntoView {
    let app_state = expect_context::<AppState>();
    
    view! {
        <footer class="h-8 border-t border-slate-700 bg-slate-800/50 flex items-center px-4 text-xs text-slate-400">
            // Connection mode
            <div class="flex items-center gap-2">
                <span>
                    {move || {
                        if app_state.connection_mode.get() == vectorize_shared::ConnectionMode::Direct {
                            "Direct Mode"
                        } else {
                            "Control Plane Mode"
                        }
                    }}
                </span>
            </div>
            
            // Spacer
            <div class="flex-1" />
            
            // Component count (when connected)
            <Show when=move || app_state.connected.get()>
                <div class="flex items-center gap-4">
                    {move || {
                        app_state.topology.get().map(|t| {
                            let sources = t.components.iter()
                                .filter(|c| c.component_kind == vectorize_shared::ComponentKind::Source)
                                .count();
                            let transforms = t.components.iter()
                                .filter(|c| c.component_kind == vectorize_shared::ComponentKind::Transform)
                                .count();
                            let sinks = t.components.iter()
                                .filter(|c| c.component_kind == vectorize_shared::ComponentKind::Sink)
                                .count();
                            
                            view! {
                                <span class="text-violet-400">{sources} " sources"</span>
                                <span class="text-cyan-400">{transforms} " transforms"</span>
                                <span class="text-orange-400">{sinks} " sinks"</span>
                            }
                        })
                    }}
                </div>
            </Show>
            
            // Version info
            <div class="ml-4">
                "Vectorize v0.1.0"
            </div>
        </footer>
    }
}

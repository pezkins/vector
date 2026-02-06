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
            
            // UI Pipeline component count
            <div class="flex items-center gap-4">
                {move || {
                    let pipeline = app_state.pipeline.get();
                    let sources = pipeline.nodes.values()
                        .filter(|n| matches!(n.node_type, vectorize_shared::NodeType::Source(_)))
                        .count();
                    let transforms = pipeline.nodes.values()
                        .filter(|n| matches!(n.node_type, vectorize_shared::NodeType::Transform(_)))
                        .count();
                    let sinks = pipeline.nodes.values()
                        .filter(|n| matches!(n.node_type, vectorize_shared::NodeType::Sink(_)))
                        .count();
                    let connections = pipeline.connections.len();
                    
                    view! {
                        <span class="text-violet-400">{sources} " sources"</span>
                        <span class="text-cyan-400">{transforms} " transforms"</span>
                        <span class="text-orange-400">{sinks} " sinks"</span>
                        <span class="text-slate-400">{connections} " connections"</span>
                    }
                }}
            </div>
            
            // Version info
            <div class="ml-4">
                "Vectorize v0.1.0"
            </div>
        </footer>
    }
}

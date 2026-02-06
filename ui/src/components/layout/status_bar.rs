//! Status Bar Component
//!
//! Bottom status bar showing connection status, quick stats, and notifications.

use leptos::*;

use crate::state::AppState;

/// Status bar at the bottom of the screen
#[component]
pub fn StatusBar() -> impl IntoView {
    let app_state = expect_context::<AppState>();
    
    // Count pipeline components
    let pipeline_stats = move || {
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
        (sources, transforms, sinks)
    };
    
    view! {
        <footer class="h-6 flex items-center justify-between px-3 bg-theme-bg border-t border-theme-border text-xs text-theme-muted flex-shrink-0">
            // Left side: Connection status
            <div class="flex items-center gap-4">
                // Connection indicator
                <div class="flex items-center gap-1.5">
                    <div class=move || {
                        let base = "w-2 h-2 rounded-full";
                        if app_state.connected.get() {
                            format!("{} bg-success", base)
                        } else {
                            format!("{} bg-theme-muted", base)
                        }
                    } />
                    <span>
                        {move || if app_state.connected.get() {
                            "Connected"
                        } else {
                            "Disconnected"
                        }}
                    </span>
                </div>
                
                // Mode indicator
                <Show when=move || app_state.connected.get()>
                    <div class="flex items-center gap-1">
                        <span class="text-theme-secondary">"Mode:"</span>
                        <span>{move || format!("{:?}", app_state.connection_mode.get())}</span>
                    </div>
                </Show>
            </div>
            
            // Center: Pipeline stats
            <Show when=move || app_state.connected.get()>
                <div class="flex items-center gap-3">
                    <div class="flex items-center gap-1">
                        <span class="w-2 h-2 rounded-sm bg-source" />
                        <span>{move || pipeline_stats().0}" sources"</span>
                    </div>
                    <div class="flex items-center gap-1">
                        <span class="w-2 h-2 rounded-sm bg-transform" />
                        <span>{move || pipeline_stats().1}" transforms"</span>
                    </div>
                    <div class="flex items-center gap-1">
                        <span class="w-2 h-2 rounded-sm bg-sink" />
                        <span>{move || pipeline_stats().2}" sinks"</span>
                    </div>
                </div>
            </Show>
            
            // Right side: Version / misc info
            <div class="flex items-center gap-2">
                <span class="text-theme-muted">"Vectorize v0.1.0"</span>
            </div>
        </footer>
    }
}

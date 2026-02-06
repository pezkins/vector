//! Topology View Component
//!
//! Visual representation of data flow across all agents and groups.

use leptos::*;

/// Topology visualization view
#[component]
pub fn TopologyView() -> impl IntoView {
    view! {
        <div class="flex-1 overflow-auto p-6">
            <div class="max-w-7xl mx-auto">
                <div class="flex items-center justify-between mb-6">
                    <div>
                        <h1 class="text-2xl font-bold text-theme">"Fleet Topology"</h1>
                        <p class="text-theme-secondary mt-1">"Visual representation of data flow across your fleet"</p>
                    </div>
                </div>
                
                // Placeholder
                <div class="bg-theme-surface rounded-xl border border-theme-border p-8">
                    <div class="text-center py-12">
                        <div class="w-20 h-20 rounded-full bg-theme-surface-hover flex items-center justify-center mx-auto mb-4">
                            <svg class="w-10 h-10 text-theme-muted" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                                <circle cx="12" cy="12" r="10" />
                                <path d="M12 6v6l4 2" />
                            </svg>
                        </div>
                        <h2 class="text-xl font-semibold text-theme mb-2">"Topology View"</h2>
                        <p class="text-theme-secondary max-w-md mx-auto">
                            "Interactive topology visualization showing how data flows between sources, transforms, and sinks across your entire fleet."
                        </p>
                        <p class="text-theme-muted text-sm mt-4">"Coming in a future update"</p>
                    </div>
                </div>
            </div>
        </div>
    }
}

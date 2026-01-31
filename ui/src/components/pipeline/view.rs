//! Pipeline View - Main pipeline builder page

use leptos::*;

use super::{ComponentPalette, PipelineCanvas};
use crate::components::common::*;
use crate::state::AppState;

/// Main pipeline builder view
#[component]
pub fn PipelineView() -> impl IntoView {
    let app_state = expect_context::<AppState>();
    let app_state_deploy = app_state.clone();
    let app_state_selected = app_state.clone();
    let app_state_close = app_state.clone();
    let app_state_props = app_state.clone();
    
    let (deploying, set_deploying) = create_signal(false);
    let (deploy_error, set_deploy_error) = create_signal(Option::<String>::None);
    let (deploy_success, set_deploy_success) = create_signal(false);
    
    // Deploy handler
    let deploy = move |_| {
        set_deploying.set(true);
        set_deploy_error.set(None);
        set_deploy_success.set(false);
        
        let app_state = app_state_deploy.clone();
        spawn_local(async move {
            match app_state.deploy_pipeline().await {
                Ok(_) => {
                    set_deploy_success.set(true);
                    // Clear success message after 3 seconds
                    gloo_timers::callback::Timeout::new(3000, move || {
                        set_deploy_success.set(false);
                    }).forget();
                }
                Err(e) => {
                    set_deploy_error.set(Some(e.to_string()));
                }
            }
            set_deploying.set(false);
        });
    };
    
    view! {
        <div class="flex-1 flex">
            // Left sidebar - Component palette
            <aside class="w-64 border-r border-slate-700 flex flex-col">
                <div class="p-4 border-b border-slate-700">
                    <h2 class="text-sm font-semibold text-slate-400 uppercase tracking-wide">
                        "Components"
                    </h2>
                </div>
                <div class="flex-1 overflow-y-auto custom-scrollbar">
                    <ComponentPalette />
                </div>
            </aside>
            
            // Main canvas area
            <div class="flex-1 flex flex-col">
                // Toolbar
                <div class="h-12 border-b border-slate-700 flex items-center px-4 gap-2">
                    <button
                        class="btn-primary flex items-center gap-2"
                        disabled=move || deploying.get()
                        on:click=deploy
                    >
                        <PlayIcon class="w-4 h-4" />
                        {move || if deploying.get() { "Deploying..." } else { "Deploy" }}
                    </button>
                    
                    <button class="btn-secondary flex items-center gap-2">
                        <RefreshIcon class="w-4 h-4" />
                        "Refresh"
                    </button>
                    
                    // Success message
                    <Show when=move || deploy_success.get()>
                        <span class="ml-2 text-green-400 text-sm animate-fade-in">
                            "✓ Deployed successfully"
                        </span>
                    </Show>
                    
                    // Error message
                    {move || deploy_error.get().map(|e| view! {
                        <span class="ml-2 text-red-400 text-sm">
                            "✗ " {e}
                        </span>
                    })}
                    
                    <div class="flex-1" />
                    
                    // View TOML button
                    <button class="btn-ghost text-sm">
                        "View TOML"
                    </button>
                </div>
                
                // Canvas
                <div class="flex-1 overflow-hidden">
                    <PipelineCanvas />
                </div>
            </div>
            
            // Right sidebar - Properties (when node selected)
            <Show when=move || app_state_selected.selected_node.get().is_some()>
                <aside class="w-80 border-l border-slate-700 flex flex-col">
                    <div class="p-4 border-b border-slate-700 flex items-center justify-between">
                        <h2 class="text-sm font-semibold text-slate-400 uppercase tracking-wide">
                            "Properties"
                        </h2>
                        <button
                            class="btn-ghost p-1"
                            on:click=move |_| app_state_close.selected_node.set(None)
                        >
                            "×"
                        </button>
                    </div>
                    <div class="flex-1 overflow-y-auto custom-scrollbar p-4">
                        <PropertiesPanel />
                    </div>
                </aside>
            </Show>
        </div>
    }
}

/// Properties panel for selected node
#[component]
fn PropertiesPanel() -> impl IntoView {
    let app_state = expect_context::<AppState>();
    
    view! {
        {move || {
            let node_id = app_state.selected_node.get()?;
            let pipeline = app_state.pipeline.get();
            let node = pipeline.nodes.get(&node_id)?;
            
            Some(view! {
                <div class="space-y-4">
                    // Node name
                    <div>
                        <label class="input-label">"Name"</label>
                        <input
                            type="text"
                            class="input"
                            value=node.name.clone()
                        />
                    </div>
                    
                    // Node type
                    <div>
                        <label class="input-label">"Type"</label>
                        <div class="text-sm text-slate-300 bg-slate-800 px-3 py-2 rounded-lg">
                            {node.node_type.display_name().to_string()}
                        </div>
                    </div>
                    
                    // Delete button
                    <div class="pt-4 border-t border-slate-700">
                        <button
                            class="btn-danger w-full flex items-center justify-center gap-2"
                            on:click=move |_| {
                                let mut pipeline = app_state.pipeline.get();
                                pipeline.remove_node(&node_id);
                                app_state.pipeline.set(pipeline);
                                app_state.selected_node.set(None);
                            }
                        >
                            <TrashIcon class="w-4 h-4" />
                            "Delete Node"
                        </button>
                    </div>
                </div>
            })
        }}
    }
}

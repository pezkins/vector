//! Component Palette
//!
//! Sidebar showing available Vector components for drag-and-drop.

use leptos::*;

use crate::components::common::*;

/// Component palette sidebar
#[component]
pub fn ComponentPalette() -> impl IntoView {
    view! {
        <div class="p-4 space-y-6">
            // Sources section
            <section>
                <h3 class="text-xs font-semibold text-violet-400 uppercase tracking-wide mb-2">
                    "Sources"
                </h3>
                <div class="space-y-1">
                    <PaletteItem component_type="stdin" label="Standard Input" category="source" />
                    <PaletteItem component_type="file" label="File" category="source" />
                    <PaletteItem component_type="http_server" label="HTTP Server" category="source" />
                    <PaletteItem component_type="demo_logs" label="Demo Logs" category="source" />
                </div>
            </section>
            
            // Transforms section
            <section>
                <h3 class="text-xs font-semibold text-cyan-400 uppercase tracking-wide mb-2">
                    "Transforms"
                </h3>
                <div class="space-y-1">
                    <PaletteItem component_type="remap" label="Remap (VRL)" category="transform" />
                    <PaletteItem component_type="filter" label="Filter" category="transform" />
                    <PaletteItem component_type="route" label="Route" category="transform" />
                </div>
            </section>
            
            // Sinks section
            <section>
                <h3 class="text-xs font-semibold text-orange-400 uppercase tracking-wide mb-2">
                    "Sinks"
                </h3>
                <div class="space-y-1">
                    <PaletteItem component_type="console" label="Console" category="sink" />
                    <PaletteItem component_type="file" label="File" category="sink" />
                    <PaletteItem component_type="http" label="HTTP" category="sink" />
                </div>
            </section>
        </div>
    }
}

/// Draggable palette item
#[component]
fn PaletteItem(
    component_type: &'static str,
    label: &'static str,
    category: &'static str,
) -> impl IntoView {
    let (dragging, set_dragging) = create_signal(false);
    
    let class = move || {
        let base = match category {
            "source" => "pipeline-node source",
            "transform" => "pipeline-node transform",
            "sink" => "pipeline-node sink",
            _ => "pipeline-node",
        };
        let drag_class = if dragging.get() { "opacity-50" } else { "" };
        format!("{} {}", base, drag_class)
    };
    
    view! {
        <div
            class=class
            draggable="true"
            on:dragstart=move |e: web_sys::DragEvent| {
                set_dragging.set(true);
                if let Some(data_transfer) = e.data_transfer() {
                    let _ = data_transfer.set_data(
                        "application/json",
                        &format!("\"{}\"", component_type)
                    );
                    data_transfer.set_effect_allowed("copy");
                }
            }
            on:dragend=move |_| {
                set_dragging.set(false);
            }
        >
            {match category {
                "source" => view! { <SourceIcon class="w-4 h-4 text-violet-400" /> }.into_view(),
                "transform" => view! { <TransformIcon class="w-4 h-4 text-cyan-400" /> }.into_view(),
                "sink" => view! { <SinkIcon class="w-4 h-4 text-orange-400" /> }.into_view(),
                _ => view! { <TransformIcon class="w-4 h-4" /> }.into_view(),
            }}
            <span class="text-sm">{label}</span>
        </div>
    }
}

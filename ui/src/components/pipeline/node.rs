//! Pipeline Node Component
//!
//! Visual representation of a node in the pipeline canvas.

use leptos::*;
use vectorize_shared::{NodeType, PipelineNode as PipelineNodeData};

use crate::components::common::*;

/// Pipeline node on the canvas
#[component]
pub fn PipelineNode<F>(
    node: PipelineNodeData,
    #[prop(into)] on_select: Callback<String>,
    selected: F,
) -> impl IntoView
where
    F: Fn() -> bool + 'static,
{
    let (dragging, set_dragging) = create_signal(false);
    let node_id = node.id.clone();
    let node_id_click = node_id.clone();
    let node_id_drag = node_id.clone();
    
    let category = match &node.node_type {
        NodeType::Source(_) => "source",
        NodeType::Transform(_) => "transform",
        NodeType::Sink(_) => "sink",
    };
    
    let class = move || {
        let base = match category {
            "source" => "pipeline-node source",
            "transform" => "pipeline-node transform",
            "sink" => "pipeline-node sink",
            _ => "pipeline-node",
        };
        let selected_class = if selected() { "selected" } else { "" };
        let drag_class = if dragging.get() { "cursor-grabbing" } else { "cursor-grab" };
        format!("{} {} {} shadow-lg", base, selected_class, drag_class)
    };
    
    let style = move || {
        format!(
            "position: absolute; left: {}px; top: {}px; min-width: 150px;",
            node.position.x,
            node.position.y
        )
    };
    
    view! {
        <div
            class=class
            style=style
            on:click=move |e| {
                e.stop_propagation();
                on_select.call(node_id_click.clone());
            }
            draggable="true"
            on:dragstart=move |e: web_sys::DragEvent| {
                set_dragging.set(true);
                // Store node ID for potential move operation
                if let Some(dt) = e.data_transfer() {
                    let _ = dt.set_data("text/plain", &node_id_drag);
                }
            }
            on:dragend=move |_| {
                set_dragging.set(false);
            }
        >
            // Icon
            {match category {
                "source" => view! { <SourceIcon class="w-5 h-5 text-violet-400" /> }.into_view(),
                "transform" => view! { <TransformIcon class="w-5 h-5 text-cyan-400" /> }.into_view(),
                "sink" => view! { <SinkIcon class="w-5 h-5 text-orange-400" /> }.into_view(),
                _ => view! { <TransformIcon class="w-5 h-5" /> }.into_view(),
            }}
            
            // Label
            <div class="flex flex-col">
                <span class="text-sm font-medium">{node.name.clone()}</span>
                <span class="text-xs text-slate-400">{node.node_type.display_name().to_string()}</span>
            </div>
            
            // Connection points (TODO: make these functional)
            {match category {
                "source" => view! {
                    <div class="absolute -right-2 top-1/2 -translate-y-1/2 w-4 h-4 rounded-full bg-violet-500 border-2 border-slate-900" />
                }.into_view(),
                "transform" => view! {
                    <div class="absolute -left-2 top-1/2 -translate-y-1/2 w-4 h-4 rounded-full bg-cyan-500 border-2 border-slate-900" />
                    <div class="absolute -right-2 top-1/2 -translate-y-1/2 w-4 h-4 rounded-full bg-cyan-500 border-2 border-slate-900" />
                }.into_view(),
                "sink" => view! {
                    <div class="absolute -left-2 top-1/2 -translate-y-1/2 w-4 h-4 rounded-full bg-orange-500 border-2 border-slate-900" />
                }.into_view(),
                _ => view! {}.into_view(),
            }}
        </div>
    }
}

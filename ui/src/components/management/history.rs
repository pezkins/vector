//! Configuration history and diff viewer component

use leptos::*;
use serde::{Deserialize, Serialize};

/// Commit info from git history
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: String,
}

/// Diff response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResponse {
    pub from_version: String,
    pub to_version: String,
    pub diff: String,
    pub has_changes: bool,
}

/// Config at version response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigAtVersion {
    pub config: Option<String>,
    pub version: String,
    pub group_name: String,
}

/// Fetch config history for a group
async fn fetch_history(group_id: &str) -> Result<Vec<CommitInfo>, String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let response = gloo_net::http::Request::get(&format!("{}/api/v1/groups/{}/history", origin, group_id))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json().await.map_err(|e| format!("Parse failed: {}", e))
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

/// Fetch diff between two versions
async fn fetch_diff(group_id: &str, from: &str, to: &str) -> Result<DiffResponse, String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let response = gloo_net::http::Request::get(
        &format!("{}/api/v1/groups/{}/diff?from={}&to={}", origin, group_id, from, to)
    )
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json().await.map_err(|e| format!("Parse failed: {}", e))
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

/// Fetch config at specific version
async fn fetch_config_at_version(group_id: &str, version: &str) -> Result<ConfigAtVersion, String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let response = gloo_net::http::Request::get(
        &format!("{}/api/v1/groups/{}/config/{}", origin, group_id, version)
    )
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json().await.map_err(|e| format!("Parse failed: {}", e))
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

/// Rollback to a specific version
async fn rollback_to_version(group_id: &str, version: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window")?;
    let origin = window.location().origin().map_err(|_| "No origin")?;
    
    let body = serde_json::json!({ "version": version });
    
    let response = gloo_net::http::Request::post(&format!("{}/api/v1/groups/{}/rollback", origin, group_id))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .map_err(|e| format!("Body error: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        Ok(())
    } else {
        Err(format!("Rollback failed: {}", response.status()))
    }
}

/// Configuration history component
#[component]
pub fn ConfigHistory(
    #[prop(into)] group_id: String,
) -> impl IntoView {
    let (history, set_history) = create_signal(Vec::<CommitInfo>::new());
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal(Option::<String>::None);
    let (selected_hash, set_selected_hash) = create_signal(Option::<String>::None);
    let (config_preview, set_config_preview) = create_signal(Option::<String>::None);
    let (diff_result, set_diff_result) = create_signal(Option::<DiffResponse>::None);
    let (compare_from, set_compare_from) = create_signal(Option::<String>::None);
    
    let group_id_clone = group_id.clone();
    
    // Fetch history on mount
    create_effect(move |_| {
        let gid = group_id_clone.clone();
        spawn_local(async move {
            set_loading.set(true);
            match fetch_history(&gid).await {
                Ok(h) => {
                    set_history.set(h);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    });
    
    view! {
        <div class="flex flex-col h-full">
            // Toolbar
            <div class="flex items-center justify-between p-2 bg-slate-800 rounded-t-lg">
                <div class="text-sm text-slate-400">
                    {move || format!("{} commits", history.get().len())}
                </div>
                {move || compare_from.get().map(|from| {
                    let from_short = from[..8.min(from.len())].to_string();
                    view! {
                        <div class="flex items-center space-x-2">
                            <span class="text-xs text-slate-400">
                                "Comparing from "
                                <code class="bg-slate-900 px-1 rounded">{from_short}</code>
                            </span>
                            <button
                                class="px-2 py-1 text-xs bg-slate-700 hover:bg-slate-600 rounded"
                                on:click=move |_| {
                                    set_compare_from.set(None);
                                    set_diff_result.set(None);
                                }
                            >
                                "Cancel"
                            </button>
                        </div>
                    }
                })}
            </div>
            
            // Main content
            <div class="flex flex-1 min-h-0">
                // Commit list
                <div class="w-1/2 border-r border-slate-700 overflow-y-auto">
                    {move || {
                        if loading.get() {
                            view! { <div class="p-4 text-slate-400">"Loading..."</div> }.into_view()
                        } else if let Some(err) = error.get() {
                            view! { <div class="p-4 text-red-400">{err}</div> }.into_view()
                        } else {
                            let commits = history.get();
                            if commits.is_empty() {
                                view! {
                                    <div class="p-4 text-slate-400 text-center">
                                        "No history yet"
                                    </div>
                                }.into_view()
                            } else {
                                view! {
                                    <div class="divide-y divide-slate-700">
                                        {commits.into_iter().map(|commit| {
                                            let hash = commit.hash.clone();
                                            let short_hash = commit.short_hash.clone();
                                            let message = commit.message.clone();
                                            let author = commit.author.clone();
                                            let timestamp = commit.timestamp.clone();
                                            let gid = group_id.clone();
                                            
                                            view! {
                                                <CommitRow
                                                    hash=hash
                                                    short_hash=short_hash
                                                    message=message
                                                    author=author
                                                    timestamp=timestamp
                                                    group_id=gid
                                                    selected_hash=selected_hash
                                                    set_selected_hash=set_selected_hash
                                                    compare_from=compare_from
                                                    set_compare_from=set_compare_from
                                                    set_config_preview=set_config_preview
                                                    set_diff_result=set_diff_result
                                                    set_history=set_history
                                                />
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                }.into_view()
                            }
                        }
                    }}
                </div>
                
                // Preview/Diff panel
                <div class="w-1/2 overflow-y-auto bg-slate-900">
                    {move || {
                        if let Some(diff) = diff_result.get() {
                            let from_short = diff.from_version[..8.min(diff.from_version.len())].to_string();
                            let to_short = diff.to_version[..8.min(diff.to_version.len())].to_string();
                            let diff_text = diff.diff.clone();
                            view! {
                                <div class="p-4">
                                    <div class="flex items-center justify-between mb-4">
                                        <h3 class="font-medium">"Diff"</h3>
                                        <button
                                            class="px-2 py-1 text-xs bg-slate-700 hover:bg-slate-600 rounded"
                                            on:click=move |_| set_diff_result.set(None)
                                        >
                                            "Close Diff"
                                        </button>
                                    </div>
                                    <div class="text-xs mb-2 text-slate-400">
                                        <code class="bg-slate-800 px-1 rounded">{from_short}</code>
                                        " → "
                                        <code class="bg-slate-800 px-1 rounded">{to_short}</code>
                                    </div>
                                    <DiffViewer diff=diff_text />
                                </div>
                            }.into_view()
                        } else if let Some(config) = config_preview.get() {
                            view! {
                                <div class="p-4">
                                    <h3 class="font-medium mb-2">"Config Preview"</h3>
                                    <pre class="text-xs text-slate-300 whitespace-pre-wrap font-mono bg-slate-950 p-3 rounded overflow-x-auto">
                                        {config}
                                    </pre>
                                </div>
                            }.into_view()
                        } else {
                            view! {
                                <div class="p-4 text-slate-500 text-center">
                                    "Select a commit to preview"
                                </div>
                            }.into_view()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

/// Individual commit row component to avoid closure complexity
#[component]
fn CommitRow(
    hash: String,
    short_hash: String,
    message: String,
    author: String,
    timestamp: String,
    group_id: String,
    selected_hash: ReadSignal<Option<String>>,
    set_selected_hash: WriteSignal<Option<String>>,
    compare_from: ReadSignal<Option<String>>,
    set_compare_from: WriteSignal<Option<String>>,
    set_config_preview: WriteSignal<Option<String>>,
    set_diff_result: WriteSignal<Option<DiffResponse>>,
    set_history: WriteSignal<Vec<CommitInfo>>,
) -> impl IntoView {
    let hash_clone = hash.clone();
    let hash_for_class = hash.clone();
    let hash_for_click = hash.clone();
    let hash_for_compare = hash.clone();
    let hash_for_compare_btn = hash.clone();
    let hash_for_rollback = hash.clone();
    let gid_for_click = group_id.clone();
    let gid_for_diff = group_id.clone();
    let gid_for_rollback = group_id.clone();
    
    view! {
        <div
            class=move || {
                let is_selected = selected_hash.get().as_ref() == Some(&hash_for_class);
                let is_compare = compare_from.get().as_ref() == Some(&hash_for_class);
                format!(
                    "p-3 cursor-pointer hover:bg-slate-800 {}",
                    if is_selected || is_compare { "bg-slate-800" } else { "" }
                )
            }
            on:click={
                let hash = hash_for_click.clone();
                let gid = gid_for_click.clone();
                move |_| {
                    set_selected_hash.set(Some(hash.clone()));
                    let hash = hash.clone();
                    let gid = gid.clone();
                    spawn_local(async move {
                        if let Ok(cfg) = fetch_config_at_version(&gid, &hash).await {
                            set_config_preview.set(cfg.config);
                        }
                    });
                }
            }
        >
            <div class="flex items-center justify-between">
                <div class="flex items-center space-x-2">
                    {move || {
                        if compare_from.get().as_ref() == Some(&hash_clone) {
                            view! { <span class="text-xs bg-blue-600 px-1 rounded">"FROM"</span> }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                    <code class="text-xs bg-slate-900 px-1 rounded text-blue-400">
                        {short_hash.clone()}
                    </code>
                </div>
                <div class="flex items-center space-x-1">
                    {move || {
                        let from = compare_from.get();
                        let current_hash = hash_for_compare.clone();
                        let gid = gid_for_diff.clone();
                        
                        if let Some(from_hash) = from {
                            if from_hash != current_hash {
                                let from_h = from_hash.clone();
                                let to_h = current_hash.clone();
                                let gid_c = gid.clone();
                                view! {
                                    <button
                                        class="px-2 py-0.5 text-xs bg-blue-600 hover:bg-blue-700 rounded"
                                        on:click=move |e| {
                                            e.stop_propagation();
                                            let from = from_h.clone();
                                            let to = to_h.clone();
                                            let gid = gid_c.clone();
                                            spawn_local(async move {
                                                if let Ok(diff) = fetch_diff(&gid, &from, &to).await {
                                                    set_diff_result.set(Some(diff));
                                                }
                                            });
                                        }
                                    >
                                        "Compare"
                                    </button>
                                }.into_view()
                            } else {
                                view! {}.into_view()
                            }
                        } else {
                            let hash = hash_for_compare_btn.clone();
                            view! {
                                <button
                                    class="px-2 py-0.5 text-xs bg-slate-700 hover:bg-slate-600 rounded"
                                    on:click=move |e| {
                                        e.stop_propagation();
                                        set_compare_from.set(Some(hash.clone()));
                                    }
                                >
                                    "Compare"
                                </button>
                            }.into_view()
                        }
                    }}
                    <button
                        class="px-2 py-0.5 text-xs bg-yellow-600 hover:bg-yellow-700 rounded"
                        on:click={
                            let hash = hash_for_rollback.clone();
                            let gid = gid_for_rollback.clone();
                            move |e| {
                                e.stop_propagation();
                                let hash = hash.clone();
                                let gid = gid.clone();
                                spawn_local(async move {
                                    if rollback_to_version(&gid, &hash).await.is_ok() {
                                        if let Ok(h) = fetch_history(&gid).await {
                                            set_history.set(h);
                                        }
                                    }
                                });
                            }
                        }
                    >
                        "Rollback"
                    </button>
                </div>
            </div>
            <div class="mt-1 text-sm truncate">{message.clone()}</div>
            <div class="mt-1 text-xs text-slate-500">
                {author.clone()}" • "{timestamp.clone()}
            </div>
        </div>
    }
}

/// Diff viewer component with syntax highlighting
#[component]
pub fn DiffViewer(
    #[prop(into)] diff: String,
) -> impl IntoView {
    // Split into owned strings for static lifetime
    let lines: Vec<String> = diff.lines().map(|s| s.to_string()).collect();
    
    view! {
        <div class="font-mono text-xs bg-slate-950 rounded overflow-x-auto">
            {lines.into_iter().map(|line| {
                let (class, prefix) = if line.starts_with('+') {
                    ("bg-green-900/30 text-green-400", "+")
                } else if line.starts_with('-') {
                    ("bg-red-900/30 text-red-400", "-")
                } else if line.starts_with("@@") {
                    ("bg-blue-900/30 text-blue-400", "@")
                } else {
                    ("text-slate-400", " ")
                };
                
                let content = if line.len() > 1 { 
                    line[1..].to_string() 
                } else { 
                    String::new() 
                };
                
                view! {
                    <div class=format!("px-2 py-0.5 {}", class)>
                        <span class="select-none mr-2 text-slate-600">{prefix}</span>
                        {content}
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}

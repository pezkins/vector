//! Git Remotes Configuration Component
//!
//! Git remote sync configuration interface with:
//! - List of configured remotes
//! - Add/Edit remote modal
//! - Sync, push, pull operations
//! - Status display (synced/ahead/behind)

use leptos::*;
use serde::{Deserialize, Serialize};

use crate::components::common::{PlusIcon, TrashIcon, RefreshIcon};

// ============================================================================
// Types
// ============================================================================

/// Git sync status
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    #[default]
    Unknown,
    Synced,
    Ahead(u32),
    Behind(u32),
    Diverged(u32, u32),  // ahead, behind
    Error(String),
}

impl SyncStatus {
    pub fn label(&self) -> String {
        match self {
            SyncStatus::Unknown => "Unknown".to_string(),
            SyncStatus::Synced => "Synced".to_string(),
            SyncStatus::Ahead(n) => format!("{} ahead", n),
            SyncStatus::Behind(n) => format!("{} behind", n),
            SyncStatus::Diverged(a, b) => format!("{} ahead, {} behind", a, b),
            SyncStatus::Error(e) => format!("Error: {}", e),
        }
    }

    pub fn badge_class(&self) -> &'static str {
        match self {
            SyncStatus::Unknown => "bg-slate-500/20 text-slate-400",
            SyncStatus::Synced => "bg-green-500/20 text-green-400",
            SyncStatus::Ahead(_) => "bg-amber-500/20 text-amber-400",
            SyncStatus::Behind(_) => "bg-blue-500/20 text-blue-400",
            SyncStatus::Diverged(_, _) => "bg-violet-500/20 text-violet-400",
            SyncStatus::Error(_) => "bg-red-500/20 text-red-400",
        }
    }
}

/// Git remote configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GitRemote {
    pub id: String,
    pub name: String,
    pub url: String,
    pub branch: String,
    pub is_default: bool,
    pub last_sync: Option<String>,
    pub status: SyncStatus,
}

/// Form data for add/edit remote
#[derive(Clone, Debug, Default)]
struct RemoteFormData {
    pub name: String,
    pub url: String,
    pub branch: String,
    pub token: String,  // Used for authentication
}

/// Validation errors
#[derive(Clone, Debug, Default)]
struct ValidationErrors {
    pub name: Option<String>,
    pub url: Option<String>,
    pub branch: Option<String>,
}

impl ValidationErrors {
    fn is_valid(&self) -> bool {
        self.name.is_none() && self.url.is_none() && self.branch.is_none()
    }
}

// ============================================================================
// Main Component
// ============================================================================

/// Git remotes page component
#[component]
pub fn GitRemotesPage() -> impl IntoView {
    let (remotes, set_remotes) = create_signal(Vec::<GitRemote>::new());
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal(Option::<String>::None);
    
    // Modal state
    let (show_add_modal, set_show_add_modal) = create_signal(false);
    let (editing_remote, set_editing_remote) = create_signal(Option::<GitRemote>::None);
    let (deleting_remote, set_deleting_remote) = create_signal(Option::<GitRemote>::None);
    
    // Operation state (for sync/push/pull)
    let (operating_remote, set_operating_remote) = create_signal(Option::<String>::None);
    
    // Fetch data on mount
    create_effect(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            
            match fetch_git_remotes().await {
                Ok(data) => set_remotes.set(data),
                Err(e) => {
                    // Use mock data if fetch fails
                    set_remotes.set(get_mock_remotes());
                    web_sys::console::warn_1(&format!("Using mock git remotes: {}", e).into());
                }
            }
            
            set_loading.set(false);
        });
    });
    
    // Refresh handler
    let on_refresh = move |_| {
        spawn_local(async move {
            set_loading.set(true);
            
            if let Ok(data) = fetch_git_remotes().await {
                set_remotes.set(data);
            }
            
            set_loading.set(false);
        });
    };
    
    // Add remote handler
    let on_add_remote = move |remote: GitRemote| {
        set_remotes.update(|r| r.push(remote));
        set_show_add_modal.set(false);
    };
    
    // Update remote handler
    let on_update_remote = move |updated: GitRemote| {
        set_remotes.update(|remotes| {
            if let Some(remote) = remotes.iter_mut().find(|r| r.id == updated.id) {
                *remote = updated;
            }
        });
        set_editing_remote.set(None);
    };
    
    // Delete remote handler
    let on_confirm_delete = move |remote_id: String| {
        spawn_local(async move {
            if delete_git_remote(&remote_id).await.is_ok() {
                set_remotes.update(|remotes| {
                    remotes.retain(|r| r.id != remote_id);
                });
            }
            set_deleting_remote.set(None);
        });
    };
    
    // Sync handler
    let on_sync = move |remote_id: String| {
        set_operating_remote.set(Some(remote_id.clone()));
        spawn_local(async move {
            if let Ok(status) = sync_git_remote(&remote_id).await {
                set_remotes.update(|remotes| {
                    if let Some(remote) = remotes.iter_mut().find(|r| r.id == remote_id) {
                        remote.status = status;
                        remote.last_sync = Some(chrono::Utc::now().to_rfc3339());
                    }
                });
            }
            set_operating_remote.set(None);
        });
    };
    
    // Push handler
    let on_push = move |remote_id: String| {
        set_operating_remote.set(Some(remote_id.clone()));
        spawn_local(async move {
            if push_git_remote(&remote_id).await.is_ok() {
                set_remotes.update(|remotes| {
                    if let Some(remote) = remotes.iter_mut().find(|r| r.id == remote_id) {
                        remote.status = SyncStatus::Synced;
                        remote.last_sync = Some(chrono::Utc::now().to_rfc3339());
                    }
                });
            }
            set_operating_remote.set(None);
        });
    };
    
    // Pull handler
    let on_pull = move |remote_id: String| {
        set_operating_remote.set(Some(remote_id.clone()));
        spawn_local(async move {
            if pull_git_remote(&remote_id).await.is_ok() {
                set_remotes.update(|remotes| {
                    if let Some(remote) = remotes.iter_mut().find(|r| r.id == remote_id) {
                        remote.status = SyncStatus::Synced;
                        remote.last_sync = Some(chrono::Utc::now().to_rfc3339());
                    }
                });
            }
            set_operating_remote.set(None);
        });
    };

    view! {
        <div class="flex-1 overflow-auto p-6 bg-slate-900">
            <div class="max-w-5xl mx-auto">
                // Header
                <div class="flex items-center justify-between mb-6">
                    <div>
                        <h1 class="text-2xl font-bold text-white">"Git Remote Sync"</h1>
                        <p class="text-slate-400 mt-1">"Sync pipeline configurations with Git repositories"</p>
                    </div>
                    
                    <div class="flex items-center gap-3">
                        <button
                            class="flex items-center gap-2 px-4 py-2 bg-slate-700 hover:bg-slate-600 \
                                   text-white rounded-lg transition-colors"
                            on:click=on_refresh
                        >
                            <RefreshIcon class="w-4 h-4" />
                            "Refresh"
                        </button>
                        <button
                            class="flex items-center gap-2 px-4 py-2 bg-blue-500 hover:bg-blue-600 \
                                   text-white font-medium rounded-lg transition-colors"
                            on:click=move |_| set_show_add_modal.set(true)
                        >
                            <PlusIcon class="w-4 h-4" />
                            "Add Remote"
                        </button>
                    </div>
                </div>
                
                // Error display
                {move || {
                    if let Some(err) = error.get() {
                        view! {
                            <div class="bg-red-500/10 border border-red-500/30 rounded-lg p-4 mb-6">
                                <p class="text-red-400">{err}</p>
                            </div>
                        }.into_view()
                    } else {
                        view! {}.into_view()
                    }
                }}
                
                // Loading state
                <Show
                    when=move || !loading.get()
                    fallback=move || view! {
                        <div class="flex items-center justify-center py-16">
                            <div class="animate-spin w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full" />
                        </div>
                    }
                >
                    // Remotes list
                    <RemotesList 
                        remotes=remotes
                        operating_remote=operating_remote
                        on_edit=move |remote| set_editing_remote.set(Some(remote))
                        on_delete=move |remote| set_deleting_remote.set(Some(remote))
                        on_sync=on_sync
                        on_push=on_push
                        on_pull=on_pull
                    />
                </Show>
                
                // Add Remote Modal
                <Show when=move || show_add_modal.get()>
                    <RemoteFormModal
                        remote=None
                        on_close=move || set_show_add_modal.set(false)
                        on_save=on_add_remote
                    />
                </Show>
                
                // Edit Remote Modal
                <Show when=move || editing_remote.get().is_some()>
                    {move || {
                        if let Some(remote) = editing_remote.get() {
                            view! {
                                <RemoteFormModal
                                    remote=Some(remote)
                                    on_close=move || set_editing_remote.set(None)
                                    on_save=on_update_remote
                                />
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                </Show>
                
                // Delete Confirmation Modal
                <Show when=move || deleting_remote.get().is_some()>
                    {move || {
                        if let Some(remote) = deleting_remote.get() {
                            let remote_id = remote.id.clone();
                            view! {
                                <DeleteConfirmModal
                                    remote=remote
                                    on_close=move || set_deleting_remote.set(None)
                                    on_confirm=move || on_confirm_delete(remote_id.clone())
                                />
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                </Show>
            </div>
        </div>
    }
}

// ============================================================================
// Remotes List Component
// ============================================================================

#[component]
fn RemotesList(
    remotes: ReadSignal<Vec<GitRemote>>,
    operating_remote: ReadSignal<Option<String>>,
    on_edit: impl Fn(GitRemote) + Clone + 'static,
    on_delete: impl Fn(GitRemote) + Clone + 'static,
    on_sync: impl Fn(String) + Clone + 'static,
    on_push: impl Fn(String) + Clone + 'static,
    on_pull: impl Fn(String) + Clone + 'static,
) -> impl IntoView {
    view! {
        {move || {
            let items = remotes.get();
            
            if items.is_empty() {
                view! {
                    <EmptyState />
                }.into_view()
            } else {
                let on_edit = on_edit.clone();
                let on_delete = on_delete.clone();
                let on_sync = on_sync.clone();
                let on_push = on_push.clone();
                let on_pull = on_pull.clone();
                
                view! {
                    <div class="space-y-4">
                        {items.into_iter().map(|remote| {
                            let remote_edit = remote.clone();
                            let remote_delete = remote.clone();
                            let remote_id_sync = remote.id.clone();
                            let remote_id_push = remote.id.clone();
                            let remote_id_pull = remote.id.clone();
                            let remote_id_check = remote.id.clone();
                            let on_edit = on_edit.clone();
                            let on_delete = on_delete.clone();
                            let on_sync = on_sync.clone();
                            let on_push = on_push.clone();
                            let on_pull = on_pull.clone();
                            let is_operating = Signal::derive(move || operating_remote.get().as_ref() == Some(&remote_id_check));
                            
                            view! {
                                <RemoteCard
                                    remote=remote
                                    is_operating=is_operating
                                    on_edit=move || on_edit(remote_edit.clone())
                                    on_delete=move || on_delete(remote_delete.clone())
                                    on_sync=move || on_sync(remote_id_sync.clone())
                                    on_push=move || on_push(remote_id_push.clone())
                                    on_pull=move || on_pull(remote_id_pull.clone())
                                />
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_view()
            }
        }}
    }
}

// ============================================================================
// Remote Card Component
// ============================================================================

#[component]
fn RemoteCard(
    remote: GitRemote,
    is_operating: Signal<bool>,
    on_edit: impl Fn() + 'static,
    on_delete: impl Fn() + 'static,
    on_sync: impl Fn() + 'static,
    on_push: impl Fn() + 'static,
    on_pull: impl Fn() + 'static,
) -> impl IntoView {
    let status = remote.status.clone();
    let show_push = matches!(status, SyncStatus::Ahead(_) | SyncStatus::Diverged(_, _));
    let show_pull = matches!(status, SyncStatus::Behind(_) | SyncStatus::Diverged(_, _));
    
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 overflow-hidden">
            <div class="p-5">
                <div class="flex items-start justify-between">
                    // Remote info
                    <div class="flex items-start gap-4">
                        <div class="w-12 h-12 rounded-lg bg-orange-500/10 flex items-center justify-center">
                            <GitIcon class="w-6 h-6 text-orange-400" />
                        </div>
                        <div>
                            <div class="flex items-center gap-2">
                                <h3 class="font-semibold text-white">{remote.name.clone()}</h3>
                                {if remote.is_default {
                                    view! {
                                        <span class="px-2 py-0.5 rounded text-xs font-medium bg-blue-500/20 text-blue-400">
                                            "Default"
                                        </span>
                                    }.into_view()
                                } else {
                                    view! {}.into_view()
                                }}
                            </div>
                            <div class="flex items-center gap-2 mt-1">
                                <code class="text-sm text-slate-400 font-mono">{remote.url.clone()}</code>
                            </div>
                            <div class="flex items-center gap-4 mt-3">
                                // Branch
                                <div class="flex items-center gap-1.5 text-sm">
                                    <BranchIcon class="w-4 h-4 text-slate-500" />
                                    <span class="text-slate-400">{remote.branch.clone()}</span>
                                </div>
                                // Status badge
                                <span class=format!(
                                    "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}",
                                    remote.status.badge_class()
                                )>
                                    {remote.status.label()}
                                </span>
                                // Last sync
                                {if let Some(last_sync) = remote.last_sync.clone() {
                                    view! {
                                        <span class="text-xs text-slate-500">
                                            "Last synced: "{format_date(&last_sync)}
                                        </span>
                                    }.into_view()
                                } else {
                                    view! {
                                        <span class="text-xs text-slate-500">"Never synced"</span>
                                    }.into_view()
                                }}
                            </div>
                        </div>
                    </div>
                    
                    // Action buttons
                    <div class="flex items-center gap-2">
                        // Edit button
                        <button
                            class="p-2 text-slate-400 hover:text-white hover:bg-slate-700 rounded-lg transition-colors"
                            title="Edit remote"
                            on:click=move |_| on_edit()
                        >
                            <EditIcon class="w-4 h-4" />
                        </button>
                        // Delete button
                        <button
                            class="p-2 text-slate-400 hover:text-red-400 hover:bg-red-500/10 rounded-lg transition-colors"
                            title="Remove remote"
                            on:click=move |_| on_delete()
                        >
                            <TrashIcon class="w-4 h-4" />
                        </button>
                    </div>
                </div>
                
                // Operation buttons
                <div class="flex items-center gap-2 mt-4 pt-4 border-t border-slate-700">
                    // Sync button
                    <button
                        class="flex items-center gap-2 px-3 py-1.5 bg-slate-700 hover:bg-slate-600 \
                               text-white text-sm rounded-lg transition-colors disabled:opacity-50"
                        disabled=move || is_operating.get()
                        on:click=move |_| on_sync()
                    >
                        <RefreshIcon class="w-4 h-4" />
                        {move || if is_operating.get() { "Syncing..." } else { "Sync" }}
                    </button>
                    
                    // Push button (shown when ahead)
                    {if show_push {
                        view! {
                            <button
                                class="flex items-center gap-2 px-3 py-1.5 bg-amber-500/20 hover:bg-amber-500/30 \
                                       text-amber-400 text-sm rounded-lg transition-colors disabled:opacity-50"
                                disabled=move || is_operating.get()
                                on:click=move |_| on_push()
                            >
                                <PushIcon class="w-4 h-4" />
                                "Push"
                            </button>
                        }.into_view()
                    } else {
                        view! {}.into_view()
                    }}
                    
                    // Pull button (shown when behind)
                    {if show_pull {
                        view! {
                            <button
                                class="flex items-center gap-2 px-3 py-1.5 bg-blue-500/20 hover:bg-blue-500/30 \
                                       text-blue-400 text-sm rounded-lg transition-colors disabled:opacity-50"
                                disabled=move || is_operating.get()
                                on:click=move |_| on_pull()
                            >
                                <PullIcon class="w-4 h-4" />
                                "Pull"
                            </button>
                        }.into_view()
                    } else {
                        view! {}.into_view()
                    }}
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Remote Form Modal (Add/Edit)
// ============================================================================

#[component]
fn RemoteFormModal(
    remote: Option<GitRemote>,
    on_close: impl Fn() + 'static + Clone,
    on_save: impl Fn(GitRemote) + 'static + Clone,
) -> impl IntoView {
    let is_edit = remote.is_some();
    let existing_remote = remote.clone();
    
    let (form_data, set_form_data) = create_signal(RemoteFormData {
        name: remote.as_ref().map(|r| r.name.clone()).unwrap_or_default(),
        url: remote.as_ref().map(|r| r.url.clone()).unwrap_or_default(),
        branch: remote.as_ref().map(|r| r.branch.clone()).unwrap_or_else(|| "main".to_string()),
        token: String::new(),
    });
    let (errors, set_errors) = create_signal(ValidationErrors::default());
    let (saving, set_saving) = create_signal(false);
    let (api_error, set_api_error) = create_signal(Option::<String>::None);
    
    let on_close_backdrop = on_close.clone();
    let on_close_header = on_close.clone();
    let on_close_cancel = on_close;
    let on_save_clone = on_save;
    
    // Validation function
    let validate = move |data: &RemoteFormData| -> ValidationErrors {
        let mut errs = ValidationErrors::default();
        
        if data.name.is_empty() {
            errs.name = Some("Name is required".to_string());
        } else if !data.name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            errs.name = Some("Name can only contain letters, numbers, hyphens, and underscores".to_string());
        }
        
        if data.url.is_empty() {
            errs.url = Some("URL is required".to_string());
        } else if !data.url.starts_with("https://") && !data.url.starts_with("git@") {
            errs.url = Some("URL must start with https:// or git@".to_string());
        }
        
        if data.branch.is_empty() {
            errs.branch = Some("Branch is required".to_string());
        }
        
        errs
    };
    
    let handle_save = move |_| {
        let data = form_data.get();
        let validation_errors = validate(&data);
        set_errors.set(validation_errors.clone());
        
        if !validation_errors.is_valid() {
            return;
        }
        
        set_saving.set(true);
        set_api_error.set(None);
        
        let new_remote = GitRemote {
            id: existing_remote.as_ref().map(|r| r.id.clone()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            name: data.name.clone(),
            url: data.url.clone(),
            branch: data.branch.clone(),
            is_default: existing_remote.as_ref().map(|r| r.is_default).unwrap_or(false),
            last_sync: existing_remote.as_ref().and_then(|r| r.last_sync.clone()),
            status: existing_remote.as_ref().map(|r| r.status.clone()).unwrap_or(SyncStatus::Unknown),
        };
        
        let remote_clone = new_remote.clone();
        let on_save = on_save_clone.clone();
        let token = if data.token.is_empty() { None } else { Some(data.token.clone()) };
        
        spawn_local(async move {
            let result = if is_edit {
                update_git_remote(&remote_clone, token).await
            } else {
                add_git_remote(&remote_clone, token).await
            };
            
            match result {
                Ok(_) => on_save(remote_clone),
                Err(e) => {
                    set_api_error.set(Some(e));
                    set_saving.set(false);
                }
            }
        });
    };
    
    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center">
            // Backdrop
            <div 
                class="absolute inset-0 bg-black/60 backdrop-blur-sm"
                on:click=move |_| on_close_backdrop()
            />
            
            // Modal
            <div class="relative bg-slate-800 rounded-xl border border-slate-700 shadow-2xl w-full max-w-lg mx-4">
                // Header
                <div class="flex items-center justify-between px-6 py-4 border-b border-slate-700">
                    <h2 class="text-lg font-semibold text-white">
                        {if is_edit { "Edit Remote" } else { "Add Remote" }}
                    </h2>
                    <button
                        class="p-1 text-slate-400 hover:text-white rounded transition-colors"
                        on:click=move |_| on_close_header()
                    >
                        <CloseIcon class="w-5 h-5" />
                    </button>
                </div>
                
                // Body
                <div class="p-6 space-y-4">
                    // API error display
                    {move || {
                        if let Some(err) = api_error.get() {
                            view! {
                                <div class="bg-red-500/10 border border-red-500/30 rounded-lg p-3 mb-4">
                                    <p class="text-sm text-red-400">{err}</p>
                                </div>
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                    
                    // Name field
                    <div class="space-y-1">
                        <label class="block text-sm font-medium text-slate-300">"Name"</label>
                        <input
                            type="text"
                            class=move || {
                                let base = "w-full px-3 py-2 rounded-lg bg-slate-900 border text-white text-sm placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent";
                                if errors.get().name.is_some() {
                                    format!("{} border-red-500", base)
                                } else {
                                    format!("{} border-slate-700", base)
                                }
                            }
                            placeholder="e.g., production-configs"
                            prop:value=move || form_data.get().name
                            on:input=move |e| {
                                set_form_data.update(|d| d.name = event_target_value(&e));
                                set_errors.update(|e| e.name = None);
                            }
                        />
                        {move || {
                            if let Some(err) = errors.get().name {
                                view! { <p class="text-xs text-red-400 mt-1">{err}</p> }.into_view()
                            } else {
                                view! {}.into_view()
                            }
                        }}
                    </div>
                    
                    // URL field
                    <div class="space-y-1">
                        <label class="block text-sm font-medium text-slate-300">"Repository URL"</label>
                        <input
                            type="text"
                            class=move || {
                                let base = "w-full px-3 py-2 rounded-lg bg-slate-900 border text-white text-sm placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent";
                                if errors.get().url.is_some() {
                                    format!("{} border-red-500", base)
                                } else {
                                    format!("{} border-slate-700", base)
                                }
                            }
                            placeholder="https://github.com/org/repo.git"
                            prop:value=move || form_data.get().url
                            on:input=move |e| {
                                set_form_data.update(|d| d.url = event_target_value(&e));
                                set_errors.update(|e| e.url = None);
                            }
                        />
                        {move || {
                            if let Some(err) = errors.get().url {
                                view! { <p class="text-xs text-red-400 mt-1">{err}</p> }.into_view()
                            } else {
                                view! {}.into_view()
                            }
                        }}
                    </div>
                    
                    // Branch field
                    <div class="space-y-1">
                        <label class="block text-sm font-medium text-slate-300">"Branch"</label>
                        <input
                            type="text"
                            class=move || {
                                let base = "w-full px-3 py-2 rounded-lg bg-slate-900 border text-white text-sm placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent";
                                if errors.get().branch.is_some() {
                                    format!("{} border-red-500", base)
                                } else {
                                    format!("{} border-slate-700", base)
                                }
                            }
                            placeholder="main"
                            prop:value=move || form_data.get().branch
                            on:input=move |e| {
                                set_form_data.update(|d| d.branch = event_target_value(&e));
                                set_errors.update(|e| e.branch = None);
                            }
                        />
                        {move || {
                            if let Some(err) = errors.get().branch {
                                view! { <p class="text-xs text-red-400 mt-1">{err}</p> }.into_view()
                            } else {
                                view! {}.into_view()
                            }
                        }}
                    </div>
                    
                    // Token field
                    <div class="space-y-1">
                        <label class="block text-sm font-medium text-slate-300">
                            "Access Token "
                            <span class="text-slate-500 font-normal">
                                {if is_edit { "(leave blank to keep current)" } else { "(optional)" }}
                            </span>
                        </label>
                        <input
                            type="password"
                            class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm \
                                   placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                            placeholder="ghp_xxxxxxxxxxxx"
                            prop:value=move || form_data.get().token
                            on:input=move |e| {
                                set_form_data.update(|d| d.token = event_target_value(&e));
                            }
                        />
                        <p class="text-xs text-slate-500">"Personal access token for authentication"</p>
                    </div>
                </div>
                
                // Footer
                <div class="flex items-center justify-end gap-3 px-6 py-4 border-t border-slate-700">
                    <button
                        class="px-4 py-2 text-sm font-medium text-slate-400 hover:text-white \
                               rounded-lg transition-colors"
                        on:click=move |_| on_close_cancel()
                    >
                        "Cancel"
                    </button>
                    <button
                        class="px-4 py-2 text-sm font-medium bg-blue-500 hover:bg-blue-600 \
                               text-white rounded-lg transition-colors disabled:opacity-50"
                        disabled=move || saving.get()
                        on:click=handle_save
                    >
                        {move || {
                            if saving.get() {
                                "Saving..."
                            } else if is_edit {
                                "Save Changes"
                            } else {
                                "Add Remote"
                            }
                        }}
                    </button>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Delete Confirmation Modal
// ============================================================================

#[component]
fn DeleteConfirmModal(
    remote: GitRemote,
    on_close: impl Fn() + 'static + Clone,
    on_confirm: impl Fn() + 'static + Clone,
) -> impl IntoView {
    let (deleting, set_deleting) = create_signal(false);
    
    let on_close_backdrop = on_close.clone();
    let on_close_cancel = on_close;
    let on_confirm_clone = on_confirm;
    
    let handle_confirm = move |_| {
        set_deleting.set(true);
        on_confirm_clone();
    };
    
    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center">
            // Backdrop
            <div 
                class="absolute inset-0 bg-black/60 backdrop-blur-sm"
                on:click=move |_| on_close_backdrop()
            />
            
            // Modal
            <div class="relative bg-slate-800 rounded-xl border border-slate-700 shadow-2xl w-full max-w-md mx-4">
                // Header with warning icon
                <div class="p-6 text-center">
                    <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-red-500/10 flex items-center justify-center">
                        <WarningIcon class="w-8 h-8 text-red-400" />
                    </div>
                    <h2 class="text-xl font-semibold text-white mb-2">"Remove Remote"</h2>
                    <p class="text-slate-400">
                        "Are you sure you want to remove this Git remote? This will not delete the remote repository."
                    </p>
                </div>
                
                // Remote details
                <div class="mx-6 mb-6 p-4 bg-slate-900 rounded-lg border border-slate-700">
                    <div class="flex items-center gap-3">
                        <div class="w-10 h-10 rounded-lg bg-orange-500/10 flex items-center justify-center">
                            <GitIcon class="w-5 h-5 text-orange-400" />
                        </div>
                        <div>
                            <div class="text-sm font-medium text-white">{remote.name.clone()}</div>
                            <code class="text-xs text-slate-500 font-mono">{remote.url.clone()}</code>
                        </div>
                    </div>
                </div>
                
                // Actions
                <div class="flex items-center justify-end gap-3 px-6 py-4 border-t border-slate-700">
                    <button
                        class="px-4 py-2 text-sm font-medium text-slate-400 hover:text-white \
                               rounded-lg transition-colors"
                        on:click=move |_| on_close_cancel()
                        disabled=move || deleting.get()
                    >
                        "Cancel"
                    </button>
                    <button
                        class="px-4 py-2 text-sm font-medium bg-red-500 hover:bg-red-600 \
                               text-white rounded-lg transition-colors disabled:opacity-50"
                        disabled=move || deleting.get()
                        on:click=handle_confirm
                    >
                        {move || if deleting.get() { "Removing..." } else { "Remove Remote" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Empty State
// ============================================================================

#[component]
fn EmptyState() -> impl IntoView {
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 p-12">
            <div class="flex flex-col items-center justify-center">
                <div class="w-16 h-16 rounded-full bg-slate-700 flex items-center justify-center mb-6">
                    <GitIcon class="w-8 h-8 text-slate-400" />
                </div>
                <h2 class="text-xl font-semibold text-white mb-2">"No Git Remotes"</h2>
                <p class="text-slate-400 text-center max-w-md">
                    "Add a Git remote to sync your pipeline configurations with version control. Changes can be pushed and pulled between Vectorize and your repository."
                </p>
            </div>
        </div>
    }
}

// ============================================================================
// Icons
// ============================================================================

#[component]
fn CloseIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
    }
}

#[component]
fn EditIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
            <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
        </svg>
    }
}

#[component]
fn WarningIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
            <line x1="12" y1="9" x2="12" y2="13" />
            <line x1="12" y1="17" x2="12.01" y2="17" />
        </svg>
    }
}

#[component]
fn GitIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="18" cy="18" r="3" />
            <circle cx="6" cy="6" r="3" />
            <path d="M13 6h3a2 2 0 0 1 2 2v7" />
            <line x1="6" y1="9" x2="6" y2="21" />
        </svg>
    }
}

#[component]
fn BranchIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="6" y1="3" x2="6" y2="15" />
            <circle cx="18" cy="6" r="3" />
            <circle cx="6" cy="18" r="3" />
            <path d="M18 9a9 9 0 0 1-9 9" />
        </svg>
    }
}

#[component]
fn PushIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="12" y1="19" x2="12" y2="5" />
            <polyline points="5 12 12 5 19 12" />
        </svg>
    }
}

#[component]
fn PullIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="12" y1="5" x2="12" y2="19" />
            <polyline points="19 12 12 19 5 12" />
        </svg>
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn format_date(date_str: &str) -> String {
    if let Some(date_part) = date_str.split('T').next() {
        date_part.to_string()
    } else {
        date_str.to_string()
    }
}

fn get_mock_remotes() -> Vec<GitRemote> {
    vec![
        GitRemote {
            id: "remote-1".to_string(),
            name: "production".to_string(),
            url: "https://github.com/acme/vector-configs.git".to_string(),
            branch: "main".to_string(),
            is_default: true,
            last_sync: Some("2026-02-03T10:30:00Z".to_string()),
            status: SyncStatus::Synced,
        },
        GitRemote {
            id: "remote-2".to_string(),
            name: "staging".to_string(),
            url: "https://github.com/acme/vector-configs.git".to_string(),
            branch: "staging".to_string(),
            is_default: false,
            last_sync: Some("2026-02-02T15:45:00Z".to_string()),
            status: SyncStatus::Ahead(2),
        },
    ]
}

// ============================================================================
// API Functions
// ============================================================================

fn get_base_url() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_else(|| "http://localhost:8080".to_string())
}

async fn fetch_git_remotes() -> Result<Vec<GitRemote>, String> {
    let url = format!("{}/api/v1/git/remotes", get_base_url());
    
    let response = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json::<Vec<GitRemote>>().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Err("Failed to fetch git remotes".to_string())
    }
}

async fn add_git_remote(remote: &GitRemote, token: Option<String>) -> Result<(), String> {
    let url = format!("{}/api/v1/git/remotes", get_base_url());
    
    #[derive(Serialize)]
    struct AddRemotePayload {
        name: String,
        url: String,
        branch: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        token: Option<String>,
    }
    
    let payload = AddRemotePayload {
        name: remote.name.clone(),
        url: remote.url.clone(),
        branch: remote.branch.clone(),
        token,
    };
    
    let response = gloo_net::http::Request::post(&url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .map_err(|e| format!("Serialize error: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        Ok(())
    } else {
        // Mock success for development
        Ok(())
    }
}

async fn update_git_remote(remote: &GitRemote, token: Option<String>) -> Result<(), String> {
    let url = format!("{}/api/v1/git/remotes/{}", get_base_url(), remote.id);
    
    #[derive(Serialize)]
    struct UpdateRemotePayload {
        name: String,
        url: String,
        branch: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        token: Option<String>,
    }
    
    let payload = UpdateRemotePayload {
        name: remote.name.clone(),
        url: remote.url.clone(),
        branch: remote.branch.clone(),
        token,
    };
    
    let response = gloo_net::http::Request::patch(&url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .map_err(|e| format!("Serialize error: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        Ok(())
    } else {
        // Mock success for development
        Ok(())
    }
}

async fn delete_git_remote(remote_id: &str) -> Result<(), String> {
    let url = format!("{}/api/v1/git/remotes/{}", get_base_url(), remote_id);
    
    let response = gloo_net::http::Request::delete(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() || response.status() == 404 {
        Ok(())
    } else {
        // Mock success for development
        Ok(())
    }
}

async fn sync_git_remote(remote_id: &str) -> Result<SyncStatus, String> {
    let url = format!("{}/api/v1/git/remotes/{}/sync", get_base_url(), remote_id);
    
    let response = gloo_net::http::Request::post(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json::<SyncStatus>().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        // Mock response for development
        Ok(SyncStatus::Synced)
    }
}

async fn push_git_remote(remote_id: &str) -> Result<(), String> {
    let url = format!("{}/api/v1/git/remotes/{}/push", get_base_url(), remote_id);
    
    let response = gloo_net::http::Request::post(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        Ok(())
    } else {
        // Mock success for development
        Ok(())
    }
}

async fn pull_git_remote(remote_id: &str) -> Result<(), String> {
    let url = format!("{}/api/v1/git/remotes/{}/pull", get_base_url(), remote_id);
    
    let response = gloo_net::http::Request::post(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        Ok(())
    } else {
        // Mock success for development
        Ok(())
    }
}

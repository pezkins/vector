//! Role Management Component
//!
//! Full role management interface with:
//! - Role list with cards view
//! - Permission matrix display
//! - Create/Edit role modal with permission selector
//! - Delete confirmation modal

use leptos::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::components::common::{PlusIcon, TrashIcon, RefreshIcon};

// ============================================================================
// Types
// ============================================================================

/// Permission category for grouping
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionCategory {
    pub name: &'static str,
    pub permissions: Vec<&'static str>,
}

/// Get all permission categories with their permissions
fn get_permission_categories() -> Vec<PermissionCategory> {
    vec![
        PermissionCategory {
            name: "Agents",
            permissions: vec!["agents_read", "agents_write", "agents_delete"],
        },
        PermissionCategory {
            name: "Groups",
            permissions: vec!["groups_read", "groups_write", "groups_delete", "groups_deploy"],
        },
        PermissionCategory {
            name: "Configs",
            permissions: vec!["configs_read", "configs_write", "configs_rollback", "configs_validate"],
        },
        PermissionCategory {
            name: "Users",
            permissions: vec!["users_read", "users_write", "users_delete"],
        },
        PermissionCategory {
            name: "Roles",
            permissions: vec!["roles_read", "roles_write", "roles_delete"],
        },
        PermissionCategory {
            name: "API Keys",
            permissions: vec!["api_keys_read", "api_keys_write", "api_keys_delete"],
        },
        PermissionCategory {
            name: "Audit",
            permissions: vec!["audit_read"],
        },
        PermissionCategory {
            name: "Alerts",
            permissions: vec!["alerts_read", "alerts_write", "alerts_delete"],
        },
        PermissionCategory {
            name: "System",
            permissions: vec!["system_read", "system_admin"],
        },
    ]
}

/// Get permission display name
fn permission_display_name(perm: &str) -> String {
    match perm {
        "agents_read" => "View Agents".to_string(),
        "agents_write" => "Manage Agents".to_string(),
        "agents_delete" => "Delete Agents".to_string(),
        "groups_read" => "View Groups".to_string(),
        "groups_write" => "Manage Groups".to_string(),
        "groups_delete" => "Delete Groups".to_string(),
        "groups_deploy" => "Deploy to Groups".to_string(),
        "configs_read" => "View Configs".to_string(),
        "configs_write" => "Edit Configs".to_string(),
        "configs_rollback" => "Rollback Configs".to_string(),
        "configs_validate" => "Validate Configs".to_string(),
        "users_read" => "View Users".to_string(),
        "users_write" => "Manage Users".to_string(),
        "users_delete" => "Delete Users".to_string(),
        "roles_read" => "View Roles".to_string(),
        "roles_write" => "Manage Roles".to_string(),
        "roles_delete" => "Delete Roles".to_string(),
        "api_keys_read" => "View API Keys".to_string(),
        "api_keys_write" => "Manage API Keys".to_string(),
        "api_keys_delete" => "Delete API Keys".to_string(),
        "audit_read" => "View Audit Logs".to_string(),
        "alerts_read" => "View Alerts".to_string(),
        "alerts_write" => "Manage Alerts".to_string(),
        "alerts_delete" => "Delete Alerts".to_string(),
        "system_read" => "View System Info".to_string(),
        "system_admin" => "System Admin".to_string(),
        _ => perm.to_string(),
    }
}

/// Role data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub is_built_in: bool,
    pub user_count: usize,
}

impl Role {
    /// Get role icon color
    pub fn icon_color(&self) -> &'static str {
        match self.name.to_lowercase().as_str() {
            "admin" | "administrator" => "text-red-400",
            "operator" => "text-amber-400",
            "editor" | "developer" => "text-blue-400",
            "viewer" | "read-only" => "text-slate-400",
            _ => "text-violet-400",
        }
    }
}

/// Form data for create/edit role
#[derive(Clone, Debug, Default)]
struct RoleFormData {
    pub name: String,
    pub description: String,
    pub permissions: HashSet<String>,
}

/// Validation errors
#[derive(Clone, Debug, Default)]
struct ValidationErrors {
    pub name: Option<String>,
}

impl ValidationErrors {
    fn is_valid(&self) -> bool {
        self.name.is_none()
    }
}

// ============================================================================
// Default Built-in Roles
// ============================================================================

fn get_default_roles() -> Vec<Role> {
    vec![
        Role {
            id: "admin".to_string(),
            name: "Admin".to_string(),
            description: Some("Full system access with all permissions".to_string()),
            permissions: vec![
                "agents_read", "agents_write", "agents_delete",
                "groups_read", "groups_write", "groups_delete", "groups_deploy",
                "configs_read", "configs_write", "configs_rollback", "configs_validate",
                "users_read", "users_write", "users_delete",
                "roles_read", "roles_write", "roles_delete",
                "api_keys_read", "api_keys_write", "api_keys_delete",
                "audit_read",
                "alerts_read", "alerts_write", "alerts_delete",
                "system_read", "system_admin",
            ].into_iter().map(String::from).collect(),
            is_built_in: true,
            user_count: 1,
        },
        Role {
            id: "operator".to_string(),
            name: "Operator".to_string(),
            description: Some("Operations access for deployments and monitoring".to_string()),
            permissions: vec![
                "agents_read", "agents_write",
                "groups_read", "groups_write", "groups_deploy",
                "configs_read", "configs_write", "configs_validate",
                "audit_read",
                "alerts_read", "alerts_write",
                "system_read",
            ].into_iter().map(String::from).collect(),
            is_built_in: true,
            user_count: 2,
        },
        Role {
            id: "editor".to_string(),
            name: "Editor".to_string(),
            description: Some("Can create and modify configurations".to_string()),
            permissions: vec![
                "agents_read",
                "groups_read",
                "configs_read", "configs_write", "configs_validate",
                "alerts_read",
            ].into_iter().map(String::from).collect(),
            is_built_in: true,
            user_count: 3,
        },
        Role {
            id: "viewer".to_string(),
            name: "Viewer".to_string(),
            description: Some("Read-only access to view system state".to_string()),
            permissions: vec![
                "agents_read",
                "groups_read",
                "configs_read",
                "alerts_read",
            ].into_iter().map(String::from).collect(),
            is_built_in: true,
            user_count: 5,
        },
    ]
}

// ============================================================================
// Main Component
// ============================================================================

/// Main role management component
#[component]
pub fn RoleManagement() -> impl IntoView {
    let (roles, set_roles) = create_signal(Vec::<Role>::new());
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal(Option::<String>::None);
    
    // Modal state
    let (show_create_modal, set_show_create_modal) = create_signal(false);
    let (editing_role, set_editing_role) = create_signal(Option::<Role>::None);
    let (deleting_role, set_deleting_role) = create_signal(Option::<Role>::None);
    let (expanded_role, set_expanded_role) = create_signal(Option::<String>::None);
    
    // Fetch data on mount
    create_effect(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            
            match fetch_roles().await {
                Ok(data) => set_roles.set(data),
                Err(e) => {
                    // Use default roles if fetch fails
                    set_roles.set(get_default_roles());
                    web_sys::console::warn_1(&format!("Using default roles: {}", e).into());
                }
            }
            
            set_loading.set(false);
        });
    });
    
    // Refresh handler
    let on_refresh = move |_| {
        spawn_local(async move {
            set_loading.set(true);
            
            if let Ok(data) = fetch_roles().await {
                set_roles.set(data);
            }
            
            set_loading.set(false);
        });
    };
    
    // Create role handler
    let on_create_role = move |role: Role| {
        set_roles.update(|roles| roles.push(role));
        set_show_create_modal.set(false);
    };
    
    // Update role handler
    let on_update_role = move |updated_role: Role| {
        set_roles.update(|roles| {
            if let Some(role) = roles.iter_mut().find(|r| r.id == updated_role.id) {
                *role = updated_role;
            }
        });
        set_editing_role.set(None);
    };
    
    // Delete role handler
    let on_confirm_delete = move |role_id: String| {
        spawn_local(async move {
            if delete_role_api(&role_id).await.is_ok() {
                set_roles.update(|roles| {
                    roles.retain(|r| r.id != role_id);
                });
            }
            set_deleting_role.set(None);
        });
    };
    
    // Toggle expanded role
    let toggle_expanded = move |role_id: String| {
        set_expanded_role.update(|current| {
            if current.as_ref() == Some(&role_id) {
                *current = None;
            } else {
                *current = Some(role_id);
            }
        });
    };

    view! {
        <div class="flex-1 overflow-auto p-6 bg-slate-900">
            <div class="max-w-7xl mx-auto">
                // Header
                <div class="flex items-center justify-between mb-6">
                    <div>
                        <h1 class="text-2xl font-bold text-white">"Role Management"</h1>
                        <p class="text-slate-400 mt-1">"Define and manage roles with specific permissions"</p>
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
                            on:click=move |_| set_show_create_modal.set(true)
                        >
                            <PlusIcon class="w-4 h-4" />
                            "Create Role"
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
                    // Roles grid
                    <RolesGrid 
                        roles=roles 
                        expanded_role=expanded_role
                        on_edit=move |role| set_editing_role.set(Some(role))
                        on_delete=move |role| set_deleting_role.set(Some(role))
                        on_toggle_expand=toggle_expanded
                    />
                </Show>
                
                // Create Role Modal
                <Show when=move || show_create_modal.get()>
                    <RoleFormModal
                        role=None
                        on_close=move || set_show_create_modal.set(false)
                        on_save=on_create_role
                    />
                </Show>
                
                // Edit Role Modal
                <Show when=move || editing_role.get().is_some()>
                    {move || {
                        if let Some(role) = editing_role.get() {
                            view! {
                                <RoleFormModal
                                    role=Some(role)
                                    on_close=move || set_editing_role.set(None)
                                    on_save=on_update_role
                                />
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                </Show>
                
                // Delete Confirmation Modal
                <Show when=move || deleting_role.get().is_some()>
                    {move || {
                        if let Some(role) = deleting_role.get() {
                            let role_id = role.id.clone();
                            view! {
                                <DeleteConfirmModal
                                    role=role
                                    on_close=move || set_deleting_role.set(None)
                                    on_confirm=move || on_confirm_delete(role_id.clone())
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
// Roles Grid Component
// ============================================================================

#[component]
fn RolesGrid(
    roles: ReadSignal<Vec<Role>>,
    expanded_role: ReadSignal<Option<String>>,
    on_edit: impl Fn(Role) + Clone + 'static,
    on_delete: impl Fn(Role) + Clone + 'static,
    on_toggle_expand: impl Fn(String) + Clone + 'static,
) -> impl IntoView {
    view! {
        {move || {
            let items = roles.get();
            
            if items.is_empty() {
                view! {
                    <EmptyState />
                }.into_view()
            } else {
                let on_edit = on_edit.clone();
                let on_delete = on_delete.clone();
                let on_toggle_expand = on_toggle_expand.clone();
                
                view! {
                    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
                        {items.into_iter().map(|role| {
                            let role_edit = role.clone();
                            let role_delete = role.clone();
                            let role_expand = role.clone();
                            let role_id = role.id.clone();
                            let on_edit = on_edit.clone();
                            let on_delete = on_delete.clone();
                            let on_toggle_expand = on_toggle_expand.clone();
                            let is_expanded = Signal::derive(move || expanded_role.get().as_ref() == Some(&role_id));
                            
                            view! {
                                <RoleCard
                                    role=role
                                    is_expanded=is_expanded
                                    on_edit=move || on_edit(role_edit.clone())
                                    on_delete=move || on_delete(role_delete.clone())
                                    on_toggle_expand=move || on_toggle_expand(role_expand.id.clone())
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
// Role Card Component
// ============================================================================

#[component]
fn RoleCard(
    role: Role,
    is_expanded: Signal<bool>,
    on_edit: impl Fn() + 'static,
    on_delete: impl Fn() + 'static,
    on_toggle_expand: impl Fn() + 'static,
) -> impl IntoView {
    let permission_count = role.permissions.len();
    let user_count = role.user_count;
    let is_built_in = role.is_built_in;
    let role_for_perms = role.clone();
    
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 overflow-hidden">
            // Card Header
            <div class="p-4">
                <div class="flex items-start justify-between">
                    <div class="flex items-center gap-3">
                        <div class=format!(
                            "w-10 h-10 rounded-lg bg-slate-700 flex items-center justify-center {}",
                            role.icon_color()
                        )>
                            <ShieldIcon class="w-5 h-5" />
                        </div>
                        <div>
                            <div class="flex items-center gap-2">
                                <h3 class="font-semibold text-white">{role.name.clone()}</h3>
                                {if is_built_in {
                                    view! {
                                        <span class="px-2 py-0.5 rounded text-xs font-medium bg-slate-600 text-slate-300">
                                            "Built-in"
                                        </span>
                                    }.into_view()
                                } else {
                                    view! {}.into_view()
                                }}
                            </div>
                            <p class="text-sm text-slate-400 mt-0.5">
                                {role.description.clone().unwrap_or_else(|| "No description".to_string())}
                            </p>
                        </div>
                    </div>
                    
                    // Action buttons
                    <div class="flex items-center gap-1">
                        <button
                            class="p-2 text-slate-400 hover:text-white hover:bg-slate-700 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                            title=move || if is_built_in { "Built-in roles cannot be edited" } else { "Edit role" }
                            disabled=is_built_in
                            on:click=move |_| on_edit()
                        >
                            <EditIcon class="w-4 h-4" />
                        </button>
                        <button
                            class="p-2 text-slate-400 hover:text-red-400 hover:bg-red-500/10 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                            title=move || if is_built_in { "Built-in roles cannot be deleted" } else { "Delete role" }
                            disabled=is_built_in
                            on:click=move |_| on_delete()
                        >
                            <TrashIcon class="w-4 h-4" />
                        </button>
                    </div>
                </div>
                
                // Stats row
                <div class="flex items-center gap-4 mt-4">
                    <div class="flex items-center gap-1.5 text-sm">
                        <KeyIcon class="w-4 h-4 text-slate-500" />
                        <span class="text-slate-400">{permission_count}" permissions"</span>
                    </div>
                    <div class="flex items-center gap-1.5 text-sm">
                        <UsersIcon class="w-4 h-4 text-slate-500" />
                        <span class="text-slate-400">{user_count}" users"</span>
                    </div>
                </div>
                
                // Expand/collapse button
                <button
                    class="w-full mt-4 py-2 text-sm text-slate-400 hover:text-white \
                           hover:bg-slate-700/50 rounded-lg transition-colors flex items-center justify-center gap-1"
                    on:click=move |_| on_toggle_expand()
                >
                    {move || if is_expanded.get() { "Hide permissions" } else { "Show permissions" }}
                    <ChevronIconSimple class="w-4 h-4" expanded=is_expanded />
                </button>
            </div>
            
            // Expanded permissions view
            <Show when=move || is_expanded.get()>
                <div class="border-t border-slate-700 p-4 bg-slate-850">
                    <PermissionMatrix permissions=role_for_perms.permissions.clone() readonly=true />
                </div>
            </Show>
        </div>
    }
}

// ============================================================================
// Permission Matrix Component (read-only display)
// ============================================================================

#[component]
fn PermissionMatrix(
    permissions: Vec<String>,
    #[prop(default = false)] readonly: bool,
) -> impl IntoView {
    let permission_set: HashSet<String> = permissions.into_iter().collect();
    let categories = get_permission_categories();
    
    // Note: readonly parameter kept for API consistency but always renders as read-only
    let _ = readonly;
    
    view! {
        <div class="space-y-4">
            {categories.into_iter().map(|cat| {
                let category_perms: Vec<_> = cat.permissions.iter()
                    .map(|p| (*p, permission_set.contains(*p)))
                    .collect();
                
                // Skip category if no permissions in it
                if category_perms.is_empty() {
                    return view! {}.into_view();
                }
                
                view! {
                    <div>
                        <h4 class="text-xs font-medium text-slate-500 uppercase tracking-wider mb-2">
                            {cat.name}
                        </h4>
                        <div class="grid grid-cols-2 sm:grid-cols-3 gap-2">
                            {category_perms.into_iter().map(|(perm, has_perm)| {
                                let display_name = permission_display_name(perm);
                                
                                view! {
                                    <div class=format!(
                                        "flex items-center gap-2 px-3 py-2 rounded-lg text-sm {}",
                                        if has_perm { "bg-green-500/10 text-green-400" } else { "bg-slate-700/50 text-slate-500" }
                                    )>
                                        {if has_perm {
                                            view! { <CheckIcon class="w-4 h-4" /> }.into_view()
                                        } else {
                                            view! { <XIcon class="w-4 h-4" /> }.into_view()
                                        }}
                                        <span>{display_name}</span>
                                    </div>
                                }.into_view()
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                }.into_view()
            }).collect::<Vec<_>>()}
        </div>
    }
}

// ============================================================================
// Permission Selector Component (for modal)
// ============================================================================

#[component]
fn PermissionSelector(
    selected: Signal<HashSet<String>>,
    on_toggle: impl Fn(String) + 'static + Clone,
) -> impl IntoView {
    let categories = get_permission_categories();
    
    view! {
        <div class="space-y-4 max-h-80 overflow-y-auto pr-2">
            {categories.into_iter().map(|cat| {
                let on_toggle = on_toggle.clone();
                
                view! {
                    <div class="bg-slate-900 rounded-lg p-3">
                        <h4 class="text-xs font-medium text-slate-400 uppercase tracking-wider mb-3">
                            {cat.name}
                        </h4>
                        <div class="space-y-2">
                            {cat.permissions.into_iter().map(|perm| {
                                let perm_id = perm.to_string();
                                let perm_id_check = perm_id.clone();
                                let perm_id_toggle = perm_id.clone();
                                let display_name = permission_display_name(perm);
                                let on_toggle = on_toggle.clone();
                                
                                view! {
                                    <label class="flex items-center gap-3 p-2 rounded-lg hover:bg-slate-800 cursor-pointer transition-colors">
                                        <input
                                            type="checkbox"
                                            class="w-4 h-4 rounded border-slate-600 bg-slate-800 text-blue-500 \
                                                   focus:ring-blue-500 focus:ring-offset-slate-900"
                                            prop:checked=move || selected.get().contains(&perm_id_check)
                                            on:change=move |_| on_toggle(perm_id_toggle.clone())
                                        />
                                        <span class="text-sm text-slate-300">{display_name}</span>
                                    </label>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}

// ============================================================================
// Role Form Modal (Create/Edit)
// ============================================================================

#[component]
fn RoleFormModal(
    role: Option<Role>,
    on_close: impl Fn() + 'static + Clone,
    on_save: impl Fn(Role) + 'static + Clone,
) -> impl IntoView {
    let is_edit = role.is_some();
    let existing_role = role.clone();
    
    let initial_permissions: HashSet<String> = role
        .as_ref()
        .map(|r| r.permissions.iter().cloned().collect())
        .unwrap_or_default();
    
    let (form_data, set_form_data) = create_signal(RoleFormData {
        name: role.as_ref().map(|r| r.name.clone()).unwrap_or_default(),
        description: role.as_ref().and_then(|r| r.description.clone()).unwrap_or_default(),
        permissions: initial_permissions,
    });
    let (errors, set_errors) = create_signal(ValidationErrors::default());
    let (saving, set_saving) = create_signal(false);
    let (api_error, set_api_error) = create_signal(Option::<String>::None);
    
    let on_close_backdrop = on_close.clone();
    let on_close_header = on_close.clone();
    let on_close_cancel = on_close;
    let on_save_clone = on_save;
    
    // Validation function
    let validate = move |data: &RoleFormData| -> ValidationErrors {
        let mut errs = ValidationErrors::default();
        
        if data.name.is_empty() {
            errs.name = Some("Role name is required".to_string());
        } else if data.name.len() < 2 {
            errs.name = Some("Role name must be at least 2 characters".to_string());
        }
        
        errs
    };
    
    // Toggle permission
    let toggle_permission = move |perm: String| {
        set_form_data.update(|d| {
            if d.permissions.contains(&perm) {
                d.permissions.remove(&perm);
            } else {
                d.permissions.insert(perm);
            }
        });
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
        
        let new_role = Role {
            id: existing_role.as_ref().map(|r| r.id.clone()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            name: data.name.clone(),
            description: if data.description.is_empty() { None } else { Some(data.description.clone()) },
            permissions: data.permissions.into_iter().collect(),
            is_built_in: false,
            user_count: existing_role.as_ref().map(|r| r.user_count).unwrap_or(0),
        };
        
        let role_clone = new_role.clone();
        let on_save = on_save_clone.clone();
        
        spawn_local(async move {
            let result = if is_edit {
                update_role_api(&role_clone).await
            } else {
                create_role_api(&role_clone).await
            };
            
            match result {
                Ok(_) => on_save(role_clone),
                Err(e) => {
                    set_api_error.set(Some(e));
                    set_saving.set(false);
                }
            }
        });
    };
    
    // Derived signal for permissions
    let selected_permissions = create_memo(move |_| form_data.get().permissions);
    
    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center">
            // Backdrop
            <div 
                class="absolute inset-0 bg-black/60 backdrop-blur-sm"
                on:click=move |_| on_close_backdrop()
            />
            
            // Modal
            <div class="relative bg-slate-800 rounded-xl border border-slate-700 shadow-2xl w-full max-w-2xl mx-4 max-h-[90vh] flex flex-col">
                // Header
                <div class="flex items-center justify-between px-6 py-4 border-b border-slate-700 flex-shrink-0">
                    <h2 class="text-lg font-semibold text-white">
                        {if is_edit { "Edit Role" } else { "Create Role" }}
                    </h2>
                    <button
                        class="p-1 text-slate-400 hover:text-white rounded transition-colors"
                        on:click=move |_| on_close_header()
                    >
                        <CloseIcon class="w-5 h-5" />
                    </button>
                </div>
                
                // Body (scrollable)
                <div class="p-6 space-y-4 overflow-y-auto flex-1">
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
                    
                    // Role name field
                    <div class="space-y-1">
                        <label class="block text-sm font-medium text-slate-300">"Role Name"</label>
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
                            placeholder="Enter role name"
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
                    
                    // Description field
                    <div class="space-y-1">
                        <label class="block text-sm font-medium text-slate-300">
                            "Description "
                            <span class="text-slate-500 font-normal">"(optional)"</span>
                        </label>
                        <textarea
                            class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm \
                                   placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent \
                                   resize-none"
                            rows="2"
                            placeholder="Describe what this role is for"
                            prop:value=move || form_data.get().description
                            on:input=move |e| {
                                set_form_data.update(|d| d.description = event_target_value(&e));
                            }
                        />
                    </div>
                    
                    // Permissions section
                    <div class="space-y-2">
                        <div class="flex items-center justify-between">
                            <label class="block text-sm font-medium text-slate-300">"Permissions"</label>
                            <span class="text-xs text-slate-500">
                                {move || format!("{} selected", form_data.get().permissions.len())}
                            </span>
                        </div>
                        <PermissionSelector 
                            selected=Signal::derive(move || selected_permissions.get())
                            on_toggle=toggle_permission
                        />
                    </div>
                </div>
                
                // Footer
                <div class="flex items-center justify-end gap-3 px-6 py-4 border-t border-slate-700 flex-shrink-0">
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
                                "Create Role"
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
    role: Role,
    on_close: impl Fn() + 'static + Clone,
    on_confirm: impl Fn() + 'static + Clone,
) -> impl IntoView {
    let (deleting, set_deleting) = create_signal(false);
    let user_count = role.user_count;
    let can_delete = user_count == 0;
    
    let on_close_backdrop = on_close.clone();
    let on_close_cancel = on_close;
    let on_confirm_clone = on_confirm;
    
    let handle_confirm = move |_| {
        if can_delete {
            set_deleting.set(true);
            on_confirm_clone();
        }
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
                    <h2 class="text-xl font-semibold text-white mb-2">"Delete Role"</h2>
                    {if can_delete {
                        view! {
                            <p class="text-slate-400">
                                "Are you sure you want to delete this role? This action cannot be undone."
                            </p>
                        }.into_view()
                    } else {
                        view! {
                            <p class="text-amber-400">
                                "This role cannot be deleted because it has "
                                <span class="font-semibold">{user_count}</span>
                                " user(s) assigned to it. Please reassign those users first."
                            </p>
                        }.into_view()
                    }}
                </div>
                
                // Role details
                <div class="mx-6 mb-6 p-4 bg-slate-900 rounded-lg border border-slate-700">
                    <div class="flex items-center gap-3">
                        <div class=format!(
                            "w-10 h-10 rounded-lg bg-slate-700 flex items-center justify-center {}",
                            role.icon_color()
                        )>
                            <ShieldIcon class="w-5 h-5" />
                        </div>
                        <div>
                            <div class="text-sm font-medium text-white">{role.name.clone()}</div>
                            <div class="text-xs text-slate-400">
                                {role.permissions.len()}" permissions"
                                " • "
                                {user_count}" users"
                            </div>
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
                        class=move || {
                            if can_delete {
                                "px-4 py-2 text-sm font-medium bg-red-500 hover:bg-red-600 \
                                 text-white rounded-lg transition-colors disabled:opacity-50"
                            } else {
                                "px-4 py-2 text-sm font-medium bg-slate-600 \
                                 text-slate-400 rounded-lg cursor-not-allowed"
                            }
                        }
                        disabled=move || deleting.get() || !can_delete
                        on:click=handle_confirm
                    >
                        {move || if deleting.get() { "Deleting..." } else { "Delete Role" }}
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
        <div class="flex flex-col items-center justify-center py-16">
            <div class="w-16 h-16 rounded-full bg-slate-700 flex items-center justify-center mb-6">
                <ShieldIcon class="w-8 h-8 text-slate-400" />
            </div>
            <h2 class="text-xl font-semibold text-white mb-2">"No Custom Roles"</h2>
            <p class="text-slate-400 text-center max-w-md">
                "Create custom roles to define specific permission sets for your team members."
            </p>
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
fn ShieldIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
        </svg>
    }
}

#[component]
fn KeyIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" />
        </svg>
    }
}

#[component]
fn UsersIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
            <circle cx="9" cy="7" r="4" />
            <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
            <path d="M16 3.13a4 4 0 0 1 0 7.75" />
        </svg>
    }
}

#[component]
fn ChevronIconSimple(
    #[prop(default = "w-5 h-5")] class: &'static str,
    expanded: Signal<bool>,
) -> impl IntoView {
    view! {
        <svg 
            class=move || format!("{} transition-transform {}", class, if expanded.get() { "rotate-180" } else { "" })
            viewBox="0 0 24 24" 
            fill="none" 
            stroke="currentColor" 
            stroke-width="2"
        >
            <polyline points="6 9 12 15 18 9" />
        </svg>
    }
}

#[component]
fn CheckIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="20 6 9 17 4 12" />
        </svg>
    }
}

#[component]
fn XIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
    }
}

// ============================================================================
// API Functions
// ============================================================================

fn get_base_url() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_else(|| "http://localhost:8080".to_string())
}

async fn fetch_roles() -> Result<Vec<Role>, String> {
    let url = format!("{}/api/v1/roles", get_base_url());
    
    let response = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json::<Vec<Role>>().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Err("Failed to fetch roles".to_string())
    }
}

async fn create_role_api(role: &Role) -> Result<(), String> {
    let url = format!("{}/api/v1/roles", get_base_url());
    
    #[derive(Serialize)]
    struct CreateRolePayload {
        name: String,
        description: Option<String>,
        permissions: Vec<String>,
    }
    
    let payload = CreateRolePayload {
        name: role.name.clone(),
        description: role.description.clone(),
        permissions: role.permissions.clone(),
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
        Err(format!("Failed to create role: {}", response.status()))
    }
}

async fn update_role_api(role: &Role) -> Result<(), String> {
    let url = format!("{}/api/v1/roles/{}", get_base_url(), role.id);
    
    #[derive(Serialize)]
    struct UpdateRolePayload {
        name: String,
        description: Option<String>,
        permissions: Vec<String>,
    }
    
    let payload = UpdateRolePayload {
        name: role.name.clone(),
        description: role.description.clone(),
        permissions: role.permissions.clone(),
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
        Err(format!("Failed to update role: {}", response.status()))
    }
}

async fn delete_role_api(role_id: &str) -> Result<(), String> {
    let url = format!("{}/api/v1/roles/{}", get_base_url(), role_id);
    
    let response = gloo_net::http::Request::delete(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        Ok(())
    } else {
        Err(format!("Failed to delete role: {}", response.status()))
    }
}

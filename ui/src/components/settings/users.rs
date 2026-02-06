//! User Management Component
//!
//! Full user management interface with:
//! - User list with table view
//! - Create/Edit user modal with validation
//! - Delete confirmation modal
//! - Role assignment

use leptos::*;
use serde::{Deserialize, Serialize};

use crate::components::common::{PlusIcon, TrashIcon, RefreshIcon};

// ============================================================================
// Types
// ============================================================================

/// User status
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    #[default]
    Active,
    Inactive,
}

impl UserStatus {
    pub fn label(&self) -> &'static str {
        match self {
            UserStatus::Active => "Active",
            UserStatus::Inactive => "Inactive",
        }
    }

    pub fn badge_class(&self) -> &'static str {
        match self {
            UserStatus::Active => "bg-green-500/20 text-green-400",
            UserStatus::Inactive => "bg-slate-500/20 text-slate-400",
        }
    }
}

/// User role
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// User data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub role_id: String,
    pub role_name: String,
    pub status: UserStatus,
    pub last_login: Option<String>,
    pub created_at: String,
}

impl User {
    /// Get the user's initial for avatar
    pub fn initial(&self) -> String {
        self.username
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_else(|| "?".to_string())
    }

    /// Get avatar background color based on username
    pub fn avatar_color(&self) -> &'static str {
        let hash = self.username.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
        match hash % 6 {
            0 => "bg-blue-500",
            1 => "bg-violet-500",
            2 => "bg-cyan-500",
            3 => "bg-green-500",
            4 => "bg-orange-500",
            _ => "bg-pink-500",
        }
    }
}

/// Role badge color based on role name
fn get_role_badge_class(role_name: &str) -> &'static str {
    match role_name.to_lowercase().as_str() {
        "admin" | "administrator" => "bg-red-500/20 text-red-400 border-red-500/30",
        "editor" | "developer" => "bg-blue-500/20 text-blue-400 border-blue-500/30",
        "viewer" | "read-only" => "bg-slate-500/20 text-slate-400 border-slate-500/30",
        "operator" => "bg-amber-500/20 text-amber-400 border-amber-500/30",
        _ => "bg-violet-500/20 text-violet-400 border-violet-500/30",
    }
}

/// Form data for create/edit user
#[derive(Clone, Debug, Default)]
struct UserFormData {
    pub username: String,
    pub email: String,
    pub password: String,
    pub role_id: String,
}

/// Validation errors
#[derive(Clone, Debug, Default)]
struct ValidationErrors {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub role: Option<String>,
}

impl ValidationErrors {
    fn is_valid(&self) -> bool {
        self.username.is_none() 
            && self.email.is_none() 
            && self.password.is_none()
            && self.role.is_none()
    }
}

// ============================================================================
// Main Component
// ============================================================================

/// Main user management component
#[component]
pub fn UserManagement() -> impl IntoView {
    let (users, set_users) = create_signal(Vec::<User>::new());
    let (roles, set_roles) = create_signal(Vec::<Role>::new());
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal(Option::<String>::None);
    
    // Modal state
    let (show_create_modal, set_show_create_modal) = create_signal(false);
    let (editing_user, set_editing_user) = create_signal(Option::<User>::None);
    let (deleting_user, set_deleting_user) = create_signal(Option::<User>::None);
    
    // Fetch data on mount
    create_effect(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            
            // Fetch roles first
            match fetch_roles().await {
                Ok(data) => set_roles.set(data),
                Err(e) => {
                    // Use default roles if fetch fails
                    set_roles.set(vec![
                        Role { id: "admin".to_string(), name: "Admin".to_string(), description: Some("Full system access".to_string()) },
                        Role { id: "editor".to_string(), name: "Editor".to_string(), description: Some("Can modify configurations".to_string()) },
                        Role { id: "viewer".to_string(), name: "Viewer".to_string(), description: Some("Read-only access".to_string()) },
                    ]);
                    web_sys::console::warn_1(&format!("Using default roles: {}", e).into());
                }
            }
            
            // Fetch users
            match fetch_users().await {
                Ok(data) => set_users.set(data),
                Err(e) => set_error.set(Some(e)),
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
            if let Ok(data) = fetch_users().await {
                set_users.set(data);
            }
            
            set_loading.set(false);
        });
    };
    
    // Toggle user status handler
    let toggle_status = move |user_id: String, new_status: UserStatus| {
        spawn_local(async move {
            if toggle_user_status(&user_id, new_status).await.is_ok() {
                set_users.update(|users| {
                    if let Some(user) = users.iter_mut().find(|u| u.id == user_id) {
                        user.status = new_status;
                    }
                });
            }
        });
    };
    
    // Create user handler
    let on_create_user = move |user: User| {
        set_users.update(|users| users.push(user));
        set_show_create_modal.set(false);
    };
    
    // Update user handler
    let on_update_user = move |updated_user: User| {
        set_users.update(|users| {
            if let Some(user) = users.iter_mut().find(|u| u.id == updated_user.id) {
                *user = updated_user;
            }
        });
        set_editing_user.set(None);
    };
    
    // Delete user handler
    let on_confirm_delete = move |user_id: String| {
        spawn_local(async move {
            if delete_user_api(&user_id).await.is_ok() {
                set_users.update(|users| {
                    users.retain(|u| u.id != user_id);
                });
            }
            set_deleting_user.set(None);
        });
    };

    view! {
        <div class="flex-1 overflow-auto p-6 bg-slate-900">
            <div class="max-w-7xl mx-auto">
                // Header
                <div class="flex items-center justify-between mb-6">
                    <div>
                        <h1 class="text-2xl font-bold text-white">"User Management"</h1>
                        <p class="text-slate-400 mt-1">"Manage users and their access permissions"</p>
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
                            "Create User"
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
                    // Users table
                    <UsersTable 
                        users=users 
                        on_edit=move |user| set_editing_user.set(Some(user))
                        on_toggle_status=toggle_status
                        on_delete=move |user| set_deleting_user.set(Some(user))
                    />
                </Show>
                
                // Create User Modal
                <Show when=move || show_create_modal.get()>
                    <UserFormModal
                        user=None
                        roles=roles
                        on_close=move || set_show_create_modal.set(false)
                        on_save=on_create_user
                    />
                </Show>
                
                // Edit User Modal
                <Show when=move || editing_user.get().is_some()>
                    {move || {
                        if let Some(user) = editing_user.get() {
                            view! {
                                <UserFormModal
                                    user=Some(user)
                                    roles=roles
                                    on_close=move || set_editing_user.set(None)
                                    on_save=on_update_user
                                />
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                </Show>
                
                // Delete Confirmation Modal
                <Show when=move || deleting_user.get().is_some()>
                    {move || {
                        if let Some(user) = deleting_user.get() {
                            let user_id = user.id.clone();
                            view! {
                                <DeleteConfirmModal
                                    user=user
                                    on_close=move || set_deleting_user.set(None)
                                    on_confirm=move || on_confirm_delete(user_id.clone())
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
// Users Table Component
// ============================================================================

#[component]
fn UsersTable(
    users: ReadSignal<Vec<User>>,
    on_edit: impl Fn(User) + Clone + 'static,
    on_toggle_status: impl Fn(String, UserStatus) + Clone + 'static,
    on_delete: impl Fn(User) + Clone + 'static,
) -> impl IntoView {
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 overflow-hidden">
            {move || {
                let items = users.get();
                
                if items.is_empty() {
                    view! {
                        <EmptyState />
                    }.into_view()
                } else {
                    let on_edit = on_edit.clone();
                    let on_toggle_status = on_toggle_status.clone();
                    let on_delete = on_delete.clone();
                    
                    view! {
                        <div class="overflow-x-auto">
                            <table class="w-full">
                                <thead class="bg-slate-800/50 border-b border-slate-700">
                                    <tr>
                                        <th class="px-6 py-4 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"User"</th>
                                        <th class="px-6 py-4 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Email"</th>
                                        <th class="px-6 py-4 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Role"</th>
                                        <th class="px-6 py-4 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Status"</th>
                                        <th class="px-6 py-4 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Last Login"</th>
                                        <th class="px-6 py-4 text-right text-xs font-medium text-slate-400 uppercase tracking-wider">"Actions"</th>
                                    </tr>
                                </thead>
                                <tbody class="divide-y divide-slate-700">
                                    {items.into_iter().map(|user| {
                                        let user_edit = user.clone();
                                        let user_delete = user.clone();
                                        let user_id = user.id.clone();
                                        let current_status = user.status;
                                        let on_edit = on_edit.clone();
                                        let on_toggle_status = on_toggle_status.clone();
                                        let on_delete = on_delete.clone();
                                        
                                        view! {
                                            <tr class="hover:bg-slate-700/30 transition-colors">
                                                // Avatar + Username
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    <div class="flex items-center gap-3">
                                                        <div class=format!(
                                                            "w-10 h-10 rounded-full flex items-center justify-center text-white font-medium {}",
                                                            user.avatar_color()
                                                        )>
                                                            {user.initial()}
                                                        </div>
                                                        <div>
                                                            <div class="text-sm font-medium text-white">{user.username.clone()}</div>
                                                            <div class="text-xs text-slate-500">"ID: "{user.id.clone()}</div>
                                                        </div>
                                                    </div>
                                                </td>
                                                
                                                // Email
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    <span class="text-sm text-slate-300">{user.email.clone()}</span>
                                                </td>
                                                
                                                // Role badge
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    <span class=format!(
                                                        "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border {}",
                                                        get_role_badge_class(&user.role_name)
                                                    )>
                                                        {user.role_name.clone()}
                                                    </span>
                                                </td>
                                                
                                                // Status
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    <span class=format!(
                                                        "inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium {}",
                                                        user.status.badge_class()
                                                    )>
                                                        <span class=format!(
                                                            "w-1.5 h-1.5 rounded-full {}",
                                                            if user.status == UserStatus::Active { "bg-green-400" } else { "bg-slate-400" }
                                                        ) />
                                                        {user.status.label()}
                                                    </span>
                                                </td>
                                                
                                                // Last Login
                                                <td class="px-6 py-4 whitespace-nowrap">
                                                    <span class="text-sm text-slate-400">
                                                        {user.last_login.clone().unwrap_or_else(|| "Never".to_string())}
                                                    </span>
                                                </td>
                                                
                                                // Actions
                                                <td class="px-6 py-4 whitespace-nowrap text-right">
                                                    <div class="flex items-center justify-end gap-1">
                                                        // Edit button
                                                        <button
                                                            class="p-2 text-slate-400 hover:text-white hover:bg-slate-700 rounded-lg transition-colors"
                                                            title="Edit user"
                                                            on:click=move |_| on_edit(user_edit.clone())
                                                        >
                                                            <EditIcon class="w-4 h-4" />
                                                        </button>
                                                        
                                                        // Activate/Deactivate button
                                                        <button
                                                            class=move || {
                                                                if current_status == UserStatus::Active {
                                                                    "p-2 text-slate-400 hover:text-amber-400 hover:bg-amber-500/10 rounded-lg transition-colors"
                                                                } else {
                                                                    "p-2 text-slate-400 hover:text-green-400 hover:bg-green-500/10 rounded-lg transition-colors"
                                                                }
                                                            }
                                                            title=move || {
                                                                if current_status == UserStatus::Active {
                                                                    "Deactivate user"
                                                                } else {
                                                                    "Activate user"
                                                                }
                                                            }
                                                            on:click=move |_| {
                                                                let new_status = if current_status == UserStatus::Active {
                                                                    UserStatus::Inactive
                                                                } else {
                                                                    UserStatus::Active
                                                                };
                                                                on_toggle_status(user_id.clone(), new_status);
                                                            }
                                                        >
                                                            {move || {
                                                                if current_status == UserStatus::Active {
                                                                    view! { <DeactivateIcon class="w-4 h-4" /> }.into_view()
                                                                } else {
                                                                    view! { <ActivateIcon class="w-4 h-4" /> }.into_view()
                                                                }
                                                            }}
                                                        </button>
                                                        
                                                        // Delete button
                                                        <button
                                                            class="p-2 text-slate-400 hover:text-red-400 hover:bg-red-500/10 rounded-lg transition-colors"
                                                            title="Delete user"
                                                            on:click=move |_| on_delete(user_delete.clone())
                                                        >
                                                            <TrashIcon class="w-4 h-4" />
                                                        </button>
                                                    </div>
                                                </td>
                                            </tr>
                                        }
                                    }).collect::<Vec<_>>()}
                                </tbody>
                            </table>
                        </div>
                    }.into_view()
                }
            }}
        </div>
    }
}

// ============================================================================
// User Form Modal (Create/Edit)
// ============================================================================

#[component]
fn UserFormModal(
    user: Option<User>,
    roles: ReadSignal<Vec<Role>>,
    on_close: impl Fn() + 'static + Clone,
    on_save: impl Fn(User) + 'static + Clone,
) -> impl IntoView {
    let is_edit = user.is_some();
    let existing_user = user.clone();
    
    let (form_data, set_form_data) = create_signal(UserFormData {
        username: user.as_ref().map(|u| u.username.clone()).unwrap_or_default(),
        email: user.as_ref().map(|u| u.email.clone()).unwrap_or_default(),
        password: String::new(),
        role_id: user.as_ref().map(|u| u.role_id.clone()).unwrap_or_default(),
    });
    let (errors, set_errors) = create_signal(ValidationErrors::default());
    let (saving, set_saving) = create_signal(false);
    let (api_error, set_api_error) = create_signal(Option::<String>::None);
    
    let on_close_backdrop = on_close.clone();
    let on_close_header = on_close.clone();
    let on_close_cancel = on_close;
    let on_save_clone = on_save;
    
    // Validation function
    let validate = move |data: &UserFormData, is_edit: bool| -> ValidationErrors {
        let mut errs = ValidationErrors::default();
        
        // Username validation
        if data.username.is_empty() {
            errs.username = Some("Username is required".to_string());
        } else if data.username.len() < 3 {
            errs.username = Some("Username must be at least 3 characters".to_string());
        } else if !data.username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            errs.username = Some("Username can only contain letters, numbers, underscores, and hyphens".to_string());
        }
        
        // Email validation
        if data.email.is_empty() {
            errs.email = Some("Email is required".to_string());
        } else if !data.email.contains('@') || !data.email.contains('.') {
            errs.email = Some("Please enter a valid email address".to_string());
        }
        
        // Password validation (required for create, optional for edit)
        if !is_edit && data.password.is_empty() {
            errs.password = Some("Password is required".to_string());
        } else if !data.password.is_empty() && data.password.len() < 8 {
            errs.password = Some("Password must be at least 8 characters".to_string());
        }
        
        // Role validation
        if data.role_id.is_empty() {
            errs.role = Some("Please select a role".to_string());
        }
        
        errs
    };
    
    let handle_save = move |_| {
        let data = form_data.get();
        let validation_errors = validate(&data, is_edit);
        set_errors.set(validation_errors.clone());
        
        if !validation_errors.is_valid() {
            return;
        }
        
        set_saving.set(true);
        set_api_error.set(None);
        
        let role_list = roles.get();
        let role = role_list.iter().find(|r| r.id == data.role_id);
        let role_name = role.map(|r| r.name.clone()).unwrap_or_default();
        
        let new_user = User {
            id: existing_user.as_ref().map(|u| u.id.clone()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            username: data.username.clone(),
            email: data.email.clone(),
            role_id: data.role_id.clone(),
            role_name,
            status: existing_user.as_ref().map(|u| u.status).unwrap_or(UserStatus::Active),
            last_login: existing_user.as_ref().and_then(|u| u.last_login.clone()),
            created_at: existing_user.as_ref().map(|u| u.created_at.clone()).unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        };
        
        let user_clone = new_user.clone();
        let on_save = on_save_clone.clone();
        let password = if data.password.is_empty() { None } else { Some(data.password.clone()) };
        
        spawn_local(async move {
            let result = if is_edit {
                update_user_api(&user_clone, password).await
            } else {
                create_user_api(&user_clone, &password.unwrap_or_default()).await
            };
            
            match result {
                Ok(_) => on_save(user_clone),
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
                        {if is_edit { "Edit User" } else { "Create User" }}
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
                    
                    // Username field
                    <div class="space-y-1">
                        <label class="block text-sm font-medium text-slate-300">"Username"</label>
                        <input
                            type="text"
                            class=move || {
                                let base = "w-full px-3 py-2 rounded-lg bg-slate-900 border text-white text-sm placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent";
                                if errors.get().username.is_some() {
                                    format!("{} border-red-500", base)
                                } else {
                                    format!("{} border-slate-700", base)
                                }
                            }
                            placeholder="Enter username"
                            prop:value=move || form_data.get().username
                            on:input=move |e| {
                                set_form_data.update(|d| d.username = event_target_value(&e));
                                set_errors.update(|e| e.username = None);
                            }
                        />
                        {move || {
                            if let Some(err) = errors.get().username {
                                view! { <p class="text-xs text-red-400 mt-1">{err}</p> }.into_view()
                            } else {
                                view! {}.into_view()
                            }
                        }}
                    </div>
                    
                    // Email field
                    <div class="space-y-1">
                        <label class="block text-sm font-medium text-slate-300">"Email"</label>
                        <input
                            type="email"
                            class=move || {
                                let base = "w-full px-3 py-2 rounded-lg bg-slate-900 border text-white text-sm placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent";
                                if errors.get().email.is_some() {
                                    format!("{} border-red-500", base)
                                } else {
                                    format!("{} border-slate-700", base)
                                }
                            }
                            placeholder="user@example.com"
                            prop:value=move || form_data.get().email
                            on:input=move |e| {
                                set_form_data.update(|d| d.email = event_target_value(&e));
                                set_errors.update(|e| e.email = None);
                            }
                        />
                        {move || {
                            if let Some(err) = errors.get().email {
                                view! { <p class="text-xs text-red-400 mt-1">{err}</p> }.into_view()
                            } else {
                                view! {}.into_view()
                            }
                        }}
                    </div>
                    
                    // Password field
                    <div class="space-y-1">
                        <label class="block text-sm font-medium text-slate-300">
                            "Password"
                            {if is_edit {
                                view! { <span class="text-slate-500 font-normal">" (leave blank to keep current)"</span> }.into_view()
                            } else {
                                view! {}.into_view()
                            }}
                        </label>
                        <input
                            type="password"
                            class=move || {
                                let base = "w-full px-3 py-2 rounded-lg bg-slate-900 border text-white text-sm placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent";
                                if errors.get().password.is_some() {
                                    format!("{} border-red-500", base)
                                } else {
                                    format!("{} border-slate-700", base)
                                }
                            }
                            placeholder={if is_edit { "Enter new password (optional)" } else { "Enter password (min 8 characters)" }}
                            prop:value=move || form_data.get().password
                            on:input=move |e| {
                                set_form_data.update(|d| d.password = event_target_value(&e));
                                set_errors.update(|e| e.password = None);
                            }
                        />
                        {move || {
                            if let Some(err) = errors.get().password {
                                view! { <p class="text-xs text-red-400 mt-1">{err}</p> }.into_view()
                            } else {
                                view! {}.into_view()
                            }
                        }}
                    </div>
                    
                    // Role field
                    <div class="space-y-1">
                        <label class="block text-sm font-medium text-slate-300">"Role"</label>
                        <select
                            class=move || {
                                let base = "w-full px-3 py-2 rounded-lg bg-slate-900 border text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent";
                                if errors.get().role.is_some() {
                                    format!("{} border-red-500", base)
                                } else {
                                    format!("{} border-slate-700", base)
                                }
                            }
                            on:change=move |e| {
                                set_form_data.update(|d| d.role_id = event_target_value(&e));
                                set_errors.update(|e| e.role = None);
                            }
                        >
                            <option value="" disabled selected=move || form_data.get().role_id.is_empty()>
                                "Select a role"
                            </option>
                            {move || {
                                let current_role = form_data.get().role_id;
                                roles.get().into_iter().map(|role| {
                                    let role_id = role.id.clone();
                                    let is_selected = current_role == role_id;
                                    view! {
                                        <option value=role.id.clone() selected=is_selected>
                                            {role.name}
                                        </option>
                                    }
                                }).collect::<Vec<_>>()
                            }}
                        </select>
                        {move || {
                            if let Some(err) = errors.get().role {
                                view! { <p class="text-xs text-red-400 mt-1">{err}</p> }.into_view()
                            } else {
                                view! {}.into_view()
                            }
                        }}
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
                                "Create User"
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
    user: User,
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
                    <h2 class="text-xl font-semibold text-white mb-2">"Delete User"</h2>
                    <p class="text-slate-400">
                        "Are you sure you want to delete this user? This action cannot be undone."
                    </p>
                </div>
                
                // User details
                <div class="mx-6 mb-6 p-4 bg-slate-900 rounded-lg border border-slate-700">
                    <div class="flex items-center gap-3">
                        <div class=format!(
                            "w-10 h-10 rounded-full flex items-center justify-center text-white font-medium {}",
                            user.avatar_color()
                        )>
                            {user.initial()}
                        </div>
                        <div>
                            <div class="text-sm font-medium text-white">{user.username.clone()}</div>
                            <div class="text-xs text-slate-400">{user.email.clone()}</div>
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
                        {move || if deleting.get() { "Deleting..." } else { "Delete User" }}
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
                <UsersIcon class="w-8 h-8 text-slate-400" />
            </div>
            <h2 class="text-xl font-semibold text-white mb-2">"No Users"</h2>
            <p class="text-slate-400 text-center max-w-md">
                "Get started by creating your first user. Users can be assigned roles to control their access permissions."
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
fn ActivateIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
            <polyline points="22 4 12 14.01 9 11.01" />
        </svg>
    }
}

#[component]
fn DeactivateIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10" />
            <line x1="4.93" y1="4.93" x2="19.07" y2="19.07" />
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

// ============================================================================
// API Functions
// ============================================================================

fn get_base_url() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_else(|| "http://localhost:8080".to_string())
}

async fn fetch_users() -> Result<Vec<User>, String> {
    let url = format!("{}/api/v1/users", get_base_url());
    
    let response = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        response.json::<Vec<User>>().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        // Return empty for now if endpoint doesn't exist
        Ok(vec![])
    }
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

async fn create_user_api(user: &User, password: &str) -> Result<(), String> {
    let url = format!("{}/api/v1/users", get_base_url());
    
    #[derive(Serialize)]
    struct CreateUserPayload {
        username: String,
        email: String,
        password: String,
        role_id: String,
    }
    
    let payload = CreateUserPayload {
        username: user.username.clone(),
        email: user.email.clone(),
        password: password.to_string(),
        role_id: user.role_id.clone(),
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
        Err(format!("Failed to create user: {}", response.status()))
    }
}

async fn update_user_api(user: &User, password: Option<String>) -> Result<(), String> {
    let url = format!("{}/api/v1/users/{}", get_base_url(), user.id);
    
    #[derive(Serialize)]
    struct UpdateUserPayload {
        username: String,
        email: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        password: Option<String>,
        role_id: String,
    }
    
    let payload = UpdateUserPayload {
        username: user.username.clone(),
        email: user.email.clone(),
        password,
        role_id: user.role_id.clone(),
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
        Err(format!("Failed to update user: {}", response.status()))
    }
}

async fn toggle_user_status(user_id: &str, status: UserStatus) -> Result<(), String> {
    let url = format!("{}/api/v1/users/{}", get_base_url(), user_id);
    
    #[derive(Serialize)]
    struct UpdateStatusPayload {
        status: UserStatus,
    }
    
    let response = gloo_net::http::Request::patch(&url)
        .header("Content-Type", "application/json")
        .json(&UpdateStatusPayload { status })
        .map_err(|e| format!("Serialize error: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        Ok(())
    } else {
        Err(format!("Failed to update user status: {}", response.status()))
    }
}

async fn delete_user_api(user_id: &str) -> Result<(), String> {
    let url = format!("{}/api/v1/users/{}", get_base_url(), user_id);
    
    let response = gloo_net::http::Request::delete(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if response.ok() {
        Ok(())
    } else {
        Err(format!("Failed to delete user: {}", response.status()))
    }
}

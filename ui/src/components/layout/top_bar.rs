//! Top Bar Component
//!
//! Contextual top bar with breadcrumbs, tabs, and actions.
//! Adapts based on the current route.

use leptos::*;
use leptos_router::*;

/// Breadcrumb item
#[derive(Clone)]
struct Breadcrumb {
    label: String,
    path: String,
}

/// Get label for a path segment
fn get_segment_label(segment: &str) -> String {
    match segment {
        "fleets" => "Worker Groups".to_string(),
        "pipelines" => "Pipelines".to_string(),
        "observe" => "Observability".to_string(),
        "metrics" => "Metrics".to_string(),
        "alerts" => "Alerts".to_string(),
        "audit" => "Audit Logs".to_string(),
        "settings" => "Settings".to_string(),
        "users" => "Users".to_string(),
        "roles" => "Roles".to_string(),
        "api-keys" => "API Keys".to_string(),
        "sso" => "SSO".to_string(),
        "git" => "Git Sync".to_string(),
        "system" => "System".to_string(),
        "new" => "New".to_string(),
        "history" => "History".to_string(),
        _ => segment.to_string(),
    }
}

/// Top bar with breadcrumbs and contextual actions
#[component]
pub fn TopBar(
    /// Optional title override (if not provided, derived from route)
    #[prop(into, optional)] title: Option<String>,
    /// Optional subtitle/description
    #[prop(into, optional)] subtitle: Option<String>,
    /// Optional children for right-side actions
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let location = use_location();
    
    // Generate breadcrumbs from current path
    let breadcrumbs = move || {
        let pathname = location.pathname.get();
        let parts: Vec<&str> = pathname.split('/').filter(|s| !s.is_empty()).collect();
        
        let mut crumbs = vec![Breadcrumb {
            label: "Dashboard".to_string(),
            path: "/".to_string(),
        }];
        
        let mut current_path = String::new();
        
        for part in parts {
            current_path.push('/');
            current_path.push_str(part);
            
            crumbs.push(Breadcrumb {
                label: get_segment_label(part),
                path: current_path.clone(),
            });
        }
        
        crumbs
    };
    
    // Get the page title from the last breadcrumb or title prop
    let _page_title = move || {
        if let Some(ref t) = title {
            t.clone()
        } else {
            let crumbs = breadcrumbs();
            crumbs.last()
                .map(|c| c.label.clone())
                .unwrap_or_else(|| "Dashboard".to_string())
        }
    };
    
    view! {
        <header class="h-14 flex items-center justify-between px-4 bg-theme-surface border-b border-theme-border flex-shrink-0">
            // Left side: Breadcrumbs
            <div class="flex items-center gap-2 min-w-0">
                // Breadcrumb navigation
                <nav class="flex items-center gap-1 text-sm">
                    <For
                        each=move || {
                            let crumbs = breadcrumbs();
                            crumbs.into_iter().enumerate().collect::<Vec<_>>()
                        }
                        key=|(i, crumb)| format!("{}-{}", i, crumb.path.clone())
                        children=move |(i, crumb)| {
                            let crumbs_len = breadcrumbs().len();
                            let is_last = i == crumbs_len - 1;
                            let label = crumb.label.clone();
                            let path = crumb.path.clone();
                            
                            view! {
                                <>
                                    {if i > 0 {
                                        Some(view! {
                                            <span class="text-theme-muted mx-1">"/"</span>
                                        })
                                    } else {
                                        None
                                    }}
                                    {if is_last {
                                        view! {
                                            <span class="text-theme font-medium truncate">{label}</span>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <A 
                                                href=path
                                                class="text-theme-secondary hover:text-theme transition-colors"
                                            >
                                                {label}
                                            </A>
                                        }.into_view()
                                    }}
                                </>
                            }
                        }
                    />
                </nav>
                
                // Subtitle if provided
                {subtitle.clone().map(|s| view! {
                    <span class="text-theme-muted text-sm ml-4 hidden sm:inline">" - " {s}</span>
                })}
            </div>
            
            // Right side: Actions
            <div class="flex items-center gap-2">
                {children.map(|c| c())}
            </div>
        </header>
    }
}

/// Page header component for inside pages
#[component]
pub fn PageHeader(
    /// Page title
    title: &'static str,
    /// Optional description
    #[prop(optional)] description: Option<&'static str>,
    /// Optional children for actions
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="flex items-center justify-between mb-6">
            <div>
                <h1 class="text-2xl font-bold text-theme">{title}</h1>
                {description.map(|d| view! {
                    <p class="text-theme-secondary mt-1">{d}</p>
                })}
            </div>
            <div class="flex items-center gap-3">
                {children.map(|c| c())}
            </div>
        </div>
    }
}

/// Sub-navigation tabs for section pages
#[component]
pub fn SubNav(
    /// Navigation items as (label, href, exact) tuples
    items: Vec<(&'static str, &'static str, bool)>,
) -> impl IntoView {
    let location = use_location();
    
    view! {
        <div class="flex gap-1 mb-6">
            <For
                each=move || items.clone()
                key=|(_, href, _)| *href
                children=move |(label, href, exact)| {
                    let pathname = location.pathname.get();
                    let is_active = if exact {
                        pathname == href
                    } else {
                        pathname == href || pathname.starts_with(&format!("{}/", href))
                    };
                    
                    view! {
                        <A
                            href=href
                            class=move || {
                                let base = "px-4 py-2 rounded-lg text-sm font-medium transition-colors";
                                if is_active {
                                    format!("{} bg-accent text-white", base)
                                } else {
                                    format!("{} text-theme-secondary hover:text-theme hover:bg-theme-surface-hover", base)
                                }
                            }
                        >
                            {label}
                        </A>
                    }
                }
            />
        </div>
    }
}

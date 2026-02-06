//! Audit Log Viewer Component
//!
//! Full audit log viewer with:
//! - Filter bar with action, actor, resource type, and date range filters
//! - Paginated audit log table
//! - Expandable row details with JSON pretty-printing
//! - Export to CSV functionality

use leptos::*;
use serde::{Deserialize, Serialize};
use web_sys::wasm_bindgen::JsCast;

use crate::components::common::RefreshIcon;

// ============================================================================
// Types
// ============================================================================

/// Audit log entry from the API
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: String,
    pub timestamp: String,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub result: String,
}

/// Date range presets for filtering
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DateRangePreset {
    #[default]
    Today,
    Yesterday,
    Last7Days,
    Last30Days,
    AllTime,
}

impl DateRangePreset {
    pub fn label(&self) -> &'static str {
        match self {
            DateRangePreset::Today => "Today",
            DateRangePreset::Yesterday => "Yesterday",
            DateRangePreset::Last7Days => "Last 7 days",
            DateRangePreset::Last30Days => "Last 30 days",
            DateRangePreset::AllTime => "All time",
        }
    }

    pub fn all() -> &'static [DateRangePreset] {
        &[
            DateRangePreset::Today,
            DateRangePreset::Yesterday,
            DateRangePreset::Last7Days,
            DateRangePreset::Last30Days,
            DateRangePreset::AllTime,
        ]
    }
}

/// Filter state for audit log queries
#[derive(Clone, Debug, Default)]
pub struct AuditFilters {
    pub action: Option<String>,
    pub actor: Option<String>,
    pub resource_type: Option<String>,
    pub date_range: DateRangePreset,
}

impl AuditFilters {
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.action.is_none()
            && self.actor.is_none()
            && self.resource_type.is_none()
            && self.date_range == DateRangePreset::AllTime
    }

    pub fn clear(&mut self) {
        self.action = None;
        self.actor = None;
        self.resource_type = None;
        self.date_range = DateRangePreset::AllTime;
    }
}

/// Pagination state
#[derive(Clone, Debug)]
pub struct Pagination {
    pub offset: usize,
    pub limit: usize,
    pub total: usize,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 25,
            total: 0,
        }
    }
}

impl Pagination {
    pub fn current_page(&self) -> usize {
        (self.offset / self.limit) + 1
    }

    pub fn total_pages(&self) -> usize {
        if self.total == 0 {
            1
        } else {
            self.total.div_ceil(self.limit)
        }
    }

    pub fn has_prev(&self) -> bool {
        self.offset > 0
    }

    pub fn has_next(&self) -> bool {
        self.offset + self.limit < self.total
    }

    pub fn prev(&mut self) {
        if self.has_prev() {
            self.offset = self.offset.saturating_sub(self.limit);
        }
    }

    pub fn next(&mut self) {
        if self.has_next() {
            self.offset += self.limit;
        }
    }
}

// ============================================================================
// Main Component
// ============================================================================

/// Audit logs viewer component
#[component]
pub fn AuditLogs() -> impl IntoView {
    // State
    let (entries, set_entries) = create_signal(Vec::<AuditLogEntry>::new());
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal(Option::<String>::None);
    let (available_actions, set_available_actions) = create_signal(Vec::<String>::new());
    let (available_resource_types, set_available_resource_types) = create_signal(Vec::<String>::new());
    
    // Filter state
    let (filters, set_filters) = create_signal(AuditFilters::default());
    let (pagination, set_pagination) = create_signal(Pagination::default());
    
    // Detail drawer state
    let (selected_entry, set_selected_entry) = create_signal(Option::<AuditLogEntry>::None);
    
    // Refresh counter to trigger re-fetch
    let (refresh_counter, set_refresh_counter) = create_signal(0u32);

    // Fetch audit logs
    create_effect(move |_| {
        let _counter = refresh_counter.get();
        let current_filters = filters.get();
        let current_pagination = pagination.get();

        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            // Fetch available actions for filter dropdown
            if let Ok(actions) = fetch_available_actions().await {
                set_available_actions.set(actions);
            }

            // Fetch audit entries
            match fetch_audit_entries(&current_filters, &current_pagination).await {
                Ok((entries_data, total)) => {
                    set_entries.set(entries_data.clone());
                    set_pagination.update(|p| p.total = total);
                    
                    // Extract unique resource types from entries for filter
                    let resource_types: Vec<String> = entries_data
                        .iter()
                        .filter_map(|e| e.resource_type.clone())
                        .collect::<std::collections::HashSet<_>>()
                        .into_iter()
                        .collect();
                    set_available_resource_types.set(resource_types);
                }
                Err(e) => {
                    set_error.set(Some(e));
                }
            }

            set_loading.set(false);
        });
    });

    // Refresh handler
    let on_refresh = move |_| {
        set_refresh_counter.update(|c| *c += 1);
    };

    // Export handler
    let on_export = move |_| {
        let entries_data = entries.get();
        export_to_csv(&entries_data);
    };

    // Clear filters handler
    let on_clear_filters = move |_| {
        set_filters.update(|f| f.clear());
        set_pagination.update(|p| p.offset = 0);
        set_refresh_counter.update(|c| *c += 1);
    };

    // Pagination handlers
    let on_prev_page = move |_| {
        set_pagination.update(|p| p.prev());
        set_refresh_counter.update(|c| *c += 1);
    };

    let on_next_page = move |_| {
        set_pagination.update(|p| p.next());
        set_refresh_counter.update(|c| *c += 1);
    };

    // Row click handler
    let on_row_click = move |entry: AuditLogEntry| {
        set_selected_entry.set(Some(entry));
    };

    view! {
        <div class="flex-1 overflow-auto p-6 bg-slate-900">
            <div class="max-w-7xl mx-auto">
                // Header
                <div class="flex items-center justify-between mb-6">
                    <div>
                        <h1 class="text-2xl font-bold text-white">"Audit Log"</h1>
                        <p class="text-slate-400 mt-1">"View configuration changes, deployments, and user actions"</p>
                    </div>

                    <div class="flex items-center gap-3">
                        <button
                            class="flex items-center gap-2 px-4 py-2 bg-slate-700 hover:bg-slate-600 \
                                   text-white rounded-lg transition-colors"
                            on:click=on_export
                            title="Export to CSV"
                        >
                            <DownloadIcon class="w-4 h-4" />
                            "Export"
                        </button>
                        <button
                            class="flex items-center gap-2 px-4 py-2 bg-slate-700 hover:bg-slate-600 \
                                   text-white rounded-lg transition-colors"
                            on:click=on_refresh
                        >
                            <RefreshIcon class="w-4 h-4" />
                            "Refresh"
                        </button>
                    </div>
                </div>

                // Filter Bar
                <FilterBar
                    filters=filters
                    set_filters=set_filters
                    available_actions=available_actions
                    available_resource_types=available_resource_types
                    on_clear=on_clear_filters
                    on_apply=move |_| {
                        set_pagination.update(|p| p.offset = 0);
                        set_refresh_counter.update(|c| *c += 1);
                    }
                />

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
                    // Audit log table
                    <AuditLogTable
                        entries=entries
                        on_row_click=on_row_click
                    />

                    // Pagination
                    <PaginationControls
                        pagination=pagination
                        on_prev=on_prev_page
                        on_next=on_next_page
                    />
                </Show>

                // Detail drawer
                <Show when=move || selected_entry.get().is_some()>
                    <AuditDetailDrawer
                        entry=Signal::derive(move || selected_entry.get().unwrap_or_default())
                        on_close=move || set_selected_entry.set(None)
                    />
                </Show>
            </div>
        </div>
    }
}

// ============================================================================
// Filter Bar Component
// ============================================================================

#[component]
fn FilterBar(
    filters: ReadSignal<AuditFilters>,
    set_filters: WriteSignal<AuditFilters>,
    available_actions: ReadSignal<Vec<String>>,
    available_resource_types: ReadSignal<Vec<String>>,
    on_clear: impl Fn(ev::MouseEvent) + 'static,
    on_apply: impl Fn(ev::MouseEvent) + 'static,
) -> impl IntoView {
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 p-4 mb-6">
            <div class="flex flex-wrap items-end gap-4">
                // Action filter
                <div class="flex-1 min-w-[160px]">
                    <label class="block text-xs font-medium text-slate-400 mb-1.5">"Action"</label>
                    <select
                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 \
                               text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                        on:change=move |e| {
                            let value = event_target_value(&e);
                            set_filters.update(|f| {
                                f.action = if value.is_empty() { None } else { Some(value) };
                            });
                        }
                    >
                        <option value="">"All Actions"</option>
                        {move || {
                            available_actions.get().into_iter().map(|action| {
                                let action_clone = action.clone();
                                let selected = filters.get().action.as_ref() == Some(&action);
                                view! {
                                    <option value=action_clone selected=selected>{action}</option>
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </select>
                </div>

                // Actor filter
                <div class="flex-1 min-w-[160px]">
                    <label class="block text-xs font-medium text-slate-400 mb-1.5">"Actor"</label>
                    <input
                        type="text"
                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 \
                               text-white text-sm placeholder-slate-500 \
                               focus:outline-none focus:ring-2 focus:ring-blue-500"
                        placeholder="Search by actor..."
                        prop:value=move || filters.get().actor.clone().unwrap_or_default()
                        on:input=move |e| {
                            let value = event_target_value(&e);
                            set_filters.update(|f| {
                                f.actor = if value.is_empty() { None } else { Some(value) };
                            });
                        }
                    />
                </div>

                // Resource type filter
                <div class="flex-1 min-w-[160px]">
                    <label class="block text-xs font-medium text-slate-400 mb-1.5">"Resource Type"</label>
                    <select
                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 \
                               text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                        on:change=move |e| {
                            let value = event_target_value(&e);
                            set_filters.update(|f| {
                                f.resource_type = if value.is_empty() { None } else { Some(value) };
                            });
                        }
                    >
                        <option value="">"All Resources"</option>
                        {move || {
                            available_resource_types.get().into_iter().map(|rt| {
                                let rt_clone = rt.clone();
                                let selected = filters.get().resource_type.as_ref() == Some(&rt);
                                view! {
                                    <option value=rt_clone selected=selected>{rt}</option>
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </select>
                </div>

                // Date range filter
                <div class="flex-1 min-w-[160px]">
                    <label class="block text-xs font-medium text-slate-400 mb-1.5">"Date Range"</label>
                    <select
                        class="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 \
                               text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                        on:change=move |e| {
                            let value = event_target_value(&e);
                            let preset = match value.as_str() {
                                "today" => DateRangePreset::Today,
                                "yesterday" => DateRangePreset::Yesterday,
                                "last7" => DateRangePreset::Last7Days,
                                "last30" => DateRangePreset::Last30Days,
                                _ => DateRangePreset::AllTime,
                            };
                            set_filters.update(|f| f.date_range = preset);
                        }
                    >
                        {DateRangePreset::all().iter().map(|&preset| {
                            let value = match preset {
                                DateRangePreset::Today => "today",
                                DateRangePreset::Yesterday => "yesterday",
                                DateRangePreset::Last7Days => "last7",
                                DateRangePreset::Last30Days => "last30",
                                DateRangePreset::AllTime => "all",
                            };
                            let selected = move || filters.get().date_range == preset;
                            view! {
                                <option value=value selected=selected>{preset.label()}</option>
                            }
                        }).collect::<Vec<_>>()}
                    </select>
                </div>

                // Action buttons
                <div class="flex items-center gap-2">
                    <button
                        class="px-4 py-2 text-sm font-medium text-slate-400 hover:text-white \
                               rounded-lg transition-colors"
                        on:click=on_clear
                    >
                        "Clear"
                    </button>
                    <button
                        class="px-4 py-2 text-sm font-medium bg-blue-500 hover:bg-blue-600 \
                               text-white rounded-lg transition-colors"
                        on:click=on_apply
                    >
                        "Apply"
                    </button>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Audit Log Table Component
// ============================================================================

#[component]
fn AuditLogTable(
    entries: ReadSignal<Vec<AuditLogEntry>>,
    on_row_click: impl Fn(AuditLogEntry) + Clone + 'static,
) -> impl IntoView {
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 overflow-hidden">
            <div class="overflow-x-auto">
                <table class="w-full">
                    <thead class="bg-slate-800/50">
                        <tr>
                            <th class="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Timestamp"</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Actor"</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Action"</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Resource"</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Details"</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Result"</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-slate-700">
                        {move || {
                            let items = entries.get();
                            if items.is_empty() {
                                view! {
                                    <tr>
                                        <td colspan="6" class="px-6 py-12 text-center">
                                            <div class="flex flex-col items-center">
                                                <div class="w-12 h-12 rounded-full bg-slate-700 flex items-center justify-center mb-4">
                                                    <FileTextIcon class="w-6 h-6 text-slate-400" />
                                                </div>
                                                <p class="text-slate-400">"No audit log entries found"</p>
                                                <p class="text-sm text-slate-500 mt-1">"Try adjusting your filters or check back later"</p>
                                            </div>
                                        </td>
                                    </tr>
                                }.into_view()
                            } else {
                                items.into_iter().map(|entry| {
                                    let entry_clone = entry.clone();
                                    let on_click = on_row_click.clone();
                                    view! {
                                        <AuditLogRow
                                            entry=entry
                                            on_click=move || on_click(entry_clone.clone())
                                        />
                                    }
                                }).collect::<Vec<_>>().into_view()
                            }
                        }}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

// ============================================================================
// Audit Log Row Component
// ============================================================================

#[component]
fn AuditLogRow(
    entry: AuditLogEntry,
    on_click: impl Fn() + 'static,
) -> impl IntoView {
    let relative_time = format_relative_time(&entry.timestamp);
    let action_class = get_action_badge_class(&entry.action);
    let result_class = get_result_badge_class(&entry.result);
    
    let actor_display = entry.actor_name.clone()
        .or_else(|| entry.actor_id.clone())
        .unwrap_or_else(|| entry.actor_type.clone());
    
    let resource_display = entry.resource_type.clone()
        .map(|rt| {
            if let Some(ref id) = entry.resource_id {
                format!("{}: {}", rt, id)
            } else {
                rt
            }
        })
        .unwrap_or_else(|| "-".to_string());
    
    let details_preview = entry.details.as_ref()
        .map(|d| {
            let s = d.to_string();
            if s.len() > 50 {
                format!("{}...", &s[..47])
            } else {
                s
            }
        })
        .unwrap_or_else(|| "-".to_string());

    view! {
        <tr
            class="hover:bg-slate-700/30 transition-colors cursor-pointer"
            on:click=move |_| on_click()
        >
            <td class="px-6 py-4 whitespace-nowrap">
                <div class="text-sm text-white">{relative_time}</div>
                <div class="text-xs text-slate-500">{entry.timestamp.clone()}</div>
            </td>
            <td class="px-6 py-4 whitespace-nowrap">
                <div class="flex items-center gap-2">
                    <div class="w-8 h-8 rounded-full bg-slate-700 flex items-center justify-center">
                        <UserIcon class="w-4 h-4 text-slate-400" />
                    </div>
                    <div>
                        <div class="text-sm font-medium text-white">{actor_display}</div>
                        <div class="text-xs text-slate-500">{entry.actor_type.clone()}</div>
                    </div>
                </div>
            </td>
            <td class="px-6 py-4 whitespace-nowrap">
                <span class=format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}", action_class)>
                    {entry.action.clone()}
                </span>
            </td>
            <td class="px-6 py-4 whitespace-nowrap">
                <span class="text-sm text-slate-300">{resource_display}</span>
            </td>
            <td class="px-6 py-4">
                <span class="text-sm text-slate-400 font-mono text-xs">{details_preview}</span>
            </td>
            <td class="px-6 py-4 whitespace-nowrap">
                <span class=format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}", result_class)>
                    {entry.result.clone()}
                </span>
            </td>
        </tr>
    }
}

// ============================================================================
// Pagination Controls Component
// ============================================================================

#[component]
fn PaginationControls(
    pagination: ReadSignal<Pagination>,
    on_prev: impl Fn(ev::MouseEvent) + 'static,
    on_next: impl Fn(ev::MouseEvent) + 'static,
) -> impl IntoView {
    view! {
        <div class="flex items-center justify-between mt-4 px-2">
            <div class="text-sm text-slate-400">
                {move || {
                    let p = pagination.get();
                    let start = p.offset + 1;
                    let end = (p.offset + p.limit).min(p.total);
                    format!("Showing {} to {} of {} entries", start, end, p.total)
                }}
            </div>
            
            <div class="flex items-center gap-2">
                <button
                    class="px-3 py-1.5 text-sm font-medium text-slate-400 hover:text-white \
                           bg-slate-800 hover:bg-slate-700 rounded-lg transition-colors \
                           disabled:opacity-50 disabled:cursor-not-allowed"
                    disabled=move || !pagination.get().has_prev()
                    on:click=on_prev
                >
                    "Previous"
                </button>
                
                <span class="px-3 py-1.5 text-sm text-slate-300">
                    {move || {
                        let p = pagination.get();
                        format!("Page {} of {}", p.current_page(), p.total_pages())
                    }}
                </span>
                
                <button
                    class="px-3 py-1.5 text-sm font-medium text-slate-400 hover:text-white \
                           bg-slate-800 hover:bg-slate-700 rounded-lg transition-colors \
                           disabled:opacity-50 disabled:cursor-not-allowed"
                    disabled=move || !pagination.get().has_next()
                    on:click=on_next
                >
                    "Next"
                </button>
            </div>
        </div>
    }
}

// ============================================================================
// Audit Detail Drawer Component
// ============================================================================

#[component]
fn AuditDetailDrawer(
    entry: Signal<AuditLogEntry>,
    on_close: impl Fn() + 'static + Clone,
) -> impl IntoView {
    let (copied, set_copied) = create_signal(false);
    
    let on_close_backdrop = on_close.clone();
    let on_close_header = on_close;

    let on_copy = move |_| {
        let entry_data = entry.get();
        if let Ok(json) = serde_json::to_string_pretty(&entry_data) {
            copy_to_clipboard(&json);
            set_copied.set(true);
            set_timeout(move || set_copied.set(false), std::time::Duration::from_secs(2));
        }
    };

    view! {
        <div class="fixed inset-0 z-50 flex justify-end">
            // Backdrop
            <div
                class="absolute inset-0 bg-black/60 backdrop-blur-sm"
                on:click=move |_| on_close_backdrop()
            />

            // Drawer
            <div class="relative w-full max-w-lg bg-slate-800 border-l border-slate-700 shadow-2xl overflow-hidden flex flex-col">
                // Header
                <div class="flex items-center justify-between px-6 py-4 border-b border-slate-700">
                    <h2 class="text-lg font-semibold text-white">"Audit Entry Details"</h2>
                    <div class="flex items-center gap-2">
                        <button
                            class="flex items-center gap-1.5 px-3 py-1.5 text-sm text-slate-400 \
                                   hover:text-white hover:bg-slate-700 rounded-lg transition-colors"
                            on:click=on_copy
                        >
                            <CopyIcon class="w-4 h-4" />
                            {move || if copied.get() { "Copied!" } else { "Copy" }}
                        </button>
                        <button
                            class="p-1.5 text-slate-400 hover:text-white rounded-lg transition-colors"
                            on:click=move |_| on_close_header()
                        >
                            <CloseIcon class="w-5 h-5" />
                        </button>
                    </div>
                </div>

                // Content
                <div class="flex-1 overflow-y-auto p-6 space-y-6">
                    // Basic info
                    <div class="space-y-4">
                        <DetailField label="ID" value=Signal::derive(move || entry.get().id.clone()) />
                        <DetailField label="Timestamp" value=Signal::derive(move || entry.get().timestamp.clone()) />
                        <DetailField label="Action" value=Signal::derive(move || entry.get().action.clone()) />
                        <DetailField label="Result" value=Signal::derive(move || entry.get().result.clone()) />
                    </div>

                    // Actor info
                    <div class="pt-4 border-t border-slate-700">
                        <h3 class="text-sm font-medium text-slate-300 mb-3">"Actor Information"</h3>
                        <div class="space-y-3">
                            <DetailField label="Type" value=Signal::derive(move || entry.get().actor_type.clone()) />
                            <DetailField label="ID" value=Signal::derive(move || entry.get().actor_id.clone().unwrap_or_else(|| "-".to_string())) />
                            <DetailField label="Name" value=Signal::derive(move || entry.get().actor_name.clone().unwrap_or_else(|| "-".to_string())) />
                        </div>
                    </div>

                    // Resource info
                    <div class="pt-4 border-t border-slate-700">
                        <h3 class="text-sm font-medium text-slate-300 mb-3">"Resource Information"</h3>
                        <div class="space-y-3">
                            <DetailField label="Type" value=Signal::derive(move || entry.get().resource_type.clone().unwrap_or_else(|| "-".to_string())) />
                            <DetailField label="ID" value=Signal::derive(move || entry.get().resource_id.clone().unwrap_or_else(|| "-".to_string())) />
                        </div>
                    </div>

                    // Request info
                    <div class="pt-4 border-t border-slate-700">
                        <h3 class="text-sm font-medium text-slate-300 mb-3">"Request Information"</h3>
                        <div class="space-y-3">
                            <DetailField label="IP Address" value=Signal::derive(move || entry.get().ip_address.clone().unwrap_or_else(|| "-".to_string())) />
                            <DetailField label="User Agent" value=Signal::derive(move || entry.get().user_agent.clone().unwrap_or_else(|| "-".to_string())) />
                        </div>
                    </div>

                    // Details JSON
                    <div class="pt-4 border-t border-slate-700">
                        <h3 class="text-sm font-medium text-slate-300 mb-3">"Details"</h3>
                        <div class="bg-slate-900 rounded-lg p-4 overflow-x-auto">
                            <pre class="text-xs text-slate-300 font-mono whitespace-pre-wrap">
                                {move || {
                                    entry.get().details
                                        .map(|d| serde_json::to_string_pretty(&d).unwrap_or_else(|_| "{}".to_string()))
                                        .unwrap_or_else(|| "No details available".to_string())
                                }}
                            </pre>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Detail Field Component
// ============================================================================

#[component]
fn DetailField(
    label: &'static str,
    value: Signal<String>,
) -> impl IntoView {
    view! {
        <div class="flex items-start gap-4">
            <span class="text-xs text-slate-500 w-24 shrink-0">{label}</span>
            <span class="text-sm text-white break-all">{value}</span>
        </div>
    }
}

// ============================================================================
// Icons
// ============================================================================

#[component]
fn DownloadIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="7 10 12 15 17 10" />
            <line x1="12" y1="15" x2="12" y2="3" />
        </svg>
    }
}

#[component]
fn FileTextIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
            <line x1="16" y1="13" x2="8" y2="13" />
            <line x1="16" y1="17" x2="8" y2="17" />
            <polyline points="10 9 9 9 8 9" />
        </svg>
    }
}

#[component]
fn UserIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
            <circle cx="12" cy="7" r="4" />
        </svg>
    }
}

#[component]
fn CopyIcon(
    #[prop(default = "w-5 h-5")] class: &'static str,
) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
        </svg>
    }
}

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

// ============================================================================
// Helper Functions
// ============================================================================

/// Format a timestamp as relative time (e.g., "2 minutes ago")
fn format_relative_time(timestamp: &str) -> String {
    // Try to parse the timestamp
    // For simplicity, we'll do a basic calculation
    // In production, you'd use a proper date library
    
    if let Ok(dt) = timestamp.parse::<chrono::DateTime<chrono::Utc>>() {
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(dt);
        
        let seconds = duration.num_seconds();
        if seconds < 60 {
            return "Just now".to_string();
        }
        
        let minutes = duration.num_minutes();
        if minutes < 60 {
            return if minutes == 1 {
                "1 minute ago".to_string()
            } else {
                format!("{} minutes ago", minutes)
            };
        }
        
        let hours = duration.num_hours();
        if hours < 24 {
            return if hours == 1 {
                "1 hour ago".to_string()
            } else {
                format!("{} hours ago", hours)
            };
        }
        
        let days = duration.num_days();
        if days < 30 {
            return if days == 1 {
                "1 day ago".to_string()
            } else {
                format!("{} days ago", days)
            };
        }
        
        format!("{} days ago", days)
    } else {
        timestamp.to_string()
    }
}

/// Get badge class for action type
fn get_action_badge_class(action: &str) -> &'static str {
    match action.to_lowercase().as_str() {
        s if s.contains("create") => "bg-green-500/20 text-green-400",
        s if s.contains("delete") || s.contains("remove") => "bg-red-500/20 text-red-400",
        s if s.contains("update") || s.contains("modify") => "bg-amber-500/20 text-amber-400",
        s if s.contains("deploy") => "bg-violet-500/20 text-violet-400",
        s if s.contains("login") || s.contains("auth") => "bg-cyan-500/20 text-cyan-400",
        _ => "bg-slate-500/20 text-slate-400",
    }
}

/// Get badge class for result
fn get_result_badge_class(result: &str) -> &'static str {
    match result.to_lowercase().as_str() {
        "success" | "ok" | "completed" => "bg-green-500/20 text-green-400",
        "failure" | "failed" | "error" => "bg-red-500/20 text-red-400",
        "pending" | "in_progress" => "bg-amber-500/20 text-amber-400",
        _ => "bg-slate-500/20 text-slate-400",
    }
}

/// Copy text to clipboard
fn copy_to_clipboard(text: &str) {
    if let Some(window) = web_sys::window() {
        let clipboard = window.navigator().clipboard();
        let text = text.to_string();
        let _ = clipboard.write_text(&text);
    }
}

/// Export entries to CSV and trigger download
fn export_to_csv(entries: &[AuditLogEntry]) {
    let mut csv = String::from("ID,Timestamp,Actor Type,Actor ID,Actor Name,Action,Resource Type,Resource ID,Result,IP Address,User Agent\n");
    
    for entry in entries {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{}\n",
            escape_csv(&entry.id),
            escape_csv(&entry.timestamp),
            escape_csv(&entry.actor_type),
            escape_csv(&entry.actor_id.clone().unwrap_or_default()),
            escape_csv(&entry.actor_name.clone().unwrap_or_default()),
            escape_csv(&entry.action),
            escape_csv(&entry.resource_type.clone().unwrap_or_default()),
            escape_csv(&entry.resource_id.clone().unwrap_or_default()),
            escape_csv(&entry.result),
            escape_csv(&entry.ip_address.clone().unwrap_or_default()),
            escape_csv(&entry.user_agent.clone().unwrap_or_default()),
        ));
    }
    
    // Create a Blob and trigger download
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            // Create blob
            let blob_parts = js_sys::Array::new();
            blob_parts.push(&wasm_bindgen::JsValue::from_str(&csv));
            
            let options = web_sys::BlobPropertyBag::new();
            options.set_type("text/csv");
            
            if let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&blob_parts, &options) {
                if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                    // Create download link
                    if let Ok(a) = document.create_element("a") {
                        let _ = a.set_attribute("href", &url);
                        let _ = a.set_attribute("download", "audit_log.csv");
                        
                        if let Some(body) = document.body() {
                            let _ = body.append_child(&a);
                            if let Some(html_a) = a.dyn_ref::<web_sys::HtmlElement>() {
                                html_a.click();
                            }
                            let _ = body.remove_child(&a);
                        }
                        
                        let _ = web_sys::Url::revoke_object_url(&url);
                    }
                }
            }
        }
    }
}

/// Escape a string for CSV
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
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

async fn fetch_available_actions() -> Result<Vec<String>, String> {
    let url = format!("{}/api/v1/audit/actions", get_base_url());

    let response = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.ok() {
        response.json::<Vec<String>>().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        // Return default actions if endpoint doesn't exist
        Ok(vec![
            "config.create".to_string(),
            "config.update".to_string(),
            "config.delete".to_string(),
            "deploy.start".to_string(),
            "deploy.complete".to_string(),
            "user.login".to_string(),
            "user.logout".to_string(),
        ])
    }
}

async fn fetch_audit_entries(
    filters: &AuditFilters,
    pagination: &Pagination,
) -> Result<(Vec<AuditLogEntry>, usize), String> {
    let mut url = format!(
        "{}/api/v1/audit?limit={}&offset={}",
        get_base_url(),
        pagination.limit,
        pagination.offset
    );

    if let Some(ref action) = filters.action {
        url.push_str(&format!("&action={}", action));
    }
    if let Some(ref actor) = filters.actor {
        url.push_str(&format!("&actor_id={}", actor));
    }
    if let Some(ref resource_type) = filters.resource_type {
        url.push_str(&format!("&resource_type={}", resource_type));
    }

    let response = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.ok() {
        #[derive(Deserialize)]
        struct AuditResponse {
            entries: Vec<AuditLogEntry>,
            total: usize,
        }

        // Try to parse as structured response first
        if let Ok(data) = response.json::<AuditResponse>().await {
            Ok((data.entries, data.total))
        } else {
            // Return mock data for development
            Ok((get_mock_audit_entries(), 50))
        }
    } else {
        // Return mock data for development
        Ok((get_mock_audit_entries(), 50))
    }
}

/// Mock data for development/demo purposes
fn get_mock_audit_entries() -> Vec<AuditLogEntry> {
    vec![
        AuditLogEntry {
            id: "audit-001".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            actor_type: "user".to_string(),
            actor_id: Some("user-123".to_string()),
            actor_name: Some("Admin User".to_string()),
            action: "config.update".to_string(),
            resource_type: Some("pipeline".to_string()),
            resource_id: Some("main-pipeline".to_string()),
            details: Some(serde_json::json!({
                "changes": {
                    "transforms": ["added remap transform"],
                    "sinks": ["updated elasticsearch config"]
                }
            })),
            ip_address: Some("192.168.1.100".to_string()),
            user_agent: Some("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)".to_string()),
            result: "success".to_string(),
        },
        AuditLogEntry {
            id: "audit-002".to_string(),
            timestamp: (chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339(),
            actor_type: "system".to_string(),
            actor_id: None,
            actor_name: Some("Scheduler".to_string()),
            action: "deploy.complete".to_string(),
            resource_type: Some("deployment".to_string()),
            resource_id: Some("deploy-456".to_string()),
            details: Some(serde_json::json!({
                "version": "1.2.3",
                "duration_ms": 1523
            })),
            ip_address: None,
            user_agent: None,
            result: "success".to_string(),
        },
        AuditLogEntry {
            id: "audit-003".to_string(),
            timestamp: (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339(),
            actor_type: "user".to_string(),
            actor_id: Some("user-456".to_string()),
            actor_name: Some("Developer".to_string()),
            action: "config.create".to_string(),
            resource_type: Some("source".to_string()),
            resource_id: Some("http_source".to_string()),
            details: Some(serde_json::json!({
                "type": "http_server",
                "address": "0.0.0.0:8080"
            })),
            ip_address: Some("10.0.0.50".to_string()),
            user_agent: Some("curl/7.79.1".to_string()),
            result: "success".to_string(),
        },
        AuditLogEntry {
            id: "audit-004".to_string(),
            timestamp: (chrono::Utc::now() - chrono::Duration::hours(2)).to_rfc3339(),
            actor_type: "api".to_string(),
            actor_id: Some("api-key-789".to_string()),
            actor_name: Some("CI/CD Pipeline".to_string()),
            action: "deploy.start".to_string(),
            resource_type: Some("deployment".to_string()),
            resource_id: Some("deploy-455".to_string()),
            details: Some(serde_json::json!({
                "trigger": "github_webhook",
                "commit": "abc123"
            })),
            ip_address: Some("34.120.55.100".to_string()),
            user_agent: Some("GitHub-Hookshot/abc1234".to_string()),
            result: "failure".to_string(),
        },
        AuditLogEntry {
            id: "audit-005".to_string(),
            timestamp: (chrono::Utc::now() - chrono::Duration::days(1)).to_rfc3339(),
            actor_type: "user".to_string(),
            actor_id: Some("user-789".to_string()),
            actor_name: Some("Ops Team".to_string()),
            action: "config.delete".to_string(),
            resource_type: Some("transform".to_string()),
            resource_id: Some("legacy_filter".to_string()),
            details: Some(serde_json::json!({
                "reason": "deprecated"
            })),
            ip_address: Some("192.168.1.200".to_string()),
            user_agent: Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64)".to_string()),
            result: "success".to_string(),
        },
    ]
}

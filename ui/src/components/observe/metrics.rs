//! Metrics Dashboard Component
//!
//! Displays comprehensive metrics for Vector components including:
//! - Overall stats (events processed, throughput, errors)
//! - Per-component breakdown table
//! - Time range selection and refresh

use leptos::*;
use serde::{Deserialize, Serialize};

use crate::components::common::RefreshIcon;
use crate::state::AppState;

/// Time range options for metrics display
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimeRange {
    #[default]
    Last15m,
    Last1h,
    Last6h,
    Last24h,
    Last7d,
}

impl TimeRange {
    pub fn label(&self) -> &'static str {
        match self {
            TimeRange::Last15m => "Last 15m",
            TimeRange::Last1h => "Last 1h",
            TimeRange::Last6h => "Last 6h",
            TimeRange::Last24h => "Last 24h",
            TimeRange::Last7d => "Last 7d",
        }
    }

    pub fn all() -> &'static [TimeRange] {
        &[
            TimeRange::Last15m,
            TimeRange::Last1h,
            TimeRange::Last6h,
            TimeRange::Last24h,
            TimeRange::Last7d,
        ]
    }
}

/// Sort column options for the component table
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortColumn {
    #[default]
    Component,
    Type,
    EventsIn,
    EventsOut,
    Errors,
    Status,
}

/// Sort direction
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

impl SortDirection {
    pub fn toggle(&self) -> Self {
        match self {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        }
    }
}

/// Aggregated metrics for display
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    pub total_events_processed: u64,
    pub events_per_second: f64,
    pub total_bytes_processed: u64,
    pub error_rate: f64,
    pub total_errors: u64,
    pub components: Vec<ComponentMetricRow>,
}

/// Per-component metrics row
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComponentMetricRow {
    pub component_id: String,
    pub component_type: String,
    pub component_kind: String,
    pub events_in: u64,
    pub events_out: u64,
    pub errors: u64,
    pub status: ComponentStatus,
}

/// Component health status
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentStatus {
    Healthy,
    Warning,
    Error,
    #[default]
    Unknown,
}

impl ComponentStatus {
    pub fn class(&self) -> &'static str {
        match self {
            ComponentStatus::Healthy => "text-green-400",
            ComponentStatus::Warning => "text-amber-400",
            ComponentStatus::Error => "text-red-400",
            ComponentStatus::Unknown => "text-slate-400",
        }
    }

    pub fn bg_class(&self) -> &'static str {
        match self {
            ComponentStatus::Healthy => "bg-green-500",
            ComponentStatus::Warning => "bg-amber-500",
            ComponentStatus::Error => "bg-red-500",
            ComponentStatus::Unknown => "bg-slate-500",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ComponentStatus::Healthy => "Healthy",
            ComponentStatus::Warning => "Warning",
            ComponentStatus::Error => "Error",
            ComponentStatus::Unknown => "Unknown",
        }
    }
}

/// Format large numbers with K/M/B suffixes
fn format_number(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Format bytes with appropriate unit
fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_099_511_627_776 {
        format!("{:.1} TB", bytes as f64 / 1_099_511_627_776.0)
    } else if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.1} KB", bytes as f64 / 1_024.0)
    } else {
        format!("{} B", bytes)
    }
}

/// Format percentage
fn format_percentage(rate: f64) -> String {
    if rate < 0.01 {
        "< 0.01%".to_string()
    } else {
        format!("{:.2}%", rate)
    }
}

/// Main metrics dashboard component
#[component]
pub fn MetricsDashboard() -> impl IntoView {
    let app_state = expect_context::<AppState>();

    // Local state
    let (time_range, set_time_range) = create_signal(TimeRange::Last15m);
    let (metrics, set_metrics) = create_signal(Option::<AggregatedMetrics>::None);
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal(Option::<String>::None);
    let (sort_column, set_sort_column) = create_signal(SortColumn::Component);
    let (sort_direction, set_sort_direction) = create_signal(SortDirection::Ascending);
    let (refreshing, set_refreshing) = create_signal(false);

    // Fetch metrics function
    let fetch_metrics = move || {
        let connected = app_state.connected.get();
        let url = app_state.url.get();

        spawn_local(async move {
            if !connected {
                set_metrics.set(None);
                set_loading.set(false);
                return;
            }

            set_loading.set(true);

            match fetch_metrics_data(&url).await {
                Ok(data) => {
                    set_metrics.set(Some(data));
                    set_error.set(None);
                }
                Err(e) => {
                    set_error.set(Some(e));
                }
            }

            set_loading.set(false);
            set_refreshing.set(false);
        });
    };

    // Fetch on mount and when connection changes
    create_effect(move |_| {
        let _connected = app_state.connected.get();
        fetch_metrics();
    });

    // Handle refresh click
    let on_refresh = move |_| {
        set_refreshing.set(true);
        fetch_metrics();
    };

    // Handle column sort click
    let handle_sort = move |column: SortColumn| {
        if sort_column.get() == column {
            set_sort_direction.update(|d| *d = d.toggle());
        } else {
            set_sort_column.set(column);
            set_sort_direction.set(SortDirection::Ascending);
        }
    };

    // Sorted components memo
    let sorted_components = create_memo(move |_| {
        let Some(m) = metrics.get() else {
            return vec![];
        };

        let mut components = m.components.clone();
        let col = sort_column.get();
        let dir = sort_direction.get();

        components.sort_by(|a, b| {
            let ordering = match col {
                SortColumn::Component => a.component_id.cmp(&b.component_id),
                SortColumn::Type => a.component_type.cmp(&b.component_type),
                SortColumn::EventsIn => a.events_in.cmp(&b.events_in),
                SortColumn::EventsOut => a.events_out.cmp(&b.events_out),
                SortColumn::Errors => a.errors.cmp(&b.errors),
                SortColumn::Status => {
                    let status_order = |s: &ComponentStatus| match s {
                        ComponentStatus::Error => 0,
                        ComponentStatus::Warning => 1,
                        ComponentStatus::Unknown => 2,
                        ComponentStatus::Healthy => 3,
                    };
                    status_order(&a.status).cmp(&status_order(&b.status))
                }
            };

            match dir {
                SortDirection::Ascending => ordering,
                SortDirection::Descending => ordering.reverse(),
            }
        });

        components
    });

    view! {
        <div class="flex-1 overflow-auto p-6 bg-slate-900">
            <div class="max-w-7xl mx-auto">
                // Check connection status
                <Show
                    when=move || app_state.connected.get()
                    fallback=move || view! { <EmptyState /> }
                >
                    // Header
                    <div class="flex items-center justify-between mb-6">
                        <div>
                            <h1 class="text-2xl font-bold text-white">"Metrics Dashboard"</h1>
                            <p class="text-slate-400 mt-1">"Real-time pipeline performance metrics"</p>
                        </div>

                        <div class="flex items-center gap-3">
                            // Time range selector
                            <TimeRangeSelector
                                selected=time_range
                                on_change=set_time_range
                            />

                            // Refresh button
                            <button
                                class="flex items-center gap-2 px-4 py-2 bg-slate-700 hover:bg-slate-600 \
                                       text-white rounded-lg transition-colors disabled:opacity-50"
                                on:click=on_refresh
                                disabled=move || refreshing.get()
                            >
                                <span class=move || if refreshing.get() { "animate-spin" } else { "" }>
                                    <RefreshIcon class="w-4 h-4" />
                                </span>
                                "Refresh"
                            </button>
                        </div>
                    </div>

                    // Loading state
                    <Show
                        when=move || !loading.get()
                        fallback=move || view! {
                            <div class="flex items-center justify-center py-16">
                                <div class="animate-spin w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full" />
                            </div>
                        }
                    >
                        // Error state
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

                        // Stats cards row
                        {move || {
                            let m = metrics.get().unwrap_or_default();
                            let error_rate = m.error_rate;
                            let has_high_error_rate = error_rate > 1.0;
                            view! {
                                <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
                                    <StatsCard
                                        title="Total Events Processed"
                                        value=format_number(m.total_events_processed)
                                        show_trend=true
                                        trend_up=true
                                        icon_bg="bg-violet-500/20"
                                        icon_color="text-violet-400"
                                    />
                                    <StatsCard
                                        title="Events/sec"
                                        value=format!("{:.1}", m.events_per_second)
                                        show_trend=false
                                        trend_up=false
                                        icon_bg="bg-cyan-500/20"
                                        icon_color="text-cyan-400"
                                    />
                                    <StatsCard
                                        title="Total Bytes Processed"
                                        value=format_bytes(m.total_bytes_processed)
                                        show_trend=true
                                        trend_up=true
                                        icon_bg="bg-blue-500/20"
                                        icon_color="text-blue-400"
                                    />
                                    <StatsCard
                                        title="Error Rate"
                                        value=format_percentage(error_rate)
                                        show_trend=has_high_error_rate
                                        trend_up=has_high_error_rate
                                        icon_bg=if has_high_error_rate { "bg-red-500/20" } else { "bg-green-500/20" }
                                        icon_color=if has_high_error_rate { "text-red-400" } else { "text-green-400" }
                                    />
                                </div>
                            }
                        }}

                        // Component breakdown section
                        <div class="bg-slate-800 rounded-xl border border-slate-700 overflow-hidden">
                            <div class="px-6 py-4 border-b border-slate-700">
                                <h2 class="text-lg font-semibold text-white">"Component Breakdown"</h2>
                                <p class="text-sm text-slate-400 mt-1">"Metrics per pipeline component"</p>
                            </div>

                            <div class="overflow-x-auto">
                                <table class="w-full">
                                    <thead class="bg-slate-800/50">
                                        <tr>
                                            <SortableHeader
                                                label="Component"
                                                column=SortColumn::Component
                                                current_column=sort_column
                                                direction=sort_direction
                                                on_click=handle_sort
                                            />
                                            <SortableHeader
                                                label="Type"
                                                column=SortColumn::Type
                                                current_column=sort_column
                                                direction=sort_direction
                                                on_click=handle_sort
                                            />
                                            <SortableHeader
                                                label="Events In"
                                                column=SortColumn::EventsIn
                                                current_column=sort_column
                                                direction=sort_direction
                                                on_click=handle_sort
                                            />
                                            <SortableHeader
                                                label="Events Out"
                                                column=SortColumn::EventsOut
                                                current_column=sort_column
                                                direction=sort_direction
                                                on_click=handle_sort
                                            />
                                            <SortableHeader
                                                label="Errors"
                                                column=SortColumn::Errors
                                                current_column=sort_column
                                                direction=sort_direction
                                                on_click=handle_sort
                                            />
                                            <SortableHeader
                                                label="Status"
                                                column=SortColumn::Status
                                                current_column=sort_column
                                                direction=sort_direction
                                                on_click=handle_sort
                                            />
                                        </tr>
                                    </thead>
                                    <tbody class="divide-y divide-slate-700">
                                        <For
                                            each=move || sorted_components.get()
                                            key=|row| row.component_id.clone()
                                            children=move |row| view! {
                                                <ComponentRow row=row />
                                            }
                                        />
                                    </tbody>
                                </table>

                                // Empty table state
                                <Show when=move || sorted_components.get().is_empty()>
                                    <div class="text-center py-8">
                                        <p class="text-slate-400">"No components found"</p>
                                        <p class="text-sm text-slate-500 mt-1">"Pipeline components will appear here when running"</p>
                                    </div>
                                </Show>
                            </div>
                        </div>
                    </Show>
                </Show>
            </div>
        </div>
    }
}

/// Empty state when not connected
#[component]
fn EmptyState() -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center py-24">
            <div class="w-16 h-16 rounded-full bg-slate-800 flex items-center justify-center mb-6">
                <svg class="w-8 h-8 text-slate-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M22 12h-4l-3 9L9 3l-3 9H2" />
                </svg>
            </div>
            <h2 class="text-xl font-semibold text-white mb-2">"No Metrics Available"</h2>
            <p class="text-slate-400 text-center max-w-md">
                "Connect to Vector to see real-time metrics for your pipeline components."
            </p>
        </div>
    }
}

/// Stats card component
#[component]
fn StatsCard(
    title: &'static str,
    value: String,
    #[prop(default = false)] show_trend: bool,
    #[prop(default = true)] trend_up: bool,
    icon_bg: &'static str,
    icon_color: &'static str,
) -> impl IntoView {
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 p-5">
            <div class="flex items-start justify-between">
                <div>
                    <p class="text-sm text-slate-400 mb-1">{title}</p>
                    <div class="flex items-baseline gap-2">
                        <span class="text-2xl font-bold text-white">{value}</span>
                        {move || {
                            if show_trend {
                                let (icon, color) = if trend_up {
                                    ("↑", "text-green-400")
                                } else {
                                    ("↓", "text-red-400")
                                };
                                view! {
                                    <span class=format!("text-sm {}", color)>{icon}</span>
                                }.into_view()
                            } else {
                                view! {}.into_view()
                            }
                        }}
                    </div>
                </div>
                <div class=format!("w-10 h-10 rounded-lg {} flex items-center justify-center", icon_bg)>
                    <svg class=format!("w-5 h-5 {}", icon_color) viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M22 12h-4l-3 9L9 3l-3 9H2" />
                    </svg>
                </div>
            </div>
        </div>
    }
}

/// Time range selector component
#[component]
fn TimeRangeSelector(
    selected: ReadSignal<TimeRange>,
    on_change: WriteSignal<TimeRange>,
) -> impl IntoView {
    view! {
        <div class="flex bg-slate-800 rounded-lg p-1 border border-slate-700">
            {TimeRange::all().iter().map(|&tr| {
                let is_selected = move || selected.get() == tr;
                view! {
                    <button
                        class=move || {
                            if is_selected() {
                                "px-3 py-1.5 text-sm font-medium rounded-md bg-blue-500 text-white transition-colors"
                            } else {
                                "px-3 py-1.5 text-sm font-medium rounded-md text-slate-400 hover:text-white transition-colors"
                            }
                        }
                        on:click=move |_| on_change.set(tr)
                    >
                        {tr.label()}
                    </button>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}

/// Sortable table header
#[component]
fn SortableHeader(
    label: &'static str,
    column: SortColumn,
    current_column: ReadSignal<SortColumn>,
    direction: ReadSignal<SortDirection>,
    on_click: impl Fn(SortColumn) + 'static + Copy,
) -> impl IntoView {
    let is_active = move || current_column.get() == column;
    let arrow = move || {
        if is_active() {
            match direction.get() {
                SortDirection::Ascending => "↑",
                SortDirection::Descending => "↓",
            }
        } else {
            ""
        }
    };

    view! {
        <th
            class="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider cursor-pointer hover:text-white transition-colors select-none"
            on:click=move |_| on_click(column)
        >
            <div class="flex items-center gap-1">
                {label}
                <span class="text-blue-400">{arrow}</span>
            </div>
        </th>
    }
}

/// Component metrics row
#[component]
fn ComponentRow(row: ComponentMetricRow) -> impl IntoView {
    let kind_class = match row.component_kind.as_str() {
        "source" => "bg-violet-500/20 text-violet-400",
        "transform" => "bg-cyan-500/20 text-cyan-400",
        "sink" => "bg-orange-500/20 text-orange-400",
        _ => "bg-slate-500/20 text-slate-400",
    };

    let error_class = if row.errors > 0 {
        "text-red-400"
    } else {
        "text-slate-400"
    };

    view! {
        <tr class="hover:bg-slate-700/30 transition-colors">
            <td class="px-6 py-4 whitespace-nowrap">
                <span class="text-sm font-medium text-white">{row.component_id.clone()}</span>
            </td>
            <td class="px-6 py-4 whitespace-nowrap">
                <span class=format!("inline-flex items-center px-2 py-1 rounded-md text-xs font-medium {}", kind_class)>
                    {row.component_type.clone()}
                </span>
            </td>
            <td class="px-6 py-4 whitespace-nowrap text-sm text-slate-300">
                {format_number(row.events_in)}
            </td>
            <td class="px-6 py-4 whitespace-nowrap text-sm text-slate-300">
                {format_number(row.events_out)}
            </td>
            <td class="px-6 py-4 whitespace-nowrap text-sm">
                <span class=error_class>
                    {format_number(row.errors)}
                </span>
            </td>
            <td class="px-6 py-4 whitespace-nowrap">
                <div class="flex items-center gap-2">
                    <div class=format!("w-2 h-2 rounded-full {}", row.status.bg_class()) />
                    <span class=format!("text-sm {}", row.status.class())>
                        {row.status.label()}
                    </span>
                </div>
            </td>
        </tr>
    }
}

/// Fetch metrics data from Vector API
async fn fetch_metrics_data(base_url: &str) -> Result<AggregatedMetrics, String> {
    // Try to fetch from Vector's GraphQL API
    let graphql_url = format!("{}/graphql", base_url);

    let query = r#"
        query {
            components {
                edges {
                    node {
                        componentId
                        componentType
                        __typename
                        ... on Source {
                            metrics {
                                receivedEventsTotal {
                                    receivedEventsTotal
                                }
                                sentEventsTotal {
                                    sentEventsTotal
                                }
                                receivedBytesTotal {
                                    receivedBytesTotal
                                }
                                sentBytesTotal {
                                    sentBytesTotal
                                }
                            }
                        }
                        ... on Transform {
                            metrics {
                                receivedEventsTotal {
                                    receivedEventsTotal
                                }
                                sentEventsTotal {
                                    sentEventsTotal
                                }
                                receivedBytesTotal {
                                    receivedBytesTotal
                                }
                                sentBytesTotal {
                                    sentBytesTotal
                                }
                            }
                        }
                        ... on Sink {
                            metrics {
                                receivedEventsTotal {
                                    receivedEventsTotal
                                }
                                sentEventsTotal {
                                    sentEventsTotal
                                }
                                receivedBytesTotal {
                                    receivedBytesTotal
                                }
                                sentBytesTotal {
                                    sentBytesTotal
                                }
                            }
                        }
                    }
                }
            }
        }
    "#;

    #[derive(Serialize)]
    struct GraphQLRequest {
        query: String,
    }

    #[derive(Deserialize)]
    struct GraphQLResponse {
        data: Option<ComponentsData>,
    }

    #[derive(Deserialize)]
    struct ComponentsData {
        components: ComponentsConnection,
    }

    #[derive(Deserialize)]
    struct ComponentsConnection {
        edges: Vec<ComponentEdge>,
    }

    #[derive(Deserialize)]
    struct ComponentEdge {
        node: ComponentNode,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ComponentNode {
        component_id: String,
        component_type: String,
        #[serde(rename = "__typename")]
        typename: String,
        metrics: Option<ComponentMetricsData>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ComponentMetricsData {
        received_events_total: Option<ReceivedEventsTotal>,
        sent_events_total: Option<SentEventsTotal>,
        received_bytes_total: Option<ReceivedBytesTotal>,
        sent_bytes_total: Option<SentBytesTotal>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ReceivedEventsTotal {
        received_events_total: u64,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SentEventsTotal {
        sent_events_total: u64,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ReceivedBytesTotal {
        received_bytes_total: u64,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SentBytesTotal {
        sent_bytes_total: u64,
    }

    let request = GraphQLRequest {
        query: query.to_string(),
    };

    let response = gloo_net::http::Request::post(&graphql_url)
        .header("Content-Type", "application/json")
        .json(&request)
        .map_err(|e| format!("Request error: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Connection error: {}", e))?;

    if !response.ok() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let result: GraphQLResponse = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let Some(data) = result.data else {
        return Err("No data in response".to_string());
    };

    // Aggregate metrics
    let mut total_events_in: u64 = 0;
    let mut total_events_out: u64 = 0;
    let mut total_bytes: u64 = 0;
    let total_errors: u64 = 0;
    let mut components = Vec::new();

    for edge in data.components.edges {
        let node = edge.node;
        let metrics = node.metrics.as_ref();

        let events_in = metrics
            .and_then(|m| m.received_events_total.as_ref())
            .map(|m| m.received_events_total)
            .unwrap_or(0);

        let events_out = metrics
            .and_then(|m| m.sent_events_total.as_ref())
            .map(|m| m.sent_events_total)
            .unwrap_or(0);

        let bytes_in = metrics
            .and_then(|m| m.received_bytes_total.as_ref())
            .map(|m| m.received_bytes_total)
            .unwrap_or(0);

        let bytes_out = metrics
            .and_then(|m| m.sent_bytes_total.as_ref())
            .map(|m| m.sent_bytes_total)
            .unwrap_or(0);

        total_events_in += events_in;
        total_events_out += events_out;
        total_bytes += bytes_in + bytes_out;

        // Determine status based on metrics
        let status = if events_in > 0 && events_out > 0 {
            ComponentStatus::Healthy
        } else if events_in > 0 {
            ComponentStatus::Warning
        } else {
            ComponentStatus::Unknown
        };

        let kind = match node.typename.as_str() {
            "Source" => "source",
            "Transform" => "transform",
            "Sink" => "sink",
            _ => "unknown",
        };

        components.push(ComponentMetricRow {
            component_id: node.component_id,
            component_type: node.component_type,
            component_kind: kind.to_string(),
            events_in,
            events_out,
            errors: 0, // Vector doesn't expose errors per component in this query
            status,
        });
    }

    // Calculate error rate (placeholder - would need error metrics from Vector)
    let error_rate = if total_events_in > 0 {
        (total_errors as f64 / total_events_in as f64) * 100.0
    } else {
        0.0
    };

    // Events per second (placeholder - would need time-series data)
    let events_per_second = (total_events_out as f64) / 900.0; // Rough estimate over 15 min

    Ok(AggregatedMetrics {
        total_events_processed: total_events_out,
        events_per_second,
        total_bytes_processed: total_bytes,
        error_rate,
        total_errors,
        components,
    })
}

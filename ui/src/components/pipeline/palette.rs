//! Component Palette
//!
//! Sidebar showing available Vector components for drag-and-drop.
//! Features search, categorization, descriptions, and recently used tracking.

use leptos::*;

use crate::components::common::*;

/// Component definition for the palette
#[derive(Clone, Copy)]
struct ComponentDef {
    component_type: &'static str,
    label: &'static str,
    description: &'static str,
    category: &'static str,
}

const RECENT_STORAGE_KEY: &str = "vectorize_recent_components";
const MAX_RECENT: usize = 5;

/// Load recently used components from localStorage
fn load_recent_components() -> Vec<String> {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(json)) = storage.get_item(RECENT_STORAGE_KEY) {
                if let Ok(parsed) = serde_json::from_str::<Vec<String>>(&json) {
                    return parsed;
                }
            }
        }
    }
    vec![]
}

/// Save recently used components to localStorage
fn save_recent_components(recent: &[String]) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(json) = serde_json::to_string(recent) {
                let _ = storage.set_item(RECENT_STORAGE_KEY, &json);
            }
        }
    }
}

/// Add a component to recently used list
fn add_to_recent(component_type: &str, set_recent: WriteSignal<Vec<String>>) {
    set_recent.update(|recent| {
        // Remove if already exists
        recent.retain(|c| c != component_type);
        // Add to front
        recent.insert(0, component_type.to_string());
        // Keep only MAX_RECENT
        recent.truncate(MAX_RECENT);
        // Save to localStorage
        save_recent_components(recent);
    });
}

/// Find a component definition by type
fn find_component(component_type: &str) -> Option<&'static ComponentDef> {
    SOURCES.iter()
        .chain(TRANSFORMS.iter())
        .chain(SINKS.iter())
        .find(|c| c.component_type == component_type)
}

// Available sources - comprehensive list of Vector sources
const SOURCES: &[ComponentDef] = &[
    // Testing & Development
    ComponentDef { component_type: "demo_logs", label: "Demo Logs", description: "Generate fake log data for testing", category: "source" },
    
    // Standard I/O
    ComponentDef { component_type: "stdin", label: "Standard Input", description: "Read from stdin", category: "source" },
    ComponentDef { component_type: "file", label: "File", description: "Read/tail log files", category: "source" },
    ComponentDef { component_type: "exec", label: "Exec", description: "Execute commands and capture output", category: "source" },
    ComponentDef { component_type: "journald", label: "Journald", description: "Read from systemd journal", category: "source" },
    
    // Network
    ComponentDef { component_type: "http_server", label: "HTTP Server", description: "Receive data via HTTP endpoint", category: "source" },
    ComponentDef { component_type: "socket", label: "Socket", description: "Listen on TCP/UDP socket", category: "source" },
    ComponentDef { component_type: "syslog", label: "Syslog", description: "Receive syslog messages", category: "source" },
    ComponentDef { component_type: "statsd", label: "StatsD", description: "Receive StatsD metrics", category: "source" },
    ComponentDef { component_type: "fluent", label: "Fluent", description: "Receive Fluentd/Fluent Bit data", category: "source" },
    
    // Message Queues
    ComponentDef { component_type: "kafka", label: "Kafka", description: "Consume from Kafka topics", category: "source" },
    ComponentDef { component_type: "nats", label: "NATS", description: "Subscribe to NATS subjects", category: "source" },
    ComponentDef { component_type: "redis", label: "Redis", description: "Read from Redis lists/channels", category: "source" },
    ComponentDef { component_type: "amqp", label: "AMQP", description: "Consume from RabbitMQ/AMQP", category: "source" },
    ComponentDef { component_type: "pulsar", label: "Pulsar", description: "Consume from Apache Pulsar", category: "source" },
    
    // Cloud Providers
    ComponentDef { component_type: "aws_s3", label: "AWS S3", description: "Read objects from S3 buckets", category: "source" },
    ComponentDef { component_type: "aws_sqs", label: "AWS SQS", description: "Receive from SQS queues", category: "source" },
    ComponentDef { component_type: "aws_kinesis_firehose", label: "AWS Kinesis Firehose", description: "Receive from Kinesis Firehose", category: "source" },
    ComponentDef { component_type: "gcp_pubsub", label: "GCP Pub/Sub", description: "Subscribe to GCP Pub/Sub", category: "source" },
    
    // Container & Orchestration
    ComponentDef { component_type: "docker_logs", label: "Docker Logs", description: "Collect Docker container logs", category: "source" },
    ComponentDef { component_type: "kubernetes_logs", label: "Kubernetes Logs", description: "Collect K8s pod logs", category: "source" },
    
    // Observability Agents
    ComponentDef { component_type: "datadog_agent", label: "Datadog Agent", description: "Receive from Datadog agent", category: "source" },
    ComponentDef { component_type: "splunk_hec", label: "Splunk HEC", description: "Receive Splunk HTTP Event Collector", category: "source" },
    ComponentDef { component_type: "opentelemetry", label: "OpenTelemetry", description: "Receive OTLP data", category: "source" },
    ComponentDef { component_type: "prometheus_scrape", label: "Prometheus Scrape", description: "Scrape Prometheus endpoints", category: "source" },
    ComponentDef { component_type: "prometheus_remote_write", label: "Prometheus Remote Write", description: "Receive Prometheus remote write", category: "source" },
    
    // Vector Internal
    ComponentDef { component_type: "internal_logs", label: "Internal Logs", description: "Vector's own logs", category: "source" },
    ComponentDef { component_type: "internal_metrics", label: "Internal Metrics", description: "Vector's own metrics", category: "source" },
    ComponentDef { component_type: "vector", label: "Vector", description: "Receive from other Vector instances", category: "source" },
    
    // Databases & Services
    ComponentDef { component_type: "mongodb_metrics", label: "MongoDB Metrics", description: "Collect MongoDB metrics", category: "source" },
    ComponentDef { component_type: "postgresql_metrics", label: "PostgreSQL Metrics", description: "Collect PostgreSQL metrics", category: "source" },
    ComponentDef { component_type: "nginx_metrics", label: "Nginx Metrics", description: "Collect Nginx metrics", category: "source" },
    ComponentDef { component_type: "apache_metrics", label: "Apache Metrics", description: "Collect Apache metrics", category: "source" },
    ComponentDef { component_type: "host_metrics", label: "Host Metrics", description: "Collect system/host metrics", category: "source" },
];

// Available transforms - comprehensive list of Vector transforms
const TRANSFORMS: &[ComponentDef] = &[
    // VRL & Scripting
    ComponentDef { component_type: "remap", label: "Remap (VRL)", description: "Transform data using VRL language", category: "transform" },
    ComponentDef { component_type: "lua", label: "Lua", description: "Transform using Lua scripts", category: "transform" },
    
    // Filtering & Routing
    ComponentDef { component_type: "filter", label: "Filter", description: "Filter events based on condition", category: "transform" },
    ComponentDef { component_type: "route", label: "Route", description: "Split events into multiple outputs", category: "transform" },
    ComponentDef { component_type: "sample", label: "Sample", description: "Sample a percentage of events", category: "transform" },
    ComponentDef { component_type: "throttle", label: "Throttle", description: "Rate limit events", category: "transform" },
    
    // Deduplication & Aggregation
    ComponentDef { component_type: "dedupe", label: "Dedupe", description: "Remove duplicate events", category: "transform" },
    ComponentDef { component_type: "reduce", label: "Reduce", description: "Aggregate events over time", category: "transform" },
    ComponentDef { component_type: "aggregate", label: "Aggregate", description: "Aggregate metrics", category: "transform" },
    
    // Type Conversion
    ComponentDef { component_type: "log_to_metric", label: "Log to Metric", description: "Convert logs to metrics", category: "transform" },
    ComponentDef { component_type: "metric_to_log", label: "Metric to Log", description: "Convert metrics to logs", category: "transform" },
    
    // Metric Transforms
    ComponentDef { component_type: "tag_cardinality_limit", label: "Tag Cardinality Limit", description: "Limit metric tag cardinality", category: "transform" },
    
    // Testing
    ComponentDef { component_type: "test_basic", label: "Test Basic", description: "Basic transform for testing", category: "transform" },
];

// Available sinks - comprehensive list of Vector sinks
const SINKS: &[ComponentDef] = &[
    // Standard Output
    ComponentDef { component_type: "console", label: "Console", description: "Print to stdout/stderr", category: "sink" },
    ComponentDef { component_type: "file", label: "File", description: "Write to files", category: "sink" },
    ComponentDef { component_type: "blackhole", label: "Blackhole", description: "Discard all events (testing)", category: "sink" },
    
    // Network
    ComponentDef { component_type: "http", label: "HTTP", description: "Send via HTTP requests", category: "sink" },
    ComponentDef { component_type: "socket", label: "Socket", description: "Send via TCP/UDP socket", category: "sink" },
    
    // Message Queues
    ComponentDef { component_type: "kafka", label: "Kafka", description: "Produce to Kafka topics", category: "sink" },
    ComponentDef { component_type: "nats", label: "NATS", description: "Publish to NATS subjects", category: "sink" },
    ComponentDef { component_type: "redis", label: "Redis", description: "Write to Redis", category: "sink" },
    ComponentDef { component_type: "amqp", label: "AMQP", description: "Publish to RabbitMQ/AMQP", category: "sink" },
    ComponentDef { component_type: "pulsar", label: "Pulsar", description: "Publish to Apache Pulsar", category: "sink" },
    
    // Cloud Storage
    ComponentDef { component_type: "aws_s3", label: "AWS S3", description: "Store in S3 buckets", category: "sink" },
    ComponentDef { component_type: "gcp_cloud_storage", label: "GCP Cloud Storage", description: "Store in GCS buckets", category: "sink" },
    ComponentDef { component_type: "azure_blob", label: "Azure Blob", description: "Store in Azure Blob Storage", category: "sink" },
    
    // AWS Services
    ComponentDef { component_type: "aws_cloudwatch_logs", label: "AWS CloudWatch Logs", description: "Send to CloudWatch Logs", category: "sink" },
    ComponentDef { component_type: "aws_cloudwatch_metrics", label: "AWS CloudWatch Metrics", description: "Send to CloudWatch Metrics", category: "sink" },
    ComponentDef { component_type: "aws_kinesis_streams", label: "AWS Kinesis Streams", description: "Send to Kinesis Data Streams", category: "sink" },
    ComponentDef { component_type: "aws_kinesis_firehose", label: "AWS Kinesis Firehose", description: "Send to Kinesis Firehose", category: "sink" },
    ComponentDef { component_type: "aws_sqs", label: "AWS SQS", description: "Send to SQS queues", category: "sink" },
    
    // GCP Services  
    ComponentDef { component_type: "gcp_pubsub", label: "GCP Pub/Sub", description: "Publish to GCP Pub/Sub", category: "sink" },
    ComponentDef { component_type: "gcp_stackdriver_logs", label: "GCP Stackdriver Logs", description: "Send to Cloud Logging", category: "sink" },
    ComponentDef { component_type: "gcp_stackdriver_metrics", label: "GCP Stackdriver Metrics", description: "Send to Cloud Monitoring", category: "sink" },
    
    // Search & Analytics
    ComponentDef { component_type: "elasticsearch", label: "Elasticsearch", description: "Index to Elasticsearch", category: "sink" },
    ComponentDef { component_type: "clickhouse", label: "ClickHouse", description: "Insert to ClickHouse", category: "sink" },
    ComponentDef { component_type: "databend", label: "Databend", description: "Insert to Databend", category: "sink" },
    
    // Observability Platforms
    ComponentDef { component_type: "datadog_logs", label: "Datadog Logs", description: "Send logs to Datadog", category: "sink" },
    ComponentDef { component_type: "datadog_metrics", label: "Datadog Metrics", description: "Send metrics to Datadog", category: "sink" },
    ComponentDef { component_type: "datadog_traces", label: "Datadog Traces", description: "Send traces to Datadog", category: "sink" },
    ComponentDef { component_type: "splunk_hec_logs", label: "Splunk HEC", description: "Send to Splunk HEC", category: "sink" },
    ComponentDef { component_type: "new_relic", label: "New Relic", description: "Send to New Relic", category: "sink" },
    ComponentDef { component_type: "honeycomb", label: "Honeycomb", description: "Send to Honeycomb", category: "sink" },
    
    // Logging Platforms
    ComponentDef { component_type: "loki", label: "Loki", description: "Send logs to Grafana Loki", category: "sink" },
    ComponentDef { component_type: "papertrail", label: "Papertrail", description: "Send to Papertrail", category: "sink" },
    ComponentDef { component_type: "logdna", label: "LogDNA/Mezmo", description: "Send to LogDNA/Mezmo", category: "sink" },
    
    // Metrics
    ComponentDef { component_type: "prometheus_exporter", label: "Prometheus Exporter", description: "Expose Prometheus metrics endpoint", category: "sink" },
    ComponentDef { component_type: "prometheus_remote_write", label: "Prometheus Remote Write", description: "Send to Prometheus remote write", category: "sink" },
    ComponentDef { component_type: "influxdb_logs", label: "InfluxDB Logs", description: "Send logs to InfluxDB", category: "sink" },
    ComponentDef { component_type: "influxdb_metrics", label: "InfluxDB Metrics", description: "Send metrics to InfluxDB", category: "sink" },
    ComponentDef { component_type: "statsd", label: "StatsD", description: "Send to StatsD server", category: "sink" },
    
    // Vector
    ComponentDef { component_type: "vector", label: "Vector", description: "Send to other Vector instances", category: "sink" },
];

/// Component palette sidebar
#[component]
pub fn ComponentPalette() -> impl IntoView {
    let (search, set_search) = create_signal(String::new());
    let (collapsed_sections, set_collapsed_sections) = create_signal::<Vec<&'static str>>(vec![]);
    let (recent_components, set_recent_components) = create_signal(load_recent_components());
    
    // Filter components based on search
    let filter_components = move |components: &'static [ComponentDef]| -> Vec<&'static ComponentDef> {
        let query = search.get().to_lowercase();
        if query.is_empty() {
            components.iter().collect()
        } else {
            components.iter()
                .filter(|c| {
                    c.label.to_lowercase().contains(&query) ||
                    c.component_type.to_lowercase().contains(&query) ||
                    c.description.to_lowercase().contains(&query)
                })
                .collect()
        }
    };
    
    let is_collapsed = move |section: &'static str| {
        collapsed_sections.get().contains(&section)
    };
    
    let toggle_section = move |section: &'static str| {
        set_collapsed_sections.update(|sections| {
            if let Some(pos) = sections.iter().position(|s| *s == section) {
                sections.remove(pos);
            } else {
                sections.push(section);
            }
        });
    };
    
    // Check if we should show recently used (not during search)
    let show_recent = move || {
        search.get().is_empty() && !recent_components.get().is_empty()
    };
    
    view! {
        <div class="flex flex-col h-full bg-theme-surface">
            // Search bar
            <div class="p-3 border-b border-theme">
                <div class="relative">
                    <SearchIcon class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-theme-muted" />
                    <input
                        type="text"
                        placeholder="Search components..."
                        class="w-full pl-9 pr-3 py-2 text-sm bg-theme-bg border border-theme rounded-lg text-theme placeholder:text-theme-muted focus:border-accent focus:ring-1 focus:ring-accent"
                        on:input=move |e| {
                            set_search.set(event_target_value(&e));
                        }
                        prop:value=search
                    />
                </div>
            </div>
            
            // Component sections
            <div class="flex-1 overflow-y-auto custom-scrollbar">
                <div class="p-3 space-y-4">
                    // Recently Used section (only when not searching)
                    <Show when=show_recent>
                        <RecentSection
                            recent=recent_components
                            set_recent=set_recent_components
                            collapsed=move || is_collapsed("recent")
                            on_toggle=move || toggle_section("recent")
                        />
                    </Show>
                    
                    // Sources section
                    <PaletteSection
                        title="Sources"
                        color="violet"
                        icon_type="source"
                        collapsed=move || is_collapsed("sources")
                        on_toggle=move || toggle_section("sources")
                        components=move || filter_components(SOURCES)
                        set_recent=set_recent_components
                    />
                    
                    // Transforms section
                    <PaletteSection
                        title="Transforms"
                        color="cyan"
                        icon_type="transform"
                        collapsed=move || is_collapsed("transforms")
                        on_toggle=move || toggle_section("transforms")
                        components=move || filter_components(TRANSFORMS)
                        set_recent=set_recent_components
                    />
                    
                    // Sinks section
                    <PaletteSection
                        title="Sinks"
                        color="orange"
                        icon_type="sink"
                        collapsed=move || is_collapsed("sinks")
                        on_toggle=move || toggle_section("sinks")
                        components=move || filter_components(SINKS)
                        set_recent=set_recent_components
                    />
                </div>
            </div>
            
            // Help text
            <div class="p-3 border-t border-theme text-xs text-theme-muted text-center">
                "Drag components to the canvas"
            </div>
        </div>
    }
}

/// Recently used components section
#[component]
fn RecentSection<F, G>(
    recent: ReadSignal<Vec<String>>,
    set_recent: WriteSignal<Vec<String>>,
    collapsed: F,
    on_toggle: G,
) -> impl IntoView
where
    F: Fn() -> bool + Clone + 'static,
    G: Fn() + 'static,
{
    let collapsed_for_content = collapsed.clone();
    
    view! {
        <section class="pb-3 mb-3 border-b border-theme">
            // Section header
            <button
                class="flex items-center justify-between w-full mb-2 group text-theme-secondary"
                on:click=move |_| on_toggle()
            >
                <div class="flex items-center gap-2">
                    <ClockIcon class="w-3.5 h-3.5" />
                    <h3 class="text-xs font-semibold uppercase tracking-wide">
                        "Recently Used"
                    </h3>
                </div>
                <ChevronIcon 
                    class="w-4 h-4 transition-transform text-theme-muted" 
                    rotated=collapsed
                />
            </button>
            
            {move || {
                if collapsed_for_content() {
                    view! {}.into_view()
                } else {
                    let recent_types = recent.get();
                    view! {
                        <div class="space-y-1">
                            {recent_types.iter().filter_map(|ct| {
                                find_component(ct).map(|comp| {
                                    view! {
                                        <PaletteItem
                                            component_type=comp.component_type
                                            label=comp.label
                                            description=comp.description
                                            category=comp.category
                                            set_recent=set_recent
                                            compact=true
                                        />
                                    }
                                })
                            }).collect_view()}
                        </div>
                    }.into_view()
                }
            }}
        </section>
    }
}

/// Collapsible palette section
#[component]
fn PaletteSection<F, G, H>(
    title: &'static str,
    color: &'static str,
    icon_type: &'static str,
    collapsed: F,
    on_toggle: G,
    components: H,
    set_recent: WriteSignal<Vec<String>>,
) -> impl IntoView 
where
    F: Fn() -> bool + Clone + 'static,
    G: Fn() + 'static,
    H: Fn() -> Vec<&'static ComponentDef> + Clone + 'static,
{
    let header_color = match color {
        "violet" => "text-violet-400",
        "cyan" => "text-cyan-400",
        "orange" => "text-orange-400",
        _ => "text-theme-secondary",
    };
    
    let border_color = match color {
        "violet" => "border-l-violet-500",
        "cyan" => "border-l-cyan-500",
        "orange" => "border-l-orange-500",
        _ => "border-l-theme",
    };
    
    let collapsed_for_content = collapsed.clone();
    let components_for_content = components.clone();
    
    view! {
        <section class=format!("pl-3 border-l-2 {}", border_color)>
            // Section header (clickable to collapse)
            <button
                class=format!("flex items-center justify-between w-full mb-2 group {}", header_color)
                on:click=move |_| on_toggle()
            >
                <div class="flex items-center gap-2">
                    {match icon_type {
                        "source" => view! { <SourceIcon class="w-3.5 h-3.5" /> }.into_view(),
                        "transform" => view! { <TransformIcon class="w-3.5 h-3.5" /> }.into_view(),
                        "sink" => view! { <SinkIcon class="w-3.5 h-3.5" /> }.into_view(),
                        _ => view! {}.into_view(),
                    }}
                    <h3 class="text-xs font-semibold uppercase tracking-wide">
                        {title}
                    </h3>
                    <span class="text-xs text-theme-muted font-normal">
                        {move || format!("({})", components().len())}
                    </span>
                </div>
                <ChevronIcon 
                    class="w-4 h-4 transition-transform text-theme-muted" 
                    rotated=collapsed
                />
            </button>
            
            // Section content
            {move || {
                if collapsed_for_content() {
                    view! {}.into_view()
                } else {
                    let items = components_for_content();
                    if items.is_empty() {
                        view! {
                            <div class="space-y-1">
                                <div class="text-xs text-theme-muted py-2 text-center">
                                    "No matches"
                                </div>
                            </div>
                        }.into_view()
                    } else {
                        view! {
                            <div class="space-y-1.5 max-h-64 overflow-y-auto custom-scrollbar pr-1">
                                {items.iter().map(|comp| {
                                    view! {
                                        <PaletteItem 
                                            component_type=comp.component_type
                                            label=comp.label
                                            description=comp.description
                                            category=comp.category
                                            set_recent=set_recent
                                            compact=false
                                        />
                                    }
                                }).collect_view()}
                            </div>
                        }.into_view()
                    }
                }
            }}
        </section>
    }
}

/// Draggable palette item
#[component]
fn PaletteItem(
    component_type: &'static str,
    label: &'static str,
    description: &'static str,
    category: &'static str,
    #[prop(optional)] set_recent: Option<WriteSignal<Vec<String>>>,
    #[prop(default = false)] compact: bool,
) -> impl IntoView {
    let (dragging, set_dragging) = create_signal(false);
    let (hovering, set_hovering) = create_signal(false);
    
    // Category-specific colors for visual distinction
    let bg_color = match category {
        "source" => "bg-violet-500/10 hover:bg-violet-500/20 border-violet-500/30 hover:border-violet-500/50",
        "transform" => "bg-cyan-500/10 hover:bg-cyan-500/20 border-cyan-500/30 hover:border-cyan-500/50",
        "sink" => "bg-orange-500/10 hover:bg-orange-500/20 border-orange-500/30 hover:border-orange-500/50",
        _ => "bg-theme-surface hover:bg-theme-surface-hover border-theme",
    };
    
    let icon_color = match category {
        "source" => "text-violet-400",
        "transform" => "text-cyan-400",
        "sink" => "text-orange-400",
        _ => "text-theme-muted",
    };
    
    let component_type_owned = component_type.to_string();
    
    view! {
        <div
            class=move || format!(
                "flex items-center gap-2.5 {} rounded-lg border cursor-grab transition-all duration-150 {} {}",
                if compact { "p-2" } else { "p-2.5" },
                bg_color,
                if dragging.get() { "opacity-50 cursor-grabbing scale-95" } else { "hover:shadow-md" }
            )
            draggable="true"
            on:mouseenter=move |_| set_hovering.set(true)
            on:mouseleave=move |_| set_hovering.set(false)
            on:dragstart={
                let component_type_for_drag = component_type_owned.clone();
                move |e: web_sys::DragEvent| {
                    set_dragging.set(true);
                    // Add to recently used when dragging starts
                    if let Some(set_recent) = set_recent {
                        add_to_recent(&component_type_for_drag, set_recent);
                    }
                    if let Some(data_transfer) = e.data_transfer() {
                        let _ = data_transfer.set_data(
                            "application/json",
                            &format!("\"{}\"", component_type)
                        );
                        data_transfer.set_effect_allowed("copy");
                    }
                }
            }
            on:dragend=move |_| {
                set_dragging.set(false);
            }
            title=description
        >
            // Icon
            <div class=format!("flex-shrink-0 {}", icon_color)>
                {match category {
                    "source" => view! { <SourceIcon class="w-4 h-4" /> }.into_view(),
                    "transform" => view! { <TransformIcon class="w-4 h-4" /> }.into_view(),
                    "sink" => view! { <SinkIcon class="w-4 h-4" /> }.into_view(),
                    _ => view! { <TransformIcon class="w-4 h-4" /> }.into_view(),
                }}
            </div>
            
            // Text
            <div class="flex-1 min-w-0">
                <div class="text-sm font-medium text-theme truncate">{label}</div>
                <Show when=move || hovering.get() && !compact>
                    <div class="text-xs text-theme-muted truncate mt-0.5">{description}</div>
                </Show>
            </div>
            
            // Drag indicator (visible on hover)
            <Show when=move || hovering.get()>
                <div class="flex-shrink-0 text-theme-muted">
                    <GripIcon class="w-3.5 h-3.5" />
                </div>
            </Show>
        </div>
    }
}

// Icons

#[component]
fn SearchIcon(#[prop(optional)] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" />
        </svg>
    }
}

#[component]
fn ChevronIcon<F>(
    #[prop(optional)] class: &'static str,
    rotated: F,
) -> impl IntoView 
where
    F: Fn() -> bool + 'static,
{
    view! {
        <svg 
            class=move || format!("{} {}", class, if rotated() { "-rotate-90" } else { "" })
            xmlns="http://www.w3.org/2000/svg" 
            fill="none" 
            viewBox="0 0 24 24" 
            stroke-width="1.5" 
            stroke="currentColor"
        >
            <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
        </svg>
    }
}

#[component]
fn ClockIcon(#[prop(optional)] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 6v6h4.5m4.5 0a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
        </svg>
    }
}

#[component]
fn GripIcon(#[prop(optional)] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 9h16.5m-16.5 6.75h16.5" />
        </svg>
    }
}

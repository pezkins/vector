//! Node Configuration Panel
//!
//! A slide-out panel for configuring selected pipeline nodes.
//! Features tabbed interface with Settings, Input, and Output views.

use leptos::*;
use vectorize_shared::NodeType;

use crate::components::common::*;
use crate::state::AppState;

/// Node configuration panel (right sidebar)
#[component]
pub fn ConfigPanel() -> impl IntoView {
    let app_state = expect_context::<AppState>();
    
    // Use fixed viewport height calculation for reliable scrolling
    // Header is ~56px, StatusBar is ~32px, so content area is calc(100vh - 88px)
    view! {
        <div 
            class="w-full bg-theme-surface border-l border-theme-border flex flex-col"
            style="height: calc(100vh - 88px);"
        >
            {move || {
                let selected_id = app_state.selected_node.get();
                let pipeline = app_state.pipeline.get();
                let node = selected_id.clone().and_then(|id| pipeline.nodes.get(&id).cloned());
                
                if let Some(node) = node {
                    let node_id = node.id.clone();
                    let node_name = node.name.clone();
                    let node_type_name = node.node_type.display_name().to_string();
                    let category = node.node_type.category().to_string();
                    let config_json = match &node.node_type {
                        NodeType::Source(c) => serde_json::to_string_pretty(&c.options).unwrap_or_default(),
                        NodeType::Transform(c) => serde_json::to_string_pretty(&c.options).unwrap_or_default(),
                        NodeType::Sink(c) => serde_json::to_string_pretty(&c.options).unwrap_or_default(),
                    };
                    let component_type = match &node.node_type {
                        NodeType::Source(c) => c.source_type.clone(),
                        NodeType::Transform(c) => c.transform_type.clone(),
                        NodeType::Sink(c) => c.sink_type.clone(),
                    };
                    let (icon_bg, icon_color) = match &node.node_type {
                        NodeType::Source(_) => ("bg-source/20", "text-source"),
                        NodeType::Transform(_) => ("bg-transform/20", "text-transform"),
                        NodeType::Sink(_) => ("bg-sink/20", "text-sink"),
                    };
                    
                    view! {
                        // Header - fixed height (70px with padding)
                        <div class="flex items-center justify-between p-4 border-b border-theme-border bg-theme-surface" style="flex-shrink: 0;">
                            <div class="flex items-center gap-3">
                                <div class=format!("p-2 rounded-lg {} {}", icon_bg, icon_color)>
                                    {match &node.node_type {
                                        NodeType::Source(_) => view! { <SourceIcon class="w-5 h-5" /> }.into_view(),
                                        NodeType::Transform(_) => view! { <TransformIcon class="w-5 h-5" /> }.into_view(),
                                        NodeType::Sink(_) => view! { <SinkIcon class="w-5 h-5" /> }.into_view(),
                                    }}
                                </div>
                                <div>
                                    <h3 class="font-semibold text-theme">{node_name.clone()}</h3>
                                    <p class="text-xs text-theme-muted">{node_type_name}</p>
                                </div>
                            </div>
                            <button
                                class="p-1 rounded hover:bg-theme-surface-hover text-theme-muted hover:text-theme transition-colors"
                                on:click=move |_| app_state.selected_node.set(None)
                                title="Close panel"
                            >
                                <CloseIcon class="w-5 h-5" />
                            </button>
                        </div>
                        
                        // Scrollable content - use overflow-y-auto with explicit flex-grow
                        <div 
                            class="p-4 bg-theme-surface"
                            style="flex: 1 1 0%; overflow-y: auto; min-height: 0;"
                        >
                            <SettingsTab
                                node_id=node_id.clone()
                                node_name=node_name.clone()
                                category=category.clone()
                                component_type=component_type.clone()
                                config_json=config_json.clone()
                            />
                        </div>
                    }.into_view()
                } else {
                    // No node selected - show empty placeholder
                    view! {
                        <div class="flex-1 flex items-center justify-center">
                            <div class="text-center p-6 rounded-xl bg-theme-bg/50 border border-theme-border/50">
                                <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-theme-surface-hover flex items-center justify-center">
                                    <SettingsIcon class="w-8 h-8 text-theme-muted" />
                                </div>
                                <p class="text-sm font-medium text-theme-secondary">"Select a component"</p>
                                <p class="text-xs mt-2 text-theme-muted max-w-48">"Click on a node in the canvas to view and edit its configuration"</p>
                            </div>
                        </div>
                    }.into_view()
                }
            }}
        </div>
    }
}

/// Tab options for the configuration panel
#[derive(Clone, Copy, PartialEq, Eq)]
enum ConfigTab {
    Form,
    Toml,
    Docs,
}

/// Settings tab with form-based configuration
#[component]
fn SettingsTab(
    node_id: String,
    node_name: String,
    category: String,
    component_type: String,
    config_json: String,
) -> impl IntoView {
    let app_state = expect_context::<AppState>();
    // Clone app_state early for use in different closures
    let app_state_for_inputs = app_state.clone();
    let app_state_for_apply = app_state.clone();
    
    let (success_msg, set_success_msg) = create_signal(false);
    let (active_tab, set_active_tab) = create_signal(ConfigTab::Form);
    let (current_name, set_current_name) = create_signal(node_name.clone());
    
    // Parse initial options
    let initial_options: std::collections::HashMap<String, serde_json::Value> = 
        serde_json::from_str(&config_json).unwrap_or_default();
    let (options, set_options) = create_signal(initial_options);
    
    // Store for TOML editor text (allows editing even with parse errors)
    let (toml_text, set_toml_text) = create_signal(String::new());
    let (toml_error, set_toml_error) = create_signal::<Option<String>>(None);
    
    let node_id_apply = node_id.clone();
    let component_type_for_form = component_type.clone();
    let component_type_for_docs = component_type.clone();
    
    let (deploying, set_deploying) = create_signal(false);
    let (deploy_error, set_deploy_error) = create_signal::<Option<String>>(None);
    
    let on_apply = move |_| {
        let new_options = options.get();
        let new_name = current_name.get();
        
        // Update the node in the pipeline (name and options)
        let mut pipeline = app_state_for_apply.pipeline.get();
        if let Some(node) = pipeline.nodes.get_mut(&node_id_apply) {
            // Update the name
            node.name = new_name;
            
            // Update the options
            match &mut node.node_type {
                NodeType::Source(ref mut c) => {
                    c.options = new_options.clone();
                }
                NodeType::Transform(ref mut c) => {
                    c.options = new_options.clone();
                }
                NodeType::Sink(ref mut c) => {
                    c.options = new_options;
                }
            }
        }
        app_state_for_apply.pipeline.set(pipeline);
        
        // Deploy the updated pipeline to Vector
        set_deploying.set(true);
        set_deploy_error.set(None);
        
        let app_state_deploy = app_state_for_apply.clone();
        spawn_local(async move {
            match app_state_deploy.deploy_pipeline().await {
                Ok(_) => {
                    set_success_msg.set(true);
                    set_timeout(move || set_success_msg.set(false), std::time::Duration::from_secs(2));
                    web_sys::console::log_1(&"Configuration applied and deployed!".into());
                }
                Err(e) => {
                    set_deploy_error.set(Some(format!("Deploy failed: {}", e)));
                    web_sys::console::error_1(&format!("Deploy failed: {}", e).into());
                }
            }
            set_deploying.set(false);
        });
    };
    
    let update_option_string = move |key: String, value: String| {
        set_options.update(|opts| {
            // Handle nested keys like "decoding.codec"
            if key.contains('.') {
                let parts: Vec<&str> = key.split('.').collect();
                if parts.len() == 2 {
                    let parent_key = parts[0];
                    let child_key = parts[1];
                    
                    if value.is_empty() || value == "bytes" {
                        // Remove the nested option or reset to default
                        if let Some(parent) = opts.get_mut(parent_key) {
                            if let Some(obj) = parent.as_object_mut() {
                                obj.remove(child_key);
                                if obj.is_empty() {
                                    opts.remove(parent_key);
                                }
                            }
                        }
                    } else {
                        // Set the nested option
                        let parent = opts.entry(parent_key.to_string())
                            .or_insert_with(|| serde_json::json!({}));
                        if let Some(obj) = parent.as_object_mut() {
                            obj.insert(child_key.to_string(), serde_json::Value::String(value));
                        }
                    }
                }
            } else {
                // Simple key
                if value.is_empty() {
                    opts.remove(&key);
                } else {
                    opts.insert(key, serde_json::Value::String(value));
                }
            }
        });
    };
    
    let update_option_number = move |key: String, value: String| {
        set_options.update(|opts| {
            if let Ok(n) = value.parse::<f64>() {
                opts.insert(key, serde_json::json!(n));
            } else if value.is_empty() {
                opts.remove(&key);
            }
        });
    };
    
    let update_option_bool = move |key: String, value: bool| {
        set_options.update(|opts| {
            opts.insert(key, serde_json::Value::Bool(value));
        });
    };
    
    // Get inputs for this node (for transforms/sinks)
    let node_id_for_inputs = node_id.clone();
    let is_transform_or_sink = category == "transform" || category == "sink";
    let category_for_display = category.clone();
    let category_for_inputs = category.clone();
    
    view! {
        <div class="space-y-4">
            // Component Info
            <div class="space-y-3">
                <h4 class="text-xs font-semibold text-theme-muted uppercase tracking-wider">
                    "Component Info"
                </h4>
                <div class="space-y-3">
                    // Editable Name field - updates on Apply, not immediately
                    <div class="space-y-1">
                        <label class="text-xs text-theme-muted">"Name"</label>
                        <input
                            type="text"
                            class="w-full px-3 py-2 rounded-lg bg-theme-bg border border-theme-border text-sm text-theme placeholder-theme-muted focus:outline-none focus:ring-2 focus:ring-accent focus:border-transparent"
                            prop:value=move || current_name.get()
                            on:input=move |e| {
                                let new_name = event_target_value(&e);
                                set_current_name.set(new_name);
                            }
                            placeholder="Component name"
                        />
                        <p class="text-xs text-theme-muted">"This name will be used as the component ID in Vector config"</p>
                    </div>
                    <div class="flex justify-between text-sm">
                        <span class="text-theme-muted">"Category"</span>
                        <span class="text-theme capitalize">{category_for_display}</span>
                    </div>
                    <div class="flex justify-between text-sm">
                        <span class="text-theme-muted">"Type"</span>
                        <span class="text-theme">{component_type.clone()}</span>
                    </div>
                </div>
            </div>
            
            // Inputs section (for transforms and sinks only)
            <Show when=move || is_transform_or_sink>
                {
                    let node_id_clone = node_id_for_inputs.clone();
                    let category_clone = category_for_inputs.clone();
                    let app_state_inputs = app_state_for_inputs.clone();
                    view! {
                        <div class="space-y-2">
                            <h4 class="text-xs font-semibold text-theme-muted uppercase tracking-wider">
                                "Inputs"
                            </h4>
                            <div class="rounded-lg bg-theme-bg/50 border border-theme-border p-3">
                                {move || {
                                    let pipeline = app_state_inputs.pipeline.get();
                                    let inputs = pipeline.get_inputs(&node_id_clone);
                                    let cat = category_clone.clone();
                                    if inputs.is_empty() {
                                        view! {
                                            <p class="text-xs text-theme-muted italic">
                                                "No inputs connected. Drag a connection from a source or transform to this component."
                                            </p>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <div class="space-y-1">
                                                {inputs.iter().map(|input| {
                                                    let input_name = input.clone();
                                                    view! {
                                                        <div class="flex items-center gap-2 text-sm">
                                                            <span class="w-2 h-2 rounded-full bg-success"></span>
                                                            <span class="text-theme-secondary font-mono text-xs">{input_name}</span>
                                                        </div>
                                                    }
                                                }).collect::<Vec<_>>()}
                                                <p class="text-xs text-theme-muted mt-2">
                                                    "These components will feed events into this " {if cat == "transform" { "transform" } else { "sink" }} "."
                                                </p>
                                            </div>
                                        }.into_view()
                                    }
                                }}
                            </div>
                        </div>
                    }
                }
            </Show>
            
            // Configuration Section with Tab Bar
            <div class="space-y-3">
                <h4 class="text-xs font-semibold text-theme-muted uppercase tracking-wider">
                    "Configuration"
                </h4>
                
                // Tab Bar
                <div class="flex gap-1 p-1 bg-theme-bg rounded-lg border border-theme-border">
                    <button
                        class=move || format!(
                            "flex-1 px-3 py-1.5 text-xs font-medium rounded-md transition-colors {}",
                            if active_tab.get() == ConfigTab::Form {
                                "bg-accent/10 text-accent border border-accent/30"
                            } else {
                                "text-theme-muted hover:text-theme-secondary hover:bg-theme-surface-hover border border-transparent"
                            }
                        )
                        on:click=move |_| set_active_tab.set(ConfigTab::Form)
                    >
                        "Form"
                    </button>
                    <button
                        class=move || format!(
                            "flex-1 px-3 py-1.5 text-xs font-medium rounded-md transition-colors {}",
                            if active_tab.get() == ConfigTab::Toml {
                                "bg-accent/10 text-accent border border-accent/30"
                            } else {
                                "text-theme-muted hover:text-theme-secondary hover:bg-theme-surface-hover border border-transparent"
                            }
                        )
                        on:click=move |_| {
                            // Initialize TOML text from current options when switching to TOML tab
                            let opts = options.get();
                            let toml_str = json_to_toml(&opts);
                            set_toml_text.set(toml_str);
                            set_toml_error.set(None);
                            set_active_tab.set(ConfigTab::Toml);
                        }
                    >
                        "TOML"
                    </button>
                    <button
                        class=move || format!(
                            "flex-1 px-3 py-1.5 text-xs font-medium rounded-md transition-colors {}",
                            if active_tab.get() == ConfigTab::Docs {
                                "bg-accent/10 text-accent border border-accent/30"
                            } else {
                                "text-theme-muted hover:text-theme-secondary hover:bg-theme-surface-hover border border-transparent"
                            }
                        )
                        on:click=move |_| set_active_tab.set(ConfigTab::Docs)
                    >
                        "Docs"
                    </button>
                </div>
                
                // Tab Content
                {move || match active_tab.get() {
                    ConfigTab::Form => view! {
                        <ComponentConfigForm 
                            component_type=component_type_for_form.clone()
                            options=options
                            update_string=update_option_string
                            update_number=update_option_number
                            update_bool=update_option_bool
                        />
                    }.into_view(),
                    ConfigTab::Toml => view! {
                        <div class="space-y-2">
                            <textarea
                                class="w-full h-48 rounded-lg bg-theme-bg border border-theme-border p-3 text-xs font-mono text-theme-secondary resize-none focus:outline-none focus:ring-2 focus:ring-accent focus:border-transparent"
                                prop:value=move || toml_text.get()
                                on:input=move |e| {
                                    let value = event_target_value(&e);
                                    set_toml_text.set(value.clone());
                                    // Try to parse TOML and update options
                                    match toml_to_json(&value) {
                                        Ok(parsed) => {
                                            set_options.set(parsed);
                                            set_toml_error.set(None);
                                        }
                                        Err(e) => {
                                            set_toml_error.set(Some(e));
                                        }
                                    }
                                }
                                spellcheck="false"
                            />
                            // Show TOML parse error if any
                            <Show when=move || toml_error.get().is_some()>
                                <div class="rounded-lg bg-error/10 border border-error/30 p-2 text-xs text-error">
                                    {move || toml_error.get().unwrap_or_default()}
                                </div>
                            </Show>
                        </div>
                    }.into_view(),
                    ConfigTab::Docs => {
                        let comp_type = component_type_for_docs.clone();
                        let docs_url = format!("https://vector.dev/docs/reference/configuration/{}/", 
                            if category == "source" { "sources" } 
                            else if category == "transform" { "transforms" } 
                            else { "sinks" }
                        );
                        view! {
                            <div class="rounded-lg bg-theme-bg border border-theme-border p-4 space-y-4">
                                <div class="flex items-center gap-3">
                                    <div class="w-10 h-10 rounded-lg bg-accent/10 flex items-center justify-center">
                                        <DocsIcon class="w-5 h-5 text-accent" />
                                    </div>
                                    <div>
                                        <p class="text-sm font-medium text-theme">{comp_type.clone()}</p>
                                        <p class="text-xs text-theme-muted">"Component Documentation"</p>
                                    </div>
                                </div>
                                <p class="text-xs text-theme-muted leading-relaxed">
                                    "Detailed documentation for the " <span class="text-theme-secondary font-mono">{comp_type}</span> " component is available on the Vector website."
                                </p>
                                <a 
                                    href=docs_url
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    class="inline-flex items-center gap-2 px-3 py-2 text-xs font-medium text-accent hover:text-accent-hover bg-accent/10 hover:bg-accent/20 rounded-lg transition-colors"
                                >
                                    <ExternalLinkIcon class="w-3.5 h-3.5" />
                                    "View Vector Docs"
                                </a>
                            </div>
                        }.into_view()
                    }
                }}
            </div>
    
            // Deploy error message
            <Show when=move || deploy_error.get().is_some()>
                <div class="rounded-lg bg-error/20 border border-error/50 p-3 text-xs text-error">
                    {move || deploy_error.get().unwrap_or_default()}
                </div>
            </Show>
            
            // Success message
            <Show when=move || success_msg.get()>
                <div class="rounded-lg bg-success/20 border border-success/50 p-3 text-xs text-success">
                    "Configuration applied and deployed!"
                </div>
            </Show>
            
            // Apply & Deploy button
            <button
                disabled=move || deploying.get()
                class=move || format!(
                    "w-full py-2 px-4 rounded-lg font-medium text-sm transition-colors flex items-center justify-center gap-2 {}",
                    if deploying.get() { 
                        "bg-theme-surface-hover text-theme-muted cursor-not-allowed" 
                    } else { 
                        "bg-accent hover:bg-accent-hover text-white" 
                    }
                )
                on:click=on_apply
            >
                {move || if deploying.get() {
                    view! {
                        <span class="animate-spin">
                            <SpinnerIcon class="w-4 h-4" />
                        </span>
                        "Deploying..."
                    }.into_view()
                } else {
                    view! {
                        <CheckIcon class="w-4 h-4" />
                        "Apply & Deploy"
                    }.into_view()
                }}
            </button>
        </div>
    }
}

/// Convert JSON options to TOML string
fn json_to_toml(options: &std::collections::HashMap<String, serde_json::Value>) -> String {
    // Convert JSON value to TOML value
    fn json_value_to_toml(v: &serde_json::Value) -> toml::Value {
        match v {
            serde_json::Value::Null => toml::Value::String("".to_string()),
            serde_json::Value::Bool(b) => toml::Value::Boolean(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    toml::Value::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    toml::Value::Float(f)
                } else {
                    toml::Value::String(n.to_string())
                }
            }
            serde_json::Value::String(s) => toml::Value::String(s.clone()),
            serde_json::Value::Array(arr) => {
                toml::Value::Array(arr.iter().map(json_value_to_toml).collect())
            }
            serde_json::Value::Object(obj) => {
                let mut table = toml::map::Map::new();
                for (k, v) in obj {
                    table.insert(k.clone(), json_value_to_toml(v));
                }
                toml::Value::Table(table)
            }
        }
    }
    
    let mut table = toml::map::Map::new();
    for (k, v) in options {
        table.insert(k.clone(), json_value_to_toml(v));
    }
    
    toml::to_string_pretty(&toml::Value::Table(table)).unwrap_or_default()
}

/// Parse TOML string back to JSON options
fn toml_to_json(toml_str: &str) -> Result<std::collections::HashMap<String, serde_json::Value>, String> {
    // Convert TOML value to JSON value
    fn toml_value_to_json(v: toml::Value) -> serde_json::Value {
        match v {
            toml::Value::String(s) => serde_json::Value::String(s),
            toml::Value::Integer(i) => serde_json::json!(i),
            toml::Value::Float(f) => serde_json::json!(f),
            toml::Value::Boolean(b) => serde_json::Value::Bool(b),
            toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
            toml::Value::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(toml_value_to_json).collect())
            }
            toml::Value::Table(table) => {
                let obj: serde_json::Map<String, serde_json::Value> = table
                    .into_iter()
                    .map(|(k, v)| (k, toml_value_to_json(v)))
                    .collect();
                serde_json::Value::Object(obj)
            }
        }
    }
    
    let parsed: toml::Value = toml::from_str(toml_str)
        .map_err(|e| format!("TOML parse error: {}", e))?;
    
    if let toml::Value::Table(table) = parsed {
        let result: std::collections::HashMap<String, serde_json::Value> = table
            .into_iter()
            .map(|(k, v)| (k, toml_value_to_json(v)))
            .collect();
        Ok(result)
    } else {
        Err("Expected TOML table at root".to_string())
    }
}

/// Form-based component configuration
#[component]
fn ComponentConfigForm<F, G, H>(
    component_type: String,
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
    update_number: G,
    update_bool: H,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
    G: Fn(String, String) + Clone + 'static,
    H: Fn(String, bool) + Clone + 'static,
{
    // Render appropriate form based on component type
    match component_type.as_str() {
        // ========== SOURCES ==========
        "demo_logs" => view! { <DemoLogsForm options=options update_string=update_string update_number=update_number /> }.into_view(),
        "file" => view! { <FileSourceForm options=options update_string=update_string update_bool=update_bool /> }.into_view(),
        "http_server" => view! { <HttpServerForm options=options update_string=update_string update_number=update_number /> }.into_view(),
        "socket" => view! { <SocketSourceForm options=options update_string=update_string update_number=update_number /> }.into_view(),
        "kafka" => view! { <KafkaSourceForm options=options update_string=update_string /> }.into_view(),
        "syslog" => view! { <SyslogSourceForm options=options update_string=update_string /> }.into_view(),
        "statsd" => view! { <StatsdSourceForm options=options update_string=update_string update_number=update_number /> }.into_view(),
        "nats" => view! { <NatsSourceForm options=options update_string=update_string /> }.into_view(),
        "redis" => view! { <RedisSourceForm options=options update_string=update_string /> }.into_view(),
        "docker_logs" => view! { <DockerLogsForm options=options update_string=update_string /> }.into_view(),
        "kubernetes_logs" => view! { <KubernetesLogsForm options=options update_string=update_string /> }.into_view(),
        "prometheus_scrape" => view! { <PrometheusScrapeForm options=options update_string=update_string update_number=update_number /> }.into_view(),
        "host_metrics" => view! { <HostMetricsForm options=options update_string=update_string update_number=update_number /> }.into_view(),
        "internal_logs" => view! { <InternalLogsForm options=options update_string=update_string /> }.into_view(),
        "internal_metrics" => view! { <InternalMetricsForm options=options update_string=update_string update_number=update_number /> }.into_view(),
        "opentelemetry" => view! { <OpenTelemetrySourceForm options=options update_string=update_string /> }.into_view(),
        
        // ========== TRANSFORMS ==========
        "remap" => view! { <RemapForm options=options update_string=update_string /> }.into_view(),
        "filter" => view! { <FilterForm options=options update_string=update_string /> }.into_view(),
        "sample" => view! { <SampleForm options=options update_number=update_number /> }.into_view(),
        "route" => view! { <RouteForm options=options update_string=update_string /> }.into_view(),
        "throttle" => view! { <ThrottleForm options=options update_string=update_string update_number=update_number /> }.into_view(),
        "dedupe" => view! { <DedupeForm options=options update_string=update_string update_number=update_number /> }.into_view(),
        "reduce" => view! { <ReduceForm options=options update_string=update_string update_number=update_number /> }.into_view(),
        "log_to_metric" => view! { <LogToMetricForm options=options update_string=update_string /> }.into_view(),
        "aggregate" => view! { <AggregateForm options=options update_string=update_string update_number=update_number /> }.into_view(),
        
        // ========== SINKS ==========
        "console" => view! { <ConsoleSinkForm options=options update_string=update_string /> }.into_view(),
        "file_sink" => view! { <FileSinkForm options=options update_string=update_string /> }.into_view(),
        "http" => view! { <HttpSinkForm options=options update_string=update_string /> }.into_view(),
        "elasticsearch" => view! { <ElasticsearchForm options=options update_string=update_string /> }.into_view(),
        "loki" => view! { <LokiForm options=options update_string=update_string /> }.into_view(),
        "kafka_sink" => view! { <KafkaSinkForm options=options update_string=update_string /> }.into_view(),
        "aws_s3" => view! { <S3SinkForm options=options update_string=update_string /> }.into_view(),
        "datadog_logs" => view! { <DatadogLogsForm options=options update_string=update_string /> }.into_view(),
        "datadog_metrics" => view! { <DatadogMetricsForm options=options update_string=update_string /> }.into_view(),
        "splunk_hec_logs" => view! { <SplunkHecForm options=options update_string=update_string /> }.into_view(),
        "prometheus_exporter" => view! { <PrometheusExporterForm options=options update_string=update_string /> }.into_view(),
        "prometheus_remote_write" => view! { <PrometheusRemoteWriteForm options=options update_string=update_string /> }.into_view(),
        "aws_cloudwatch_logs" => view! { <CloudWatchLogsForm options=options update_string=update_string /> }.into_view(),
        "influxdb_logs" | "influxdb_metrics" => view! { <InfluxDBLogsForm options=options update_string=update_string /> }.into_view(),
        "vector" => view! { <VectorSinkForm options=options update_string=update_string /> }.into_view(),
        "blackhole" => view! { <BlackholeForm options=options update_string=update_string update_bool=update_bool /> }.into_view(),
        "socket_sink" => view! { <SocketSinkForm options=options update_string=update_string /> }.into_view(),
        
        // Default: show generic form with JSON editor hint
        _ => view! { <GenericForm options=options update_string=update_string /> }.into_view(),
    }
}

// ============= Form Field Components =============

#[component]
fn FormField(
    label: &'static str,
    #[prop(optional)] description: &'static str,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="space-y-1">
            <label class="text-xs font-medium text-theme-secondary">{label}</label>
            {children()}
            {if !description.is_empty() {
                view! { <p class="text-xs text-theme-muted">{description}</p> }.into_view()
            } else {
                view! {}.into_view()
            }}
        </div>
    }
}

#[component]
fn TextInput<V, F>(
    value: V,
    placeholder: &'static str,
    on_change: F,
) -> impl IntoView 
where
    V: Fn() -> String + 'static,
    F: Fn(String) + 'static,
{
    view! {
        <input
            type="text"
            class="w-full px-3 py-2 text-sm rounded-lg bg-theme-bg border border-theme-border text-theme placeholder-theme-muted focus:outline-none focus:ring-2 focus:ring-accent"
            placeholder=placeholder
            prop:value=value
            on:input=move |e| on_change(event_target_value(&e))
        />
    }
}

#[component]
fn NumberInput<V, F>(
    value: V,
    placeholder: &'static str,
    on_change: F,
) -> impl IntoView 
where
    V: Fn() -> String + 'static,
    F: Fn(String) + 'static,
{
    view! {
        <input
            type="number"
            class="w-full px-3 py-2 text-sm rounded-lg bg-theme-bg border border-theme-border text-theme placeholder-theme-muted focus:outline-none focus:ring-2 focus:ring-accent"
            placeholder=placeholder
            prop:value=value
            on:input=move |e| on_change(event_target_value(&e))
        />
    }
}

#[component]
fn SelectInput<V, F>(
    value: V,
    options: Vec<(&'static str, &'static str)>,
    on_change: F,
) -> impl IntoView 
where
    V: Fn() -> String + Clone + 'static,
    F: Fn(String) + 'static,
{
    view! {
        <select
            class="w-full px-3 py-2 text-sm rounded-lg bg-theme-bg border border-theme-border text-theme focus:outline-none focus:ring-2 focus:ring-accent"
            on:change=move |e| on_change(event_target_value(&e))
        >
            {options.iter().map(|(val, label)| {
                let current = value();
                let selected = current == *val;
                view! {
                    <option value=*val selected=selected>{*label}</option>
                }
            }).collect_view()}
        </select>
    }
}

#[component]
fn TextArea<V, F>(
    value: V,
    placeholder: &'static str,
    rows: u32,
    on_change: F,
) -> impl IntoView 
where
    V: Fn() -> String + 'static,
    F: Fn(String) + 'static,
{
    view! {
        <textarea
            class="w-full px-3 py-2 text-sm rounded-lg bg-theme-bg border border-theme-border text-theme placeholder-theme-muted font-mono focus:outline-none focus:ring-2 focus:ring-accent resize-none"
            placeholder=placeholder
            rows=rows
            prop:value=value
            on:input=move |e| on_change(event_target_value(&e))
            spellcheck="false"
        />
    }
}

// ============= Component-Specific Forms =============

/// Demo Logs source configuration
#[component]
fn DemoLogsForm<F, G>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
    update_number: G,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
    G: Fn(String, String) + Clone + 'static,
{
    let update_format = update_string.clone();
    let update_interval = update_number.clone();
    let update_count = update_number.clone();
    let update_lines = update_string.clone();
    let update_sequence = update_string.clone();
    let update_decoding = update_string.clone();
    let update_framing = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Format" description="Log output format">
                <SelectInput
                    value=move || options.get().get("format").and_then(|v| v.as_str()).unwrap_or("json").to_string()
                    options=vec![
                        ("json", "JSON (fake HTTP logs)"),
                        ("apache_common", "Apache Common Log"),
                        ("apache_error", "Apache Error Log"),
                        ("syslog", "Syslog RFC 5424"),
                        ("bsd_syslog", "BSD Syslog RFC 3164"),
                        ("shuffle", "Shuffle (random from lines)")
                    ]
                    on_change=move |v| update_format("format".to_string(), v)
                />
            </FormField>
            <FormField label="Interval (seconds)" description="Time between log events (default: 1.0)">
                <NumberInput
                    value=move || options.get().get("interval").and_then(|v| v.as_f64()).map(|n| n.to_string()).unwrap_or("1.0".to_string())
                    placeholder="1.0"
                    on_change=move |v| update_interval("interval".to_string(), v)
                />
            </FormField>
            <FormField label="Count" description="Total events to generate (empty = infinite)">
                <NumberInput
                    value=move || options.get().get("count").and_then(|v| v.as_u64()).map(|n| n.to_string()).unwrap_or_default()
                    placeholder="Leave empty for infinite"
                    on_change=move |v| update_count("count".to_string(), v)
                />
            </FormField>
            <FormField label="Sequence" description="Include incrementing sequence number (shuffle format only)">
                <SelectInput
                    value=move || {
                        if options.get().get("sequence").and_then(|v| v.as_bool()).unwrap_or(false) {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        }
                    }
                    options=vec![("false", "No"), ("true", "Yes")]
                    on_change=move |v| update_sequence("sequence".to_string(), v)
                />
            </FormField>
            <FormField label="Custom Lines" description="Custom log lines (one per line, used with 'shuffle' format)">
                <TextArea
                    value=move || {
                        options.get().get("lines")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n"))
                            .unwrap_or_default()
                    }
                    placeholder="Custom log line 1\nCustom log line 2"
                    rows=3
                    on_change=move |v| update_lines("lines".to_string(), v)
                />
            </FormField>
            
            // Advanced Options section
            <div class="pt-3 border-t border-theme-border">
                <h5 class="text-xs font-semibold text-theme-muted uppercase tracking-wider mb-3">"Advanced Options"</h5>
                
                <FormField label="Decoding Codec" description="How to decode raw bytes into events">
                    <SelectInput
                        value=move || {
                            options.get()
                                .get("decoding")
                                .and_then(|v| v.as_object())
                                .and_then(|obj| obj.get("codec"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("bytes")
                                .to_string()
                        }
                        options=vec![
                            ("bytes", "Bytes (raw, default)"),
                            ("json", "JSON"),
                            ("syslog", "Syslog"),
                            ("gelf", "GELF (Graylog)"),
                            ("native", "Native (Vector protobuf)"),
                            ("native_json", "Native JSON"),
                        ]
                        on_change=move |v| update_decoding("decoding.codec".to_string(), v)
                    />
                </FormField>
                
                <FormField label="Framing Method" description="How to separate events in the byte stream">
                    <SelectInput
                        value=move || {
                            options.get()
                                .get("framing")
                                .and_then(|v| v.as_object())
                                .and_then(|obj| obj.get("method"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("bytes")
                                .to_string()
                        }
                        options=vec![
                            ("bytes", "Bytes (default)"),
                            ("newline_delimited", "Newline Delimited"),
                            ("character_delimited", "Character Delimited"),
                            ("length_delimited", "Length Delimited"),
                            ("octet_counting", "Octet Counting (RFC 6587)"),
                        ]
                        on_change=move |v| update_framing("framing.method".to_string(), v)
                    />
                </FormField>
            </div>
        </div>
    }
}

/// File source configuration
#[allow(unused_variables)]
#[component]
fn FileSourceForm<F, G>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
    update_bool: G,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
    G: Fn(String, bool) + Clone + 'static,
{
    let update_include = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Include Paths" description="File paths or globs to read (one per line)">
                <TextArea
                    value=move || {
                        options.get().get("include")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n"))
                            .unwrap_or_default()
                    }
                    placeholder="/var/log/*.log"
                    rows=3
                    on_change=move |v| update_include("include".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// HTTP Server source configuration
#[allow(unused_variables)]
#[component]
fn HttpServerForm<F, G>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
    update_number: G,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
    G: Fn(String, String) + Clone + 'static,
{
    let update_address = update_string.clone();
    let update_path = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Address" description="Address to listen on">
                <TextInput
                    value=move || options.get().get("address").and_then(|v| v.as_str()).unwrap_or("0.0.0.0:8080").to_string()
                    placeholder="0.0.0.0:8080"
                    on_change=move |v| update_address("address".to_string(), v)
                />
            </FormField>
            <FormField label="Path" description="HTTP path to accept events">
                <TextInput
                    value=move || options.get().get("path").and_then(|v| v.as_str()).unwrap_or("/").to_string()
                    placeholder="/"
                    on_change=move |v| update_path("path".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Socket source configuration
#[allow(unused_variables)]
#[component]
fn SocketSourceForm<F, G>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
    update_number: G,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
    G: Fn(String, String) + Clone + 'static,
{
    let update_mode = update_string.clone();
    let update_address = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Mode" description="Socket mode">
                <SelectInput
                    value=move || options.get().get("mode").and_then(|v| v.as_str()).unwrap_or("tcp").to_string()
                    options=vec![("tcp", "TCP"), ("udp", "UDP"), ("unix", "Unix Socket")]
                    on_change=move |v| update_mode("mode".to_string(), v)
                />
            </FormField>
            <FormField label="Address" description="Address to listen on">
                <TextInput
                    value=move || options.get().get("address").and_then(|v| v.as_str()).unwrap_or("0.0.0.0:9000").to_string()
                    placeholder="0.0.0.0:9000"
                    on_change=move |v| update_address("address".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Kafka source configuration  
#[component]
fn KafkaSourceForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_bootstrap = update_string.clone();
    let update_topics = update_string.clone();
    let update_group = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Bootstrap Servers" description="Kafka broker addresses">
                <TextInput
                    value=move || options.get().get("bootstrap_servers").and_then(|v| v.as_str()).unwrap_or("localhost:9092").to_string()
                    placeholder="localhost:9092"
                    on_change=move |v| update_bootstrap("bootstrap_servers".to_string(), v)
                />
            </FormField>
            <FormField label="Topics" description="Topics to consume (comma-separated)">
                <TextInput
                    value=move || options.get().get("topics").and_then(|v| v.as_array()).map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(",")).unwrap_or_default()
                    placeholder="logs,events"
                    on_change=move |v| update_topics("topics".to_string(), v)
                />
            </FormField>
            <FormField label="Group ID" description="Consumer group ID">
                <TextInput
                    value=move || options.get().get("group_id").and_then(|v| v.as_str()).unwrap_or("vector").to_string()
                    placeholder="vector"
                    on_change=move |v| update_group("group_id".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Syslog source configuration
#[component]
fn SyslogSourceForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_mode = update_string.clone();
    let update_address = update_string.clone();
    let update_path = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Mode" description="Connection mode">
                <SelectInput
                    value=move || options.get().get("mode").and_then(|v| v.as_str()).unwrap_or("tcp").to_string()
                    options=vec![("tcp", "TCP"), ("udp", "UDP"), ("unix", "Unix Socket")]
                    on_change=move |v| update_mode("mode".to_string(), v)
                />
            </FormField>
            <FormField label="Address" description="Address to listen on (TCP/UDP)">
                <TextInput
                    value=move || options.get().get("address").and_then(|v| v.as_str()).unwrap_or("0.0.0.0:514").to_string()
                    placeholder="0.0.0.0:514"
                    on_change=move |v| update_address("address".to_string(), v)
                />
            </FormField>
            <FormField label="Socket Path" description="Unix socket path (if mode is unix)">
                <TextInput
                    value=move || options.get().get("path").and_then(|v| v.as_str()).unwrap_or("/var/run/syslog").to_string()
                    placeholder="/var/run/syslog"
                    on_change=move |v| update_path("path".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// StatsD source configuration
#[allow(unused_variables)]
#[component]
fn StatsdSourceForm<F, G>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
    update_number: G,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
    G: Fn(String, String) + Clone + 'static,
{
    let update_mode = update_string.clone();
    let update_address = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Mode" description="Connection mode">
                <SelectInput
                    value=move || options.get().get("mode").and_then(|v| v.as_str()).unwrap_or("udp").to_string()
                    options=vec![("udp", "UDP"), ("tcp", "TCP"), ("unix", "Unix Socket")]
                    on_change=move |v| update_mode("mode".to_string(), v)
                />
            </FormField>
            <FormField label="Address" description="Address to listen on">
                <TextInput
                    value=move || options.get().get("address").and_then(|v| v.as_str()).unwrap_or("0.0.0.0:8125").to_string()
                    placeholder="0.0.0.0:8125"
                    on_change=move |v| update_address("address".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// NATS source configuration
#[component]
fn NatsSourceForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_url = update_string.clone();
    let update_subject = update_string.clone();
    let update_queue = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="URL" description="NATS server URL">
                <TextInput
                    value=move || options.get().get("url").and_then(|v| v.as_str()).unwrap_or("nats://localhost:4222").to_string()
                    placeholder="nats://localhost:4222"
                    on_change=move |v| update_url("url".to_string(), v)
                />
            </FormField>
            <FormField label="Subject" description="Subject to subscribe to">
                <TextInput
                    value=move || options.get().get("subject").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="logs.>"
                    on_change=move |v| update_subject("subject".to_string(), v)
                />
            </FormField>
            <FormField label="Queue Group" description="Queue group name (optional)">
                <TextInput
                    value=move || options.get().get("queue").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="vector"
                    on_change=move |v| update_queue("queue".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Redis source configuration
#[component]
fn RedisSourceForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_url = update_string.clone();
    let update_method = update_string.clone();
    let update_key = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="URL" description="Redis server URL">
                <TextInput
                    value=move || options.get().get("url").and_then(|v| v.as_str()).unwrap_or("redis://localhost:6379").to_string()
                    placeholder="redis://localhost:6379"
                    on_change=move |v| update_url("url".to_string(), v)
                />
            </FormField>
            <FormField label="Method" description="Data type to read">
                <SelectInput
                    value=move || options.get().get("method").and_then(|v| v.as_str()).unwrap_or("list").to_string()
                    options=vec![("list", "List (LPOP)"), ("channel", "Pub/Sub Channel")]
                    on_change=move |v| update_method("method".to_string(), v)
                />
            </FormField>
            <FormField label="Key" description="Key or channel name">
                <TextInput
                    value=move || options.get().get("key").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="logs"
                    on_change=move |v| update_key("key".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Docker Logs source configuration
#[component]
fn DockerLogsForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_host = update_string.clone();
    let update_include = update_string.clone();
    let update_exclude = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Docker Host" description="Docker daemon socket">
                <TextInput
                    value=move || options.get().get("docker_host").and_then(|v| v.as_str()).unwrap_or("unix:///var/run/docker.sock").to_string()
                    placeholder="unix:///var/run/docker.sock"
                    on_change=move |v| update_host("docker_host".to_string(), v)
                />
            </FormField>
            <FormField label="Include Containers" description="Container name patterns to include (one per line)">
                <TextArea
                    value=move || options.get().get("include_containers").and_then(|v| v.as_array()).map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n")).unwrap_or_default()
                    placeholder="my-app*\nwebserver"
                    rows=2
                    on_change=move |v| update_include("include_containers".to_string(), v)
                />
            </FormField>
            <FormField label="Exclude Containers" description="Container name patterns to exclude (one per line)">
                <TextArea
                    value=move || options.get().get("exclude_containers").and_then(|v| v.as_array()).map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n")).unwrap_or_default()
                    placeholder="vector*"
                    rows=2
                    on_change=move |v| update_exclude("exclude_containers".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Kubernetes Logs source configuration
#[component]
fn KubernetesLogsForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_exclude_ns = update_string.clone();
    let update_include_labels = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Exclude Namespaces" description="Namespaces to exclude (one per line)">
                <TextArea
                    value=move || options.get().get("exclude_namespaces").and_then(|v| v.as_array()).map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n")).unwrap_or_default()
                    placeholder="kube-system\nkube-public"
                    rows=2
                    on_change=move |v| update_exclude_ns("exclude_namespaces".to_string(), v)
                />
            </FormField>
            <FormField label="Include Labels" description="Pod labels to include (key=value, one per line)">
                <TextArea
                    value=move || options.get().get("include_labels").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="app=myapp\nenvironment=production"
                    rows=2
                    on_change=move |v| update_include_labels("include_labels".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Prometheus Scrape source configuration
#[component]
fn PrometheusScrapeForm<F, G>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
    update_number: G,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
    G: Fn(String, String) + Clone + 'static,
{
    let update_endpoints = update_string.clone();
    let update_interval = update_number.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Endpoints" description="Prometheus endpoints to scrape (one per line)">
                <TextArea
                    value=move || options.get().get("endpoints").and_then(|v| v.as_array()).map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n")).unwrap_or_default()
                    placeholder="http://localhost:9090/metrics"
                    rows=3
                    on_change=move |v| update_endpoints("endpoints".to_string(), v)
                />
            </FormField>
            <FormField label="Scrape Interval (seconds)" description="How often to scrape">
                <NumberInput
                    value=move || options.get().get("scrape_interval_secs").and_then(|v| v.as_u64()).map(|n| n.to_string()).unwrap_or("15".to_string())
                    placeholder="15"
                    on_change=move |v| update_interval("scrape_interval_secs".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Host Metrics source configuration
#[component]
fn HostMetricsForm<F, G>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
    update_number: G,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
    G: Fn(String, String) + Clone + 'static,
{
    let update_interval = update_number.clone();
    #[allow(unused_variables)]
    let update_collectors = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Scrape Interval (seconds)" description="How often to collect metrics">
                <NumberInput
                    value=move || options.get().get("scrape_interval_secs").and_then(|v| v.as_u64()).map(|n| n.to_string()).unwrap_or("15".to_string())
                    placeholder="15"
                    on_change=move |v| update_interval("scrape_interval_secs".to_string(), v)
                />
            </FormField>
            <p class="text-xs text-slate-500">"Collectors: cpu, disk, filesystem, load, host, memory, network"</p>
        </div>
    }
}

/// Internal Logs source configuration
#[allow(unused_variables)]
#[component]
fn InternalLogsForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    view! {
        <div class="space-y-3">
            <p class="text-sm text-slate-400">"Captures Vector's internal logs for self-monitoring."</p>
            <p class="text-xs text-slate-500">"No additional configuration required."</p>
        </div>
    }
}

/// Internal Metrics source configuration
#[allow(unused_variables)]
#[component]
fn InternalMetricsForm<F, G>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
    update_number: G,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
    G: Fn(String, String) + Clone + 'static,
{
    let update_interval = update_number.clone();
    
    view! {
        <div class="space-y-3">
            <p class="text-sm text-slate-400">"Captures Vector's internal metrics for self-monitoring."</p>
            <FormField label="Scrape Interval (seconds)" description="How often to collect metrics">
                <NumberInput
                    value=move || options.get().get("scrape_interval_secs").and_then(|v| v.as_u64()).map(|n| n.to_string()).unwrap_or("1".to_string())
                    placeholder="1"
                    on_change=move |v| update_interval("scrape_interval_secs".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// OpenTelemetry source configuration
#[component]
fn OpenTelemetrySourceForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_grpc_address = update_string.clone();
    let update_http_address = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="gRPC Address" description="Address for gRPC OTLP receiver">
                <TextInput
                    value=move || options.get().get("grpc").and_then(|v| v.as_object()).and_then(|o| o.get("address")).and_then(|v| v.as_str()).unwrap_or("0.0.0.0:4317").to_string()
                    placeholder="0.0.0.0:4317"
                    on_change=move |v| update_grpc_address("grpc.address".to_string(), v)
                />
            </FormField>
            <FormField label="HTTP Address" description="Address for HTTP OTLP receiver">
                <TextInput
                    value=move || options.get().get("http").and_then(|v| v.as_object()).and_then(|o| o.get("address")).and_then(|v| v.as_str()).unwrap_or("0.0.0.0:4318").to_string()
                    placeholder="0.0.0.0:4318"
                    on_change=move |v| update_http_address("http.address".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Remap transform configuration
#[component]
fn RemapForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_source = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="VRL Source" description="Vector Remap Language script">
                <TextArea
                    value=move || options.get().get("source").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder=".message = parse_json!(.message)\n.timestamp = now()"
                    rows=6
                    on_change=move |v| update_source("source".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Filter transform configuration
/// See: https://vector.dev/docs/reference/configuration/transforms/filter/
/// 
/// The filter transform filters events based on a condition.
/// Events matching the condition are forwarded, others are dropped.
#[component]
fn FilterForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_type = update_string.clone();
    let update_condition = update_string.clone();
    
    // Condition type options from Vector docs
    // https://vector.dev/docs/reference/configuration/transforms/filter/#available-syntaxes
    let condition_types = vec![
        ("vrl", "VRL (Vector Remap Language)"),
        ("datadog_search", "Datadog Search"),
        ("is_log", "Is Log Event"),
        ("is_metric", "Is Metric Event"),
        ("is_trace", "Is Trace Event"),
    ];
    
    view! {
        <div class="space-y-4">
            // Condition Type dropdown
            <FormField 
                label="Condition Type" 
                description="The type of condition syntax to use for filtering"
            >
                <SelectInput
                    value=move || {
                        let opts = options.get();
                        // Check if condition is an object with type, or just a string (VRL shorthand)
                        if let Some(cond) = opts.get("condition") {
                            if let Some(obj) = cond.as_object() {
                                obj.get("type").and_then(|v| v.as_str()).unwrap_or("vrl").to_string()
                            } else {
                                // String condition = VRL shorthand
                                "vrl".to_string()
                            }
                        } else {
                            opts.get("condition_type").and_then(|v| v.as_str()).unwrap_or("vrl").to_string()
                        }
                    }
                    options=condition_types.clone()
                    on_change=move |v| update_type("condition_type".to_string(), v)
                />
            </FormField>
            
            // Condition expression
            <FormField 
                label="Condition (required)" 
                description="Expression that returns true for events to keep. Events not matching are dropped."
            >
                <TextArea
                    value=move || {
                        let opts = options.get();
                        if let Some(cond) = opts.get("condition") {
                            if let Some(s) = cond.as_str() {
                                // VRL shorthand
                                s.to_string()
                            } else if let Some(obj) = cond.as_object() {
                                // Object with source field
                                obj.get("source").and_then(|v| v.as_str()).unwrap_or("").to_string()
                            } else {
                                "".to_string()
                            }
                        } else {
                            "".to_string()
                        }
                    }
                    placeholder="true"
                    rows=5
                    on_change=move |v| update_condition("condition".to_string(), v)
                />
            </FormField>
            
            // Information about condition types
            <div class="rounded-lg bg-slate-900/50 border border-slate-700 p-3 space-y-3">
                <p class="text-xs font-semibold text-slate-400">"Condition Type Reference:"</p>
                
                <div class="space-y-2 text-xs">
                    <div>
                        <p class="text-slate-400 font-medium">"VRL (default)"</p>
                        <p class="text-slate-500">"Vector Remap Language boolean expression"</p>
                    </div>
                    <div>
                        <p class="text-slate-400 font-medium">"Datadog Search"</p>
                        <p class="text-slate-500">"Datadog Search query string syntax"</p>
                    </div>
                    <div>
                        <p class="text-slate-400 font-medium">"is_log / is_metric / is_trace"</p>
                        <p class="text-slate-500">"Filter by event type (no condition source needed)"</p>
                    </div>
                </div>
            </div>
            
            // VRL Examples
            <div class="rounded-lg bg-slate-900/50 border border-slate-700 p-3 space-y-2">
                <p class="text-xs font-semibold text-slate-400">"VRL Condition Examples:"</p>
                <div class="text-xs font-mono space-y-2">
                    <div>
                        <p class="text-slate-500">"# Keep only error logs:"</p>
                        <p class="text-cyan-400">".level == \"error\""</p>
                    </div>
                    <div>
                        <p class="text-slate-500">"# Drop debug logs:"</p>
                        <p class="text-cyan-400">".level != \"debug\""</p>
                    </div>
                    <div>
                        <p class="text-slate-500">"# Filter by HTTP status:"</p>
                        <p class="text-cyan-400">".status_code >= 400"</p>
                    </div>
                    <div>
                        <p class="text-slate-500">"# Multiple conditions:"</p>
                        <p class="text-cyan-400">".level == \"error\" || .status_code >= 500"</p>
                    </div>
                    <div>
                        <p class="text-slate-500">"# Check field existence:"</p>
                        <p class="text-cyan-400">"exists(.user_id)"</p>
                    </div>
                    <div>
                        <p class="text-slate-500">"# String contains:"</p>
                        <p class="text-cyan-400">"contains(string!(.message), \"error\")"</p>
                    </div>
                    <div>
                        <p class="text-slate-500">"# Regex match:"</p>
                        <p class="text-cyan-400">"match(.message, r'ERROR|FATAL')"</p>
                    </div>
                    <div>
                        <p class="text-slate-500">"# Pass all events (useful for testing):"</p>
                        <p class="text-cyan-400">"true"</p>
                    </div>
                </div>
            </div>
            
            // Datadog Search Examples  
            <div class="rounded-lg bg-slate-900/50 border border-slate-700 p-3 space-y-2">
                <p class="text-xs font-semibold text-slate-400">"Datadog Search Examples:"</p>
                <div class="text-xs font-mono space-y-2">
                    <div>
                        <p class="text-slate-500">"# Filter by service:"</p>
                        <p class="text-cyan-400">"service:web"</p>
                    </div>
                    <div>
                        <p class="text-slate-500">"# Multiple conditions:"</p>
                        <p class="text-cyan-400">"service:web AND status:error"</p>
                    </div>
                    <div>
                        <p class="text-slate-500">"# Wildcard match:"</p>
                        <p class="text-cyan-400">"*stack"</p>
                    </div>
                </div>
            </div>
            
            // Link to docs
            <div class="text-xs text-slate-500">
                <a 
                    href="https://vector.dev/docs/reference/configuration/transforms/filter/" 
                    target="_blank" 
                    class="text-blue-400 hover:text-blue-300 underline"
                >
                    "View full Filter documentation →"
                </a>
            </div>
        </div>
    }
}

/// Sample transform configuration
#[component]
fn SampleForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_number: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_rate = update_number.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Rate" description="Sample 1 in N events">
                <NumberInput
                    value=move || options.get().get("rate").and_then(|v| v.as_u64()).map(|n| n.to_string()).unwrap_or("10".to_string())
                    placeholder="10"
                    on_change=move |v| update_rate("rate".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Route transform configuration
#[allow(unused_variables)]
#[component]
fn RouteForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    view! {
        <div class="space-y-3">
            <p class="text-sm text-slate-400">"Route events to different outputs based on conditions."</p>
            <div class="rounded-lg bg-slate-900/50 p-3">
                <p class="text-xs font-medium text-slate-400 mb-2">"Example Configuration (JSON):"</p>
                <pre class="text-xs text-cyan-400 font-mono overflow-x-auto">
                    {r#"{
  "route": {
    "errors": ".level == \"error\"",
    "warnings": ".level == \"warn\""
  }
}"#}
                </pre>
            </div>
            <p class="text-xs text-slate-500">"Use JSON editor for complex routing rules."</p>
        </div>
    }
}

/// Throttle transform configuration
#[component]
fn ThrottleForm<F, G>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
    update_number: G,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
    G: Fn(String, String) + Clone + 'static,
{
    let update_threshold = update_number.clone();
    let update_window = update_number.clone();
    let update_key_field = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Threshold" description="Max events per window">
                <NumberInput
                    value=move || options.get().get("threshold").and_then(|v| v.as_u64()).map(|n| n.to_string()).unwrap_or("10".to_string())
                    placeholder="10"
                    on_change=move |v| update_threshold("threshold".to_string(), v)
                />
            </FormField>
            <FormField label="Window (seconds)" description="Time window for rate limiting">
                <NumberInput
                    value=move || options.get().get("window_secs").and_then(|v| v.as_u64()).map(|n| n.to_string()).unwrap_or("1".to_string())
                    placeholder="1"
                    on_change=move |v| update_window("window_secs".to_string(), v)
                />
            </FormField>
            <FormField label="Key Field" description="Field to group events by (optional)">
                <TextInput
                    value=move || options.get().get("key_field").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder=".host"
                    on_change=move |v| update_key_field("key_field".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Dedupe transform configuration
#[component]
fn DedupeForm<F, G>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
    update_number: G,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
    G: Fn(String, String) + Clone + 'static,
{
    let update_fields = update_string.clone();
    let update_cache = update_number.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Fields" description="Fields to use for deduplication (one per line)">
                <TextArea
                    value=move || {
                        options.get().get("fields")
                            .and_then(|v| v.as_object())
                            .and_then(|o| o.get("match"))
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n"))
                            .unwrap_or_default()
                    }
                    placeholder="message\nhost"
                    rows=3
                    on_change=move |v| update_fields("fields.match".to_string(), v)
                />
            </FormField>
            <FormField label="Cache Size" description="Number of events to cache">
                <NumberInput
                    value=move || options.get().get("cache").and_then(|v| v.as_object()).and_then(|o| o.get("num_events")).and_then(|v| v.as_u64()).map(|n| n.to_string()).unwrap_or("5000".to_string())
                    placeholder="5000"
                    on_change=move |v| update_cache("cache.num_events".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Reduce transform configuration
#[allow(unused_variables)]
#[component]
fn ReduceForm<F, G>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
    update_number: G,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
    G: Fn(String, String) + Clone + 'static,
{
    let update_group_by = update_string.clone();
    let update_merge = update_string.clone();
    let update_ends_when = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Group By" description="Fields to group events by (one per line)">
                <TextArea
                    value=move || {
                        options.get().get("group_by")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n"))
                            .unwrap_or_default()
                    }
                    placeholder="host\nservice"
                    rows=2
                    on_change=move |v| update_group_by("group_by".to_string(), v)
                />
            </FormField>
            <FormField label="Ends When" description="VRL condition to close a reduce window">
                <TextArea
                    value=move || options.get().get("ends_when").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder=".complete == true"
                    rows=2
                    on_change=move |v| update_ends_when("ends_when".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Log to Metric transform configuration
#[allow(unused_variables)]
#[component]
fn LogToMetricForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    view! {
        <div class="space-y-3">
            <p class="text-sm text-slate-400">"Convert logs to metrics."</p>
            <div class="rounded-lg bg-slate-900/50 p-3">
                <p class="text-xs font-medium text-slate-400 mb-2">"Example Configuration (JSON):"</p>
                <pre class="text-xs text-cyan-400 font-mono overflow-x-auto">
                    {r#"{
  "metrics": [
    {
      "type": "counter",
      "field": "request_count",
      "name": "requests_total",
      "increment_by_value": true
    }
  ]
}"#}
                </pre>
            </div>
            <p class="text-xs text-slate-500">"Use JSON editor for metric definitions."</p>
        </div>
    }
}

/// Aggregate transform configuration
#[component]
fn AggregateForm<F, G>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
    update_number: G,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
    G: Fn(String, String) + Clone + 'static,
{
    let update_interval = update_number.clone();
    let update_mode = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Interval (seconds)" description="Aggregation window">
                <NumberInput
                    value=move || options.get().get("interval_ms").and_then(|v| v.as_u64()).map(|n| (n / 1000).to_string()).unwrap_or("10".to_string())
                    placeholder="10"
                    on_change=move |v| update_interval("interval_ms".to_string(), v)
                />
            </FormField>
            <FormField label="Mode" description="Aggregation mode">
                <SelectInput
                    value=move || options.get().get("mode").and_then(|v| v.as_str()).unwrap_or("auto").to_string()
                    options=vec![("auto", "Auto"), ("sum", "Sum"), ("mean", "Mean")]
                    on_change=move |v| update_mode("mode".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Console sink configuration
#[component]
fn ConsoleSinkForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_codec = update_string.clone();
    let update_target = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Encoding Codec" description="Output format">
                <SelectInput
                    value=move || options.get().get("encoding").and_then(|v| v.get("codec")).and_then(|v| v.as_str()).unwrap_or("json").to_string()
                    options=vec![("json", "JSON"), ("text", "Plain Text"), ("logfmt", "Logfmt")]
                    on_change=move |v| update_codec("encoding.codec".to_string(), v)
                />
            </FormField>
            <FormField label="Target" description="Output target">
                <SelectInput
                    value=move || options.get().get("target").and_then(|v| v.as_str()).unwrap_or("stdout").to_string()
                    options=vec![("stdout", "Standard Out"), ("stderr", "Standard Error")]
                    on_change=move |v| update_target("target".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// File sink configuration
#[component]
fn FileSinkForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_path = update_string.clone();
    let update_codec = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Path" description="Output file path (supports templates)">
                <TextInput
                    value=move || options.get().get("path").and_then(|v| v.as_str()).unwrap_or("/var/log/vector/%Y-%m-%d.log").to_string()
                    placeholder="/var/log/vector/%Y-%m-%d.log"
                    on_change=move |v| update_path("path".to_string(), v)
                />
            </FormField>
            <FormField label="Encoding Codec" description="Output format">
                <SelectInput
                    value=move || options.get().get("encoding").and_then(|v| v.get("codec")).and_then(|v| v.as_str()).unwrap_or("json").to_string()
                    options=vec![("json", "JSON"), ("text", "Plain Text"), ("ndjson", "NDJSON")]
                    on_change=move |v| update_codec("encoding.codec".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// HTTP sink configuration
#[component]
fn HttpSinkForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_uri = update_string.clone();
    let update_method = update_string.clone();
    let update_codec = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="URI" description="Destination URL">
                <TextInput
                    value=move || options.get().get("uri").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="https://api.example.com/logs"
                    on_change=move |v| update_uri("uri".to_string(), v)
                />
            </FormField>
            <FormField label="Method" description="HTTP method">
                <SelectInput
                    value=move || options.get().get("method").and_then(|v| v.as_str()).unwrap_or("post").to_string()
                    options=vec![("post", "POST"), ("put", "PUT")]
                    on_change=move |v| update_method("method".to_string(), v)
                />
            </FormField>
            <FormField label="Encoding Codec" description="Payload format">
                <SelectInput
                    value=move || options.get().get("encoding").and_then(|v| v.get("codec")).and_then(|v| v.as_str()).unwrap_or("json").to_string()
                    options=vec![("json", "JSON"), ("text", "Plain Text"), ("ndjson", "NDJSON")]
                    on_change=move |v| update_codec("encoding.codec".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Elasticsearch sink configuration
#[component]
fn ElasticsearchForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_endpoints = update_string.clone();
    let update_index = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Endpoints" description="Elasticsearch URLs (one per line)">
                <TextArea
                    value=move || {
                        options.get().get("endpoints")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n"))
                            .unwrap_or("http://localhost:9200".to_string())
                    }
                    placeholder="http://localhost:9200"
                    rows=2
                    on_change=move |v| update_endpoints("endpoints".to_string(), v)
                />
            </FormField>
            <FormField label="Index" description="Target index name">
                <TextInput
                    value=move || options.get().get("index").and_then(|v| v.as_str()).unwrap_or("vector-%Y.%m.%d").to_string()
                    placeholder="vector-%Y.%m.%d"
                    on_change=move |v| update_index("index".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Loki sink configuration
#[component]
fn LokiForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_endpoint = update_string.clone();
    let update_labels = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Endpoint" description="Loki API endpoint">
                <TextInput
                    value=move || options.get().get("endpoint").and_then(|v| v.as_str()).unwrap_or("http://localhost:3100").to_string()
                    placeholder="http://localhost:3100"
                    on_change=move |v| update_endpoint("endpoint".to_string(), v)
                />
            </FormField>
            <FormField label="Labels" description="Static labels (key=value, one per line)">
                <TextArea
                    value=move || options.get().get("labels").and_then(|v| v.as_object()).map(|o| o.iter().map(|(k, v)| format!("{}={}", k, v.as_str().unwrap_or(""))).collect::<Vec<_>>().join("\n")).unwrap_or_default()
                    placeholder="job=vector\napp=myapp"
                    rows=2
                    on_change=move |v| update_labels("labels".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Kafka sink configuration
#[component]
fn KafkaSinkForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_bootstrap = update_string.clone();
    let update_topic = update_string.clone();
    let update_codec = update_string.clone();
    let update_key_field = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Bootstrap Servers" description="Kafka broker addresses">
                <TextInput
                    value=move || options.get().get("bootstrap_servers").and_then(|v| v.as_str()).unwrap_or("localhost:9092").to_string()
                    placeholder="localhost:9092"
                    on_change=move |v| update_bootstrap("bootstrap_servers".to_string(), v)
                />
            </FormField>
            <FormField label="Topic" description="Kafka topic to produce to">
                <TextInput
                    value=move || options.get().get("topic").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="logs"
                    on_change=move |v| update_topic("topic".to_string(), v)
                />
            </FormField>
            <FormField label="Key Field" description="Event field to use as partition key (optional)">
                <TextInput
                    value=move || options.get().get("key_field").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder=".host"
                    on_change=move |v| update_key_field("key_field".to_string(), v)
                />
            </FormField>
            <FormField label="Encoding Codec" description="Message format">
                <SelectInput
                    value=move || options.get().get("encoding").and_then(|v| v.get("codec")).and_then(|v| v.as_str()).unwrap_or("json").to_string()
                    options=vec![("json", "JSON"), ("text", "Plain Text")]
                    on_change=move |v| update_codec("encoding.codec".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// AWS S3 sink configuration
#[component]
fn S3SinkForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_bucket = update_string.clone();
    let update_prefix = update_string.clone();
    let update_region = update_string.clone();
    let update_codec = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Bucket" description="S3 bucket name">
                <TextInput
                    value=move || options.get().get("bucket").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="my-logs-bucket"
                    on_change=move |v| update_bucket("bucket".to_string(), v)
                />
            </FormField>
            <FormField label="Key Prefix" description="S3 object key prefix">
                <TextInput
                    value=move || options.get().get("key_prefix").and_then(|v| v.as_str()).unwrap_or("logs/%Y/%m/%d/").to_string()
                    placeholder="logs/%Y/%m/%d/"
                    on_change=move |v| update_prefix("key_prefix".to_string(), v)
                />
            </FormField>
            <FormField label="Region" description="AWS region">
                <TextInput
                    value=move || options.get().get("region").and_then(|v| v.as_str()).unwrap_or("us-east-1").to_string()
                    placeholder="us-east-1"
                    on_change=move |v| update_region("region".to_string(), v)
                />
            </FormField>
            <FormField label="Encoding" description="Output format">
                <SelectInput
                    value=move || options.get().get("encoding").and_then(|v| v.get("codec")).and_then(|v| v.as_str()).unwrap_or("ndjson").to_string()
                    options=vec![("ndjson", "NDJSON"), ("json", "JSON Array"), ("text", "Plain Text")]
                    on_change=move |v| update_codec("encoding.codec".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Datadog Logs sink configuration
#[component]
fn DatadogLogsForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_api_key = update_string.clone();
    let update_site = update_string.clone();
    let update_service = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="API Key" description="Datadog API key">
                <TextInput
                    value=move || options.get().get("default_api_key").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="your-api-key"
                    on_change=move |v| update_api_key("default_api_key".to_string(), v)
                />
            </FormField>
            <FormField label="Site" description="Datadog site">
                <SelectInput
                    value=move || options.get().get("site").and_then(|v| v.as_str()).unwrap_or("datadoghq.com").to_string()
                    options=vec![
                        ("datadoghq.com", "US1 (datadoghq.com)"),
                        ("us3.datadoghq.com", "US3 (us3.datadoghq.com)"),
                        ("us5.datadoghq.com", "US5 (us5.datadoghq.com)"),
                        ("datadoghq.eu", "EU (datadoghq.eu)"),
                        ("ap1.datadoghq.com", "AP1 (ap1.datadoghq.com)"),
                    ]
                    on_change=move |v| update_site("site".to_string(), v)
                />
            </FormField>
            <FormField label="Service" description="Default service name">
                <TextInput
                    value=move || options.get().get("service").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="my-app"
                    on_change=move |v| update_service("service".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Datadog Metrics sink configuration
#[component]
fn DatadogMetricsForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_api_key = update_string.clone();
    let update_site = update_string.clone();
    let update_namespace = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="API Key" description="Datadog API key">
                <TextInput
                    value=move || options.get().get("default_api_key").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="your-api-key"
                    on_change=move |v| update_api_key("default_api_key".to_string(), v)
                />
            </FormField>
            <FormField label="Site" description="Datadog site">
                <SelectInput
                    value=move || options.get().get("site").and_then(|v| v.as_str()).unwrap_or("datadoghq.com").to_string()
                    options=vec![
                        ("datadoghq.com", "US1 (datadoghq.com)"),
                        ("us3.datadoghq.com", "US3 (us3.datadoghq.com)"),
                        ("us5.datadoghq.com", "US5 (us5.datadoghq.com)"),
                        ("datadoghq.eu", "EU (datadoghq.eu)"),
                        ("ap1.datadoghq.com", "AP1 (ap1.datadoghq.com)"),
                    ]
                    on_change=move |v| update_site("site".to_string(), v)
                />
            </FormField>
            <FormField label="Namespace" description="Metric namespace prefix">
                <TextInput
                    value=move || options.get().get("default_namespace").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="vector"
                    on_change=move |v| update_namespace("default_namespace".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Splunk HEC sink configuration
#[component]
fn SplunkHecForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_endpoint = update_string.clone();
    let update_token = update_string.clone();
    let update_index = update_string.clone();
    let update_source = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Endpoint" description="Splunk HEC endpoint URL">
                <TextInput
                    value=move || options.get().get("endpoint").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="https://splunk.example.com:8088"
                    on_change=move |v| update_endpoint("endpoint".to_string(), v)
                />
            </FormField>
            <FormField label="Token" description="HEC authentication token">
                <TextInput
                    value=move || options.get().get("default_token").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="your-hec-token"
                    on_change=move |v| update_token("default_token".to_string(), v)
                />
            </FormField>
            <FormField label="Index" description="Target Splunk index">
                <TextInput
                    value=move || options.get().get("index").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="main"
                    on_change=move |v| update_index("index".to_string(), v)
                />
            </FormField>
            <FormField label="Source" description="Event source">
                <TextInput
                    value=move || options.get().get("source").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="vector"
                    on_change=move |v| update_source("source".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Prometheus Exporter sink configuration
#[component]
fn PrometheusExporterForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_address = update_string.clone();
    let update_namespace = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Address" description="Address to expose metrics endpoint">
                <TextInput
                    value=move || options.get().get("address").and_then(|v| v.as_str()).unwrap_or("0.0.0.0:9598").to_string()
                    placeholder="0.0.0.0:9598"
                    on_change=move |v| update_address("address".to_string(), v)
                />
            </FormField>
            <FormField label="Namespace" description="Metric namespace prefix">
                <TextInput
                    value=move || options.get().get("default_namespace").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="vector"
                    on_change=move |v| update_namespace("default_namespace".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Prometheus Remote Write sink configuration
#[component]
fn PrometheusRemoteWriteForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_endpoint = update_string.clone();
    let update_namespace = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Endpoint" description="Remote write endpoint URL">
                <TextInput
                    value=move || options.get().get("endpoint").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="http://prometheus:9090/api/v1/write"
                    on_change=move |v| update_endpoint("endpoint".to_string(), v)
                />
            </FormField>
            <FormField label="Namespace" description="Metric namespace prefix">
                <TextInput
                    value=move || options.get().get("default_namespace").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="vector"
                    on_change=move |v| update_namespace("default_namespace".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// AWS CloudWatch Logs sink configuration
#[component]
fn CloudWatchLogsForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_group = update_string.clone();
    let update_stream = update_string.clone();
    let update_region = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Log Group Name" description="CloudWatch log group name">
                <TextInput
                    value=move || options.get().get("group_name").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="/vector/logs"
                    on_change=move |v| update_group("group_name".to_string(), v)
                />
            </FormField>
            <FormField label="Stream Name" description="CloudWatch log stream name (supports templates)">
                <TextInput
                    value=move || options.get().get("stream_name").and_then(|v| v.as_str()).unwrap_or("{{ host }}").to_string()
                    placeholder="{{ host }}"
                    on_change=move |v| update_stream("stream_name".to_string(), v)
                />
            </FormField>
            <FormField label="Region" description="AWS region">
                <TextInput
                    value=move || options.get().get("region").and_then(|v| v.as_str()).unwrap_or("us-east-1").to_string()
                    placeholder="us-east-1"
                    on_change=move |v| update_region("region".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// InfluxDB Logs sink configuration
#[component]
fn InfluxDBLogsForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_endpoint = update_string.clone();
    let update_org = update_string.clone();
    let update_bucket = update_string.clone();
    let update_token = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Endpoint" description="InfluxDB API endpoint">
                <TextInput
                    value=move || options.get().get("endpoint").and_then(|v| v.as_str()).unwrap_or("http://localhost:8086").to_string()
                    placeholder="http://localhost:8086"
                    on_change=move |v| update_endpoint("endpoint".to_string(), v)
                />
            </FormField>
            <FormField label="Organization" description="InfluxDB organization">
                <TextInput
                    value=move || options.get().get("org").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="my-org"
                    on_change=move |v| update_org("org".to_string(), v)
                />
            </FormField>
            <FormField label="Bucket" description="InfluxDB bucket">
                <TextInput
                    value=move || options.get().get("bucket").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="logs"
                    on_change=move |v| update_bucket("bucket".to_string(), v)
                />
            </FormField>
            <FormField label="Token" description="InfluxDB API token">
                <TextInput
                    value=move || options.get().get("token").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="your-token"
                    on_change=move |v| update_token("token".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Vector sink configuration (send to other Vector instances)
#[component]
fn VectorSinkForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_address = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Address" description="Remote Vector instance address">
                <TextInput
                    value=move || options.get().get("address").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="vector.example.com:6000"
                    on_change=move |v| update_address("address".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Blackhole sink configuration
#[allow(unused_variables)]
#[component]
fn BlackholeForm<F, G>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
    update_bool: G,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
    G: Fn(String, bool) + Clone + 'static,
{
    let update_print = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <p class="text-sm text-slate-400">"Discards all events. Useful for testing and benchmarking."</p>
            <FormField label="Print Interval" description="Events between status prints (0 = never)">
                <SelectInput
                    value=move || options.get().get("print_interval_secs").and_then(|v| v.as_u64()).map(|n| n.to_string()).unwrap_or("0".to_string())
                    options=vec![("0", "Never"), ("1", "Every second"), ("10", "Every 10 seconds")]
                    on_change=move |v| update_print("print_interval_secs".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Socket sink configuration
#[component]
fn SocketSinkForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    let update_mode = update_string.clone();
    let update_address = update_string.clone();
    let update_codec = update_string.clone();
    
    view! {
        <div class="space-y-3">
            <FormField label="Mode" description="Socket mode">
                <SelectInput
                    value=move || options.get().get("mode").and_then(|v| v.as_str()).unwrap_or("tcp").to_string()
                    options=vec![("tcp", "TCP"), ("udp", "UDP"), ("unix", "Unix Socket")]
                    on_change=move |v| update_mode("mode".to_string(), v)
                />
            </FormField>
            <FormField label="Address" description="Destination address">
                <TextInput
                    value=move || options.get().get("address").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    placeholder="127.0.0.1:9000"
                    on_change=move |v| update_address("address".to_string(), v)
                />
            </FormField>
            <FormField label="Encoding" description="Message format">
                <SelectInput
                    value=move || options.get().get("encoding").and_then(|v| v.get("codec")).and_then(|v| v.as_str()).unwrap_or("json").to_string()
                    options=vec![("json", "JSON"), ("text", "Plain Text")]
                    on_change=move |v| update_codec("encoding.codec".to_string(), v)
                />
            </FormField>
        </div>
    }
}

/// Generic form for unsupported component types
#[allow(unused_variables)]
#[component]
fn GenericForm<F>(
    options: ReadSignal<std::collections::HashMap<String, serde_json::Value>>,
    update_string: F,
) -> impl IntoView 
where
    F: Fn(String, String) + Clone + 'static,
{
    view! {
        <div class="space-y-3">
            <p class="text-xs text-slate-500">
                "Use the JSON editor for this component type."
            </p>
            <p class="text-xs text-slate-400 italic">
                "Click 'Show JSON' above to edit configuration."
            </p>
        </div>
    }
}

/// Check icon
#[component]
fn CheckIcon(#[prop(optional)] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
        </svg>
    }
}

/// Spinner icon for loading states
#[component]
fn SpinnerIcon(#[prop(optional)] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
    }
}

/// Close icon
#[component]
fn CloseIcon(#[prop(optional)] class: &'static str) -> impl IntoView {
    view! {
        <svg class=class xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
        </svg>
    }
}

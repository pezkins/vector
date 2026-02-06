//! GraphQL Subscription Client
//!
//! WebSocket-based client for Vector's GraphQL subscriptions.
//! Uses the graphql-ws protocol for real-time event streaming.

use serde::Deserialize;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{CloseEvent, ErrorEvent, MessageEvent, WebSocket};

/// Subscription client for Vector's GraphQL WebSocket API
pub struct SubscriptionClient {
    ws_url: String,
}

/// Callback type for receiving events
pub type EventCallback = Rc<dyn Fn(serde_json::Value)>;

/// Handle to control an active subscription
pub struct SubscriptionHandle {
    ws: Rc<RefCell<Option<WebSocket>>>,
}

impl SubscriptionHandle {
    /// Cancel the subscription
    pub fn cancel(&self) {
        if let Some(ws) = self.ws.borrow_mut().take() {
            // Send complete message before closing
            let complete_msg = r#"{"id":"1","type":"complete"}"#;
            let _ = ws.send_with_str(complete_msg);
            let _ = ws.close();
        }
    }
}

impl Drop for SubscriptionHandle {
    fn drop(&mut self) {
        self.cancel();
    }
}

impl SubscriptionClient {
    /// Create a new subscription client
    /// 
    /// # Arguments
    /// * `base_url` - HTTP URL of Vector API (e.g., "http://localhost:8686")
    ///   Will be converted to WebSocket URL automatically
    pub fn new(base_url: &str) -> Self {
        // Convert http:// to ws:// and https:// to wss://
        let ws_url = base_url
            .replace("http://", "ws://")
            .replace("https://", "wss://");
        
        Self { 
            ws_url: format!("{}/graphql", ws_url.trim_end_matches('/'))
        }
    }
    
    /// Subscribe to output events from specified components
    /// 
    /// # Arguments
    /// * `component_patterns` - Glob patterns for components (e.g., ["demo", "transform_*"])
    /// * `on_event` - Callback invoked for each received event
    /// 
    /// # Returns
    /// A handle that can be used to cancel the subscription
    pub fn subscribe_output_events(
        &self,
        component_patterns: Vec<String>,
        on_event: EventCallback,
    ) -> SubscriptionHandle {
        self.subscribe_events(component_patterns, None, on_event)
    }
    
    /// Subscribe to both input and output events from specified components
    /// 
    /// # Arguments
    /// * `output_patterns` - Glob patterns for output events
    /// * `input_patterns` - Optional glob patterns for input events
    /// * `on_event` - Callback invoked for each received event
    /// 
    /// # Returns
    /// A handle that can be used to cancel the subscription
    pub fn subscribe_events(
        &self,
        output_patterns: Vec<String>,
        input_patterns: Option<Vec<String>>,
        on_event: EventCallback,
    ) -> SubscriptionHandle {
        let ws_url = self.ws_url.clone();
        
        web_sys::console::log_1(&format!("Connecting to WebSocket: {}", ws_url).into());
        
        // Create WebSocket with graphql-transport-ws subprotocol
        let ws = match WebSocket::new_with_str(&ws_url, "graphql-transport-ws") {
            Ok(ws) => ws,
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to create WebSocket: {:?}", e).into());
                return SubscriptionHandle {
                    ws: Rc::new(RefCell::new(None)),
                };
            }
        };
        
        // Store WebSocket in Rc for sharing across callbacks
        let ws_rc = Rc::new(RefCell::new(Some(ws.clone())));
        let ws_for_handle = ws_rc.clone();
        
        // Build the subscription query
        let outputs_json = serde_json::to_string(&output_patterns)
            .unwrap_or_else(|_| "[\"*\"]".to_string());
        
        // Build a clean subscription query with optional inputsPatterns
        let subscribe_query = if let Some(inputs) = input_patterns {
            let inputs_json = serde_json::to_string(&inputs)
                .unwrap_or_else(|_| "[]".to_string());
            format!(
                r#"subscription {{ outputEventsByComponentIdPatterns(outputsPatterns: {}, inputsPatterns: {}, interval: 1000, limit: 10) {{ ... on Log {{ componentId componentKind message timestamp }} ... on Metric {{ componentId componentKind name timestamp value }} }} }}"#,
                outputs_json, inputs_json
            )
        } else {
            format!(
                r#"subscription {{ outputEventsByComponentIdPatterns(outputsPatterns: {}, interval: 1000, limit: 10) {{ ... on Log {{ componentId componentKind message timestamp }} ... on Metric {{ componentId componentKind name timestamp value }} }} }}"#,
                outputs_json
            )
        };
        
        // Set up open handler - send connection_init
        let ws_for_open = ws.clone();
        let subscribe_query_clone = subscribe_query.clone();
        let onopen_callback = Closure::<dyn FnMut()>::new(move || {
            web_sys::console::log_1(&"WebSocket connected, sending connection_init".into());
            
            // Send connection_init
            let init_msg = r#"{"type":"connection_init"}"#;
            if let Err(e) = ws_for_open.send_with_str(init_msg) {
                web_sys::console::error_1(&format!("Failed to send init: {:?}", e).into());
            }
        });
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();
        
        // Set up message handler
        let ws_for_msg = ws.clone();
        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                let text: String = txt.into();
                
                if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                    match server_msg.msg_type.as_str() {
                        "connection_ack" => {
                            web_sys::console::log_1(&"Received connection_ack, sending subscribe".into());
                            
                            // Build subscribe message with proper JSON serialization
                            let payload = serde_json::json!({
                                "id": "1",
                                "type": "subscribe",
                                "payload": {
                                    "query": subscribe_query_clone.replace('\n', " ").trim()
                                }
                            });
                            
                            let sub_msg = payload.to_string();
                            web_sys::console::log_1(&format!("Subscribe message: {}", &sub_msg).into());
                            
                            if let Err(e) = ws_for_msg.send_with_str(&sub_msg) {
                                web_sys::console::error_1(&format!("Failed to send subscribe: {:?}", e).into());
                            } else {
                                web_sys::console::log_1(&"Sent subscribe message".into());
                            }
                        }
                        "next" => {
                            web_sys::console::log_1(&"Received 'next' message from server".into());
                            // Extract the event data
                            if let Some(payload) = &server_msg.payload {
                                web_sys::console::log_1(&format!("Payload: {:?}", payload).into());
                                if let Some(data) = payload.get("data") {
                                    web_sys::console::log_1(&format!("Data: {:?}", data).into());
                                    if let Some(events) = data.get("outputEventsByComponentIdPatterns") {
                                        web_sys::console::log_1(&format!("Events: {:?}", events).into());
                                        if let Some(arr) = events.as_array() {
                                            web_sys::console::log_1(&format!("Processing {} events", arr.len()).into());
                                            for event in arr {
                                                on_event(event.clone());
                                            }
                                        } else {
                                            web_sys::console::log_1(&"Single event (not array)".into());
                                            on_event(events.clone());
                                        }
                                    } else {
                                        web_sys::console::log_1(&"No outputEventsByComponentIdPatterns field".into());
                                    }
                                } else {
                                    web_sys::console::log_1(&"No data field in payload".into());
                                }
                            } else {
                                web_sys::console::log_1(&"No payload in next message".into());
                            }
                        }
                        "error" => {
                            web_sys::console::error_1(&format!("GraphQL error: {:?}", server_msg.payload).into());
                        }
                        "complete" => {
                            web_sys::console::log_1(&"Subscription completed by server".into());
                            web_sys::console::log_1(&format!("Complete message payload: {:?}", server_msg.payload).into());
                        }
                        _ => {
                            // Ignore other message types
                        }
                    }
                }
            }
        });
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();
        
        // Set up error handler
        let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
            web_sys::console::error_1(&format!("WebSocket error: {}", e.message()).into());
        });
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();
        
        // Set up close handler
        let onclose_callback = Closure::<dyn FnMut(_)>::new(move |e: CloseEvent| {
            web_sys::console::log_1(&format!("WebSocket closed: code={}, reason={}", e.code(), e.reason()).into());
        });
        ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
        onclose_callback.forget();
        
        SubscriptionHandle {
            ws: ws_for_handle,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ServerMessage {
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(default)]
    #[allow(dead_code)]
    id: Option<String>,
    #[serde(default)]
    payload: Option<serde_json::Value>,
}

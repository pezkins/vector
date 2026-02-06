---
name: vectorize-live-data
description: Work with live data streaming for Vectorize UI using Vector's GraphQL subscriptions. Use when debugging real-time events, GraphQL subscriptions, WebSocket connections, data preview panel, or live event streaming.
---

# Live Data Streaming

Real-time data streaming from Vector to the Vectorize UI via GraphQL subscriptions over WebSocket.

## Implementation Status: COMPLETE

Live data streaming is fully implemented:
- `ui/src/client/subscription.rs` - WebSocket subscription client (graphql-ws protocol)
- `ui/src/components/pipeline/view.rs` - DataPreviewPanel with EventRow display
- `ui/src/client/direct.rs` - `fetch_pipeline()` loads config from Vector API

### Features Implemented
- WebSocket subscriptions with graphql-ws protocol
- HTTP-style log formatting (`METHOD /path → STATUS`)
- Status-based color coding (4xx=yellow, 5xx=red, 3xx=blue)
- Fixed-height panel (256px) with scrollable events
- Load pipeline topology from Vector API
- Auto-layout nodes left-to-right
- Input/Output split view for transforms (shows events before and after processing)
- Raw log display with word-wrap (preserves original formatting)

## Vector's GraphQL Subscription API

Vector supports GraphQL subscriptions over WebSocket at `ws://localhost:8686/graphql`.

### Subscription Query

```graphql
subscription {
  outputEvents(componentsPatterns: ["*"]) {
    ... on Log {
      message
      timestamp
      metadata
    }
    ... on Metric {
      name
      timestamp
      value {
        ... on Counter { value }
        ... on Gauge { value }
      }
    }
    ... on Trace {
      traceId
    }
  }
}
```

### Filter by Component

```graphql
subscription {
  outputEvents(componentsPatterns: ["demo", "transform_*"]) {
    ... on Log {
      message
      timestamp
    }
  }
}
```

## Key Files

| File | Purpose |
|------|---------|
| `ui/src/client/subscription.rs` | WebSocket subscription client using graphql-ws protocol |
| `ui/src/client/mod.rs` | Exports `SubscriptionClient` and `SubscriptionHandle` |
| `ui/src/components/pipeline/view.rs` | DataPreviewPanel with real-time event display |

## How It Works

### 1. Subscription Client (`subscription.rs`)

```rust
use gloo_net::websocket::{Message, WebSocket};
use futures::{SinkExt, StreamExt};

pub struct SubscriptionClient {
    ws_url: String,
}

impl SubscriptionClient {
    pub fn new(base_url: &str) -> Self {
        // Convert http:// to ws://
        let ws_url = base_url
            .replace("http://", "ws://")
            .replace("https://", "wss://");
        Self { ws_url: format!("{}/graphql", ws_url) }
    }

    pub async fn subscribe_events(
        &self,
        components: Vec<String>,
        on_event: impl Fn(serde_json::Value) + 'static,
    ) -> Result<(), JsValue> {
        let ws = WebSocket::open(&self.ws_url)?;
        let (mut write, mut read) = ws.split();

        // Send connection init (graphql-ws protocol)
        let init = serde_json::json!({
            "type": "connection_init"
        });
        write.send(Message::Text(init.to_string())).await?;

        // Send subscription
        let subscribe = serde_json::json!({
            "id": "1",
            "type": "subscribe",
            "payload": {
                "query": format!(r#"
                    subscription {{
                        outputEvents(componentsPatterns: {:?}) {{
                            ... on Log {{
                                message
                                timestamp
                            }}
                        }}
                    }}
                "#, components)
            }
        });
        write.send(Message::Text(subscribe.to_string())).await?;

        // Process incoming messages
        while let Some(msg) = read.next().await {
            if let Ok(Message::Text(text)) = msg {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if json["type"] == "next" {
                        if let Some(data) = json["payload"]["data"].as_object() {
                            on_event(serde_json::Value::Object(data.clone()));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
```

### 3. Update State Management

In `ui/src/state/mod.rs`, add subscription handle:

```rust
pub struct AppState {
    // ... existing fields ...
    
    /// Active subscription handle
    pub subscription_handle: RwSignal<Option<gloo_timers::callback::Interval>>,
    
    /// Live events buffer
    pub live_events: RwSignal<Vec<serde_json::Value>>,
}
```

### 4. Replace DataPreviewPanel

In `ui/src/components/pipeline/view.rs`, replace `generate_sample_events()` with:

```rust
fn start_subscription(
    app_state: AppState,
    component_id: String,
    set_events: WriteSignal<Vec<serde_json::Value>>,
) {
    spawn_local(async move {
        let client = SubscriptionClient::new(&app_state.url.get());
        let _ = client.subscribe_events(
            vec![component_id],
            move |event| {
                set_events.update(|events| {
                    events.push(event);
                    if events.len() > 100 {
                        events.remove(0);
                    }
                });
            },
        ).await;
    });
}
```

## Testing

1. Start Vector with demo config:
   ```bash
   ./target/release/vectorize --config config/demo.toml
   ```

2. Check GraphQL playground at `http://localhost:8686/playground`

3. Test subscription manually:
   ```graphql
   subscription {
     outputEvents(componentsPatterns: ["demo"]) {
       ... on Log { message timestamp }
     }
   }
   ```

## Key Files to Modify

| File | Changes |
|------|---------|
| `ui/Cargo.toml` | Add gloo-net websocket feature |
| `ui/src/client/mod.rs` | Export subscription module |
| `ui/src/client/subscription.rs` | New file - WebSocket client |
| `ui/src/components/pipeline/view.rs` | Replace fake data with subscription |
| `ui/src/state/mod.rs` | Add live event state |

## GraphQL-WS Protocol

Vector uses the `graphql-ws` protocol. Message flow:

```
Client                          Server
  |-- connection_init ----------->|
  |<--------- connection_ack -----|
  |-- subscribe (query) --------->|
  |<--------- next (data) --------|
  |<--------- next (data) --------|
  |-- complete ------------------>|
```

## Troubleshooting

1. **No events received**: Check `componentsPatterns` matches your source IDs
2. **WebSocket fails**: Ensure Vector API is enabled with `VECTOR_API_ENABLED=true`
3. **Browser caching**: Hard refresh after rebuilding UI (`Cmd+Shift+R`)

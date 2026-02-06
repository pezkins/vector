---
name: vectorize-project
description: Onboard to the Vectorize project - a visual pipeline builder for Vector. Use when starting work on Vectorize, asking about project structure, build process, architecture, or when the user mentions Vectorize, Vector, pipeline builder, or observability UI.
---

# Vectorize Project

Vectorize wraps [Vector](https://vector.dev) (high-performance observability data pipeline) with a visual web UI for building and managing pipelines.

## Quick Start

```bash
# Build UI (WASM) first
cd ui && npm install && npx tailwindcss -i input.css -o output.css && trunk build --release && cd ..

# Build binaries
cargo build --release -p vector -p vectorize

# Run (starts Vector + opens UI at http://localhost:8080)
./target/release/vectorize --config config/demo.toml
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Vectorize                         │
│  ┌─────────────────┐     ┌─────────────────────┐   │
│  │ vectorize-cli   │     │   vector (binary)   │   │
│  │ Axum :8080      │────►│   GraphQL API :8686 │   │
│  │ (serves UI)     │     │   (subprocess)      │   │
│  └─────────────────┘     └─────────────────────┘   │
│          │                                          │
│          ▼                                          │
│  ┌─────────────────┐                               │
│  │ Leptos UI (WASM)│                               │
│  │ (embedded)      │                               │
│  └─────────────────┘                               │
└─────────────────────────────────────────────────────┘
```

## Key Directories

| Path | Purpose |
|------|---------|
| `vectorize-cli/` | Main CLI binary - wraps Vector + serves UI |
| `ui/` | Leptos web UI (built separately with Trunk) |
| `shared/` | Shared types between UI and backend |
| `src/` | Vector source code (fork) |
| `config/` | Sample Vector configs |
| `.cursor/rules/` | Agent configuration (BACKEND.md, UI.md) |

## Key Files

| File | Purpose |
|------|---------|
| `vectorize-cli/src/main.rs` | CLI entry, starts Vector + UI server |
| `vectorize-cli/src/server.rs` | Axum server, proxies to Vector API |
| `ui/src/app.rs` | Root UI component, routing |
| `ui/src/components/pipeline/view.rs` | Main pipeline builder view + DataPreviewPanel |
| `ui/src/components/pipeline/config_panel.rs` | Component config forms |
| `ui/src/client/direct.rs` | GraphQL client + fetch_pipeline() |
| `ui/src/client/subscription.rs` | WebSocket subscription client |
| `ui/src/state/mod.rs` | Global state management |

## Current State (2026-02-02)

**Working:**
- Pipeline builder with drag-and-drop canvas
- Component palette (all Vector sources, transforms, sinks)
- Configuration deployment to Vector (RFC 541)
- Auto-connect when running inside Vectorize
- Live data streaming (WebSocket GraphQL subscriptions)
- Real-time event display with HTTP-style formatting
- Load pipeline from Vector API on startup
- Auto-layout nodes left-to-right based on topology
- Form-based component configuration (demo_logs has all options)
- Component renaming (updates on Apply & Deploy)
- Fixed-height Data Preview panel with scrolling
- **Scrollable Configuration Panel** (uses calc(100vh - 88px) for reliable height)
- Filter transform with proper condition handling (VRL, Datadog Search, is_log, etc.)
- Input/Output event split view for transforms/sinks

**Not Started:**
- VRL playground integration
- Multi-node control plane
- Comprehensive form options for all component types (only demo_logs complete)

## Development Commands

```bash
# Dev mode (hot reload UI)
cd ui && trunk serve --open

# Run Vector separately for development
./target/release/vector --config config/demo.toml

# Full rebuild
cd ui && trunk build --release && cd .. && cargo build --release -p vectorize
```

## Vector GraphQL API

Vector exposes GraphQL at `http://localhost:8686/graphql`:

- **Queries**: `components`, `health`
- **Mutations**: Config reload via `/config` endpoint (RFC 541)
- **Subscriptions**: `outputEvents` for live data streaming (WebSocket)

## Tech Stack

- **Backend**: Rust, Axum, Tokio
- **UI**: Leptos (WASM), TailwindCSS
- **Build**: Trunk (WASM), Cargo
- **Vector**: GraphQL API, TOML config

## Common Issues

1. **UI not updating**: Must rebuild BOTH UI and CLI, then hard refresh browser. See `vectorize-webui-reload` skill.
2. **Vector not starting**: Check if port 8686 is in use
3. **Build fails**: Ensure UI is built first (`trunk build --release`)
4. **CSS scrolling not working**: Don't rely on `h-full` chain. Use explicit `calc(100vh - Xpx)` heights.
5. **trunk color flag error**: Use `env -u NO_COLOR trunk build --release`

## Additional Resources

- See `.cursor/PLAN.md` for detailed project plan
- See `.cursor/rules/BACKEND.md` for backend patterns
- See `.cursor/rules/UI.md` for UI patterns
- See `.cursor/skills/vectorize-webui-reload/SKILL.md` for proper UI rebuild process
- See `.cursor/skills/vectorize-live-data/SKILL.md` for WebSocket subscription details

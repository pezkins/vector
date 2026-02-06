# Vectorize - Visual Pipeline Builder for Vector

## Project Overview

Vectorize is a fork of [Vector](https://vector.dev) that adds a visual web UI for building and managing observability data pipelines. The goal is a unified product where users can:

- Build pipelines visually with drag-and-drop
- See data flowing in real-time
- Deploy configurations with one click
- Manage single or multiple Vector nodes

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Vectorize                               │
│                                                              │
│   ┌──────────────┐         ┌──────────────────────────┐     │
│   │  vectorize   │         │      vector (binary)     │     │
│   │  (CLI/UI)    │ ──────► │      GraphQL API :8686   │     │
│   │  :8080       │         │                          │     │
│   └──────────────┘         └──────────────────────────┘     │
│         │                            │                       │
│         │ serves                     │ runs                  │
│         ▼                            ▼                       │
│   ┌──────────────┐         ┌──────────────────────────┐     │
│   │  Leptos UI   │         │   Sources/Transforms/    │     │
│   │  (WASM)      │         │   Sinks Pipeline         │     │
│   └──────────────┘         └──────────────────────────┘     │
└─────────────────────────────────────────────────────────────┘
```

## Project Structure

```
vectorize/
├── src/                    # Vector source code
├── lib/                    # Vector libraries
├── vectorize-cli/          # NEW: Vectorize CLI (wraps Vector + serves UI)
│   └── src/
│       ├── main.rs         # CLI entry point
│       ├── server.rs       # Axum server for UI + API proxy
│       └── vector_manager.rs # Starts Vector subprocess
├── ui/                     # NEW: Leptos web UI (WASM)
│   └── src/
│       ├── app.rs          # Main app component
│       ├── components/     # UI components
│       ├── client/         # Vector API client
│       └── state/          # Global state management
├── shared/                 # NEW: Shared types between UI and backend
├── vrl/                    # Forked VRL (Vector Remap Language)
└── .cursor/                # Cursor rules and plans
    ├── rules/
    │   ├── BACKEND.md      # Backend engineer agent config
    │   └── UI.md           # UI engineer agent config
    └── PLAN.md             # This file
```

## Current Status (2026-02-02)

### Completed
- [x] Repository renamed from `pezkins/vector` to `pezkins/vectorize`
- [x] Project restructured - vectorize is now the main directory
- [x] Vectorize CLI created (`vectorize-cli/`)
- [x] Leptos UI created (`ui/`) with:
  - Connection screen (auto-connects when running inside Vectorize)
  - Pipeline builder with drag-and-drop canvas
  - Component palette with all Vector sources/transforms/sinks
  - Data Preview panel with live streaming events
  - Configuration panel with form-based inputs
  - Header with navigation
  - Status bar
- [x] Shared types library (`shared/`)
- [x] UI builds successfully with Trunk
- [x] Vectorize CLI builds successfully
- [x] GitHub Actions workflow for building
- [x] Config deployment to Vector via RFC 541 API
- [x] **Live data streaming**: WebSocket subscriptions in `ui/src/client/subscription.rs`
- [x] **Load pipeline from Vector API**: UI fetches current config from Vector on startup
- [x] **Auto-layout**: Nodes are automatically positioned left-to-right based on topology
- [x] **Form-based configuration**: Component-specific forms for demo_logs, file, http_server, remap, filter, console, etc.
- [x] **Component renaming**: Editable name field in config panel (updates on Apply)
- [x] **HTTP log formatting**: Events show as `METHOD /path → STATUS` with status-based coloring
- [x] **Fixed-height Data Preview**: Panel stays at 256px with scrollable events
- [x] **Scrollable Configuration Panel**: Uses calc(100vh - 88px) for reliable scrolling
- [x] **Filter transform condition types**: VRL, Datadog Search, is_log, is_metric, is_trace
- [x] **Input/Output event display**: Transforms show both input and output events in split view
- [x] **Proper TOML generation for filter conditions**: Handles condition_type correctly

### In Progress / Next Steps
- [ ] **Comprehensive form options for all components**: Only demo_logs has all options from docs
- [ ] **Implement VRL playground**: Client-side VRL execution with WASM
- [ ] **Multi-node support**: Control plane for managing multiple Vector instances
- [ ] **Component-specific event filtering**: Currently subscribes to `*` pattern

### Known Issues
- WebSocket subscription subscribes to all events (`*` pattern) - works but could be optimized
- Component options (like format, interval) are not loaded from Vector (API doesn't expose them)
- Only demo_logs source has comprehensive form options; other components need similar treatment

### Technical Notes
1. The `ui/` WASM crate is NOT in the workspace members (intentional - built separately with Trunk)
2. Need to build UI first (`cd ui && trunk build --release`) before building vectorize-cli
3. Vector and Vectorize are two separate binaries that ship together
4. Pipeline loads from Vector's GraphQL API - no localStorage persistence

## How to Build

```bash
# 1. Build the UI (WASM)
cd ui
npm install  # First time only
npx tailwindcss -i input.css -o output.css
trunk build --release

# 2. Build Vector + Vectorize CLI
cd ..
cargo build --release -p vector -p vectorize

# 3. Run Vectorize
./target/release/vectorize
# This starts Vector subprocess + opens UI at http://localhost:8080
```

## How to Run (Development)

```bash
# Terminal 1: Run Vector with API enabled
./target/release/vector --config config/vector.yaml

# Terminal 2: Run UI dev server (hot reload)
cd ui
trunk serve --open
```

## Key Files

| File | Purpose |
|------|---------|
| `vectorize-cli/src/main.rs` | CLI entry point, starts Vector + UI server |
| `vectorize-cli/src/server.rs` | Axum server, serves UI, proxies to Vector API |
| `vectorize-cli/src/vector_manager.rs` | Starts/manages Vector subprocess |
| `ui/src/app.rs` | Root UI component, routing, auto-connect logic |
| `ui/src/components/pipeline/canvas.rs` | Drag-and-drop pipeline canvas |
| `ui/src/client/direct.rs` | Direct client for Vector's GraphQL API |
| `shared/src/config.rs` | Pipeline configuration types |
| `shared/src/messages.rs` | API message types |

## Agent Resources

### Skills (`.cursor/skills/`)

Skills help agents get up to speed quickly on specific tasks:

| Skill | Purpose |
|-------|---------|
| `vectorize-project` | Project overview, architecture, build process |
| `vectorize-live-data` | Implementing GraphQL subscriptions for live data |
| `vectorize-webui-reload` | **CRITICAL** - Proper process for rebuilding and reloading UI |

### Rules (`.cursor/rules/`)

Rules auto-apply to guide agent behavior:

| Rule | Purpose |
|------|---------|
| `BACKEND.md` | Backend/Rust patterns, async, error handling |
| `UI.md` | Leptos components, signals, WASM patterns |
| `browser-automation.mdc` | Browser MCP screenshot size limits (max 1600x900) |
| `documentation-updates.mdc` | **Always applies** - ensures docs stay current after development |
| `use-subagents.mdc` | **Always applies** - delegate specialized work to appropriate subagents |

### Subagents (`.cursor/agents/`)

Subagents are specialized agents that can be invoked for focused tasks:

| Subagent | Purpose |
|----------|---------|
| `data-streaming` | Implement GraphQL subscriptions and live data streaming |
| `ui-builder` | Build Leptos components with signals and TailwindCSS |
| `build-runner` | Build, deploy, and troubleshoot the project |
| `vector-expert` | Work with Vector's GraphQL API and config system |

**Invoke with:** "Use the data-streaming subagent to implement WebSocket subscriptions"

## GitHub Repository

- **URL**: https://github.com/pezkins/vectorize
- **Branch**: main (or current working branch)

## Notes for Next Session

1. Open Cursor in `/Users/pezkins/github/vectorize/`
2. Read the project skill: `.cursor/skills/vectorize-project/SKILL.md`
3. The workspace is Vector's workspace with vectorize-cli and shared added as members
4. UI is built separately with Trunk (not part of Cargo workspace)

### Quick Start
```bash
# Build and run (if already built)
./target/release/vectorize --config config/demo.toml

# Full rebuild
cd ui && trunk build --release && cd ..
cargo build --release -p vector -p vectorize
./target/release/vectorize --config config/demo.toml
```

### Current Priority
1. **Comprehensive form options for all components** - Read Vector docs and add all options like done for demo_logs
2. **Input events for filter not showing** - Debug why filter transform doesn't display input events
3. **VRL Playground** - Client-side VRL execution with WASM for testing transforms
4. **Multi-node support** - Control plane for managing multiple Vector instances

### URLs when running
- **UI**: http://localhost:8080
- **Vector API**: http://localhost:8686
- **GraphQL Playground**: http://localhost:8686/playground

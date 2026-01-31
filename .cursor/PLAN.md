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

## Current Status (2026-01-31)

### Completed
- [x] Repository renamed from `pezkins/vector` to `pezkins/vectorize`
- [x] Project restructured - vectorize is now the main directory
- [x] Vectorize CLI created (`vectorize-cli/`)
- [x] Leptos UI created (`ui/`) with:
  - Connection screen (auto-connects when running inside Vectorize)
  - Pipeline builder with drag-and-drop canvas
  - Component palette (sources, transforms, sinks)
  - Header with navigation
  - Status bar
- [x] Shared types library (`shared/`)
- [x] UI builds successfully with Trunk
- [x] Vectorize CLI builds successfully
- [x] GitHub Actions workflow for building

### In Progress / Next Steps
- [ ] **Build both binaries together**: `cargo build --release -p vector -p vectorize`
- [ ] **Test the full flow**: Run `vectorize` which starts Vector + UI
- [ ] **Fix any remaining compilation issues** in the workspace
- [ ] **Implement live data view**: GraphQL subscriptions for real-time events
- [ ] **Implement VRL playground**: Client-side VRL execution with WASM
- [ ] **Multi-node support**: Control plane for managing multiple Vector instances

### Known Issues to Address
1. The `ui/` WASM crate is NOT in the workspace members (intentional - built separately with Trunk)
2. Need to build UI first (`cd ui && trunk build --release`) before building vectorize-cli
3. Vector and Vectorize are two separate binaries that ship together

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

## Agent Configuration

Two agent configuration files are provided in `.cursor/rules/`:

- **BACKEND.md**: For backend/Rust development - async patterns, error handling, Vector integration
- **UI.md**: For UI development - Leptos components, signals, WASM patterns

## GitHub Repository

- **URL**: https://github.com/pezkins/vectorize
- **Branch**: main (or current working branch)

## Notes for Next Session

1. Open Cursor in `/Users/pezkins/github/vectorize/`
2. The workspace is Vector's workspace with vectorize-cli and shared added as members
3. UI is built separately with Trunk (not part of Cargo workspace)
4. To test everything:
   ```bash
   # Build UI
   cd ui && trunk build --release && cd ..
   # Build binaries
   cargo build --release -p vector -p vectorize
   # Run
   ./target/release/vectorize
   ```

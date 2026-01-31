# Vectorize

**Visual Pipeline Builder for Vector** - A unified observability tool that combines Vector with a modern web UI for building and managing data pipelines visually.

## What is Vectorize?

Vectorize wraps [Vector](https://vector.dev) (a high-performance observability data pipeline) with a visual web interface, allowing you to:

- **Build pipelines visually** - Drag and drop sources, transforms, and sinks
- **See data in real-time** - Watch events flow through your pipeline
- **Deploy with one click** - Push configurations to running Vector instances
- **Manage multiple nodes** - Control multiple Vector instances from one UI

## Quick Start

### Install

```bash
# Download the latest release
curl -sSL https://vectorize.dev/install.sh | sh

# Or build from source
cargo install --path vectorize-cli
```

### Run

```bash
# Start Vectorize (Vector + Web UI)
vectorize

# This will:
# 1. Start Vector with the API enabled
# 2. Open a web browser to the Vectorize UI
# 3. You're ready to build pipelines!
```

The UI will be available at `http://localhost:8080` and Vector's API at `http://localhost:8686`.

### Options

```bash
# Custom ports
vectorize --port 3000 --vector-api-port 9090

# Use a specific Vector config
vectorize --config /path/to/vector.toml

# Use a specific Vector binary
vectorize --vector-bin /usr/local/bin/vector

# Don't open browser automatically
vectorize start --no-open

# Pass commands directly to Vector
vectorize vector --help
vectorize vector validate --config config.toml
```

## Architecture

Vectorize is a single binary that:

1. **Manages Vector** - Starts and monitors a Vector process
2. **Serves the UI** - Embedded web application (Leptos/WASM)
3. **Proxies API calls** - Routes UI requests to Vector's GraphQL API

```
┌─────────────────────────────────────────────┐
│                 Vectorize                    │
│  ┌─────────────┐      ┌─────────────────┐   │
│  │  Web UI     │      │  Vector Process │   │
│  │  :8080      │ ──── │  :8686          │   │
│  │  (embedded) │      │  (subprocess)   │   │
│  └─────────────┘      └─────────────────┘   │
└─────────────────────────────────────────────┘
```

## Development

### Prerequisites

- Rust 1.70+
- Node.js 18+ (for TailwindCSS)
- [Trunk](https://trunkrs.dev/) for WASM builds

### Build from Source

```bash
# Clone the repo
git clone https://github.com/pezkins/vectorize.git
cd vectorize

# Build the UI (WASM)
cd ui
npm install
npx tailwindcss -i input.css -o output.css
trunk build --release
cd ..

# Build the CLI
cargo build --release -p vectorize

# Run
./target/release/vectorize
```

### Development Mode

For rapid UI development:

```bash
# Terminal 1: Run Vector
vector --config config/dev.toml

# Terminal 2: Run UI dev server (hot reload)
cd ui
trunk serve --open
```

## Project Structure

```
vectorize/
├── vectorize-cli/     # Main binary (Vector + UI wrapper)
├── ui/                # Leptos web application (WASM)
├── shared/            # Shared types between UI and backend
├── control-plane/     # Multi-node control plane (future)
├── vector/            # Forked Vector (source of truth)
└── vrl/               # Forked VRL (Vector Remap Language)
```

## Roadmap

- [x] Basic pipeline builder UI
- [x] Unified binary wrapper
- [x] Direct Vector connection
- [ ] Live data streaming view
- [ ] VRL playground integration
- [ ] Multi-node control plane
- [ ] Pipeline templates
- [ ] Visual pipeline validation

## License

MIT OR Apache-2.0 (same as Vector)

## Acknowledgments

Built on top of [Vector](https://vector.dev) by Datadog.

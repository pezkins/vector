# Vectorize

**The Visual Control Plane for Vector** - Build, manage, and deploy [Vector](https://vector.dev) data pipelines at scale.

---

## Why Vectorize?

Managing Vector configurations across multiple servers is hard. Vectorize makes it easy:

| Without Vectorize | With Vectorize |
|-------------------|----------------|
| Edit TOML files manually | Visual drag-and-drop pipeline builder |
| SSH into each server to deploy | One-click deployment to all agents |
| No version history | Git-backed config versioning with rollback |
| No visibility across fleet | Unified dashboard for all Vector instances |
| Hope configs are valid | 4-layer validation before deployment |

## Quick Start

### 1. Install

```bash
# Build from source
git clone https://github.com/pezkins/vectorize.git
cd vectorize
cargo build --release -p vectorize

# Or download binary (when available)
curl -L https://github.com/pezkins/vectorize/releases/latest/download/vectorize -o vectorize
chmod +x vectorize
```

### 2. Run

```bash
# Start Vectorize with a demo pipeline
./target/release/vectorize --config config/demo.toml

# Opens automatically at http://localhost:8080
```

### 3. First-Time Setup

1. Visit http://localhost:8080/setup
2. Create your admin account
3. Start building pipelines!

---

## Key Features

### Visual Pipeline Builder
Build Vector pipelines with a drag-and-drop canvas. Connect sources, transforms, and sinks visually - no TOML required.

### Multi-Agent Management
Manage your entire Vector fleet from one UI:
- **Agent Registry**: Auto-discover Vector instances
- **Worker Groups**: Organize by environment (prod, staging, dev)
- **Health Monitoring**: Real-time status and alerts

### Git-Based Versioning
Every config change is version controlled:
- Full commit history with diffs
- One-click rollback to any version
- Sync to GitHub/GitLab for backup and collaboration

### Advanced Deployment
Deploy with confidence:
- **Rolling deployments**: One agent at a time with health checks
- **Canary deployments**: Test on a subset before full rollout
- **Approval workflows**: Require sign-off for production changes
- **Version enforcement**: Block deployments to mixed-version fleets

### Enterprise Security
- JWT authentication with session management
- Role-based access control (25+ permissions)
- SSO support (OIDC/SAML)
- Complete audit logging

### Live Data Sampling
Sample live events from production without impact:
- Rate-limited tap API for safe sampling
- Real-time event viewer in UI
- Test transforms against live data

---

## Deployment Modes

Vectorize runs as a **single binary** in two modes:

### Single-Node Mode (Default)

Perfect for development or single-server deployments:

```bash
./vectorize --config /path/to/vector.toml
```

```
┌─────────────────────────────────────────────┐
│                 Vectorize                    │
│  ┌─────────────┐      ┌─────────────────┐   │
│  │  Web UI     │      │  Vector Process │   │
│  │  :8080      │ ──── │  :8686          │   │
│  └─────────────┘      └─────────────────┘   │
│  ┌─────────────┐      ┌─────────────────┐   │
│  │  SQLite     │      │  Git Store      │   │
│  └─────────────┘      └─────────────────┘   │
└─────────────────────────────────────────────┘
```

### Multi-Agent Mode

Scale to hundreds of Vector instances:

```bash
# Control Plane (central server)
./vectorize --port 8080

# Agents (on each Vector host - lightweight, ~10MB memory)
./vectorize agent --control-plane http://control-plane:8080 --name prod-1
./vectorize agent --control-plane http://control-plane:8080 --name prod-2
```

```
                   ┌─────────────────────────────┐
                   │   Vectorize Control Plane    │
                   │   UI + API + DB + Git Store  │
                   └─────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  Production  │    │   Staging    │    │     Dev      │
│  Agent 1-3   │    │  Agent 1-2   │    │   Agent 1    │
└──────────────┘    └──────────────┘    └──────────────┘
```

---

## CLI Reference

Full command-line interface for automation and CI/CD:

```bash
# Manage agents
vectorize agents list
vectorize agents register --name prod-1 --url http://server:8686

# Manage worker groups
vectorize groups create --name production --strategy rolling
vectorize groups agents production

# Deploy configurations
vectorize config set production --file pipeline.toml
vectorize config validate pipeline.toml
vectorize deploy create production --strategy canary

# View deployment status
vectorize deploy status <deployment-id>
vectorize deploy approve <deployment-id>
```

See `vectorize --help` for all commands.

---

## API

Full REST API for integration. See [API.md](API.md) for complete documentation.

```bash
# Example: Create a worker group
curl -X POST http://localhost:8080/api/v1/groups \
  -H "Content-Type: application/json" \
  -d '{"name":"production","deployment_strategy":"rolling"}'

# Example: Deploy to a group
curl -X POST http://localhost:8080/api/v1/groups/production/deployments \
  -H "Content-Type: application/json" \
  -d '{"strategy":"rolling","rolling_options":{"batch_size":2}}'
```

---

## Configuration Validation

Vectorize validates configurations through 4 layers before deployment:

| Layer | What it checks |
|-------|----------------|
| **1. Syntax** | Valid TOML |
| **2. Schema** | Required fields, valid types, known components |
| **3. Vector** | `vector validate` - VRL syntax, component compatibility |
| **4. Functional** | Run sample data through transforms |

```bash
# Validate via CLI
vectorize config validate my-pipeline.toml --mode full

# Validate via API
curl -X POST http://localhost:8080/api/v1/validate \
  -H "Content-Type: application/json" \
  -d '{"config":"[sources.demo]\ntype=\"demo_logs\""}'
```

---

## Development

### Prerequisites

- Rust 1.70+
- Node.js 18+ (for TailwindCSS)
- [Trunk](https://trunkrs.dev/) for WASM builds

### Build

```bash
# Build UI
cd ui && npm install && npx tailwindcss -i input.css -o output.css && trunk build --release && cd ..

# Build CLI
cargo build --release -p vectorize

# Run tests
cargo test -p vectorize
```

### Project Structure

```
vectorize/
├── vectorize-cli/     # Rust backend (API, DB, Git, deployments)
├── ui/                # Leptos frontend (WASM)
├── shared/            # Shared types
├── config/            # Example Vector configs
└── API.md             # API documentation
```

---

## Roadmap

All core phases complete:

- [x] **Phase 1**: Multi-agent management, Git config store
- [x] **Phase 2**: Config versioning, deployment, rollback
- [x] **Phase 3**: 4-layer validation, functional testing
- [x] **Phase 4**: Health monitoring, alerts, topology
- [x] **Phase 5**: RBAC (25+ permissions), SSO, audit logging
- [x] **Phase 6**: Rolling/canary deployments, approval workflows
- [x] **Phase 7**: Live data sampling with rate limiting
- [x] **Phase 8**: Full CLI mirroring API
- [x] **Phase 9**: Remote Git sync (GitHub/GitLab)

### Future
- [ ] Terraform provider
- [ ] High availability (active-passive)
- [ ] Metrics dashboards

---

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `VECTORIZE_URL` | Server URL (for CLI) | `http://localhost:8080` |
| `VECTORIZE_CONTROL_PLANE` | Control plane URL (agent mode) | - |
| `VECTORIZE_AGENT_NAME` | Agent name | hostname |
| `VECTORIZE_API_KEY` | API key for auth | - |
| `VECTORIZE_GROUP` | Worker group to join | - |
| `VECTOR_API_URL` | Local Vector API | `http://localhost:8686` |
| `VECTOR_CONFIG_PATH` | Vector config file path | `/etc/vector/vector.toml` |

---

## License

Proprietary - All rights reserved.

## Acknowledgments

Built on top of [Vector](https://vector.dev) by Datadog.

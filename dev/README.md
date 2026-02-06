# Vectorize Development Environment

This directory contains a Docker Compose setup for local development and testing of the Vectorize multi-agent control plane.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Vectorize Control Plane                       │
│                    (localhost:8080)                              │
└─────────────────────────────────────────────────────────────────┘
                              │
          ┌───────────────────┼───────────────────┐
          │                   │                   │
          ▼                   ▼                   ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  Production     │  │  Production     │  │  Production     │
│  Agent 1        │  │  Agent 2        │  │  Agent 3        │
│  :8686          │  │  :8687          │  │  :8688          │
└─────────────────┘  └─────────────────┘  └─────────────────┘

┌─────────────────┐  ┌─────────────────┐
│  Staging        │  │  Staging        │
│  Agent 1        │  │  Agent 2        │
│  :8689          │  │  :8690          │
└─────────────────┘  └─────────────────┘
```

## Quick Start

```bash
# Start all agents
./scripts/start-dev.sh

# Stop all agents
./scripts/stop-dev.sh
```

## Agent Details

| Agent | Port | Group | Config |
|-------|------|-------|--------|
| vector-prod-1 | 8686 | production | JSON logs + parse + filter |
| vector-prod-2 | 8687 | production | JSON logs + parse + filter |
| vector-prod-3 | 8688 | production | JSON logs + parse + filter |
| vector-staging-1 | 8689 | staging | Syslog logs + metadata |
| vector-staging-2 | 8690 | staging | Syslog logs + metadata |

## Useful Commands

```bash
# View logs from all agents
docker-compose logs -f

# View logs from specific agent
docker-compose logs -f vector-prod-1

# Scale production group
docker-compose up -d --scale vector-prod-1=5

# Restart an agent
docker-compose restart vector-prod-1

# Check agent health
curl http://localhost:8686/health

# Query agent topology via GraphQL
curl http://localhost:8686/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ sources { nodes { componentId componentType } } }"}'

# Reset everything (removes volumes)
docker-compose down -v
```

## Configuration

Configs are mounted read-only from:
- `configs/production/vector.toml` - Production group config
- `configs/staging/vector.toml` - Staging group config

To test config updates:
1. Modify the config file
2. Vector will auto-reload (using `--watch-config`)

## Network

All containers are on the `vectorize-net` bridge network.

From within containers, agents can reach each other using container names:
- `http://vector-prod-1:8686`
- `http://vector-staging-1:8686`
- etc.

## Testing Scenarios

1. **Basic Registration**: Start agents, register them with Vectorize
2. **Health Monitoring**: Stop an agent, verify Vectorize detects it
3. **Config Deployment**: Deploy new config, verify all agents update
4. **Rolling Deployment**: Deploy with rolling strategy, watch order
5. **Agent Failure**: Kill agent mid-deployment, test recovery

# Vectorize API Reference

Complete REST API documentation for Vectorize control plane.

**Base URL**: `http://localhost:8080/api/v1`

## Authentication

Most endpoints require JWT authentication. Include the token in the `Authorization` header:

```
Authorization: Bearer <jwt_token>
```

### Setup (First-Time Only)

```bash
# Check if setup is needed
GET /setup/status
# Response: { "needs_setup": true/false, "has_admin": false }

# Create admin account
POST /setup/init
Content-Type: application/json
{
  "username": "admin",
  "email": "admin@example.com",
  "password": "your_password"
}
```

### Login

```bash
POST /auth/login
Content-Type: application/json
{
  "identifier": "admin",  # username or email
  "password": "your_password"
}
# Response: { "token": "jwt_token", "user": {...} }

POST /auth/logout  # Invalidates current session
GET /auth/me       # Get current user info
```

---

## Agent Management

### List Agents

```bash
GET /agents
# Response: [{ "id": "...", "name": "...", "url": "...", "status": "healthy" }, ...]
```

### Get Agent

```bash
GET /agents/:id
# Response: { "id": "...", "name": "...", "url": "...", "group_id": "...", "vector_version": "..." }
```

### Register Agent

```bash
POST /agents
Content-Type: application/json
{
  "name": "prod-agent-1",
  "url": "http://192.168.1.10:8686",
  "group_id": "optional-group-id"
}
```

### Update Agent

```bash
PUT /agents/:id
Content-Type: application/json
{
  "name": "new-name",
  "group_id": "new-group-id"
}
```

### Delete Agent

```bash
DELETE /agents/:id
```

---

## Worker Groups

### List Groups

```bash
GET /groups
# Response: [{ "id": "...", "name": "...", "deployment_strategy": "rolling", "agent_count": 3 }, ...]
```

### Get Group

```bash
GET /groups/:id
```

### Create Group

```bash
POST /groups
Content-Type: application/json
{
  "name": "production",
  "description": "Production Vector instances",
  "deployment_strategy": "rolling",  # basic, rolling, canary
  "requires_approval": true,
  "approvers": ["user1@example.com"]
}
```

### Update Group

```bash
PUT /groups/:id
Content-Type: application/json
{
  "name": "new-name",
  "deployment_strategy": "canary",
  "requires_approval": false
}
```

### Delete Group

```bash
DELETE /groups/:id
```

### List Agents in Group

```bash
GET /groups/:id/agents
```

---

## Configuration Management

### Get Current Config

```bash
GET /groups/:id/config
# Response: { "config": "...", "version": "abc123...", "group_name": "..." }
```

### Get Config at Version

```bash
GET /groups/:id/config/:version
```

### Update Config

```bash
PUT /groups/:id/config
Content-Type: application/json
{
  "config": "[sources.demo]\ntype = \"demo_logs\"\n..."
}
# Response: { "success": true, "version": "new_commit_hash" }
```

### Get Config History

```bash
GET /groups/:id/history?limit=50
# Response: [{ "hash": "...", "message": "...", "timestamp": "..." }, ...]
```

### Rollback Config

```bash
POST /groups/:id/rollback
Content-Type: application/json
{
  "version": "commit_hash_to_rollback_to"
}
```

### Get Config Diff

```bash
GET /groups/:id/diff?from=hash1&to=hash2
# Response: { "diff": "...", "has_changes": true }
```

---

## Deployments

### Create Deployment

```bash
POST /groups/:id/deployments
Content-Type: application/json
{
  "config_version": "optional_version",  # defaults to current
  "force": false,  # ignore version mismatch
  "rolling_options": {
    "batch_size": 2,
    "batch_delay_secs": 30,
    "pause_on_failure": true,
    "max_failures": 1
  },
  "canary_options": {
    "canary_percentage": 10,
    "canary_wait_secs": 300,
    "auto_promote": false
  }
}
# Response: { "deployment_id": "...", "status": "pending_approval", "requires_approval": true }
```

### Get Deployment Status

```bash
GET /deployments/:id
# Response: {
#   "id": "...",
#   "status": "in_progress",
#   "stats": { "total": 5, "completed": 2, "failed": 0, "in_progress": 1, "pending": 2 },
#   "agents": [...]
# }
```

### List Deployments

```bash
GET /groups/:id/deployments?limit=50
```

### Approve Deployment

```bash
POST /deployments/:id/approve
Content-Type: application/json
{
  "approved_by": "admin"
}
```

### Reject Deployment

```bash
POST /deployments/:id/reject
Content-Type: application/json
{
  "rejected_by": "admin",
  "reason": "Config issue found"
}
```

### Cancel Deployment

```bash
POST /deployments/:id/cancel
```

### Check Version Consistency

```bash
GET /groups/:id/versions
# Response: { "consistent": true, "versions": [{ "version": "0.54.0", "agents": [...] }] }
```

---

## Validation

### Quick Validation (Syntax Only)

```bash
POST /validate/quick
Content-Type: application/json
{
  "config": "[sources.demo]\ntype = \"demo_logs\"\n..."
}
# Response: { "valid": true, "errors": [], "warnings": [] }
```

### Full Validation (Syntax + Schema + Vector)

```bash
POST /validate
Content-Type: application/json
{
  "config": "[sources.demo]\ntype = \"demo_logs\"\n..."
}
```

---

## Functional Testing (Layer 4)

### Start Functional Test

Run sample data through a configuration to verify transforms work correctly.

```bash
POST /test
Content-Type: application/json
{
  "config": "[sources.demo]\ntype = \"stdin\"\n...",
  "sample_events": [
    {"message": "test event 1", "level": "info"},
    {"message": "error occurred", "level": "error"}
  ],
  "source_id": "demo",  # optional
  "timeout_secs": 30    # optional, default 30
}
# Response: { "test_id": "uuid", "status": "running", "message": "..." }
```

### Get Test Results

```bash
GET /test/:id
# Response: {
#   "test_id": "...",
#   "status": "completed",
#   "input_events": 2,
#   "output_events": [...],
#   "output_count": 1,
#   "dropped_count": 1,
#   "duration_ms": 1234,
#   "errors": [],
#   "started_at": "...",
#   "completed_at": "..."
# }
```

### List Recent Tests

```bash
GET /test
# Response: [{ "test_id": "...", "status": "...", ... }, ...]
```

---

## Health & Monitoring

### Health Check All Agents

```bash
GET /health/agents
# Response: [{ "agent_id": "...", "healthy": true, "latency_ms": 45 }, ...]
```

### Get Agent Health History

```bash
GET /health/agents/:id/history?limit=100
```

### Get Aggregated Metrics

```bash
GET /metrics
```

### Get Aggregated Topology

```bash
GET /topology
```

---

## Alerts

### List Alert Rules

```bash
GET /alerts/rules
```

### Create Alert Rule

```bash
POST /alerts/rules
Content-Type: application/json
{
  "name": "Agent Down",
  "condition": {
    "type": "agent_unhealthy",
    "consecutive_failures": 3
  },
  "channels": ["channel-id-1"],
  "enabled": true
}
```

### List Notification Channels

```bash
GET /alerts/channels
```

### Create Notification Channel

```bash
POST /alerts/channels
Content-Type: application/json
{
  "name": "Slack Alerts",
  "type": "slack",
  "config": {
    "webhook_url": "https://hooks.slack.com/..."
  }
}
```

### Test Notification Channel

```bash
POST /alerts/channels/:id/test
```

---

## User Management (Requires RBAC)

### List Users

```bash
GET /users
```

### Create User

```bash
POST /users
Content-Type: application/json
{
  "username": "newuser",
  "email": "user@example.com",
  "password": "password",
  "role_id": "role-id"
}
```

### Update User

```bash
PUT /users/:id
```

### Delete User

```bash
DELETE /users/:id
```

---

## Role Management (Requires RBAC)

### List Roles

```bash
GET /roles
```

### List All Permissions

```bash
GET /roles/permissions
```

### Create Role

```bash
POST /roles
Content-Type: application/json
{
  "name": "deployer",
  "description": "Can deploy configs",
  "permissions": ["groups:read", "groups:deploy", "configs:read"]
}
```

---

## Audit Logging

### Query Audit Logs

```bash
GET /audit?actor_id=user-id&action=deployment.create&limit=100
```

### List Audit Actions

```bash
GET /audit/actions
```

---

## Live Data Sampling (Tap)

### Get Tap Configuration

```bash
GET /tap/config
# Response: { "rate_limiting": {...}, "websocket_enabled": true }
```

### Sample from Agent

```bash
GET /tap/:agent_id/sample?patterns=*&limit=10
```

### Check Rate Limit

```bash
GET /tap/:agent_id/rate-limit
# Response: { "can_sample": true, "config": {...} }
```

### Get WebSocket Info

```bash
GET /tap/:agent_id/ws-info
# Response: { "websocket_url": "ws://localhost:8686/graphql", "protocol": "graphql-transport-ws" }
```

---

## Git Remote Sync

### List Remotes

```bash
GET /git/remotes
```

### Configure Remote

```bash
POST /git/remotes
Content-Type: application/json
{
  "name": "origin",
  "url": "git@github.com:org/configs.git"
}
```

### Delete Remote

```bash
DELETE /git/remotes/:name
```

### Push to Remote

```bash
POST /git/remotes/:name/push
Content-Type: application/json
{
  "branch": "main"
}
```

### Pull from Remote

```bash
POST /git/remotes/:name/pull
Content-Type: application/json
{
  "branch": "main"
}
```

### Sync with Remote

```bash
POST /git/remotes/:name/sync
Content-Type: application/json
{
  "branch": "main"
}
```

### Get Sync Status

```bash
GET /git/remotes/:name/status
# Response: { "ahead": 2, "behind": 0, "synced": false }
```

### List Branches

```bash
GET /git/branches
# Response: { "current": "main", "branches": [...] }
```

### Create Branch

```bash
POST /git/branches
Content-Type: application/json
{
  "name": "feature-branch"
}
```

### Checkout Branch

```bash
POST /git/branches/:name/checkout
```

---

## Error Responses

All error responses follow this format:

```json
{
  "error": "Description of the error"
}
```

Common HTTP status codes:
- `400` - Bad Request (validation error, invalid input)
- `401` - Unauthorized (missing or invalid token)
- `403` - Forbidden (insufficient permissions)
- `404` - Not Found
- `409` - Conflict (e.g., duplicate name)
- `429` - Too Many Requests (rate limited)
- `500` - Internal Server Error

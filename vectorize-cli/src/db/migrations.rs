//! Database migrations for Vectorize
//!
//! Migrations are run in order on startup. Each migration is idempotent.

use sqlx::SqlitePool;
use tracing::info;

/// Run all migrations in order
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Create migrations tracking table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS _migrations (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#
    )
    .execute(pool)
    .await?;
    
    // Define migrations
    let migrations: Vec<(&str, &str)> = vec![
        ("001_initial_schema", MIGRATION_001_INITIAL_SCHEMA),
        ("002_audit_log", MIGRATION_002_AUDIT_LOG),
        ("003_worker_groups", MIGRATION_003_WORKER_GROUPS),
        ("004_deployments", MIGRATION_004_DEPLOYMENTS),
    ];
    
    // Run each migration if not already applied
    for (name, sql) in migrations {
        let applied: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM _migrations WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(pool)
        .await?;
        
        if applied.is_none() {
            info!("Applying migration: {}", name);
            
            // Execute the migration SQL
            // Split by semicolons and execute each statement
            for statement in sql.split(';').filter(|s| !s.trim().is_empty()) {
                sqlx::query(statement)
                    .execute(pool)
                    .await?;
            }
            
            // Record the migration
            sqlx::query("INSERT INTO _migrations (name) VALUES (?)")
                .bind(name)
                .execute(pool)
                .await?;
            
            info!("Migration {} applied successfully", name);
        }
    }
    
    Ok(())
}

/// Migration 001: Initial schema
/// Creates core tables: agents, health_checks, users, roles, api_keys, sessions
const MIGRATION_001_INITIAL_SCHEMA: &str = r#"
-- Roles table (for RBAC)
CREATE TABLE IF NOT EXISTS roles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    permissions TEXT NOT NULL DEFAULT '[]',  -- JSON array of permission strings
    is_builtin INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT
);

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE,
    email TEXT UNIQUE,
    password_hash TEXT,                       -- NULL for SSO users
    sso_provider TEXT,                        -- 'local', 'oidc', 'saml'
    sso_subject TEXT,                         -- External user ID from SSO
    role_id TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT,
    last_login TEXT,
    FOREIGN KEY (role_id) REFERENCES roles(id)
);

-- API keys table
CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    key_hash TEXT NOT NULL,                   -- Only store hash
    key_prefix TEXT NOT NULL,                 -- First 8 chars for identification
    user_id TEXT,                             -- Optional: associated user
    permissions TEXT,                         -- Optional: override permissions (JSON)
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT,
    last_used TEXT,
    revoked_at TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- Sessions table (for JWT refresh)
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    refresh_token_hash TEXT NOT NULL,
    user_agent TEXT,
    ip_address TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL,
    revoked_at TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- Agents table
CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,                -- Name is the unique identifier
    url TEXT NOT NULL,                        -- URL can be duplicated (containers share internal URL)
    group_id TEXT,
    status TEXT NOT NULL DEFAULT 'unknown',   -- healthy, unhealthy, unreachable, unknown
    vector_version TEXT,
    last_seen TEXT,
    registered_at TEXT NOT NULL DEFAULT (datetime('now')),
    metadata TEXT                             -- JSON for extra data
);

-- Health checks history
CREATE TABLE IF NOT EXISTS health_checks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    healthy INTEGER NOT NULL,
    latency_ms INTEGER,
    error TEXT,
    checked_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_agents_group ON agents(group_id);
CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);
CREATE INDEX IF NOT EXISTS idx_health_checks_agent ON health_checks(agent_id);
CREATE INDEX IF NOT EXISTS idx_health_checks_time ON health_checks(checked_at);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_api_keys_prefix ON api_keys(key_prefix);
CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id)
"#;

/// Migration 002: Audit log table
const MIGRATION_002_AUDIT_LOG: &str = r#"
-- Audit log table
CREATE TABLE IF NOT EXISTS audit_log (
    id TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    actor_type TEXT NOT NULL,                 -- 'user', 'api_key', 'system'
    actor_id TEXT,                            -- user_id or api_key_id
    actor_name TEXT,                          -- username or key name (for display)
    action TEXT NOT NULL,                     -- e.g., 'config.deploy', 'user.create'
    resource_type TEXT,                       -- e.g., 'group', 'agent', 'user'
    resource_id TEXT,
    details TEXT,                             -- JSON with action-specific details
    ip_address TEXT,
    user_agent TEXT,
    result TEXT NOT NULL DEFAULT 'success'    -- 'success', 'failure', 'denied'
);

-- Indexes for audit log queries
CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_actor ON audit_log(actor_id);
CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_log(action);
CREATE INDEX IF NOT EXISTS idx_audit_resource ON audit_log(resource_type, resource_id)
"#;

/// Migration 003: Worker groups table
const MIGRATION_003_WORKER_GROUPS: &str = r#"
-- Worker groups table
CREATE TABLE IF NOT EXISTS worker_groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    deployment_strategy TEXT NOT NULL DEFAULT 'rolling',  -- rolling, canary, blue_green, all_at_once
    requires_approval INTEGER NOT NULL DEFAULT 0,
    approvers TEXT,                           -- JSON array of user IDs/emails
    config_path TEXT,                         -- Path in git repo
    current_config_version TEXT,              -- Git commit hash
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT,
    created_by TEXT,
    FOREIGN KEY (created_by) REFERENCES users(id)
);

-- Update agents table to reference worker_groups
-- (already has group_id column from migration 001)

-- Create index
CREATE INDEX IF NOT EXISTS idx_groups_name ON worker_groups(name)
"#;

/// Migration 004: Deployments table
const MIGRATION_004_DEPLOYMENTS: &str = r#"
-- Deployments table
CREATE TABLE IF NOT EXISTS deployments (
    id TEXT PRIMARY KEY,
    group_id TEXT NOT NULL,
    config_version TEXT NOT NULL,             -- Git commit hash
    strategy TEXT NOT NULL,                   -- basic, rolling, canary
    status TEXT NOT NULL DEFAULT 'pending',   -- pending, pending_approval, in_progress, completed, failed, cancelled
    options TEXT,                             -- JSON with strategy options
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    started_at TEXT,
    completed_at TEXT,
    created_by TEXT,
    approved_by TEXT,
    approved_at TEXT,
    rejected_by TEXT,
    rejected_at TEXT,
    rejection_reason TEXT,
    error TEXT,
    FOREIGN KEY (group_id) REFERENCES worker_groups(id),
    FOREIGN KEY (created_by) REFERENCES users(id),
    FOREIGN KEY (approved_by) REFERENCES users(id),
    FOREIGN KEY (rejected_by) REFERENCES users(id)
);

-- Deployment agents (tracks per-agent status)
CREATE TABLE IF NOT EXISTS deployment_agents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    deployment_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',   -- pending, deploying, success, failed, skipped
    started_at TEXT,
    completed_at TEXT,
    error TEXT,
    FOREIGN KEY (deployment_id) REFERENCES deployments(id) ON DELETE CASCADE,
    FOREIGN KEY (agent_id) REFERENCES agents(id)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_deployments_group ON deployments(group_id);
CREATE INDEX IF NOT EXISTS idx_deployments_status ON deployments(status);
CREATE INDEX IF NOT EXISTS idx_deployment_agents_deployment ON deployment_agents(deployment_id);
CREATE INDEX IF NOT EXISTS idx_deployment_agents_agent ON deployment_agents(agent_id)
"#;

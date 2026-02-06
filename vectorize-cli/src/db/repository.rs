//! Database repository implementations
//!
//! Provides CRUD operations for all database models.

use sqlx::SqlitePool;
use uuid::Uuid;

use super::models::*;

// =============================================================================
// Agent Repository
// =============================================================================

pub struct AgentRepository;

impl AgentRepository {
    /// Create a new agent
    pub async fn create(
        pool: &SqlitePool,
        name: &str,
        url: &str,
        group_id: Option<&str>,
    ) -> Result<Agent, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        
        sqlx::query_as::<_, Agent>(
            r#"
            INSERT INTO agents (id, name, url, group_id, status)
            VALUES (?, ?, ?, ?, 'unknown')
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(name)
        .bind(url)
        .bind(group_id)
        .fetch_one(pool)
        .await
    }
    
    /// Get agent by ID
    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Agent>, sqlx::Error> {
        sqlx::query_as::<_, Agent>("SELECT * FROM agents WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }
    
    /// Get agent by URL
    pub async fn get_by_url(pool: &SqlitePool, url: &str) -> Result<Option<Agent>, sqlx::Error> {
        sqlx::query_as::<_, Agent>("SELECT * FROM agents WHERE url = ?")
            .bind(url)
            .fetch_optional(pool)
            .await
    }
    
    /// Get agent by name
    pub async fn get_by_name(pool: &SqlitePool, name: &str) -> Result<Option<Agent>, sqlx::Error> {
        sqlx::query_as::<_, Agent>("SELECT * FROM agents WHERE name = ?")
            .bind(name)
            .fetch_optional(pool)
            .await
    }
    
    /// List all agents
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Agent>, sqlx::Error> {
        sqlx::query_as::<_, Agent>("SELECT * FROM agents ORDER BY name")
            .fetch_all(pool)
            .await
    }
    
    /// List agents by group
    pub async fn list_by_group(pool: &SqlitePool, group_id: &str) -> Result<Vec<Agent>, sqlx::Error> {
        sqlx::query_as::<_, Agent>("SELECT * FROM agents WHERE group_id = ? ORDER BY name")
            .bind(group_id)
            .fetch_all(pool)
            .await
    }
    
    /// List unassigned agents (no group)
    pub async fn list_unassigned(pool: &SqlitePool) -> Result<Vec<Agent>, sqlx::Error> {
        sqlx::query_as::<_, Agent>("SELECT * FROM agents WHERE group_id IS NULL ORDER BY name")
            .fetch_all(pool)
            .await
    }
    
    /// Update agent
    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        name: Option<&str>,
        group_id: Option<Option<&str>>,  // None = don't update, Some(None) = set to NULL
    ) -> Result<Option<Agent>, sqlx::Error> {
        let agent = Self::get_by_id(pool, id).await?;
        if agent.is_none() {
            return Ok(None);
        }
        let agent = agent.unwrap();
        
        let new_name = name.unwrap_or(&agent.name);
        let new_group_id = match group_id {
            None => agent.group_id.as_deref(),
            Some(gid) => gid,
        };
        
        sqlx::query_as::<_, Agent>(
            r#"
            UPDATE agents SET name = ?, group_id = ?
            WHERE id = ?
            RETURNING *
            "#
        )
        .bind(new_name)
        .bind(new_group_id)
        .bind(id)
        .fetch_optional(pool)
        .await
    }
    
    /// Update agent status
    pub async fn update_status(
        pool: &SqlitePool,
        id: &str,
        status: &str,
        vector_version: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE agents 
            SET status = ?, vector_version = ?, last_seen = datetime('now')
            WHERE id = ?
            "#
        )
        .bind(status)
        .bind(vector_version)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }
    
    /// Delete agent
    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM agents WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
    
    /// Record health check
    pub async fn record_health_check(
        pool: &SqlitePool,
        agent_id: &str,
        healthy: bool,
        latency_ms: Option<i64>,
        error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO health_checks (agent_id, healthy, latency_ms, error)
            VALUES (?, ?, ?, ?)
            "#
        )
        .bind(agent_id)
        .bind(healthy)
        .bind(latency_ms)
        .bind(error)
        .execute(pool)
        .await?;
        Ok(())
    }
    
    /// Get recent health checks for an agent
    pub async fn get_health_checks(
        pool: &SqlitePool,
        agent_id: &str,
        limit: i64,
    ) -> Result<Vec<HealthCheck>, sqlx::Error> {
        sqlx::query_as::<_, HealthCheck>(
            r#"
            SELECT * FROM health_checks 
            WHERE agent_id = ? 
            ORDER BY checked_at DESC 
            LIMIT ?
            "#
        )
        .bind(agent_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

// =============================================================================
// Worker Group Repository
// =============================================================================

pub struct WorkerGroupRepository;

impl WorkerGroupRepository {
    /// Create a new worker group
    pub async fn create(
        pool: &SqlitePool,
        name: &str,
        description: Option<&str>,
        created_by: Option<&str>,
    ) -> Result<WorkerGroup, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        
        sqlx::query_as::<_, WorkerGroup>(
            r#"
            INSERT INTO worker_groups (id, name, description, created_by)
            VALUES (?, ?, ?, ?)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(name)
        .bind(description)
        .bind(created_by)
        .fetch_one(pool)
        .await
    }
    
    /// Get group by ID
    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Option<WorkerGroup>, sqlx::Error> {
        sqlx::query_as::<_, WorkerGroup>("SELECT * FROM worker_groups WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }
    
    /// Get group by name
    pub async fn get_by_name(pool: &SqlitePool, name: &str) -> Result<Option<WorkerGroup>, sqlx::Error> {
        sqlx::query_as::<_, WorkerGroup>("SELECT * FROM worker_groups WHERE name = ?")
            .bind(name)
            .fetch_optional(pool)
            .await
    }
    
    /// List all groups
    pub async fn list(pool: &SqlitePool) -> Result<Vec<WorkerGroup>, sqlx::Error> {
        sqlx::query_as::<_, WorkerGroup>("SELECT * FROM worker_groups ORDER BY name")
            .fetch_all(pool)
            .await
    }
    
    /// Update group
    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        name: Option<&str>,
        description: Option<Option<&str>>,
        deployment_strategy: Option<&str>,
        requires_approval: Option<bool>,
        approvers: Option<&str>,
    ) -> Result<Option<WorkerGroup>, sqlx::Error> {
        let group = Self::get_by_id(pool, id).await?;
        if group.is_none() {
            return Ok(None);
        }
        let group = group.unwrap();
        
        sqlx::query_as::<_, WorkerGroup>(
            r#"
            UPDATE worker_groups SET 
                name = ?,
                description = ?,
                deployment_strategy = ?,
                requires_approval = ?,
                approvers = ?,
                updated_at = datetime('now')
            WHERE id = ?
            RETURNING *
            "#
        )
        .bind(name.unwrap_or(&group.name))
        .bind(description.unwrap_or(group.description.as_deref()))
        .bind(deployment_strategy.unwrap_or(&group.deployment_strategy))
        .bind(requires_approval.unwrap_or(group.requires_approval))
        .bind(approvers.or(group.approvers.as_deref()))
        .bind(id)
        .fetch_optional(pool)
        .await
    }
    
    /// Update group config version
    pub async fn update_config_version(
        pool: &SqlitePool,
        id: &str,
        version: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE worker_groups 
            SET current_config_version = ?, updated_at = datetime('now')
            WHERE id = ?
            "#
        )
        .bind(version)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }
    
    /// Delete group
    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM worker_groups WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
    
    /// Get agent count for group
    pub async fn get_agent_count(pool: &SqlitePool, id: &str) -> Result<i64, sqlx::Error> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents WHERE group_id = ?")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(count.0)
    }
    
    /// Get agent health counts for a group
    /// Returns (total, healthy, unhealthy)
    pub async fn get_agent_health_counts(pool: &SqlitePool, group_id: &str) -> Result<(i64, i64, i64), sqlx::Error> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents WHERE group_id = ?")
            .bind(group_id)
            .fetch_one(pool)
            .await?;
            
        let healthy: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents WHERE group_id = ? AND status = 'healthy'")
            .bind(group_id)
            .fetch_one(pool)
            .await?;
            
        let unhealthy: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents WHERE group_id = ? AND status = 'unhealthy'")
            .bind(group_id)
            .fetch_one(pool)
            .await?;
            
        Ok((total.0, healthy.0, unhealthy.0))
    }
}

// =============================================================================
// User Repository
// =============================================================================

pub struct UserRepository;

impl UserRepository {
    /// Create a new user
    pub async fn create(
        pool: &SqlitePool,
        username: &str,
        email: &str,
        password_hash: &str,
        role_id: &str,
    ) -> Result<User, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        
        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, username, email, password_hash, sso_provider, role_id)
            VALUES (?, ?, ?, ?, 'local', ?)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .bind(role_id)
        .fetch_one(pool)
        .await
    }
    
    /// Get user by ID (used in future auth phase)
    #[allow(dead_code)]
    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }
    
    /// Get user by username
    pub async fn get_by_username(pool: &SqlitePool, username: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(pool)
            .await
    }
    
    /// Get user by email
    pub async fn get_by_email(pool: &SqlitePool, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
            .bind(email)
            .fetch_optional(pool)
            .await
    }
    
    /// List all users (used in future auth phase)
    #[allow(dead_code)]
    pub async fn list(pool: &SqlitePool) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY username")
            .fetch_all(pool)
            .await
    }
    
    /// Update last login
    pub async fn update_last_login(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET last_login = datetime('now') WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
    
    /// Delete user (used in future auth phase)
    #[allow(dead_code)]
    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

// =============================================================================
// Role Repository (used in future RBAC phase)
// =============================================================================

#[allow(dead_code)]
pub struct RoleRepository;

#[allow(dead_code)]
impl RoleRepository {
    /// Get role by ID
    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Role>, sqlx::Error> {
        sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }
    
    /// List all roles
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Role>, sqlx::Error> {
        sqlx::query_as::<_, Role>("SELECT * FROM roles ORDER BY name")
            .fetch_all(pool)
            .await
    }
    
    /// Create a custom role
    pub async fn create(
        pool: &SqlitePool,
        name: &str,
        description: Option<&str>,
        permissions: &[String],
    ) -> Result<Role, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let permissions_json = serde_json::to_string(permissions).unwrap_or_else(|_| "[]".to_string());
        
        sqlx::query_as::<_, Role>(
            r#"
            INSERT INTO roles (id, name, description, permissions, is_builtin)
            VALUES (?, ?, ?, ?, 0)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(name)
        .bind(description)
        .bind(&permissions_json)
        .fetch_one(pool)
        .await
    }
    
    /// Delete a custom role (cannot delete built-in roles)
    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM roles WHERE id = ? AND is_builtin = 0")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

// =============================================================================
// API Key Repository (used in future auth phase)
// =============================================================================

#[allow(dead_code)]
pub struct ApiKeyRepository;

#[allow(dead_code)]
impl ApiKeyRepository {
    /// Create a new API key (returns the full key only once)
    pub async fn create(
        pool: &SqlitePool,
        name: &str,
        key_hash: &str,
        key_prefix: &str,
        user_id: Option<&str>,
        permissions: Option<&str>,
        expires_at: Option<&str>,
    ) -> Result<ApiKey, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        
        sqlx::query_as::<_, ApiKey>(
            r#"
            INSERT INTO api_keys (id, name, key_hash, key_prefix, user_id, permissions, expires_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(name)
        .bind(key_hash)
        .bind(key_prefix)
        .bind(user_id)
        .bind(permissions)
        .bind(expires_at)
        .fetch_one(pool)
        .await
    }
    
    /// Get API key by hash (for authentication)
    pub async fn get_by_hash(pool: &SqlitePool, key_hash: &str) -> Result<Option<ApiKey>, sqlx::Error> {
        sqlx::query_as::<_, ApiKey>(
            "SELECT * FROM api_keys WHERE key_hash = ? AND revoked_at IS NULL"
        )
        .bind(key_hash)
        .fetch_optional(pool)
        .await
    }
    
    /// List API keys for a user
    pub async fn list_by_user(pool: &SqlitePool, user_id: &str) -> Result<Vec<ApiKey>, sqlx::Error> {
        sqlx::query_as::<_, ApiKey>(
            "SELECT * FROM api_keys WHERE user_id = ? ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }
    
    /// Update last used timestamp
    pub async fn update_last_used(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE api_keys SET last_used = datetime('now') WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
    
    /// Revoke an API key
    pub async fn revoke(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE api_keys SET revoked_at = datetime('now') WHERE id = ? AND revoked_at IS NULL"
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

// =============================================================================
// Audit Log Repository (used in future audit phase)
// =============================================================================

#[allow(dead_code)]
pub struct AuditLogRepository;

#[allow(dead_code)]
impl AuditLogRepository {
    /// Create audit log entry
    pub async fn create(
        pool: &SqlitePool,
        actor_type: &str,
        actor_id: Option<&str>,
        actor_name: Option<&str>,
        action: &str,
        resource_type: Option<&str>,
        resource_id: Option<&str>,
        details: Option<&str>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        result: &str,
    ) -> Result<AuditLogEntry, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        
        sqlx::query_as::<_, AuditLogEntry>(
            r#"
            INSERT INTO audit_log (
                id, actor_type, actor_id, actor_name, action, 
                resource_type, resource_id, details, ip_address, user_agent, result
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(actor_type)
        .bind(actor_id)
        .bind(actor_name)
        .bind(action)
        .bind(resource_type)
        .bind(resource_id)
        .bind(details)
        .bind(ip_address)
        .bind(user_agent)
        .bind(result)
        .fetch_one(pool)
        .await
    }
    
    /// List audit log entries with optional filters
    pub async fn list(
        pool: &SqlitePool,
        actor_id: Option<&str>,
        action: Option<&str>,
        resource_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditLogEntry>, sqlx::Error> {
        let mut query = String::from("SELECT * FROM audit_log WHERE 1=1");
        
        if actor_id.is_some() {
            query.push_str(" AND actor_id = ?");
        }
        if action.is_some() {
            query.push_str(" AND action = ?");
        }
        if resource_type.is_some() {
            query.push_str(" AND resource_type = ?");
        }
        
        query.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");
        
        let mut q = sqlx::query_as::<_, AuditLogEntry>(&query);
        
        if let Some(aid) = actor_id {
            q = q.bind(aid);
        }
        if let Some(a) = action {
            q = q.bind(a);
        }
        if let Some(rt) = resource_type {
            q = q.bind(rt);
        }
        
        q.bind(limit).bind(offset).fetch_all(pool).await
    }
}

// =============================================================================
// Deployment Repository
// =============================================================================

pub struct DeploymentRepository;

impl DeploymentRepository {
    /// Create a new deployment
    pub async fn create(
        pool: &SqlitePool,
        group_id: &str,
        config_version: &str,
        strategy: &str,
        options: Option<&str>,
        created_by: Option<&str>,
    ) -> Result<Deployment, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        
        sqlx::query_as::<_, Deployment>(
            r#"
            INSERT INTO deployments (id, group_id, config_version, strategy, status, options, created_by)
            VALUES (?, ?, ?, ?, 'pending', ?, ?)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(group_id)
        .bind(config_version)
        .bind(strategy)
        .bind(options)
        .bind(created_by)
        .fetch_one(pool)
        .await
    }
    
    /// Get deployment by ID
    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Deployment>, sqlx::Error> {
        sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }
    
    /// List deployments for a group
    pub async fn list_by_group(
        pool: &SqlitePool,
        group_id: &str,
        limit: i64,
    ) -> Result<Vec<Deployment>, sqlx::Error> {
        sqlx::query_as::<_, Deployment>(
            "SELECT * FROM deployments WHERE group_id = ? ORDER BY created_at DESC LIMIT ?"
        )
        .bind(group_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
    
    /// Get active deployment for a group (in_progress or queued)
    pub async fn get_active_for_group(
        pool: &SqlitePool,
        group_id: &str,
    ) -> Result<Option<Deployment>, sqlx::Error> {
        sqlx::query_as::<_, Deployment>(
            "SELECT * FROM deployments WHERE group_id = ? AND status IN ('pending', 'queued', 'in_progress', 'pending_approval') ORDER BY created_at DESC LIMIT 1"
        )
        .bind(group_id)
        .fetch_optional(pool)
        .await
    }
    
    /// Get queued deployments for a group
    pub async fn get_queued_for_group(
        pool: &SqlitePool,
        group_id: &str,
    ) -> Result<Vec<Deployment>, sqlx::Error> {
        sqlx::query_as::<_, Deployment>(
            "SELECT * FROM deployments WHERE group_id = ? AND status = 'queued' ORDER BY created_at ASC"
        )
        .bind(group_id)
        .fetch_all(pool)
        .await
    }
    
    /// Update deployment status
    pub async fn update_status(
        pool: &SqlitePool,
        id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        
        // Set started_at when transitioning to in_progress
        if status == "in_progress" {
            sqlx::query(
                "UPDATE deployments SET status = ?, started_at = ?, error = ? WHERE id = ?"
            )
            .bind(status)
            .bind(&now)
            .bind(error)
            .bind(id)
            .execute(pool)
            .await?;
        } 
        // Set completed_at when transitioning to completed or failed
        else if status == "completed" || status == "failed" || status == "cancelled" {
            sqlx::query(
                "UPDATE deployments SET status = ?, completed_at = ?, error = ? WHERE id = ?"
            )
            .bind(status)
            .bind(&now)
            .bind(error)
            .bind(id)
            .execute(pool)
            .await?;
        } else {
            sqlx::query(
                "UPDATE deployments SET status = ?, error = ? WHERE id = ?"
            )
            .bind(status)
            .bind(error)
            .bind(id)
            .execute(pool)
            .await?;
        }
        Ok(())
    }
    
    /// Approve a deployment
    /// Note: approved_by can be a user ID or name (stored in approved_at comment)
    pub async fn approve(
        pool: &SqlitePool,
        id: &str,
        approved_by: &str,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let approval_note = format!("{} by {}", now, approved_by);
        
        // Don't set approved_by as FK since we may not have a valid user ID
        // Store approval info in approved_at field
        sqlx::query(
            "UPDATE deployments SET status = 'queued', approved_at = ? WHERE id = ? AND status = 'pending_approval'"
        )
        .bind(&approval_note)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }
    
    /// Reject a deployment
    /// Note: rejected_by can be a user ID or name (stored in rejected_at comment)
    pub async fn reject(
        pool: &SqlitePool,
        id: &str,
        rejected_by: &str,
        reason: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let rejection_note = format!("{} by {}", now, rejected_by);
        
        // Don't set rejected_by as FK since we may not have a valid user ID
        // Store rejection info in rejected_at field
        sqlx::query(
            "UPDATE deployments SET status = 'cancelled', rejected_at = ?, rejection_reason = ? WHERE id = ? AND status = 'pending_approval'"
        )
        .bind(&rejection_note)
        .bind(reason)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }
    
    /// Add an agent to a deployment
    pub async fn add_agent(
        pool: &SqlitePool,
        deployment_id: &str,
        agent_id: &str,
    ) -> Result<DeploymentAgent, sqlx::Error> {
        sqlx::query_as::<_, DeploymentAgent>(
            r#"
            INSERT INTO deployment_agents (deployment_id, agent_id, status)
            VALUES (?, ?, 'pending')
            RETURNING *
            "#
        )
        .bind(deployment_id)
        .bind(agent_id)
        .fetch_one(pool)
        .await
    }
    
    /// Get agents for a deployment
    pub async fn get_agents(
        pool: &SqlitePool,
        deployment_id: &str,
    ) -> Result<Vec<DeploymentAgent>, sqlx::Error> {
        sqlx::query_as::<_, DeploymentAgent>(
            "SELECT * FROM deployment_agents WHERE deployment_id = ? ORDER BY id"
        )
        .bind(deployment_id)
        .fetch_all(pool)
        .await
    }
    
    /// Update deployment agent status
    pub async fn update_agent_status(
        pool: &SqlitePool,
        deployment_id: &str,
        agent_id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        
        if status == "in_progress" {
            sqlx::query(
                "UPDATE deployment_agents SET status = ?, started_at = ?, error = ? WHERE deployment_id = ? AND agent_id = ?"
            )
            .bind(status)
            .bind(&now)
            .bind(error)
            .bind(deployment_id)
            .bind(agent_id)
            .execute(pool)
            .await?;
        } else if status == "completed" || status == "failed" {
            sqlx::query(
                "UPDATE deployment_agents SET status = ?, completed_at = ?, error = ? WHERE deployment_id = ? AND agent_id = ?"
            )
            .bind(status)
            .bind(&now)
            .bind(error)
            .bind(deployment_id)
            .bind(agent_id)
            .execute(pool)
            .await?;
        } else {
            sqlx::query(
                "UPDATE deployment_agents SET status = ?, error = ? WHERE deployment_id = ? AND agent_id = ?"
            )
            .bind(status)
            .bind(error)
            .bind(deployment_id)
            .bind(agent_id)
            .execute(pool)
            .await?;
        }
        Ok(())
    }
    
    /// Get next pending agent for a deployment (for rolling deployments)
    pub async fn get_next_pending_agent(
        pool: &SqlitePool,
        deployment_id: &str,
    ) -> Result<Option<DeploymentAgent>, sqlx::Error> {
        sqlx::query_as::<_, DeploymentAgent>(
            "SELECT * FROM deployment_agents WHERE deployment_id = ? AND status = 'pending' ORDER BY id LIMIT 1"
        )
        .bind(deployment_id)
        .fetch_optional(pool)
        .await
    }
    
    /// Check if all agents are completed
    pub async fn all_agents_completed(
        pool: &SqlitePool,
        deployment_id: &str,
    ) -> Result<bool, sqlx::Error> {
        let pending: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM deployment_agents WHERE deployment_id = ? AND status NOT IN ('completed', 'failed')"
        )
        .bind(deployment_id)
        .fetch_one(pool)
        .await?;
        
        Ok(pending.0 == 0)
    }
    
    /// Get deployment stats (for progress tracking)
    pub async fn get_stats(
        pool: &SqlitePool,
        deployment_id: &str,
    ) -> Result<DeploymentStats, sqlx::Error> {
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM deployment_agents WHERE deployment_id = ?"
        )
        .bind(deployment_id)
        .fetch_one(pool)
        .await?;
        
        let completed: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM deployment_agents WHERE deployment_id = ? AND status = 'completed'"
        )
        .bind(deployment_id)
        .fetch_one(pool)
        .await?;
        
        let failed: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM deployment_agents WHERE deployment_id = ? AND status = 'failed'"
        )
        .bind(deployment_id)
        .fetch_one(pool)
        .await?;
        
        let in_progress: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM deployment_agents WHERE deployment_id = ? AND status = 'in_progress'"
        )
        .bind(deployment_id)
        .fetch_one(pool)
        .await?;
        
        Ok(DeploymentStats {
            total: total.0 as u32,
            completed: completed.0 as u32,
            failed: failed.0 as u32,
            in_progress: in_progress.0 as u32,
            pending: (total.0 - completed.0 - failed.0 - in_progress.0) as u32,
        })
    }
}

/// Deployment statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeploymentStats {
    pub total: u32,
    pub completed: u32,
    pub failed: u32,
    pub in_progress: u32,
    pub pending: u32,
}

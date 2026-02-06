//! Database module for Vectorize runtime storage
//!
//! Uses SQLite for storing:
//! - Agent registry
//! - Health check history
//! - Users and roles (RBAC)
//! - API keys
//! - Sessions
//! - Audit logs

pub mod migrations;
pub mod models;
pub mod repository;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;
use tracing::info;

/// Database connection pool
#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Create a new database connection
    /// 
    /// If the database file doesn't exist, it will be created.
    /// Migrations are run automatically on startup.
    pub async fn new(db_path: &Path) -> Result<Self, sqlx::Error> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                sqlx::Error::Configuration(format!("Failed to create database directory: {}", e).into())
            })?;
        }
        
        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
        info!("Connecting to database: {}", db_path.display());
        
        let options = SqliteConnectOptions::from_str(&db_url)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .busy_timeout(std::time::Duration::from_secs(30));
        
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        
        let db = Self { pool };
        
        // Run migrations
        db.run_migrations().await?;
        
        // Seed default data (roles, etc.)
        db.seed_defaults().await?;
        
        info!("Database initialized successfully");
        Ok(db)
    }
    
    /// Run database migrations
    async fn run_migrations(&self) -> Result<(), sqlx::Error> {
        info!("Running database migrations...");
        migrations::run_migrations(&self.pool).await
    }
    
    /// Seed default data (built-in roles, etc.)
    async fn seed_defaults(&self) -> Result<(), sqlx::Error> {
        info!("Seeding default data...");
        
        // Create built-in roles if they don't exist
        let roles = [
            ("admin", "Administrator", r#"["*"]"#, true),
            ("operator", "Operator", r#"["agents:*","groups:*","config:*","deployments:*","tap:*","git:read"]"#, true),
            ("viewer", "Viewer", r#"["agents:read","groups:read","config:read","topology:read","metrics:read"]"#, true),
        ];
        
        for (id, name, permissions, is_builtin) in roles {
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO roles (id, name, description, permissions, is_builtin, created_at)
                VALUES (?, ?, ?, ?, ?, datetime('now'))
                "#
            )
            .bind(id)
            .bind(name)
            .bind(format!("{} role", name))
            .bind(permissions)
            .bind(is_builtin)
            .execute(&self.pool)
            .await?;
        }
        
        Ok(())
    }
    
    /// Get the connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
    
    /// Check if this is a fresh database (no users exist)
    pub async fn is_fresh(&self) -> Result<bool, sqlx::Error> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        Ok(count.0 == 0)
    }
    
    /// Close the database connection
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::repository::*;
    use tempfile::tempdir;
    
    async fn create_test_db() -> (Database, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::new(&db_path).await.unwrap();
        (db, dir)
    }
    
    #[tokio::test]
    async fn test_database_creation() {
        let (db, _dir) = create_test_db().await;
        
        // Should be fresh (no users)
        assert!(db.is_fresh().await.unwrap());
        
        // Built-in roles should exist
        let roles: Vec<(String,)> = sqlx::query_as("SELECT id FROM roles WHERE is_builtin = 1")
            .fetch_all(db.pool())
            .await
            .unwrap();
        assert_eq!(roles.len(), 3);
        
        db.close().await;
    }
    
    // =========================================================================
    // Agent Repository Tests
    // =========================================================================
    
    #[tokio::test]
    async fn test_agent_create() {
        let (db, _dir) = create_test_db().await;
        
        let agent = AgentRepository::create(db.pool(), "test-agent", "http://localhost:8080", None)
            .await
            .unwrap();
        
        assert_eq!(agent.name, "test-agent");
        assert_eq!(agent.url, "http://localhost:8080");
        assert_eq!(agent.status, "unknown");
        assert!(agent.group_id.is_none());
        
        db.close().await;
    }
    
    #[tokio::test]
    async fn test_agent_get_by_id() {
        let (db, _dir) = create_test_db().await;
        
        let agent = AgentRepository::create(db.pool(), "agent1", "http://localhost:8080", None)
            .await
            .unwrap();
        
        let found = AgentRepository::get_by_id(db.pool(), &agent.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "agent1");
        
        let not_found = AgentRepository::get_by_id(db.pool(), "nonexistent").await.unwrap();
        assert!(not_found.is_none());
        
        db.close().await;
    }
    
    #[tokio::test]
    async fn test_agent_get_by_url() {
        let (db, _dir) = create_test_db().await;
        
        AgentRepository::create(db.pool(), "agent1", "http://localhost:8080", None)
            .await
            .unwrap();
        
        let found = AgentRepository::get_by_url(db.pool(), "http://localhost:8080").await.unwrap();
        assert!(found.is_some());
        
        let not_found = AgentRepository::get_by_url(db.pool(), "http://other:8080").await.unwrap();
        assert!(not_found.is_none());
        
        db.close().await;
    }
    
    #[tokio::test]
    async fn test_agent_list() {
        let (db, _dir) = create_test_db().await;
        
        AgentRepository::create(db.pool(), "agent1", "http://localhost:8081", None).await.unwrap();
        AgentRepository::create(db.pool(), "agent2", "http://localhost:8082", None).await.unwrap();
        AgentRepository::create(db.pool(), "agent3", "http://localhost:8083", None).await.unwrap();
        
        let agents = AgentRepository::list(db.pool()).await.unwrap();
        assert_eq!(agents.len(), 3);
        
        db.close().await;
    }
    
    #[tokio::test]
    async fn test_agent_update() {
        let (db, _dir) = create_test_db().await;
        
        let agent = AgentRepository::create(db.pool(), "agent1", "http://localhost:8080", None)
            .await
            .unwrap();
        
        let updated = AgentRepository::update(db.pool(), &agent.id, Some("renamed-agent"), None)
            .await
            .unwrap();
        
        assert!(updated.is_some());
        assert_eq!(updated.unwrap().name, "renamed-agent");
        
        db.close().await;
    }
    
    #[tokio::test]
    async fn test_agent_update_status() {
        let (db, _dir) = create_test_db().await;
        
        let agent = AgentRepository::create(db.pool(), "agent1", "http://localhost:8080", None)
            .await
            .unwrap();
        
        AgentRepository::update_status(db.pool(), &agent.id, "healthy", Some("0.54.0"))
            .await
            .unwrap();
        
        let updated = AgentRepository::get_by_id(db.pool(), &agent.id).await.unwrap().unwrap();
        assert_eq!(updated.status, "healthy");
        assert_eq!(updated.vector_version, Some("0.54.0".to_string()));
        assert!(updated.last_seen.is_some());
        
        db.close().await;
    }
    
    #[tokio::test]
    async fn test_agent_delete() {
        let (db, _dir) = create_test_db().await;
        
        let agent = AgentRepository::create(db.pool(), "agent1", "http://localhost:8080", None)
            .await
            .unwrap();
        
        let deleted = AgentRepository::delete(db.pool(), &agent.id).await.unwrap();
        assert!(deleted);
        
        let not_found = AgentRepository::get_by_id(db.pool(), &agent.id).await.unwrap();
        assert!(not_found.is_none());
        
        // Delete nonexistent should return false
        let not_deleted = AgentRepository::delete(db.pool(), "nonexistent").await.unwrap();
        assert!(!not_deleted);
        
        db.close().await;
    }
    
    #[tokio::test]
    async fn test_agent_health_checks() {
        let (db, _dir) = create_test_db().await;
        
        let agent = AgentRepository::create(db.pool(), "agent1", "http://localhost:8080", None)
            .await
            .unwrap();
        
        // Record some health checks
        AgentRepository::record_health_check(db.pool(), &agent.id, true, Some(10), None).await.unwrap();
        AgentRepository::record_health_check(db.pool(), &agent.id, true, Some(15), None).await.unwrap();
        AgentRepository::record_health_check(db.pool(), &agent.id, false, None, Some("Connection refused")).await.unwrap();
        
        let checks = AgentRepository::get_health_checks(db.pool(), &agent.id, 10).await.unwrap();
        assert_eq!(checks.len(), 3);
        
        // At least one should be unhealthy
        let has_unhealthy = checks.iter().any(|c| !c.healthy);
        assert!(has_unhealthy);
        
        db.close().await;
    }
    
    // =========================================================================
    // Worker Group Repository Tests
    // =========================================================================
    
    #[tokio::test]
    async fn test_worker_group_create() {
        let (db, _dir) = create_test_db().await;
        
        let group = WorkerGroupRepository::create(
            db.pool(), 
            "production", 
            Some("Production servers"), 
            None
        ).await.unwrap();
        
        assert_eq!(group.name, "production");
        assert_eq!(group.description, Some("Production servers".to_string()));
        // Default strategy is "basic"
        assert_eq!(group.deployment_strategy, "basic");
        assert!(!group.requires_approval);
        
        db.close().await;
    }
    
    #[tokio::test]
    async fn test_worker_group_get_by_name() {
        let (db, _dir) = create_test_db().await;
        
        WorkerGroupRepository::create(db.pool(), "staging", None, None).await.unwrap();
        
        let found = WorkerGroupRepository::get_by_name(db.pool(), "staging").await.unwrap();
        assert!(found.is_some());
        
        let not_found = WorkerGroupRepository::get_by_name(db.pool(), "nonexistent").await.unwrap();
        assert!(not_found.is_none());
        
        db.close().await;
    }
    
    #[tokio::test]
    async fn test_worker_group_list() {
        let (db, _dir) = create_test_db().await;
        
        // Default group already exists from seeding
        let initial = WorkerGroupRepository::list(db.pool()).await.unwrap();
        let initial_count = initial.len();
        
        WorkerGroupRepository::create(db.pool(), "production", None, None).await.unwrap();
        WorkerGroupRepository::create(db.pool(), "staging", None, None).await.unwrap();
        
        let groups = WorkerGroupRepository::list(db.pool()).await.unwrap();
        assert_eq!(groups.len(), initial_count + 2);
        
        db.close().await;
    }
    
    #[tokio::test]
    async fn test_worker_group_update() {
        let (db, _dir) = create_test_db().await;
        
        let group = WorkerGroupRepository::create(db.pool(), "production", None, None).await.unwrap();
        
        let updated = WorkerGroupRepository::update(
            db.pool(),
            &group.id,
            Some("prod"),
            Some(Some("Production environment")),
            Some("rolling"),
            Some(true),
            None,
        ).await.unwrap();
        
        assert!(updated.is_some());
        let updated = updated.unwrap();
        assert_eq!(updated.name, "prod");
        assert_eq!(updated.description, Some("Production environment".to_string()));
        assert_eq!(updated.deployment_strategy, "rolling");
        assert!(updated.requires_approval);
        
        db.close().await;
    }
    
    #[tokio::test]
    async fn test_worker_group_delete() {
        let (db, _dir) = create_test_db().await;
        
        let group = WorkerGroupRepository::create(db.pool(), "temp-group", None, None).await.unwrap();
        
        let deleted = WorkerGroupRepository::delete(db.pool(), &group.id).await.unwrap();
        assert!(deleted);
        
        let not_found = WorkerGroupRepository::get_by_id(db.pool(), &group.id).await.unwrap();
        assert!(not_found.is_none());
        
        db.close().await;
    }
    
    #[tokio::test]
    async fn test_worker_group_agent_count() {
        let (db, _dir) = create_test_db().await;
        
        let group = WorkerGroupRepository::create(db.pool(), "production", None, None).await.unwrap();
        
        // Initially no agents
        let count = WorkerGroupRepository::get_agent_count(db.pool(), &group.id).await.unwrap();
        assert_eq!(count, 0);
        
        // Add agents to group
        AgentRepository::create(db.pool(), "agent1", "http://localhost:8081", Some(&group.id)).await.unwrap();
        AgentRepository::create(db.pool(), "agent2", "http://localhost:8082", Some(&group.id)).await.unwrap();
        
        let count = WorkerGroupRepository::get_agent_count(db.pool(), &group.id).await.unwrap();
        assert_eq!(count, 2);
        
        db.close().await;
    }
    
    #[tokio::test]
    async fn test_agent_list_by_group() {
        let (db, _dir) = create_test_db().await;
        
        let group1 = WorkerGroupRepository::create(db.pool(), "group1", None, None).await.unwrap();
        let group2 = WorkerGroupRepository::create(db.pool(), "group2", None, None).await.unwrap();
        
        AgentRepository::create(db.pool(), "agent1", "http://localhost:8081", Some(&group1.id)).await.unwrap();
        AgentRepository::create(db.pool(), "agent2", "http://localhost:8082", Some(&group1.id)).await.unwrap();
        AgentRepository::create(db.pool(), "agent3", "http://localhost:8083", Some(&group2.id)).await.unwrap();
        
        let group1_agents = AgentRepository::list_by_group(db.pool(), &group1.id).await.unwrap();
        assert_eq!(group1_agents.len(), 2);
        
        let group2_agents = AgentRepository::list_by_group(db.pool(), &group2.id).await.unwrap();
        assert_eq!(group2_agents.len(), 1);
        
        db.close().await;
    }
    
    // =========================================================================
    // User Repository Tests
    // =========================================================================
    
    #[tokio::test]
    async fn test_user_create() {
        let (db, _dir) = create_test_db().await;
        
        let user = UserRepository::create(
            db.pool(),
            "admin",
            "admin@test.com",
            "hashed_password",
            "admin",
        ).await.unwrap();
        
        assert_eq!(user.username, Some("admin".to_string()));
        assert_eq!(user.email, Some("admin@test.com".to_string()));
        assert_eq!(user.role_id, "admin");
        assert_eq!(user.sso_provider, Some("local".to_string()));
        assert!(user.is_active);
        
        // DB should no longer be fresh
        assert!(!db.is_fresh().await.unwrap());
        
        db.close().await;
    }
    
    #[tokio::test]
    async fn test_user_get_by_username() {
        let (db, _dir) = create_test_db().await;
        
        UserRepository::create(db.pool(), "testuser", "test@test.com", "hash", "viewer").await.unwrap();
        
        let found = UserRepository::get_by_username(db.pool(), "testuser").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().email, Some("test@test.com".to_string()));
        
        let not_found = UserRepository::get_by_username(db.pool(), "nonexistent").await.unwrap();
        assert!(not_found.is_none());
        
        db.close().await;
    }
    
    #[tokio::test]
    async fn test_user_get_by_email() {
        let (db, _dir) = create_test_db().await;
        
        UserRepository::create(db.pool(), "testuser", "test@test.com", "hash", "viewer").await.unwrap();
        
        let found = UserRepository::get_by_email(db.pool(), "test@test.com").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().username, Some("testuser".to_string()));
        
        db.close().await;
    }
    
    #[tokio::test]
    async fn test_user_update_last_login() {
        let (db, _dir) = create_test_db().await;
        
        let user = UserRepository::create(db.pool(), "testuser", "test@test.com", "hash", "viewer").await.unwrap();
        assert!(user.last_login.is_none());
        
        UserRepository::update_last_login(db.pool(), &user.id).await.unwrap();
        
        let updated = UserRepository::get_by_id(db.pool(), &user.id).await.unwrap().unwrap();
        assert!(updated.last_login.is_some());
        
        db.close().await;
    }
    
    // =========================================================================
    // Config Version Tests
    // =========================================================================
    
    #[tokio::test]
    async fn test_worker_group_config_version() {
        let (db, _dir) = create_test_db().await;
        
        let group = WorkerGroupRepository::create(db.pool(), "production", None, None).await.unwrap();
        assert!(group.current_config_version.is_none());
        
        WorkerGroupRepository::update_config_version(db.pool(), &group.id, "abc123").await.unwrap();
        
        let updated = WorkerGroupRepository::get_by_id(db.pool(), &group.id).await.unwrap().unwrap();
        assert_eq!(updated.current_config_version, Some("abc123".to_string()));
        
        db.close().await;
    }
}

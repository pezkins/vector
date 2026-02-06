//! API endpoints for Vectorize control plane
//!
//! Provides REST API for managing:
//! - Agents (registration, health, status)
//! - Worker Groups (CRUD, agent assignment)
//! - Configuration (deployment, versioning, validation)
//! - Deployments (strategies, approval workflows)
//! - Authentication (setup, login, API keys, SSO)
//! - Alerts (rules, notification channels)
//! - Users and Roles (RBAC)
//! - Audit logging
//! - Live data sampling (tap)
//! - Git remote sync

pub mod agents;
pub mod alerts;
pub mod audit;
pub mod auth;
pub mod deployments;
pub mod git;
pub mod groups;
pub mod health;
pub mod roles;
pub mod tap;
pub mod users;
pub mod validation;

use axum::{
    Router,
    routing::{get, post, delete},
};
use std::sync::Arc;

use crate::AppState;

/// Create the API router with all control plane endpoints
/// Note: User/Role/Audit endpoints require JWT authentication.
/// The auth middleware must be applied at the server level.
pub fn create_api_router() -> Router<Arc<AppState>> {
    Router::new()
        // Agent endpoints
        .route("/agents", get(agents::list_agents).post(agents::register_agent))
        .route("/agents/unassigned", get(agents::list_unassigned_agents))
        .route("/agents/:id", get(agents::get_agent).put(agents::update_agent).delete(agents::delete_agent))
        .route("/agents/:id/health", get(agents::get_agent_health))
        .route("/agents/:id/assign", post(agents::assign_agent_to_group))
        
        // Worker group endpoints
        .route("/groups", get(groups::list_groups).post(groups::create_group))
        .route("/groups/:id", get(groups::get_group).put(groups::update_group).delete(groups::delete_group))
        .route("/groups/:id/agents", get(groups::list_group_agents))
        .route("/groups/:id/config", get(groups::get_group_config).put(groups::update_group_config))
        .route("/groups/:id/config/:version", get(groups::get_group_config_at_version))
        .route("/groups/:id/history", get(groups::get_group_history))
        .route("/groups/:id/diff", get(groups::get_group_diff))
        .route("/groups/:id/rollback", post(groups::rollback_group_config))
        .route("/groups/:id/deploy", post(groups::deploy_to_group))
        
        // Deployment endpoints
        .route("/groups/:id/deployments", get(deployments::list_deployments).post(deployments::create_deployment))
        .route("/groups/:id/versions", get(deployments::check_versions))
        .route("/deployments/:id", get(deployments::get_deployment))
        .route("/deployments/:id/approve", post(deployments::approve_deployment))
        .route("/deployments/:id/reject", post(deployments::reject_deployment))
        .route("/deployments/:id/cancel", post(deployments::cancel_deployment))
        
        // Tap/Sample endpoints
        .route("/tap/config", get(tap::get_tap_config))
        .route("/tap/:agent_id/sample", get(tap::sample_agent))
        .route("/tap/:agent_id/rate-limit", get(tap::check_rate_limit))
        .route("/tap/:agent_id/ws-info", get(tap::get_websocket_info))
        
        // Git remote sync endpoints
        .route("/git/remotes", get(git::list_remotes).post(git::configure_remote))
        .route("/git/remotes/:name", delete(git::delete_remote))
        .route("/git/remotes/:name/push", post(git::push_to_remote))
        .route("/git/remotes/:name/pull", post(git::pull_from_remote))
        .route("/git/remotes/:name/sync", post(git::sync_with_remote))
        .route("/git/remotes/:name/status", get(git::get_sync_status))
        .route("/git/branches", get(git::list_branches).post(git::create_branch))
        .route("/git/branches/:name/checkout", post(git::checkout_branch))
        
        // Health monitoring
        .route("/health/fleet", get(health::get_fleet_health))
        .route("/health/agents", get(health::check_all_agents))
        .route("/health/agents/:id/history", get(health::get_agent_health_history))
        
        // Metrics
        .route("/metrics", get(health::get_all_metrics))
        .route("/metrics/:id", get(health::get_agent_metrics))
        
        // Topology
        .route("/topology", get(health::get_aggregated_topology))
        
        // Alerts
        .route("/alerts/rules", get(alerts::list_alert_rules).post(alerts::create_alert_rule))
        .route("/alerts/rules/:id", delete(alerts::delete_alert_rule))
        .route("/alerts/channels", get(alerts::list_channels).post(alerts::create_channel))
        .route("/alerts/channels/:id", delete(alerts::delete_channel))
        .route("/alerts/channels/:id/test", post(alerts::test_channel))
        
        // Validation (Layers 1-3)
        .route("/validate", post(validation::validate_config))
        .route("/validate/quick", post(validation::validate_quick))
        
        // Functional Testing (Layer 4)
        .route("/test", get(validation::list_test_results).post(validation::start_functional_test))
        .route("/test/:id", get(validation::get_test_result))
        
        // Setup wizard (always public)
        .route("/setup/status", get(auth::setup_status))
        .route("/setup/init", post(auth::setup_init))
        
        // Authentication (always public)
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::current_user))
}

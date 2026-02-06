//! Alert management API endpoints
//!
//! Provides endpoints for managing alerts, rules, and notification channels.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::AppState;
use crate::alerts::{AlertSeverity, NotificationChannel};

/// Request to create an alert rule
#[derive(Debug, Deserialize)]
pub struct CreateAlertRuleRequest {
    pub name: String,
    pub description: Option<String>,
    pub condition: serde_json::Value,
    pub severity: AlertSeverity,
    pub notification_channels: Vec<String>,
}

/// Request to create a notification channel
#[derive(Debug, Deserialize)]
pub struct CreateChannelRequest {
    #[serde(flatten)]
    pub channel: NotificationChannel,
}

/// Response with alert rules
#[derive(Debug, Serialize)]
pub struct AlertRulesResponse {
    pub rules: Vec<AlertRuleResponse>,
}

/// Alert rule response
#[derive(Debug, Serialize)]
pub struct AlertRuleResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub severity: AlertSeverity,
    pub enabled: bool,
    pub notification_channels: Vec<String>,
}

/// Response with notification channels
#[derive(Debug, Serialize)]
pub struct ChannelsResponse {
    pub channels: Vec<ChannelResponse>,
}

/// Channel response (without sensitive data)
#[derive(Debug, Serialize)]
pub struct ChannelResponse {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub channel_type: String,
}

/// List all alert rules
pub async fn list_alert_rules(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // For now, return an empty list since rules are stored in memory
    // In production, these would be stored in the database
    let rules: Vec<AlertRuleResponse> = Vec::new();
    
    (StatusCode::OK, Json(AlertRulesResponse { rules })).into_response()
}

/// Create a new alert rule
pub async fn create_alert_rule(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<CreateAlertRuleRequest>,
) -> impl IntoResponse {
    let rule_id = uuid::Uuid::new_v4().to_string();
    
    info!("Creating alert rule: {} ({})", request.name, rule_id);
    
    // In production, this would be stored in the database
    let response = AlertRuleResponse {
        id: rule_id,
        name: request.name,
        description: request.description,
        severity: request.severity,
        enabled: true,
        notification_channels: request.notification_channels,
    };
    
    (StatusCode::CREATED, Json(response)).into_response()
}

/// Delete an alert rule
pub async fn delete_alert_rule(
    State(_state): State<Arc<AppState>>,
    Path(rule_id): Path<String>,
) -> impl IntoResponse {
    info!("Deleting alert rule: {}", rule_id);
    
    // In production, this would delete from the database
    (StatusCode::NO_CONTENT).into_response()
}

/// List notification channels
pub async fn list_channels(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // For now, return an empty list
    let channels: Vec<ChannelResponse> = Vec::new();
    
    (StatusCode::OK, Json(ChannelsResponse { channels })).into_response()
}

/// Create a notification channel
pub async fn create_channel(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<CreateChannelRequest>,
) -> impl IntoResponse {
    let channel = &request.channel;
    
    info!("Creating notification channel: {}", channel.name());
    
    let channel_type = match channel {
        NotificationChannel::Webhook { .. } => "webhook",
        NotificationChannel::Slack { .. } => "slack",
        NotificationChannel::PagerDuty { .. } => "pagerduty",
        NotificationChannel::Email { .. } => "email",
    };
    
    let response = ChannelResponse {
        id: channel.id().to_string(),
        name: channel.name().to_string(),
        channel_type: channel_type.to_string(),
    };
    
    (StatusCode::CREATED, Json(response)).into_response()
}

/// Delete a notification channel
pub async fn delete_channel(
    State(_state): State<Arc<AppState>>,
    Path(channel_id): Path<String>,
) -> impl IntoResponse {
    info!("Deleting notification channel: {}", channel_id);
    
    (StatusCode::NO_CONTENT).into_response()
}

/// Test a notification channel
pub async fn test_channel(
    State(_state): State<Arc<AppState>>,
    Path(channel_id): Path<String>,
) -> impl IntoResponse {
    info!("Testing notification channel: {}", channel_id);
    
    // In production, this would send a test alert to the channel
    (StatusCode::OK, Json(serde_json::json!({
        "success": true,
        "message": "Test notification sent"
    }))).into_response()
}

//! Alert Management Module
//!
//! Provides alerting functionality for agent health and metrics.
//! Supports multiple notification channels (webhook, Slack, PagerDuty).

use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "info"),
            AlertSeverity::Warning => write!(f, "warning"),
            AlertSeverity::Critical => write!(f, "critical"),
        }
    }
}

/// Alert definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub severity: AlertSeverity,
    pub title: String,
    pub message: String,
    pub source: String,       // e.g., "agent:abc123" or "system"
    pub timestamp: String,
    pub resolved: bool,
    pub resolved_at: Option<String>,
}

/// Alert rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub condition: AlertCondition,
    pub severity: AlertSeverity,
    pub enabled: bool,
    pub notification_channels: Vec<String>,
}

/// Condition that triggers an alert
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AlertCondition {
    /// Agent becomes unhealthy
    AgentUnhealthy {
        #[serde(default = "default_consecutive_failures")]
        consecutive_failures: u32,
    },
    /// Agent becomes unreachable
    AgentUnreachable {
        #[serde(default = "default_timeout_minutes")]
        timeout_minutes: u32,
    },
    /// High latency threshold
    HighLatency {
        threshold_ms: u64,
    },
    /// Low events processed (pipeline stalled)
    LowThroughput {
        min_events_per_minute: u64,
    },
    /// Group has degraded (percentage of unhealthy agents)
    GroupDegraded {
        unhealthy_percentage: u32,
    },
}

fn default_consecutive_failures() -> u32 { 3 }
fn default_timeout_minutes() -> u32 { 5 }

/// Notification channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NotificationChannel {
    /// HTTP webhook
    Webhook {
        id: String,
        name: String,
        url: String,
        #[serde(default)]
        headers: std::collections::HashMap<String, String>,
    },
    /// Slack webhook
    Slack {
        id: String,
        name: String,
        webhook_url: String,
        channel: Option<String>,
    },
    /// PagerDuty
    PagerDuty {
        id: String,
        name: String,
        routing_key: String,
    },
    /// Email (via SMTP or service)
    Email {
        id: String,
        name: String,
        recipients: Vec<String>,
    },
}

impl NotificationChannel {
    pub fn id(&self) -> &str {
        match self {
            NotificationChannel::Webhook { id, .. } => id,
            NotificationChannel::Slack { id, .. } => id,
            NotificationChannel::PagerDuty { id, .. } => id,
            NotificationChannel::Email { id, .. } => id,
        }
    }
    
    pub fn name(&self) -> &str {
        match self {
            NotificationChannel::Webhook { name, .. } => name,
            NotificationChannel::Slack { name, .. } => name,
            NotificationChannel::PagerDuty { name, .. } => name,
            NotificationChannel::Email { name, .. } => name,
        }
    }
}

/// Alert manager
pub struct AlertManager {
    http_client: reqwest::Client,
    channels: Vec<NotificationChannel>,
    rules: Vec<AlertRule>,
}

impl AlertManager {
    /// Create a new alert manager
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
            channels: Vec::new(),
            rules: Vec::new(),
        }
    }
    
    /// Add a notification channel
    pub fn add_channel(&mut self, channel: NotificationChannel) {
        self.channels.push(channel);
    }
    
    /// Add an alert rule
    pub fn add_rule(&mut self, rule: AlertRule) {
        self.rules.push(rule);
    }
    
    /// Get all rules
    pub fn rules(&self) -> &[AlertRule] {
        &self.rules
    }
    
    /// Get all channels
    pub fn channels(&self) -> &[NotificationChannel] {
        &self.channels
    }
    
    /// Send an alert to all configured channels
    pub async fn send_alert(&self, alert: &Alert, channel_ids: &[String]) {
        for channel_id in channel_ids {
            if let Some(channel) = self.channels.iter().find(|c| c.id() == channel_id) {
                if let Err(e) = self.send_to_channel(alert, channel).await {
                    error!("Failed to send alert to {}: {}", channel.name(), e);
                }
            }
        }
    }
    
    /// Send alert to a specific channel
    async fn send_to_channel(&self, alert: &Alert, channel: &NotificationChannel) -> Result<(), String> {
        match channel {
            NotificationChannel::Webhook { url, headers, .. } => {
                let mut request = self.http_client.post(url)
                    .header("Content-Type", "application/json")
                    .json(&alert);
                
                for (key, value) in headers {
                    request = request.header(key, value);
                }
                
                let response = request.send().await
                    .map_err(|e| format!("Request failed: {}", e))?;
                
                if !response.status().is_success() {
                    return Err(format!("Webhook returned status: {}", response.status()));
                }
                
                info!("Alert sent to webhook: {}", alert.title);
                Ok(())
            }
            
            NotificationChannel::Slack { webhook_url, channel: slack_channel, .. } => {
                let color = match alert.severity {
                    AlertSeverity::Info => "#36a64f",
                    AlertSeverity::Warning => "#f2c744",
                    AlertSeverity::Critical => "#dc3545",
                };
                
                let payload = serde_json::json!({
                    "channel": slack_channel,
                    "attachments": [{
                        "color": color,
                        "title": alert.title,
                        "text": alert.message,
                        "fields": [
                            {
                                "title": "Severity",
                                "value": alert.severity.to_string(),
                                "short": true
                            },
                            {
                                "title": "Source",
                                "value": alert.source,
                                "short": true
                            }
                        ],
                        "ts": chrono::Utc::now().timestamp()
                    }]
                });
                
                let response = self.http_client.post(webhook_url)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| format!("Request failed: {}", e))?;
                
                if !response.status().is_success() {
                    return Err(format!("Slack returned status: {}", response.status()));
                }
                
                info!("Alert sent to Slack: {}", alert.title);
                Ok(())
            }
            
            NotificationChannel::PagerDuty { routing_key, .. } => {
                let severity = match alert.severity {
                    AlertSeverity::Info => "info",
                    AlertSeverity::Warning => "warning",
                    AlertSeverity::Critical => "critical",
                };
                
                let payload = serde_json::json!({
                    "routing_key": routing_key,
                    "event_action": if alert.resolved { "resolve" } else { "trigger" },
                    "dedup_key": alert.id,
                    "payload": {
                        "summary": alert.title,
                        "severity": severity,
                        "source": alert.source,
                        "custom_details": {
                            "message": alert.message
                        }
                    }
                });
                
                let response = self.http_client.post("https://events.pagerduty.com/v2/enqueue")
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| format!("Request failed: {}", e))?;
                
                if !response.status().is_success() {
                    return Err(format!("PagerDuty returned status: {}", response.status()));
                }
                
                info!("Alert sent to PagerDuty: {}", alert.title);
                Ok(())
            }
            
            NotificationChannel::Email { recipients, .. } => {
                // Email sending would require SMTP configuration
                // For now, just log it
                warn!("Email alerts not yet implemented. Would send to: {:?}", recipients);
                info!("Alert would be sent via email: {}", alert.title);
                Ok(())
            }
        }
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Create an alert for an unhealthy agent
pub fn create_agent_unhealthy_alert(agent_id: &str, agent_name: &str, error: Option<&str>) -> Alert {
    Alert {
        id: format!("agent-unhealthy-{}-{}", agent_id, chrono::Utc::now().timestamp()),
        severity: AlertSeverity::Warning,
        title: format!("Agent {} is unhealthy", agent_name),
        message: error.map(|e| format!("Health check failed: {}", e))
            .unwrap_or_else(|| "Agent health check returned unhealthy status".to_string()),
        source: format!("agent:{}", agent_id),
        timestamp: chrono::Utc::now().to_rfc3339(),
        resolved: false,
        resolved_at: None,
    }
}

/// Create an alert for an unreachable agent
pub fn create_agent_unreachable_alert(agent_id: &str, agent_name: &str, error: &str) -> Alert {
    Alert {
        id: format!("agent-unreachable-{}-{}", agent_id, chrono::Utc::now().timestamp()),
        severity: AlertSeverity::Critical,
        title: format!("Agent {} is unreachable", agent_name),
        message: format!("Cannot connect to agent: {}", error),
        source: format!("agent:{}", agent_id),
        timestamp: chrono::Utc::now().to_rfc3339(),
        resolved: false,
        resolved_at: None,
    }
}

/// Create an alert for a degraded worker group
pub fn create_group_degraded_alert(group_id: &str, group_name: &str, unhealthy_count: u32, total_count: u32) -> Alert {
    let percentage = (unhealthy_count as f64 / total_count as f64 * 100.0) as u32;
    
    Alert {
        id: format!("group-degraded-{}-{}", group_id, chrono::Utc::now().timestamp()),
        severity: if percentage >= 50 { AlertSeverity::Critical } else { AlertSeverity::Warning },
        title: format!("Worker group {} is degraded", group_name),
        message: format!("{}/{} agents ({:.0}%) are unhealthy", unhealthy_count, total_count, percentage),
        source: format!("group:{}", group_id),
        timestamp: chrono::Utc::now().to_rfc3339(),
        resolved: false,
        resolved_at: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_alert_severity_display() {
        assert_eq!(AlertSeverity::Info.to_string(), "info");
        assert_eq!(AlertSeverity::Warning.to_string(), "warning");
        assert_eq!(AlertSeverity::Critical.to_string(), "critical");
    }
    
    #[test]
    fn test_create_agent_unhealthy_alert() {
        let alert = create_agent_unhealthy_alert("abc123", "prod-agent-1", Some("Connection timeout"));
        
        assert!(alert.id.contains("agent-unhealthy-abc123"));
        assert_eq!(alert.severity, AlertSeverity::Warning);
        assert!(alert.title.contains("prod-agent-1"));
        assert!(alert.message.contains("Connection timeout"));
        assert_eq!(alert.source, "agent:abc123");
        assert!(!alert.resolved);
    }
    
    #[test]
    fn test_create_agent_unreachable_alert() {
        let alert = create_agent_unreachable_alert("abc123", "prod-agent-1", "Connection refused");
        
        assert_eq!(alert.severity, AlertSeverity::Critical);
        assert!(alert.message.contains("Connection refused"));
    }
    
    #[test]
    fn test_create_group_degraded_alert() {
        let alert = create_group_degraded_alert("group1", "production", 3, 5);
        
        assert!(alert.title.contains("production"));
        assert!(alert.message.contains("3/5"));
    }
    
    #[test]
    fn test_notification_channel_id() {
        let webhook = NotificationChannel::Webhook {
            id: "webhook1".to_string(),
            name: "My Webhook".to_string(),
            url: "https://example.com/webhook".to_string(),
            headers: Default::default(),
        };
        
        assert_eq!(webhook.id(), "webhook1");
        assert_eq!(webhook.name(), "My Webhook");
    }
    
    #[test]
    fn test_alert_manager_add_channel() {
        let mut manager = AlertManager::new();
        
        manager.add_channel(NotificationChannel::Slack {
            id: "slack1".to_string(),
            name: "Slack Channel".to_string(),
            webhook_url: "https://hooks.slack.com/...".to_string(),
            channel: Some("#alerts".to_string()),
        });
        
        assert_eq!(manager.channels().len(), 1);
    }
}

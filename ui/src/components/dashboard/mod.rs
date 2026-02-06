//! Dashboard Components
//!
//! Main dashboard view with fleet health, activity, alerts, and quick actions.

use leptos::*;
use serde::{Deserialize, Serialize};

mod health_card;
mod activity_feed;
mod alerts_panel;
mod quick_actions;

pub use health_card::HealthCard;
pub use activity_feed::ActivityFeed;
pub use alerts_panel::AlertsPanel;
pub use quick_actions::QuickActions;

/// Main dashboard view
#[component]
pub fn Dashboard() -> impl IntoView {
    view! {
        <div class="flex-1 overflow-auto p-6 bg-slate-900">
            <div class="max-w-7xl mx-auto">
                // Page header
                <div class="mb-6">
                    <h1 class="text-2xl font-bold text-white">"Dashboard"</h1>
                    <p class="text-slate-400 mt-1">"Overview of your Vector fleet"</p>
                </div>
                
                // Top row - Health and Quick Actions
                <div class="grid grid-cols-1 lg:grid-cols-3 gap-6 mb-6">
                    <div class="lg:col-span-2">
                        <HealthCard />
                    </div>
                    <div>
                        <QuickActions />
                    </div>
                </div>
                
                // Bottom row - Activity and Alerts
                <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                    <ActivityFeed />
                    <AlertsPanel />
                </div>
            </div>
        </div>
    }
}

/// Shared types for dashboard data

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FleetHealth {
    pub total_agents: u32,
    pub healthy: u32,
    pub unhealthy: u32,
    pub unknown: u32,
    pub version_distribution: Vec<VersionCount>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionCount {
    pub version: String,
    pub count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActivityItem {
    pub id: String,
    pub activity_type: ActivityType,
    pub description: String,
    pub timestamp: String,
    pub user: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityType {
    Deployment,
    ConfigChange,
    AgentRegistered,
    AgentRemoved,
    AlertTriggered,
    AlertResolved,
}

impl ActivityType {
    pub fn icon_class(&self) -> &'static str {
        match self {
            ActivityType::Deployment => "text-blue-400",
            ActivityType::ConfigChange => "text-purple-400",
            ActivityType::AgentRegistered => "text-green-400",
            ActivityType::AgentRemoved => "text-red-400",
            ActivityType::AlertTriggered => "text-amber-400",
            ActivityType::AlertResolved => "text-green-400",
        }
    }

    /// Returns a human-readable label for the activity type
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            ActivityType::Deployment => "Deployment",
            ActivityType::ConfigChange => "Config Change",
            ActivityType::AgentRegistered => "Agent Registered",
            ActivityType::AgentRemoved => "Agent Removed",
            ActivityType::AlertTriggered => "Alert Triggered",
            ActivityType::AlertResolved => "Alert Resolved",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlertSummary {
    pub id: String,
    pub severity: AlertSeverity,
    pub title: String,
    pub source: String,
    pub timestamp: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Critical,
    Warning,
    Info,
}

impl AlertSeverity {
    pub fn class(&self) -> &'static str {
        match self {
            AlertSeverity::Critical => "bg-red-500",
            AlertSeverity::Warning => "bg-amber-500",
            AlertSeverity::Info => "bg-blue-500",
        }
    }
    
    pub fn text_class(&self) -> &'static str {
        match self {
            AlertSeverity::Critical => "text-red-400",
            AlertSeverity::Warning => "text-amber-400",
            AlertSeverity::Info => "text-blue-400",
        }
    }
}

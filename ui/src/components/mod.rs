//! UI Components
//!
//! This module contains all UI components organized by feature:
//! - `layout`: Core layout components (AppShell, Sidebar, TopBar, BottomPanel)
//! - `dashboard`: Main dashboard overview
//! - `fleet`: Agent and topology management
//! - `pipeline`: Pipeline builder components
//! - `management`: Control plane management (groups, deployments)
//! - `observe`: Metrics, alerts, and audit logs
//! - `settings`: User and system settings
//! - `tap`: Live data sampling/tap viewer
//! - `common`: Shared/reusable components (legacy, being migrated to layout)
//! - `setup`: First-time setup wizard

pub mod layout;
pub mod common;
pub mod dashboard;
pub mod management;
pub mod observe;
pub mod pipeline;
pub mod settings;
pub mod setup;
pub mod tap;

//! Layout Components
//!
//! This module contains the core layout components for the Vectorize UI:
//! - `AppShell` - Main layout container with tabs, content, optional bottom panel
//! - `MainTabs` - Top-level horizontal tab navigation
//! - `Sidebar` - Collapsible icon/text navigation (for pipeline pages)
//! - `TopBar` - Contextual breadcrumbs, tabs, and actions
//! - `BottomPanel` - Resizable panel with tabs for data preview, logs, etc.
//! - `DataPreviewPanel` - Data preview tab content with toolbar
//! - `StatusBar` - Connection status and quick stats

mod app_shell;
mod main_tabs;
mod sidebar;
mod top_bar;
mod bottom_panel;
mod status_bar;

pub use app_shell::AppShell;
pub use main_tabs::MainTabs;
pub use sidebar::PipelineSidebar;
pub use bottom_panel::{BottomPanel, DataPreviewPanel};
pub use status_bar::StatusBar;

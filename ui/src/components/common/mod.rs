//! Common/Shared UI Components
//!
//! Reusable components used throughout the application.

mod header;
mod sidebar;
mod status_bar;
mod icons;

// Note: Header, Sidebar, and StatusBar are defined here but the app uses
// the versions from layout/ module. These are kept for potential future use.
#[allow(unused_imports)]
pub use header::Header;
#[allow(unused_imports)]
pub use sidebar::Sidebar;
#[allow(unused_imports)]
pub use status_bar::StatusBar;
pub use icons::*;

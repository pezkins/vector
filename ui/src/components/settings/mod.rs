//! Settings Components
//!
//! This module contains settings management components:
//! - `users`: User management (create, edit, delete users)
//! - `roles`: Role management (create, edit, delete roles with permissions)
//! - `api_keys`: API key management (create, revoke keys)
//! - `sso`: SSO configuration (OIDC, SAML)
//! - `git`: Git remote configuration (sync, push, pull)
//! - `system`: System settings (general, deployment, retention, features)

mod api_keys;
mod git;
mod roles;
mod sso;
mod system;
mod users;

// Re-exports for use by other modules (e.g., routing)
#[allow(unused_imports)]
pub use api_keys::ApiKeysPage;
#[allow(unused_imports)]
pub use git::GitRemotesPage;
#[allow(unused_imports)]
pub use roles::RoleManagement;
#[allow(unused_imports)]
pub use sso::SsoConfigPage;
#[allow(unused_imports)]
pub use system::SystemSettingsPage;
#[allow(unused_imports)]
pub use users::UserManagement;

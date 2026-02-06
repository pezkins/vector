//! Single Sign-On (SSO) Module
//!
//! Provides OIDC (OpenID Connect) integration for SSO authentication.
//! Supports providers like Okta, Auth0, Azure AD, Google, etc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

// =============================================================================
// SSO Provider Configuration
// =============================================================================

/// SSO provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoProviderConfig {
    /// Provider ID (unique identifier)
    pub id: String,
    /// Display name
    pub name: String,
    /// Provider type (oidc, saml)
    pub provider_type: SsoProviderType,
    /// Whether this provider is enabled
    pub enabled: bool,
    /// OIDC configuration (if provider_type is oidc)
    pub oidc: Option<OidcConfig>,
    /// SAML configuration (if provider_type is saml)
    pub saml: Option<SamlConfig>,
    /// Role mapping rules
    pub role_mapping: Option<RoleMappingConfig>,
}

/// SSO provider type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SsoProviderType {
    Oidc,
    Saml,
}

/// OIDC provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    /// Issuer URL (e.g., https://example.okta.com)
    pub issuer: String,
    /// Client ID
    pub client_id: String,
    /// Client secret (encrypted in storage)
    #[serde(skip_serializing)]
    pub client_secret: String,
    /// Authorization endpoint (auto-discovered if not set)
    pub authorization_endpoint: Option<String>,
    /// Token endpoint (auto-discovered if not set)
    pub token_endpoint: Option<String>,
    /// Userinfo endpoint (auto-discovered if not set)
    pub userinfo_endpoint: Option<String>,
    /// JWKS URI (auto-discovered if not set)
    pub jwks_uri: Option<String>,
    /// Requested scopes
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
}

fn default_scopes() -> Vec<String> {
    vec!["openid".into(), "profile".into(), "email".into()]
}

/// SAML provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlConfig {
    /// IdP Entity ID
    pub idp_entity_id: String,
    /// IdP SSO URL
    pub idp_sso_url: String,
    /// IdP certificate (PEM format)
    pub idp_certificate: String,
    /// SP Entity ID (our service)
    pub sp_entity_id: String,
    /// SP ACS URL (Assertion Consumer Service)
    pub sp_acs_url: String,
    /// Signed assertions required
    #[serde(default = "default_true")]
    pub want_assertions_signed: bool,
}

fn default_true() -> bool { true }

/// Role mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleMappingConfig {
    /// Default role for new users
    pub default_role: String,
    /// Claim name for roles (e.g., "groups", "roles")
    pub role_claim: String,
    /// Mapping from claim values to Vectorize roles
    pub mappings: HashMap<String, String>,
}

// =============================================================================
// OIDC Authentication Flow
// =============================================================================

/// OIDC state parameter (stored during auth flow)
#[derive(Debug, Serialize, Deserialize)]
pub struct OidcState {
    pub provider_id: String,
    pub nonce: String,
    pub return_url: Option<String>,
    pub created_at: i64,
}

/// OIDC token response
#[derive(Debug, Deserialize)]
pub struct OidcTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub id_token: Option<String>,
    pub refresh_token: Option<String>,
}

/// OIDC user info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcUserInfo {
    pub sub: String,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub name: Option<String>,
    pub preferred_username: Option<String>,
    pub picture: Option<String>,
    pub groups: Option<Vec<String>>,
}

/// SSO Manager
pub struct SsoManager {
    http_client: reqwest::Client,
    providers: HashMap<String, SsoProviderConfig>,
}

impl SsoManager {
    /// Create a new SSO manager
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
            providers: HashMap::new(),
        }
    }
    
    /// Add a provider
    pub fn add_provider(&mut self, config: SsoProviderConfig) {
        self.providers.insert(config.id.clone(), config);
    }
    
    /// Get a provider by ID
    pub fn get_provider(&self, id: &str) -> Option<&SsoProviderConfig> {
        self.providers.get(id)
    }
    
    /// List all providers
    pub fn list_providers(&self) -> Vec<&SsoProviderConfig> {
        self.providers.values().collect()
    }
    
    /// Generate OIDC authorization URL
    pub fn generate_auth_url(
        &self,
        provider_id: &str,
        redirect_uri: &str,
        state: &str,
        nonce: &str,
    ) -> Result<String, String> {
        let provider = self.providers.get(provider_id)
            .ok_or_else(|| "Provider not found".to_string())?;
        
        if !provider.enabled {
            return Err("Provider is disabled".to_string());
        }
        
        let oidc = provider.oidc.as_ref()
            .ok_or_else(|| "Not an OIDC provider".to_string())?;
        
        let auth_endpoint = oidc.authorization_endpoint.as_ref()
            .map(|s| s.clone())
            .unwrap_or_else(|| format!("{}/oauth2/v1/authorize", oidc.issuer));
        
        let scopes = oidc.scopes.join(" ");
        
        let params = [
            ("client_id", oidc.client_id.as_str()),
            ("response_type", "code"),
            ("redirect_uri", redirect_uri),
            ("scope", &scopes),
            ("state", state),
            ("nonce", nonce),
        ];
        
        let query = params.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        
        Ok(format!("{}?{}", auth_endpoint, query))
    }
    
    /// Exchange authorization code for tokens
    pub async fn exchange_code(
        &self,
        provider_id: &str,
        code: &str,
        redirect_uri: &str,
    ) -> Result<OidcTokenResponse, String> {
        let provider = self.providers.get(provider_id)
            .ok_or_else(|| "Provider not found".to_string())?;
        
        let oidc = provider.oidc.as_ref()
            .ok_or_else(|| "Not an OIDC provider".to_string())?;
        
        let token_endpoint = oidc.token_endpoint.as_ref()
            .map(|s| s.clone())
            .unwrap_or_else(|| format!("{}/oauth2/v1/token", oidc.issuer));
        
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", &oidc.client_id),
            ("client_secret", &oidc.client_secret),
        ];
        
        let response = self.http_client
            .post(&token_endpoint)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Token request failed: {}", e))?;
        
        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("Token exchange failed: {}", error));
        }
        
        response.json::<OidcTokenResponse>()
            .await
            .map_err(|e| format!("Failed to parse token response: {}", e))
    }
    
    /// Get user info from OIDC provider
    pub async fn get_user_info(
        &self,
        provider_id: &str,
        access_token: &str,
    ) -> Result<OidcUserInfo, String> {
        let provider = self.providers.get(provider_id)
            .ok_or_else(|| "Provider not found".to_string())?;
        
        let oidc = provider.oidc.as_ref()
            .ok_or_else(|| "Not an OIDC provider".to_string())?;
        
        let userinfo_endpoint = oidc.userinfo_endpoint.as_ref()
            .map(|s| s.clone())
            .unwrap_or_else(|| format!("{}/oauth2/v1/userinfo", oidc.issuer));
        
        let response = self.http_client
            .get(&userinfo_endpoint)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| format!("Userinfo request failed: {}", e))?;
        
        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("Failed to get user info: {}", error));
        }
        
        response.json::<OidcUserInfo>()
            .await
            .map_err(|e| format!("Failed to parse user info: {}", e))
    }
    
    /// Map user groups/roles to Vectorize role
    pub fn map_role(
        &self,
        provider_id: &str,
        user_info: &OidcUserInfo,
    ) -> String {
        let provider = match self.providers.get(provider_id) {
            Some(p) => p,
            None => return "viewer".to_string(),
        };
        
        let mapping = match &provider.role_mapping {
            Some(m) => m,
            None => return "viewer".to_string(),
        };
        
        // Check if user has any mapped groups
        if let Some(groups) = &user_info.groups {
            for group in groups {
                if let Some(role) = mapping.mappings.get(group) {
                    debug!("Mapped group '{}' to role '{}'", group, role);
                    return role.clone();
                }
            }
        }
        
        // Return default role
        mapping.default_role.clone()
    }
}

impl Default for SsoManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Generate a random state parameter
pub fn generate_state() -> String {
    use rand::Rng;
    let random_bytes: [u8; 32] = rand::thread_rng().gen();
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, random_bytes)
}

/// Generate a random nonce
pub fn generate_nonce() -> String {
    use rand::Rng;
    let random_bytes: [u8; 16] = rand::thread_rng().gen();
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, random_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_state() {
        let state = generate_state();
        assert!(!state.is_empty());
        assert!(state.len() > 20);
    }
    
    #[test]
    fn test_sso_provider_config() {
        let config = SsoProviderConfig {
            id: "okta".to_string(),
            name: "Okta".to_string(),
            provider_type: SsoProviderType::Oidc,
            enabled: true,
            oidc: Some(OidcConfig {
                issuer: "https://example.okta.com".to_string(),
                client_id: "client123".to_string(),
                client_secret: "secret".to_string(),
                authorization_endpoint: None,
                token_endpoint: None,
                userinfo_endpoint: None,
                jwks_uri: None,
                scopes: default_scopes(),
            }),
            saml: None,
            role_mapping: Some(RoleMappingConfig {
                default_role: "viewer".to_string(),
                role_claim: "groups".to_string(),
                mappings: [
                    ("admins".to_string(), "admin".to_string()),
                    ("operators".to_string(), "operator".to_string()),
                ].into_iter().collect(),
            }),
        };
        
        assert_eq!(config.id, "okta");
        assert!(config.enabled);
    }
    
    #[test]
    fn test_role_mapping() {
        let mut manager = SsoManager::new();
        
        manager.add_provider(SsoProviderConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            provider_type: SsoProviderType::Oidc,
            enabled: true,
            oidc: Some(OidcConfig {
                issuer: "https://test.com".to_string(),
                client_id: "client".to_string(),
                client_secret: "secret".to_string(),
                authorization_endpoint: None,
                token_endpoint: None,
                userinfo_endpoint: None,
                jwks_uri: None,
                scopes: default_scopes(),
            }),
            saml: None,
            role_mapping: Some(RoleMappingConfig {
                default_role: "viewer".to_string(),
                role_claim: "groups".to_string(),
                mappings: [
                    ("admins".to_string(), "admin".to_string()),
                ].into_iter().collect(),
            }),
        });
        
        // User with admin group
        let admin_user = OidcUserInfo {
            sub: "user1".to_string(),
            email: Some("user@example.com".to_string()),
            email_verified: Some(true),
            name: Some("User".to_string()),
            preferred_username: None,
            picture: None,
            groups: Some(vec!["admins".to_string()]),
        };
        
        assert_eq!(manager.map_role("test", &admin_user), "admin");
        
        // User without special group
        let regular_user = OidcUserInfo {
            sub: "user2".to_string(),
            email: Some("user2@example.com".to_string()),
            email_verified: Some(true),
            name: None,
            preferred_username: None,
            picture: None,
            groups: Some(vec!["users".to_string()]),
        };
        
        assert_eq!(manager.map_role("test", &regular_user), "viewer");
    }
}

//! Remote repository operations for git store
//!
//! Handles push, pull, and sync with remote git repositories.
//! This module is prepared for future remote sync phase.

#![allow(dead_code)]

use git2::{
    Cred, FetchOptions, PushOptions, RemoteCallbacks,
    Repository,
};
use std::path::Path;
use tracing::{info, warn, error};

use super::GitStoreError;

/// Remote repository configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RemoteConfig {
    pub name: String,
    pub url: String,
    pub branch: String,
    pub auto_push: bool,
    pub auto_pull: bool,
    pub auth: RemoteAuth,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            name: "origin".to_string(),
            url: String::new(),
            branch: "main".to_string(),
            auto_push: false,
            auto_pull: false,
            auth: RemoteAuth::None,
        }
    }
}

/// Authentication method for remote
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RemoteAuth {
    None,
    SshKey {
        #[serde(default)]
        private_key_path: Option<String>,
    },
    Token {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
}

impl Default for RemoteAuth {
    fn default() -> Self {
        RemoteAuth::None
    }
}

/// Remote operations manager
pub struct RemoteManager<'a> {
    repo: &'a Repository,
}

impl<'a> RemoteManager<'a> {
    pub fn new(repo: &'a Repository) -> Self {
        Self { repo }
    }
    
    /// Add a remote
    pub fn add_remote(&self, name: &str, url: &str) -> Result<(), GitStoreError> {
        self.repo.remote(name, url)?;
        info!("Added remote '{}' with URL: {}", name, url);
        Ok(())
    }
    
    /// Remove a remote
    pub fn remove_remote(&self, name: &str) -> Result<(), GitStoreError> {
        self.repo.remote_delete(name)?;
        info!("Removed remote '{}'", name);
        Ok(())
    }
    
    /// List remotes
    pub fn list_remotes(&self) -> Result<Vec<(String, String)>, GitStoreError> {
        let remotes = self.repo.remotes()?;
        let mut result = Vec::new();
        
        for name in remotes.iter().flatten() {
            if let Ok(remote) = self.repo.find_remote(name) {
                let url = remote.url().unwrap_or("").to_string();
                result.push((name.to_string(), url));
            }
        }
        
        Ok(result)
    }
    
    /// Check if a remote exists
    pub fn has_remote(&self, name: &str) -> bool {
        self.repo.find_remote(name).is_ok()
    }
    
    /// Fetch from remote
    pub fn fetch(&self, remote_name: &str, auth: &RemoteAuth) -> Result<(), GitStoreError> {
        let mut remote = self.repo.find_remote(remote_name)?;
        
        let callbacks = self.create_callbacks(auth);
        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);
        
        info!("Fetching from remote '{}'...", remote_name);
        remote.fetch::<&str>(&[], Some(&mut fetch_options), None)?;
        info!("Fetch complete");
        
        Ok(())
    }
    
    /// Pull from remote (fetch + merge)
    pub fn pull(&self, remote_name: &str, branch: &str, auth: &RemoteAuth) -> Result<(), GitStoreError> {
        // First fetch
        self.fetch(remote_name, auth)?;
        
        // Get the fetch head
        let fetch_head = self.repo.find_reference("FETCH_HEAD")?;
        let fetch_commit = self.repo.reference_to_annotated_commit(&fetch_head)?;
        
        // Perform merge analysis
        let (analysis, _) = self.repo.merge_analysis(&[&fetch_commit])?;
        
        if analysis.is_up_to_date() {
            info!("Already up to date");
            return Ok(());
        }
        
        if analysis.is_fast_forward() {
            // Fast-forward merge
            let refname = format!("refs/heads/{}", branch);
            let mut reference = self.repo.find_reference(&refname)?;
            reference.set_target(fetch_commit.id(), "Fast-forward pull")?;
            self.repo.set_head(&refname)?;
            self.repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
            info!("Fast-forward merge complete");
        } else if analysis.is_normal() {
            // Normal merge required
            warn!("Merge required - this may cause conflicts");
            return Err(GitStoreError::Conflict(
                "Remote has diverged. Please resolve conflicts manually.".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Push to remote
    pub fn push(&self, remote_name: &str, branch: &str, auth: &RemoteAuth) -> Result<(), GitStoreError> {
        let mut remote = self.repo.find_remote(remote_name)?;
        
        let callbacks = self.create_callbacks(auth);
        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);
        
        let refspec = format!("refs/heads/{}:refs/heads/{}", branch, branch);
        
        info!("Pushing to remote '{}'...", remote_name);
        remote.push::<&str>(&[&refspec], Some(&mut push_options))?;
        info!("Push complete");
        
        Ok(())
    }
    
    /// Get sync status (commits ahead/behind remote)
    pub fn sync_status(&self, remote_name: &str, branch: &str) -> Result<SyncStatus, GitStoreError> {
        let local_ref = format!("refs/heads/{}", branch);
        let remote_ref = format!("refs/remotes/{}/{}", remote_name, branch);
        
        let local_oid = match self.repo.find_reference(&local_ref) {
            Ok(r) => r.target(),
            Err(_) => return Ok(SyncStatus::default()),
        };
        
        let remote_oid = match self.repo.find_reference(&remote_ref) {
            Ok(r) => r.target(),
            Err(_) => {
                return Ok(SyncStatus {
                    ahead: 0,
                    behind: 0,
                    local_head: local_oid.map(|o| o.to_string()),
                    remote_head: None,
                    synced: false,
                });
            }
        };
        
        let (ahead, behind) = match (local_oid, remote_oid) {
            (Some(local), Some(remote)) => {
                self.repo.graph_ahead_behind(local, remote)?
            }
            _ => (0, 0),
        };
        
        Ok(SyncStatus {
            ahead,
            behind,
            local_head: local_oid.map(|o| o.to_string()),
            remote_head: remote_oid.map(|o| o.to_string()),
            synced: ahead == 0 && behind == 0,
        })
    }
    
    /// Create git callbacks with authentication
    fn create_callbacks(&self, auth: &RemoteAuth) -> RemoteCallbacks<'a> {
        let auth = auth.clone();
        let mut callbacks = RemoteCallbacks::new();
        
        callbacks.credentials(move |_url, username_from_url, allowed_types| {
            match &auth {
                RemoteAuth::None => {
                    // Try SSH agent first
                    if allowed_types.contains(git2::CredentialType::SSH_KEY) {
                        let username = username_from_url.unwrap_or("git");
                        Cred::ssh_key_from_agent(username)
                    } else {
                        Cred::default()
                    }
                }
                RemoteAuth::SshKey { private_key_path } => {
                    let username = username_from_url.unwrap_or("git");
                    if let Some(key_path) = private_key_path {
                        Cred::ssh_key(
                            username,
                            None,
                            Path::new(key_path),
                            None,
                        )
                    } else {
                        // Try default SSH key locations
                        let home = dirs::home_dir().unwrap_or_default();
                        let default_key = home.join(".ssh").join("id_rsa");
                        if default_key.exists() {
                            Cred::ssh_key(username, None, &default_key, None)
                        } else {
                            Cred::ssh_key_from_agent(username)
                        }
                    }
                }
                RemoteAuth::Token { token } => {
                    // Use token as password with 'git' or 'oauth2' as username
                    let username = username_from_url.unwrap_or("oauth2");
                    Cred::userpass_plaintext(username, token)
                }
                RemoteAuth::Basic { username, password } => {
                    Cred::userpass_plaintext(username, password)
                }
            }
        });
        
        callbacks.push_update_reference(|refname, status| {
            if let Some(msg) = status {
                error!("Failed to push {}: {}", refname, msg);
            } else {
                info!("Updated {}", refname);
            }
            Ok(())
        });
        
        callbacks
    }
}

/// Sync status between local and remote
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SyncStatus {
    pub ahead: usize,
    pub behind: usize,
    pub local_head: Option<String>,
    pub remote_head: Option<String>,
    pub synced: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_remote_config_default() {
        let config = RemoteConfig::default();
        assert_eq!(config.name, "origin");
        assert_eq!(config.branch, "main");
        assert!(!config.auto_push);
    }
}

//! Git-based configuration store for Vectorize
//!
//! Provides version-controlled configuration storage with:
//! - Local-first design (works without external services)
//! - Auto-commit on config changes
//! - Version history and rollback
//! - Optional remote sync (GitHub, GitLab, etc.)

pub mod repository;

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use git2::{Repository, Signature, IndexAddOption};
use tracing::info;
use thiserror::Error;

/// Git store errors
#[derive(Error, Debug)]
pub enum GitStoreError {
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Repository not initialized")]
    NotInitialized,
    
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("Conflict detected: {0}")]
    Conflict(String),
    
    #[error("Lock error: {0}")]
    Lock(String),
}

/// Git-based configuration store
/// Thread-safe via internal Mutex
pub struct GitStore {
    repo: Mutex<Repository>,
    path: PathBuf,
}

impl GitStore {
    /// Open or initialize a git repository at the given path
    pub fn open_or_init(path: &Path) -> Result<Self, GitStoreError> {
        // Ensure the directory exists
        std::fs::create_dir_all(path)?;
        
        let repo = match Repository::open(path) {
            Ok(repo) => {
                info!("Opened existing git repository at {}", path.display());
                repo
            }
            Err(_) => {
                info!("Initializing new git repository at {}", path.display());
                let repo = Repository::init(path)?;
                
                // Create initial directory structure
                Self::create_directory_structure(path)?;
                
                // Create initial commit
                Self::create_initial_commit(&repo)?;
                
                repo
            }
        };
        
        Ok(Self {
            repo: Mutex::new(repo),
            path: path.to_path_buf(),
        })
    }
    
    /// Lock the repository for operations
    fn lock_repo(&self) -> Result<std::sync::MutexGuard<'_, Repository>, GitStoreError> {
        self.repo.lock().map_err(|e| GitStoreError::Lock(e.to_string()))
    }
    
    /// Create the default directory structure
    fn create_directory_structure(path: &Path) -> Result<(), GitStoreError> {
        // Create directories
        let dirs = [
            "groups",
            "templates",
            ".vectorize",
        ];
        
        for dir in dirs {
            std::fs::create_dir_all(path.join(dir))?;
        }
        
        // Create .gitkeep files to ensure directories are tracked
        for dir in dirs {
            let gitkeep = path.join(dir).join(".gitkeep");
            if !gitkeep.exists() {
                std::fs::write(&gitkeep, "")?;
            }
        }
        
        // Create README
        let readme_content = r#"# Vectorize Configuration Repository

This repository contains Vector pipeline configurations managed by Vectorize.

## Directory Structure

- `groups/` - Worker group configurations
  - `{group-name}/`
    - `config.toml` - Vector configuration
    - `group.yaml` - Group metadata
- `templates/` - Reusable configuration templates
- `.vectorize/` - Vectorize internal state

## Usage

This repository is managed automatically by Vectorize. Manual edits are supported
but should be committed through Vectorize for proper tracking.

For remote sync, configure a git remote in Vectorize settings.
"#;
        std::fs::write(path.join("README.md"), readme_content)?;
        
        // Create state file
        let state_content = "# Vectorize State\n# Auto-generated - do not edit manually\n\nversion: 1\n";
        std::fs::write(path.join(".vectorize").join("state.yaml"), state_content)?;
        
        Ok(())
    }
    
    /// Create the initial commit
    fn create_initial_commit(repo: &Repository) -> Result<(), GitStoreError> {
        let mut index = repo.index()?;
        
        // Add all files
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        index.write()?;
        
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        
        let sig = Self::default_signature(repo)?;
        
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Initial Vectorize configuration repository",
            &tree,
            &[],  // No parent for initial commit
        )?;
        
        info!("Created initial commit");
        Ok(())
    }
    
    /// Get default signature for commits
    fn default_signature(repo: &Repository) -> Result<Signature<'static>, GitStoreError> {
        // Try to get from git config, fall back to defaults
        match repo.signature() {
            Ok(sig) => Ok(sig),
            Err(_) => {
                Ok(Signature::now("Vectorize", "vectorize@local")?)
            }
        }
    }
    
    /// Get the repository path
    pub fn path(&self) -> &Path {
        &self.path
    }
    
    /// Get path to a group's directory
    pub fn group_path(&self, group_name: &str) -> PathBuf {
        self.path.join("groups").join(group_name)
    }
    
    /// Get path to a group's config file
    pub fn group_config_path(&self, group_name: &str) -> PathBuf {
        self.group_path(group_name).join("config.toml")
    }
    
    /// Create a new group directory
    pub fn create_group(&self, group_name: &str) -> Result<PathBuf, GitStoreError> {
        let group_path = self.group_path(group_name);
        std::fs::create_dir_all(&group_path)?;
        
        // Create default group.yaml
        let group_yaml = format!(
            "# Group: {}\nname: {}\ncreated_at: {}\n",
            group_name,
            group_name,
            chrono::Utc::now().to_rfc3339()
        );
        std::fs::write(group_path.join("group.yaml"), group_yaml)?;
        
        // Create empty config
        let config = format!(
            "# Vector Configuration for {}\n# Managed by Vectorize\n\n[api]\nenabled = true\naddress = \"0.0.0.0:8686\"\n",
            group_name
        );
        std::fs::write(group_path.join("config.toml"), config)?;
        
        // Commit the new group
        self.commit(&format!("Create group: {}", group_name))?;
        
        Ok(group_path)
    }
    
    /// Write config for a group
    pub fn write_config(&self, group_name: &str, config: &str) -> Result<String, GitStoreError> {
        let config_path = self.group_config_path(group_name);
        
        // Ensure group directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        std::fs::write(&config_path, config)?;
        
        // Commit the change
        let commit_hash = self.commit(&format!("Update config for group: {}", group_name))?;
        
        Ok(commit_hash)
    }
    
    /// Read config for a group
    pub fn read_config(&self, group_name: &str) -> Result<Option<String>, GitStoreError> {
        let config_path = self.group_config_path(group_name);
        
        if config_path.exists() {
            Ok(Some(std::fs::read_to_string(config_path)?))
        } else {
            Ok(None)
        }
    }
    
    /// Delete a group
    pub fn delete_group(&self, group_name: &str) -> Result<(), GitStoreError> {
        let group_path = self.group_path(group_name);
        
        if group_path.exists() {
            std::fs::remove_dir_all(&group_path)?;
            self.commit(&format!("Delete group: {}", group_name))?;
        }
        
        Ok(())
    }
    
    /// Commit all changes
    pub fn commit(&self, message: &str) -> Result<String, GitStoreError> {
        let repo = self.lock_repo()?;
        
        let mut index = repo.index()?;
        
        // Add all changes (including deletions)
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        
        // Also handle deletions by updating the index
        index.update_all(["*"].iter(), None)?;
        
        index.write()?;
        
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        
        let sig = Self::default_signature(&repo)?;
        
        // Get parent commit (HEAD)
        let parent = match repo.head() {
            Ok(head) => {
                let commit = head.peel_to_commit()?;
                Some(commit)
            }
            Err(_) => None,
        };
        
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        
        let oid = repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &parents,
        )?;
        
        let commit_hash = oid.to_string();
        info!("Committed: {} ({})", message, &commit_hash[..8]);
        
        Ok(commit_hash)
    }
    
    /// Get version history for a group
    pub fn get_history(&self, group_name: Option<&str>, limit: usize) -> Result<Vec<CommitInfo>, GitStoreError> {
        let repo = self.lock_repo()?;
        
        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME)?;
        
        let path_filter = group_name.map(|name| format!("groups/{}/", name));
        
        let mut commits = Vec::new();
        
        for oid_result in revwalk {
            if commits.len() >= limit {
                break;
            }
            
            let oid = oid_result?;
            let commit = repo.find_commit(oid)?;
            
            // If filtering by group, check if commit affects that path
            if let Some(ref _path) = path_filter {
                // For simplicity, we include all commits for now
                // A more sophisticated implementation would check the diff
                let message = commit.message().unwrap_or("");
                if !message.contains(group_name.unwrap_or("")) && !commits.is_empty() {
                    // Skip commits not related to this group (except first few)
                    continue;
                }
            }
            
            commits.push(CommitInfo {
                hash: oid.to_string(),
                short_hash: oid.to_string()[..8].to_string(),
                message: commit.message().unwrap_or("").to_string(),
                author: commit.author().name().unwrap_or("Unknown").to_string(),
                timestamp: chrono::DateTime::from_timestamp(commit.time().seconds(), 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default(),
            });
        }
        
        Ok(commits)
    }
    
    /// Get config at a specific version
    pub fn get_config_at_version(&self, group_name: &str, commit_hash: &str) -> Result<Option<String>, GitStoreError> {
        let repo = self.lock_repo()?;
        
        let oid = git2::Oid::from_str(commit_hash)?;
        let commit = repo.find_commit(oid)?;
        let tree = commit.tree()?;
        
        let path = format!("groups/{}/config.toml", group_name);
        
        match tree.get_path(Path::new(&path)) {
            Ok(entry) => {
                let blob = repo.find_blob(entry.id())?;
                let content = std::str::from_utf8(blob.content())
                    .map_err(|e| GitStoreError::InvalidPath(e.to_string()))?;
                Ok(Some(content.to_string()))
            }
            Err(_) => Ok(None),
        }
    }
    
    /// Rollback to a specific version
    pub fn rollback(&self, group_name: &str, commit_hash: &str) -> Result<String, GitStoreError> {
        // Get the config at the specified version
        let old_config = self.get_config_at_version(group_name, commit_hash)?
            .ok_or_else(|| GitStoreError::InvalidPath(format!("Config not found at version {}", commit_hash)))?;
        
        // Write it as the current config
        let new_hash = self.write_config(group_name, &old_config)?;
        
        info!("Rolled back {} to version {}", group_name, &commit_hash[..8]);
        
        Ok(new_hash)
    }
    
    /// Get diff between two versions
    pub fn diff(&self, from_hash: &str, to_hash: &str) -> Result<String, GitStoreError> {
        let repo = self.lock_repo()?;
        
        let from_oid = git2::Oid::from_str(from_hash)?;
        let to_oid = git2::Oid::from_str(to_hash)?;
        
        let from_commit = repo.find_commit(from_oid)?;
        let to_commit = repo.find_commit(to_oid)?;
        
        let from_tree = from_commit.tree()?;
        let to_tree = to_commit.tree()?;
        
        let diff = repo.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)?;
        
        let mut diff_text = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let prefix = match line.origin() {
                '+' => "+",
                '-' => "-",
                ' ' => " ",
                _ => "",
            };
            diff_text.push_str(prefix);
            diff_text.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
            true
        })?;
        
        Ok(diff_text)
    }
    
    /// Check if there are uncommitted changes
    pub fn has_changes(&self) -> Result<bool, GitStoreError> {
        let repo = self.lock_repo()?;
        let statuses = repo.statuses(None)?;
        Ok(!statuses.is_empty())
    }
    
    /// Get current HEAD commit hash
    pub fn head_hash(&self) -> Result<String, GitStoreError> {
        let repo = self.lock_repo()?;
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit.id().to_string())
    }
    
    // =========================================================================
    // Remote Repository Operations
    // =========================================================================
    
    /// Configure a remote repository
    pub fn configure_remote(&self, name: &str, url: &str) -> Result<(), GitStoreError> {
        let repo = self.lock_repo()?;
        
        // Check if remote already exists
        if repo.find_remote(name).is_ok() {
            // Update existing remote
            repo.remote_set_url(name, url)?;
            info!("Updated remote '{}' to {}", name, url);
        } else {
            // Create new remote
            repo.remote(name, url)?;
            info!("Added remote '{}' at {}", name, url);
        }
        
        Ok(())
    }
    
    /// Remove a remote
    pub fn remove_remote(&self, name: &str) -> Result<(), GitStoreError> {
        let repo = self.lock_repo()?;
        repo.remote_delete(name)?;
        info!("Removed remote '{}'", name);
        Ok(())
    }
    
    /// List configured remotes
    pub fn list_remotes(&self) -> Result<Vec<RemoteInfo>, GitStoreError> {
        let repo = self.lock_repo()?;
        let remotes = repo.remotes()?;
        
        let mut result = Vec::new();
        for name in remotes.iter().flatten() {
            if let Ok(remote) = repo.find_remote(name) {
                result.push(RemoteInfo {
                    name: name.to_string(),
                    url: remote.url().unwrap_or("").to_string(),
                    push_url: remote.pushurl().map(|s| s.to_string()),
                });
            }
        }
        
        Ok(result)
    }
    
    /// Push to a remote repository
    /// 
    /// Note: This uses command-line git for authentication support.
    /// For programmatic use with SSH keys or tokens, configure git credentials externally.
    pub fn push(&self, remote: &str, branch: Option<&str>) -> Result<PushResult, GitStoreError> {
        let branch = branch.unwrap_or("main");
        
        // Use command-line git for better auth support
        let output = std::process::Command::new("git")
            .current_dir(&self.path)
            .args(["push", "-u", remote, branch])
            .output()
            .map_err(|e| GitStoreError::Git(git2::Error::from_str(&e.to_string())))?;
        
        if output.status.success() {
            info!("Pushed to {} ({})", remote, branch);
            Ok(PushResult {
                success: true,
                remote: remote.to_string(),
                branch: branch.to_string(),
                message: String::from_utf8_lossy(&output.stdout).to_string(),
            })
        } else {
            let error = String::from_utf8_lossy(&output.stderr).to_string();
            Err(GitStoreError::Git(git2::Error::from_str(&error)))
        }
    }
    
    /// Fetch from a remote repository
    pub fn fetch(&self, remote: &str) -> Result<(), GitStoreError> {
        let output = std::process::Command::new("git")
            .current_dir(&self.path)
            .args(["fetch", remote])
            .output()
            .map_err(|e| GitStoreError::Git(git2::Error::from_str(&e.to_string())))?;
        
        if output.status.success() {
            info!("Fetched from {}", remote);
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr).to_string();
            Err(GitStoreError::Git(git2::Error::from_str(&error)))
        }
    }
    
    /// Pull from a remote repository
    pub fn pull(&self, remote: &str, branch: Option<&str>) -> Result<PullResult, GitStoreError> {
        let branch = branch.unwrap_or("main");
        
        let output = std::process::Command::new("git")
            .current_dir(&self.path)
            .args(["pull", remote, branch])
            .output()
            .map_err(|e| GitStoreError::Git(git2::Error::from_str(&e.to_string())))?;
        
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let already_up_to_date = stdout.contains("Already up to date");
            
            info!("Pulled from {} ({})", remote, branch);
            Ok(PullResult {
                success: true,
                remote: remote.to_string(),
                branch: branch.to_string(),
                updated: !already_up_to_date,
                message: stdout,
            })
        } else {
            let error = String::from_utf8_lossy(&output.stderr).to_string();
            
            // Check for merge conflicts
            if error.contains("CONFLICT") || error.contains("Automatic merge failed") {
                return Err(GitStoreError::Conflict(error));
            }
            
            Err(GitStoreError::Git(git2::Error::from_str(&error)))
        }
    }
    
    /// Get sync status with remote
    pub fn sync_status(&self, remote: &str, branch: Option<&str>) -> Result<SyncStatus, GitStoreError> {
        let branch = branch.unwrap_or("main");
        
        // Fetch first to get latest remote state
        self.fetch(remote)?;
        
        // Get local and remote refs
        let output = std::process::Command::new("git")
            .current_dir(&self.path)
            .args(["rev-list", "--left-right", "--count", &format!("{}...{}/{}", branch, remote, branch)])
            .output()
            .map_err(|e| GitStoreError::Git(git2::Error::from_str(&e.to_string())))?;
        
        if output.status.success() {
            let counts = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = counts.trim().split_whitespace().collect();
            
            let ahead = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
            let behind = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            
            Ok(SyncStatus {
                remote: remote.to_string(),
                branch: branch.to_string(),
                ahead,
                behind,
                synced: ahead == 0 && behind == 0,
            })
        } else {
            // Remote branch might not exist
            Ok(SyncStatus {
                remote: remote.to_string(),
                branch: branch.to_string(),
                ahead: 0,
                behind: 0,
                synced: true,
            })
        }
    }
    
    /// Sync bidirectionally with remote (pull then push)
    pub fn sync(&self, remote: &str, branch: Option<&str>) -> Result<SyncResult, GitStoreError> {
        let branch = branch.unwrap_or("main");
        
        // Pull first
        let pull_result = self.pull(remote, Some(branch));
        let pull_success = pull_result.is_ok();
        let pull_error = pull_result.err().map(|e| e.to_string());
        
        if !pull_success {
            if let Some(ref err) = pull_error {
                if err.contains("CONFLICT") {
                    return Err(GitStoreError::Conflict(err.clone()));
                }
            }
        }
        
        // Push if pull succeeded
        let (push_success, push_error) = if pull_success {
            match self.push(remote, Some(branch)) {
                Ok(_) => (true, None),
                Err(e) => (false, Some(e.to_string())),
            }
        } else {
            (false, None)
        };
        
        Ok(SyncResult {
            remote: remote.to_string(),
            branch: branch.to_string(),
            pull_success,
            pull_error,
            push_success,
            push_error,
        })
    }
    
    /// Create a new branch
    pub fn create_branch(&self, name: &str) -> Result<(), GitStoreError> {
        let repo = self.lock_repo()?;
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        
        repo.branch(name, &commit, false)?;
        info!("Created branch '{}'", name);
        Ok(())
    }
    
    /// Switch to a branch
    pub fn checkout_branch(&self, name: &str) -> Result<(), GitStoreError> {
        let output = std::process::Command::new("git")
            .current_dir(&self.path)
            .args(["checkout", name])
            .output()
            .map_err(|e| GitStoreError::Git(git2::Error::from_str(&e.to_string())))?;
        
        if output.status.success() {
            info!("Checked out branch '{}'", name);
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr).to_string();
            Err(GitStoreError::Git(git2::Error::from_str(&error)))
        }
    }
    
    /// List branches
    pub fn list_branches(&self) -> Result<Vec<BranchInfo>, GitStoreError> {
        let repo = self.lock_repo()?;
        let branches = repo.branches(None)?;
        
        let head = repo.head().ok().and_then(|h| h.shorthand().map(|s| s.to_string()));
        
        let mut result = Vec::new();
        for branch in branches {
            let (branch, branch_type) = branch?;
            if let Some(name) = branch.name()? {
                result.push(BranchInfo {
                    name: name.to_string(),
                    is_current: Some(name.to_string()) == head,
                    is_remote: matches!(branch_type, git2::BranchType::Remote),
                });
            }
        }
        
        Ok(result)
    }
    
    /// Get current branch name
    pub fn current_branch(&self) -> Result<String, GitStoreError> {
        let repo = self.lock_repo()?;
        let head = repo.head()?;
        Ok(head.shorthand().unwrap_or("HEAD").to_string())
    }
}

/// Information about a remote
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub push_url: Option<String>,
}

/// Result of a push operation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PushResult {
    pub success: bool,
    pub remote: String,
    pub branch: String,
    pub message: String,
}

/// Result of a pull operation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PullResult {
    pub success: bool,
    pub remote: String,
    pub branch: String,
    pub updated: bool,
    pub message: String,
}

/// Sync status with remote
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncStatus {
    pub remote: String,
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub synced: bool,
}

/// Result of a sync operation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncResult {
    pub remote: String,
    pub branch: String,
    pub pull_success: bool,
    pub pull_error: Option<String>,
    pub push_success: bool,
    pub push_error: Option<String>,
}

/// Information about a branch
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
}

/// Information about a commit
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_init_repository() {
        let dir = tempdir().unwrap();
        let _store = GitStore::open_or_init(dir.path()).unwrap();
        
        // Should have created directory structure
        assert!(dir.path().join("groups").exists());
        assert!(dir.path().join("templates").exists());
        assert!(dir.path().join(".vectorize").exists());
        assert!(dir.path().join("README.md").exists());
    }
    
    #[test]
    fn test_reopen_existing_repository() {
        let dir = tempdir().unwrap();
        
        // Create and close
        {
            let store = GitStore::open_or_init(dir.path()).unwrap();
            store.create_group("test-group").unwrap();
        }
        
        // Reopen
        let store = GitStore::open_or_init(dir.path()).unwrap();
        let config = store.read_config("test-group").unwrap();
        assert!(config.is_some());
    }
    
    #[test]
    fn test_create_group() {
        let dir = tempdir().unwrap();
        let store = GitStore::open_or_init(dir.path()).unwrap();
        
        store.create_group("production").unwrap();
        
        assert!(dir.path().join("groups/production").exists());
        assert!(dir.path().join("groups/production/config.toml").exists());
        assert!(dir.path().join("groups/production/group.yaml").exists());
    }
    
    #[test]
    fn test_create_multiple_groups() {
        let dir = tempdir().unwrap();
        let store = GitStore::open_or_init(dir.path()).unwrap();
        
        store.create_group("production").unwrap();
        store.create_group("staging").unwrap();
        store.create_group("development").unwrap();
        
        assert!(dir.path().join("groups/production").exists());
        assert!(dir.path().join("groups/staging").exists());
        assert!(dir.path().join("groups/development").exists());
    }
    
    #[test]
    fn test_write_and_read_config() {
        let dir = tempdir().unwrap();
        let store = GitStore::open_or_init(dir.path()).unwrap();
        
        store.create_group("test").unwrap();
        
        let config = "[sources.test]\ntype = \"demo_logs\"\n";
        store.write_config("test", config).unwrap();
        
        let read_config = store.read_config("test").unwrap();
        assert_eq!(read_config, Some(config.to_string()));
    }
    
    #[test]
    fn test_read_nonexistent_config() {
        let dir = tempdir().unwrap();
        let store = GitStore::open_or_init(dir.path()).unwrap();
        
        let config = store.read_config("nonexistent").unwrap();
        assert!(config.is_none());
    }
    
    #[test]
    fn test_config_versioning() {
        let dir = tempdir().unwrap();
        let store = GitStore::open_or_init(dir.path()).unwrap();
        
        store.create_group("test").unwrap();
        
        // Write first version
        let hash1 = store.write_config("test", "config version 1").unwrap();
        
        // Write second version
        let hash2 = store.write_config("test", "config version 2").unwrap();
        
        // Hashes should be different
        assert_ne!(hash1, hash2);
        
        // Current config should be version 2
        let current = store.read_config("test").unwrap().unwrap();
        assert_eq!(current, "config version 2");
    }
    
    #[test]
    fn test_get_config_at_version() {
        let dir = tempdir().unwrap();
        let store = GitStore::open_or_init(dir.path()).unwrap();
        
        store.create_group("test").unwrap();
        
        let hash1 = store.write_config("test", "config v1").unwrap();
        let _hash2 = store.write_config("test", "config v2").unwrap();
        
        // Get config at first version
        let old_config = store.get_config_at_version("test", &hash1).unwrap();
        assert!(old_config.is_some());
        assert_eq!(old_config.unwrap(), "config v1");
    }
    
    #[test]
    fn test_history() {
        let dir = tempdir().unwrap();
        let store = GitStore::open_or_init(dir.path()).unwrap();
        
        store.create_group("test").unwrap();
        store.write_config("test", "config v1").unwrap();
        store.write_config("test", "config v2").unwrap();
        
        let history = store.get_history(None, 10).unwrap();
        
        // Should have initial commit + create group + 2 config writes
        assert!(history.len() >= 3);
    }
    
    #[test]
    fn test_history_limit() {
        let dir = tempdir().unwrap();
        let store = GitStore::open_or_init(dir.path()).unwrap();
        
        store.create_group("test").unwrap();
        for i in 1..=10 {
            store.write_config("test", &format!("config v{}", i)).unwrap();
        }
        
        let history = store.get_history(None, 5).unwrap();
        assert_eq!(history.len(), 5);
    }
    
    #[test]
    fn test_history_by_path() {
        let dir = tempdir().unwrap();
        let store = GitStore::open_or_init(dir.path()).unwrap();
        
        store.create_group("group1").unwrap();
        store.create_group("group2").unwrap();
        
        store.write_config("group1", "config 1").unwrap();
        store.write_config("group2", "config 2").unwrap();
        store.write_config("group1", "config 1 updated").unwrap();
        
        // Get history for group1 only
        let history = store.get_history(Some("groups/group1"), 10).unwrap();
        
        // Should only include commits for group1
        for commit in &history {
            assert!(!commit.message.contains("group2"));
        }
    }
    
    #[test]
    fn test_diff_between_versions() {
        let dir = tempdir().unwrap();
        let store = GitStore::open_or_init(dir.path()).unwrap();
        
        store.create_group("test").unwrap();
        
        let hash1 = store.write_config("test", "line1\nline2\nline3\n").unwrap();
        let hash2 = store.write_config("test", "line1\nline2_modified\nline3\n").unwrap();
        
        let diff = store.diff(&hash1, &hash2).unwrap();
        
        // Diff should show the change
        assert!(diff.contains("line2") || diff.contains("modified"));
    }
    
    #[test]
    fn test_rollback() {
        let dir = tempdir().unwrap();
        let store = GitStore::open_or_init(dir.path()).unwrap();
        
        store.create_group("test").unwrap();
        
        let hash1 = store.write_config("test", "original config").unwrap();
        let _hash2 = store.write_config("test", "changed config").unwrap();
        
        // Rollback to hash1
        store.rollback("test", &hash1).unwrap();
        
        // Config should be reverted to original
        let config = store.read_config("test").unwrap().unwrap();
        assert_eq!(config, "original config");
    }
    
    #[test]
    fn test_list_groups_via_filesystem() {
        let dir = tempdir().unwrap();
        let store = GitStore::open_or_init(dir.path()).unwrap();
        
        store.create_group("alpha").unwrap();
        store.create_group("beta").unwrap();
        store.create_group("gamma").unwrap();
        
        // List groups by reading the groups directory
        let groups_dir = dir.path().join("groups");
        let mut groups: Vec<String> = std::fs::read_dir(&groups_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        groups.sort();
        
        assert_eq!(groups.len(), 3);
        assert!(groups.contains(&"alpha".to_string()));
        assert!(groups.contains(&"beta".to_string()));
        assert!(groups.contains(&"gamma".to_string()));
    }
    
    #[test]
    fn test_delete_group() {
        let dir = tempdir().unwrap();
        let store = GitStore::open_or_init(dir.path()).unwrap();
        
        store.create_group("to-delete").unwrap();
        assert!(dir.path().join("groups/to-delete").exists());
        
        store.delete_group("to-delete").unwrap();
        assert!(!dir.path().join("groups/to-delete").exists());
    }
    
    #[test]
    fn test_delete_nonexistent_group() {
        let dir = tempdir().unwrap();
        let store = GitStore::open_or_init(dir.path()).unwrap();
        
        // Should not panic
        let result = store.delete_group("nonexistent");
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_commit_message_format() {
        let dir = tempdir().unwrap();
        let store = GitStore::open_or_init(dir.path()).unwrap();
        
        store.create_group("mygroup").unwrap();
        store.write_config("mygroup", "test config").unwrap();
        
        let history = store.get_history(None, 10).unwrap();
        
        // Should have meaningful commit messages
        let has_create_msg = history.iter().any(|c| c.message.contains("mygroup") || c.message.contains("Create"));
        let has_update_msg = history.iter().any(|c| c.message.contains("mygroup") || c.message.contains("Update"));
        
        assert!(has_create_msg || has_update_msg);
    }
    
    #[test]
    fn test_large_config() {
        let dir = tempdir().unwrap();
        let store = GitStore::open_or_init(dir.path()).unwrap();
        
        store.create_group("test").unwrap();
        
        // Create a large config
        let mut large_config = String::new();
        for i in 0..1000 {
            large_config.push_str(&format!("[sources.source_{}]\ntype = \"demo_logs\"\n\n", i));
        }
        
        store.write_config("test", &large_config).unwrap();
        
        let read = store.read_config("test").unwrap().unwrap();
        assert_eq!(read.len(), large_config.len());
    }
    
    #[test]
    fn test_special_characters_in_config() {
        let dir = tempdir().unwrap();
        let store = GitStore::open_or_init(dir.path()).unwrap();
        
        store.create_group("test").unwrap();
        
        let config = r#"
[sources.demo]
type = "demo_logs"

[transforms.filter]
type = "filter"
inputs = ["demo"]
condition = '.status == 200 && contains(string!(.message), "success")'
"#;
        
        store.write_config("test", config).unwrap();
        
        let read = store.read_config("test").unwrap().unwrap();
        assert_eq!(read, config);
    }
}

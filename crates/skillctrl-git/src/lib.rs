//! Git operations for skillctrl.

use std::path::{Path, PathBuf};
use git2::{
    build::{CheckoutBuilder, RepoBuilder},
    BranchType, Cred, DiffOptions, FetchOptions, Repository,
};
use tokio::task;
use skillctrl_core::{Error, Result};

/// Git source configuration.
#[derive(Debug, Clone)]
pub struct GitSource {
    /// Source name.
    pub name: String,

    /// Repository URL.
    pub repo_url: String,

    /// Branch name.
    pub branch: String,

    /// Local cache directory.
    pub cache_dir: PathBuf,
}

impl GitSource {
    /// Creates a new git source.
    pub fn new(name: String, repo_url: String, branch: String, cache_dir: PathBuf) -> Self {
        Self {
            name,
            repo_url,
            branch,
            cache_dir,
        }
    }

    /// Returns the path to the cached repository.
    pub fn cache_path(&self) -> PathBuf {
        self.cache_dir.join(&self.name)
    }
}

/// Git operations manager.
pub struct GitManager {
    /// Cache directory for cloned repositories.
    cache_dir: PathBuf,

    /// SSH private key path (optional).
    ssh_key_path: Option<PathBuf>,

    /// SSH passphrase (optional).
    ssh_passphrase: Option<String>,
}

impl GitManager {
    /// Creates a new git manager.
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            ssh_key_path: None,
            ssh_passphrase: None,
        }
    }

    /// Sets SSH authentication credentials.
    pub fn with_ssh_auth(
        mut self,
        key_path: PathBuf,
        passphrase: Option<String>,
    ) -> Self {
        self.ssh_key_path = Some(key_path);
        self.ssh_passphrase = passphrase;
        self
    }

    /// Clones a repository to the cache.
    pub async fn clone(&self, source: &GitSource) -> Result<PathBuf> {
        let repo_url = source.repo_url.clone();
        let cache_path = source.cache_path();
        let branch = source.branch.clone();
        let ssh_key_path = self.ssh_key_path.clone();
        let ssh_passphrase = self.ssh_passphrase.clone();

        task::spawn_blocking(move || {
            Self::clone_blocking(&repo_url, &cache_path, &branch, ssh_key_path, ssh_passphrase)
        })
        .await
        .map_err(|e| Error::Git(format!("task join error: {}", e)))?
    }

    fn clone_blocking(
        repo_url: &str,
        cache_path: &Path,
        branch: &str,
        ssh_key_path: Option<PathBuf>,
        ssh_passphrase: Option<String>,
    ) -> Result<PathBuf> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::Git(format!("failed to create cache directory: {}", e))
            })?;
        }

        // Check if already cloned
        if cache_path.exists() {
            // Fetch and update instead
            return Self::fetch_blocking(
                repo_url,
                cache_path,
                branch,
                ssh_key_path,
                ssh_passphrase,
            );
        }

        // Prepare fetch options with auth
        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(Self::remote_callbacks(
            ssh_key_path,
            ssh_passphrase,
        )?);

        // Clone the repository
        let repo = RepoBuilder::new()
            .branch(branch)
            .fetch_options(fetch_options)
            .clone(repo_url, cache_path)
            .map_err(|e| Error::Git(format!("clone failed: {}", e)))?;

        Ok(repo.path().to_path_buf())
    }

    /// Fetches updates for an existing repository.
    pub async fn fetch(&self, source: &GitSource) -> Result<PathBuf> {
        let repo_url = source.repo_url.clone();
        let cache_path = source.cache_path();
        let branch = source.branch.clone();
        let ssh_key_path = self.ssh_key_path.clone();
        let ssh_passphrase = self.ssh_passphrase.clone();

        task::spawn_blocking(move || {
            Self::fetch_blocking(&repo_url, &cache_path, &branch, ssh_key_path, ssh_passphrase)
        })
        .await
        .map_err(|e| Error::Git(format!("task join error: {}", e)))?
    }

    fn fetch_blocking(
        _repo_url: &str,
        cache_path: &Path,
        branch: &str,
        ssh_key_path: Option<PathBuf>,
        ssh_passphrase: Option<String>,
    ) -> Result<PathBuf> {
        let repo = Repository::open(cache_path)
            .map_err(|e| Error::Git(format!("failed to open repository: {}", e)))?;

        // Find the remote
        let mut remote = repo
            .find_remote("origin")
            .or_else(|_| {
                // If origin doesn't exist, try to find any remote
                repo.remotes()
                    .iter()
                    .flatten()
                    .flatten()
                    .filter_map(|name| repo.find_remote(name).ok())
                    .next()
                    .ok_or_else(|| {
                        Error::Git("no remote found in repository".to_string())
                    })
            })
            .map_err(|e| Error::Git(format!("failed to find remote: {}", e)))?;

        // Fetch
        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(Self::remote_callbacks(
            ssh_key_path,
            ssh_passphrase,
        )?);

        remote
            .fetch(&[branch], Some(&mut fetch_options), None)
            .map_err(|e| Error::Git(format!("fetch failed: {}", e)))?;

        // Get the remote branch
        let fetch_head = repo
            .find_reference(&format!("refs/remotes/origin/{}", branch))
            .map_err(|e| Error::Git(format!("failed to find remote branch: {}", e)))?;

        let fetch_commit = repo
            .reference_to_annotated_commit(&fetch_head)
            .map_err(|e| Error::Git(format!("failed to get fetch commit: {}", e)))?;

        // Set the local branch to the fetched commit
        let refname = format!("refs/heads/{}", branch);
        match repo.find_branch(&refname, BranchType::Local) {
            Ok(mut _branch) => {
                // Branch exists, set it to the fetch commit
                repo.reference_to_annotated_commit(&fetch_head)
                    .map_err(|e| Error::Git(format!("failed to set head: {}", e)))?;

                let mut reference = repo
                    .find_reference(&refname)
                    .map_err(|e| Error::Git(format!("failed to find reference: {}", e)))?;

                reference
                    .set_target(fetch_commit.id(), "update branch")
                    .map_err(|e| Error::Git(format!("failed to set target: {}", e)))?;

                repo.set_head(&refname)
                    .map_err(|e| Error::Git(format!("failed to set head: {}", e)))?;

                repo.checkout_head(Some(CheckoutBuilder::default().force()))
                    .map_err(|e| Error::Git(format!("failed to checkout: {}", e)))?;
            }
            Err(_) => {
                // Branch doesn't exist, create it
                let commit = fetch_head
                    .peel_to_commit()
                    .map_err(|e| Error::Git(format!("failed to peel to commit: {}", e)))?;
                repo.branch(branch, &commit, false)
                    .map_err(|e| Error::Git(format!("failed to create branch: {}", e)))?;

                repo.set_head(&refname)
                    .map_err(|e| Error::Git(format!("failed to set head: {}", e)))?;

                repo.checkout_head(Some(CheckoutBuilder::default().force()))
                    .map_err(|e| Error::Git(format!("failed to checkout: {}", e)))?;
            }
        }

        Ok(cache_path.to_path_buf())
    }

    /// Gets the current commit hash of a repository.
    pub async fn current_commit(&self, source: &GitSource) -> Result<String> {
        let cache_path = source.cache_path();

        task::spawn_blocking(move || {
            let repo = Repository::open(&cache_path).map_err(|e| {
                Error::Git(format!("failed to open repository: {}", e))
            })?;

            let head = repo.head().map_err(|e| {
                Error::Git(format!("failed to get HEAD: {}", e))
            })?;

            let commit = head.peel_to_commit().map_err(|e| {
                Error::Git(format!("failed to peel to commit: {}", e))
            })?;

            Ok(commit.id().to_string())
        })
        .await
        .map_err(|e| Error::Git(format!("task join error: {}", e)))?
    }

    /// Lists files changed since a commit.
    pub async fn list_changed_files(
        &self,
        source: &GitSource,
        since: &str,
    ) -> Result<Vec<PathBuf>> {
        let cache_path = source.cache_path();
        let since = since.to_string();

        task::spawn_blocking(move || {
            let repo = Repository::open(&cache_path)
                .map_err(|e| Error::Git(format!("failed to open repository: {}", e)))?;

            let old_commit = repo
                .revparse_single(&since)
                .map_err(|e| Error::Git(format!("failed to find commit: {}", e)))?;

            let head = repo.head().map_err(|e| {
                Error::Git(format!("failed to get HEAD: {}", e))
            })?;

            let old_tree = old_commit.peel_to_tree().map_err(|e| {
                Error::Git(format!("failed to peel to tree: {}", e))
            })?;

            let new_tree = head.peel_to_tree().map_err(|e| {
                Error::Git(format!("failed to peel to tree: {}", e))
            })?;

            let mut diff_opts = DiffOptions::default();
            let diff = repo
                .diff_tree_to_tree(Some(&old_tree), Some(&new_tree), Some(&mut diff_opts))
                .map_err(|e| Error::Git(format!("failed to diff: {}", e)))?;

            let mut files = Vec::new();
            diff.foreach(
                &mut |delta, _| {
                    if let Some(path) = delta.new_file().path() {
                        files.push(path.to_path_buf());
                    }
                    true
                },
                None,
                None,
                None,
            )
            .map_err(|e| Error::Git(format!("failed to iterate diff: {}", e)))?;

            Ok(files)
        })
        .await
        .map_err(|e| Error::Git(format!("task join error: {}", e)))?
    }

    /// Creates remote callbacks with authentication.
    fn remote_callbacks(
        ssh_key_path: Option<PathBuf>,
        ssh_passphrase: Option<String>,
    ) -> Result<git2::RemoteCallbacks<'static>> {
        let mut callbacks = git2::RemoteCallbacks::new();

        // Only implement SSH auth for now
        if ssh_key_path.is_some() {
            let ssh_key_path_clone = ssh_key_path.clone();
            let ssh_passphrase_clone = ssh_passphrase.clone();

            callbacks.credentials(move |url, username_from_url, _allowed_types| {
                // Prefer SSH key authentication
                if let Some(ref key_path) = ssh_key_path_clone {
                    let username = username_from_url.unwrap_or("git");
                    return Cred::ssh_key(
                        username,
                        None,
                        key_path.as_path(),
                        ssh_passphrase_clone.as_deref(),
                    );
                }

                // Fall back to default
                Err(git2::Error::from_str("no authentication method configured"))
            });
        }

        Ok(callbacks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_git_source() {
        let temp_dir = TempDir::new().unwrap();
        let source = GitSource::new(
            "test".to_string(),
            "https://github.com/test/repo.git".to_string(),
            "main".to_string(),
            temp_dir.path().to_path_buf(),
        );

        assert_eq!(source.name, "test");
        assert_eq!(source.branch, "main");
        assert_eq!(
            source.cache_path(),
            temp_dir.path().join("test")
        );
    }

    #[test]
    fn test_git_manager() {
        let temp_dir = TempDir::new().unwrap();
        let manager = GitManager::new(temp_dir.path().to_path_buf());

        assert_eq!(manager.cache_dir, temp_dir.path());
        assert!(manager.ssh_key_path.is_none());
    }
}

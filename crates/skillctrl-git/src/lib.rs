//! Git operations for skillctrl.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use git2::{
    build::{CheckoutBuilder, RepoBuilder},
    BranchType, Cred, CredentialType, DiffOptions, FetchOptions, Repository,
};
use skillctrl_core::{Error, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::task;

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

    /// SSH private key path (optional).
    pub ssh_key_path: Option<PathBuf>,

    /// SSH passphrase (optional).
    pub ssh_passphrase: Option<String>,

    /// HTTPS access token (optional).
    pub access_token: Option<String>,
}

impl GitSource {
    /// Creates a new git source.
    pub fn new(name: String, repo_url: String, branch: String, cache_dir: PathBuf) -> Self {
        Self {
            name,
            repo_url,
            branch,
            cache_dir,
            ssh_key_path: None,
            ssh_passphrase: None,
            access_token: None,
        }
    }

    /// Returns the path to the cached repository.
    pub fn cache_path(&self) -> PathBuf {
        self.cache_dir.join(&self.name)
    }

    /// Configures SSH authentication for this source.
    pub fn with_ssh_auth(mut self, key_path: PathBuf, passphrase: Option<String>) -> Self {
        self.ssh_key_path = Some(key_path);
        self.ssh_passphrase = passphrase;
        self.access_token = None;
        self
    }

    /// Configures HTTPS token authentication for this source.
    pub fn with_https_auth(mut self, access_token: String) -> Self {
        self.access_token = Some(access_token);
        self.ssh_key_path = None;
        self.ssh_passphrase = None;
        self
    }
}

/// Git operations manager.
pub struct GitManager;

impl GitManager {
    /// Creates a new git manager.
    pub fn new(_cache_dir: PathBuf) -> Self {
        Self
    }

    /// Clones a repository to the cache.
    pub async fn clone(&self, source: &GitSource) -> Result<PathBuf> {
        let repo_url = source.repo_url.clone();
        let cache_path = source.cache_path();
        let branch = source.branch.clone();
        let ssh_key_path = source.ssh_key_path.clone();
        let ssh_passphrase = source.ssh_passphrase.clone();
        let access_token = source.access_token.clone();

        task::spawn_blocking(move || {
            Self::clone_blocking(
                &repo_url,
                &cache_path,
                &branch,
                ssh_key_path,
                ssh_passphrase,
                access_token,
            )
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
        access_token: Option<String>,
    ) -> Result<PathBuf> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::Git(format!("failed to create cache directory: {}", e)))?;
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
                access_token,
            );
        }

        let git2_result = (|| -> Result<PathBuf> {
            let mut fetch_options = FetchOptions::new();
            fetch_options.remote_callbacks(Self::remote_callbacks(
                ssh_key_path.clone(),
                ssh_passphrase.clone(),
                access_token.clone(),
            )?);

            RepoBuilder::new()
                .branch(branch)
                .fetch_options(fetch_options)
                .clone(repo_url, cache_path)
                .map_err(|e| Error::Git(format!("clone failed: {}", e)))?;

            Ok(cache_path.to_path_buf())
        })();

        match git2_result {
            Ok(path) => Ok(path),
            Err(git2_error) => {
                if cache_path.exists() {
                    let _ = std::fs::remove_dir_all(cache_path);
                }

                Self::clone_with_git_cli(
                    repo_url,
                    cache_path,
                    branch,
                    ssh_key_path.as_deref(),
                    access_token.as_deref(),
                )
                .map_err(|fallback_error| combine_git_errors("clone", git2_error, fallback_error))
            }
        }
    }

    /// Fetches updates for an existing repository.
    pub async fn fetch(&self, source: &GitSource) -> Result<PathBuf> {
        let repo_url = source.repo_url.clone();
        let cache_path = source.cache_path();
        let branch = source.branch.clone();
        let ssh_key_path = source.ssh_key_path.clone();
        let ssh_passphrase = source.ssh_passphrase.clone();
        let access_token = source.access_token.clone();

        task::spawn_blocking(move || {
            Self::fetch_blocking(
                &repo_url,
                &cache_path,
                &branch,
                ssh_key_path,
                ssh_passphrase,
                access_token,
            )
        })
        .await
        .map_err(|e| Error::Git(format!("task join error: {}", e)))?
    }

    fn fetch_blocking(
        repo_url: &str,
        cache_path: &Path,
        branch: &str,
        ssh_key_path: Option<PathBuf>,
        ssh_passphrase: Option<String>,
        access_token: Option<String>,
    ) -> Result<PathBuf> {
        let git2_result = (|| -> Result<PathBuf> {
            let repo = Repository::open(cache_path)
                .map_err(|e| Error::Git(format!("failed to open repository: {}", e)))?;

            let mut remote = repo
                .find_remote("origin")
                .or_else(|_| {
                    repo.remotes()
                        .iter()
                        .flatten()
                        .flatten()
                        .filter_map(|name| repo.find_remote(name).ok())
                        .next()
                        .ok_or_else(|| Error::Git("no remote found in repository".to_string()))
                })
                .map_err(|e| Error::Git(format!("failed to find remote: {}", e)))?;

            let mut fetch_options = FetchOptions::new();
            fetch_options.remote_callbacks(Self::remote_callbacks(
                ssh_key_path.clone(),
                ssh_passphrase.clone(),
                access_token.clone(),
            )?);

            remote
                .fetch(&[branch], Some(&mut fetch_options), None)
                .map_err(|e| Error::Git(format!("fetch failed: {}", e)))?;

            let fetch_head = repo
                .find_reference(&format!("refs/remotes/origin/{}", branch))
                .map_err(|e| Error::Git(format!("failed to find remote branch: {}", e)))?;

            let fetch_commit = repo
                .reference_to_annotated_commit(&fetch_head)
                .map_err(|e| Error::Git(format!("failed to get fetch commit: {}", e)))?;

            let refname = format!("refs/heads/{}", branch);
            match repo.find_branch(branch, BranchType::Local) {
                Ok(mut _branch) => {
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
        })();

        match git2_result {
            Ok(path) => Ok(path),
            Err(git2_error) => Self::fetch_with_git_cli(
                repo_url,
                cache_path,
                branch,
                ssh_key_path.as_deref(),
                access_token.as_deref(),
            )
            .map_err(|fallback_error| combine_git_errors("fetch", git2_error, fallback_error)),
        }
    }

    /// Gets the current commit hash of a repository.
    pub async fn current_commit(&self, source: &GitSource) -> Result<String> {
        let cache_path = source.cache_path();

        task::spawn_blocking(move || {
            let repo = Repository::open(&cache_path)
                .map_err(|e| Error::Git(format!("failed to open repository: {}", e)))?;

            let head = repo
                .head()
                .map_err(|e| Error::Git(format!("failed to get HEAD: {}", e)))?;

            let commit = head
                .peel_to_commit()
                .map_err(|e| Error::Git(format!("failed to peel to commit: {}", e)))?;

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

            let head = repo
                .head()
                .map_err(|e| Error::Git(format!("failed to get HEAD: {}", e)))?;

            let old_tree = old_commit
                .peel_to_tree()
                .map_err(|e| Error::Git(format!("failed to peel to tree: {}", e)))?;

            let new_tree = head
                .peel_to_tree()
                .map_err(|e| Error::Git(format!("failed to peel to tree: {}", e)))?;

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
        access_token: Option<String>,
    ) -> Result<git2::RemoteCallbacks<'static>> {
        let mut callbacks = git2::RemoteCallbacks::new();
        let ssh_key_path_clone = ssh_key_path.clone();
        let ssh_passphrase_clone = ssh_passphrase.clone();
        let access_token_clone = access_token.clone();

        callbacks.credentials(move |url, username_from_url, allowed_types| {
            if let Some(ref access_token) = access_token_clone {
                if allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT) {
                    let username = https_username_for_url(url, username_from_url);
                    return Cred::userpass_plaintext(&username, access_token);
                }
            }

            if let Some(ref key_path) = ssh_key_path_clone {
                let username = username_from_url.unwrap_or("git");
                return Cred::ssh_key(
                    username,
                    None,
                    key_path.as_path(),
                    ssh_passphrase_clone.as_deref(),
                );
            }

            if allowed_types.contains(CredentialType::SSH_KEY) {
                if let Some(username) = username_from_url {
                    if let Ok(cred) = Cred::ssh_key_from_agent(username) {
                        return Ok(cred);
                    }
                }
            }

            if allowed_types.contains(CredentialType::DEFAULT) {
                if let Ok(cred) = Cred::default() {
                    return Ok(cred);
                }
            }

            if allowed_types.contains(CredentialType::USERNAME) {
                if let Some(username) = username_from_url {
                    return Cred::username(username);
                }
            }

            Err(git2::Error::from_str(
                "no compatible authentication method configured",
            ))
        });

        Ok(callbacks)
    }

    fn clone_with_git_cli(
        repo_url: &str,
        cache_path: &Path,
        branch: &str,
        ssh_key_path: Option<&Path>,
        access_token: Option<&str>,
    ) -> Result<PathBuf> {
        let mut command = Command::new("git");
        command
            .arg("clone")
            .arg("--branch")
            .arg(branch)
            .arg("--single-branch")
            .arg(repo_url)
            .arg(cache_path);
        configure_git_cli_command(&mut command, repo_url, ssh_key_path, access_token);
        run_git_command(command, "git clone")?;

        Ok(cache_path.to_path_buf())
    }

    fn fetch_with_git_cli(
        repo_url: &str,
        cache_path: &Path,
        branch: &str,
        ssh_key_path: Option<&Path>,
        access_token: Option<&str>,
    ) -> Result<PathBuf> {
        let mut set_url = Command::new("git");
        set_url
            .arg("-C")
            .arg(cache_path)
            .arg("remote")
            .arg("set-url")
            .arg("origin")
            .arg(repo_url);
        configure_git_cli_command(&mut set_url, repo_url, ssh_key_path, access_token);
        run_git_command(set_url, "git remote set-url")?;

        let mut fetch = Command::new("git");
        fetch
            .arg("-C")
            .arg(cache_path)
            .arg("fetch")
            .arg("--prune")
            .arg("origin")
            .arg(branch);
        configure_git_cli_command(&mut fetch, repo_url, ssh_key_path, access_token);
        run_git_command(fetch, "git fetch")?;

        let mut checkout = Command::new("git");
        checkout
            .arg("-C")
            .arg(cache_path)
            .arg("checkout")
            .arg("-B")
            .arg(branch)
            .arg("FETCH_HEAD");
        configure_git_cli_command(&mut checkout, repo_url, ssh_key_path, access_token);
        run_git_command(checkout, "git checkout")?;

        Ok(cache_path.to_path_buf())
    }
}

fn https_username_for_url(url: &str, username_from_url: Option<&str>) -> String {
    if let Some(username) = username_from_url {
        if !username.is_empty() {
            return username.to_string();
        }
    }

    if url.contains("github.com") {
        return "x-access-token".to_string();
    }

    if url.contains("gitlab") {
        return "oauth2".to_string();
    }

    "git".to_string()
}

fn configure_git_cli_command(
    command: &mut Command,
    repo_url: &str,
    ssh_key_path: Option<&Path>,
    access_token: Option<&str>,
) {
    command.env("GIT_TERMINAL_PROMPT", "0");

    if let Some(key_path) = ssh_key_path {
        command.env("GIT_SSH_COMMAND", git_ssh_command(key_path));
    }

    if let Some(token) = access_token {
        command.env("GIT_CONFIG_COUNT", "1");
        command.env("GIT_CONFIG_KEY_0", "http.extraHeader");
        command.env(
            "GIT_CONFIG_VALUE_0",
            https_basic_auth_header(repo_url, token),
        );
    }
}

fn git_ssh_command(key_path: &Path) -> String {
    format!(
        "ssh -i {} -o IdentitiesOnly=yes",
        shell_quote(key_path.to_string_lossy().as_ref())
    )
}

fn https_basic_auth_header(repo_url: &str, access_token: &str) -> String {
    let username = https_username_for_url(repo_url, None);
    let encoded = STANDARD.encode(format!("{}:{}", username, access_token));
    format!("Authorization: Basic {}", encoded)
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn run_git_command(mut command: Command, description: &str) -> Result<()> {
    let output = command
        .output()
        .map_err(|e| Error::Git(format!("{} failed to start: {}", description, e)))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let details = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("exit status {}", output.status)
    };

    Err(Error::Git(format!("{} failed: {}", description, details)))
}

fn combine_git_errors(operation: &str, primary: Error, fallback: Error) -> Error {
    Error::Git(format!(
        "{} failed with libgit2: {}; fallback to system git also failed: {}",
        operation,
        simplify_git_error(primary),
        simplify_git_error(fallback)
    ))
}

fn simplify_git_error(error: Error) -> String {
    match error {
        Error::Git(message) => message,
        other => other.to_string(),
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
        assert_eq!(source.cache_path(), temp_dir.path().join("test"));
        assert!(source.ssh_key_path.is_none());
        assert!(source.access_token.is_none());
    }

    #[test]
    fn test_git_manager() {
        let temp_dir = TempDir::new().unwrap();
        let manager = GitManager::new(temp_dir.path().to_path_buf());

        let _ = manager;
    }

    #[test]
    fn test_git_source_auth_builders() {
        let temp_dir = TempDir::new().unwrap();
        let ssh_source = GitSource::new(
            "test".to_string(),
            "git@github.com:test/repo.git".to_string(),
            "main".to_string(),
            temp_dir.path().to_path_buf(),
        )
        .with_ssh_auth(PathBuf::from("/tmp/test-key"), None);

        assert_eq!(
            ssh_source.ssh_key_path,
            Some(PathBuf::from("/tmp/test-key"))
        );
        assert!(ssh_source.access_token.is_none());

        let https_source = GitSource::new(
            "test".to_string(),
            "https://github.com/test/repo.git".to_string(),
            "main".to_string(),
            temp_dir.path().to_path_buf(),
        )
        .with_https_auth("token".to_string());

        assert_eq!(https_source.access_token.as_deref(), Some("token"));
        assert!(https_source.ssh_key_path.is_none());
    }

    #[test]
    fn test_https_username_for_url() {
        assert_eq!(
            https_username_for_url("https://github.com/test/repo.git", None),
            "x-access-token"
        );
        assert_eq!(
            https_username_for_url("https://gitlab.com/test/repo.git", None),
            "oauth2"
        );
        assert_eq!(
            https_username_for_url("https://example.com/test/repo.git", Some("alice")),
            "alice"
        );
    }

    #[test]
    fn test_https_basic_auth_header() {
        assert_eq!(
            https_basic_auth_header("https://github.com/test/repo.git", "token"),
            "Authorization: Basic eC1hY2Nlc3MtdG9rZW46dG9rZW4="
        );
    }

    #[test]
    fn test_git_ssh_command_quotes_key_path() {
        assert_eq!(
            git_ssh_command(Path::new("/tmp/test key")),
            "ssh -i '/tmp/test key' -o IdentitiesOnly=yes"
        );
    }
}

//! State management for skillctrl.

use rusqlite::{params, Connection};
use skillctrl_core::{Endpoint, Error, Result, Scope};
use std::path::{Path, PathBuf};

// Re-export GitSource from the git module
pub use skillctrl_git::GitSource;

/// State manager.
///
/// Manages the persistent state of skillctrl, including sources,
/// installations, and backups.
pub struct StateManager {
    /// Database connection (wrapped in Mutex for thread safety).
    conn: std::sync::Arc<std::sync::Mutex<Connection>>,
}

impl StateManager {
    /// Opens or creates the state database.
    pub async fn open(path: &Path) -> Result<Self> {
        let path = path.to_path_buf();

        let conn = tokio::task::spawn_blocking(move || {
            // Create parent directory if needed
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let conn = Connection::open(&path)
                .map_err(|e| Error::Database(format!("failed to open database: {}", e)))?;

            // Initialize schema
            Self::init_schema(&conn)?;

            Ok::<_, Error>(conn)
        })
        .await
        .map_err(|e| Error::Database(format!("task join error: {}", e)))??;

        Ok(Self {
            conn: std::sync::Arc::new(std::sync::Mutex::new(conn)),
        })
    }

    /// Opens state in the default location.
    pub async fn open_default() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| Error::Config("could not determine config directory".to_string()))?;

        let state_dir = config_dir.join("skillctrl");
        std::fs::create_dir_all(&state_dir)
            .map_err(|e| Error::Database(format!("failed to create state directory: {}", e)))?;

        let db_path = state_dir.join("state.db");
        Self::open(&db_path).await
    }

    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sources (
                name TEXT PRIMARY KEY,
                repo_url TEXT NOT NULL,
                branch TEXT NOT NULL,
                cache_path TEXT NOT NULL,
                ssh_key_path TEXT,
                access_token TEXT,
                last_commit TEXT,
                updated_at TEXT
            )",
            [],
        )
        .map_err(|e| Error::Database(format!("failed to create sources table: {}", e)))?;

        Self::ensure_column(conn, "sources", "ssh_key_path", "TEXT")?;
        Self::ensure_column(conn, "sources", "access_token", "TEXT")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS installations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bundle_id TEXT NOT NULL,
                bundle_version TEXT NOT NULL,
                source_name TEXT,
                endpoint TEXT NOT NULL,
                scope TEXT NOT NULL,
                project_path TEXT,
                installed_at TEXT NOT NULL,
                files_created TEXT NOT NULL,
                backup_path TEXT,
                UNIQUE(bundle_id, endpoint, scope, project_path)
            )",
            [],
        )
        .map_err(|e| Error::Database(format!("failed to create installations table: {}", e)))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                installation_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                original_hash TEXT,
                FOREIGN KEY(installation_id) REFERENCES installations(id) ON DELETE CASCADE
            )",
            [],
        )
        .map_err(|e| Error::Database(format!("failed to create files table: {}", e)))?;

        Ok(())
    }

    fn ensure_column(conn: &Connection, table: &str, column: &str, definition: &str) -> Result<()> {
        let pragma = format!("PRAGMA table_info({})", table);
        let mut stmt = conn
            .prepare(&pragma)
            .map_err(|e| Error::Database(format!("failed to inspect {} schema: {}", table, e)))?;

        let columns = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| Error::Database(format!("failed to inspect {} columns: {}", table, e)))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::Database(format!("failed to collect {} columns: {}", table, e)))?;

        if columns.iter().any(|existing| existing == column) {
            return Ok(());
        }

        conn.execute(
            &format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition),
            [],
        )
        .map_err(|e| {
            Error::Database(format!("failed to add column {}.{}: {}", table, column, e))
        })?;

        Ok(())
    }

    /// Registers a source.
    pub async fn register_source(&self, source: &GitSource) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::Database(format!("failed to lock database: {}", e)))?;
        let name = source.name.clone();
        let repo_url = source.repo_url.clone();
        let branch = source.branch.clone();
        let cache_path = source
            .cache_dir
            .join(&source.name)
            .to_string_lossy()
            .to_string();
        let ssh_key_path = source
            .ssh_key_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string());
        let access_token = source.access_token.clone();

        conn.execute(
            "INSERT INTO sources (name, repo_url, branch, cache_path, ssh_key_path, access_token)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(name) DO UPDATE SET
                repo_url = excluded.repo_url,
                branch = excluded.branch,
                cache_path = excluded.cache_path,
                ssh_key_path = excluded.ssh_key_path,
                access_token = excluded.access_token",
            params![
                name,
                repo_url,
                branch,
                cache_path,
                ssh_key_path,
                access_token
            ],
        )
        .map_err(|e| Error::Database(format!("failed to register source: {}", e)))?;

        Ok(())
    }

    /// Gets a single registered source by name.
    pub async fn get_source(&self, name: &str) -> Result<Option<SourceEntry>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::Database(format!("failed to lock database: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT name, repo_url, branch, cache_path, ssh_key_path, access_token, last_commit, updated_at
                 FROM sources
                 WHERE name = ?1",
            )
            .map_err(|e| Error::Database(format!("failed to prepare source query: {}", e)))?;

        let mut rows = stmt
            .query(params![name])
            .map_err(|e| Error::Database(format!("failed to query source: {}", e)))?;

        rows.next()
            .map_err(|e| Error::Database(format!("failed to read source row: {}", e)))?
            .map(map_source_entry)
            .transpose()
            .map_err(|e| Error::Database(format!("failed to decode source row: {}", e)))
    }

    /// Lists all registered sources.
    pub async fn list_sources(&self) -> Result<Vec<SourceEntry>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::Database(format!("failed to lock database: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT name, repo_url, branch, cache_path, ssh_key_path, access_token, last_commit, updated_at
                 FROM sources
                 ORDER BY name ASC",
            )
            .map_err(|e| Error::Database(format!("failed to list sources: {}", e)))?;

        let entries = stmt
            .query_map([], map_source_entry)
            .map_err(|e| Error::Database(format!("failed to map sources: {}", e)))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::Database(format!("failed to collect sources: {}", e)))?;

        Ok(entries)
    }

    /// Removes a registered source.
    pub async fn remove_source(&self, name: &str) -> Result<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::Database(format!("failed to lock database: {}", e)))?;

        let removed = conn
            .execute("DELETE FROM sources WHERE name = ?1", params![name])
            .map_err(|e| Error::Database(format!("failed to remove source: {}", e)))?;

        Ok(removed > 0)
    }

    /// Updates sync metadata for a registered source.
    pub async fn update_source_sync_status(
        &self,
        name: &str,
        last_commit: Option<&str>,
    ) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::Database(format!("failed to lock database: {}", e)))?;

        let updated_at = chrono::Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "UPDATE sources
                 SET last_commit = ?2, updated_at = ?3
                 WHERE name = ?1",
                params![name, last_commit, updated_at],
            )
            .map_err(|e| Error::Database(format!("failed to update source status: {}", e)))?;

        if affected == 0 {
            return Err(Error::NotFound(format!("source '{}' not found", name)));
        }

        Ok(())
    }

    /// Records an installation.
    pub async fn record_installation(&self, install: &InstallationRecord) -> Result<i64> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::Database(format!("failed to lock database: {}", e)))?;
        let install = install.clone();
        let files_json = serde_json::to_string(&install.files_created)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        conn.execute(
            "INSERT INTO installations (
                bundle_id, bundle_version, source_name, endpoint, scope,
                project_path, installed_at, files_created, backup_path
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                install.bundle_id,
                install.bundle_version.to_string(),
                install.source_name,
                install.endpoint.to_string(),
                scope_to_string(install.scope),
                install
                    .project_path
                    .map(|p| p.to_string_lossy().to_string()),
                install.installed_at.to_rfc3339(),
                files_json,
                install.backup_path.map(|p| p.to_string_lossy().to_string()),
            ],
        )
        .map_err(|e| Error::Database(format!("failed to record installation: {}", e)))?;

        let id = conn.last_insert_rowid();

        for file_path in &install.files_created {
            conn.execute(
                "INSERT INTO files (installation_id, path) VALUES (?1, ?2)",
                params![id, file_path.to_string_lossy().as_ref()],
            )
            .map_err(|e| Error::Database(format!("failed to record file: {}", e)))?;
        }

        Ok(id)
    }

    /// Removes an installation record.
    pub async fn remove_installation(
        &self,
        bundle_id: &str,
        endpoint: &Endpoint,
        scope: Scope,
        project_path: Option<&Path>,
    ) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::Database(format!("failed to lock database: {}", e)))?;
        let bundle_id = bundle_id.to_string();
        let endpoint = endpoint.to_string();
        let project_path = project_path.map(|p| p.to_string_lossy().to_string());

        conn.execute(
            "DELETE FROM installations
             WHERE bundle_id = ?1 AND endpoint = ?2 AND scope = ?3 AND project_path = ?4",
            params![bundle_id, endpoint, scope_to_string(scope), project_path,],
        )
        .map_err(|e| Error::Database(format!("failed to remove installation: {}", e)))?;

        Ok(())
    }

    /// Queries installation records.
    pub async fn query_installations(
        &self,
        bundle_id: Option<&str>,
        endpoint: Option<&Endpoint>,
        scope: Option<Scope>,
        project_path: Option<&Path>,
    ) -> Result<Vec<InstallationRecord>> {
        let conn = self.conn.clone();
        let bundle_id = bundle_id.map(|s| s.to_string());
        let endpoint = endpoint.map(|e| e.to_string());
        let scope = scope;
        let project_path = project_path.map(|p| p.to_string_lossy().to_string());

        tokio::task::spawn_blocking(move || {
            let conn = conn
                .lock()
                .map_err(|e| Error::Database(format!("failed to lock database: {}", e)))?;

            let mut clauses = Vec::new();
            let mut params = Vec::new();

            if let Some(bundle_id) = &bundle_id {
                clauses.push("bundle_id = ?");
                params.push(Some(bundle_id.clone()));
            }

            if let Some(endpoint) = &endpoint {
                clauses.push("endpoint = ?");
                params.push(Some(endpoint.clone()));
            }

            if let Some(scope) = scope {
                clauses.push("scope = ?");
                params.push(Some(scope_to_string(scope)));
            }

            match &project_path {
                Some(project_path) => {
                    clauses.push("project_path = ?");
                    params.push(Some(project_path.clone()));
                }
                None if scope == Some(Scope::User) => {
                    clauses.push("project_path IS NULL");
                }
                None => {}
            }

            let filter_clause = if clauses.is_empty() {
                "".to_string()
            } else {
                format!("WHERE {}", clauses.join(" AND "))
            };

            let sql = format!(
                "SELECT bundle_id, bundle_version, source_name, endpoint, scope,
                        project_path, installed_at, files_created, backup_path
                 FROM installations {}
                 ORDER BY installed_at DESC",
                filter_clause
            );

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| Error::Database(format!("failed to query installations: {}", e)))?;

            let params_ref: Vec<&dyn rusqlite::ToSql> = params
                .iter()
                .map(|p| match p {
                    Some(ref s) => s as &dyn rusqlite::ToSql,
                    None => &"" as &dyn rusqlite::ToSql,
                })
                .collect();

            let records = stmt
                .query_map(params_ref.as_slice(), |row| {
                    let files_json: String = row.get(7)?;
                    let files_created: Vec<PathBuf> =
                        serde_json::from_str(&files_json).map_err(|e| {
                            rusqlite::Error::ToSqlConversionFailure(
                                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                            )
                        })?;

                    Ok(InstallationRecord {
                        bundle_id: row.get(0)?,
                        bundle_version: semver::Version::parse(&row.get::<_, String>(1)?).map_err(
                            |e| {
                                rusqlite::Error::ToSqlConversionFailure(
                                    Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                                )
                            },
                        )?,
                        source_name: row.get(2)?,
                        endpoint: row.get::<_, String>(3)?.parse().map_err(|_| {
                            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "invalid endpoint",
                            ))
                                as Box<dyn std::error::Error + Send + Sync>)
                        })?,
                        scope: scope_from_string(&row.get::<_, String>(4)?),
                        project_path: row.get::<_, Option<String>>(5)?.map(|s| PathBuf::from(s)),
                        installed_at: chrono::DateTime::parse_from_rfc3339(
                            &row.get::<_, String>(6)?,
                        )
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .map_err(|e| {
                            rusqlite::Error::ToSqlConversionFailure(
                                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                            )
                        })?,
                        files_created,
                        backup_path: row.get::<_, Option<String>>(8)?.map(|s| PathBuf::from(s)),
                    })
                })
                .map_err(|e| Error::Database(format!("failed to map installations: {}", e)))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| Error::Database(format!("failed to collect installations: {}", e)))?;

            Ok::<_, Error>(records)
        })
        .await
        .map_err(|e| Error::Database(format!("task join error: {}", e)))?
    }
}

/// A git source entry in the database.
#[derive(Debug, Clone)]
pub struct SourceEntry {
    /// Source name.
    pub name: String,

    /// Repository URL.
    pub repo_url: String,

    /// Branch name.
    pub branch: String,

    /// Cache directory path.
    pub cache_path: PathBuf,

    /// SSH private key path (if configured).
    pub ssh_key_path: Option<PathBuf>,

    /// HTTPS access token (if configured).
    pub access_token: Option<String>,

    /// Last commit hash (if fetched).
    pub last_commit: Option<String>,

    /// Last update timestamp.
    pub updated_at: Option<String>,
}

/// An installation record.
#[derive(Debug, Clone)]
pub struct InstallationRecord {
    /// Bundle ID.
    pub bundle_id: String,

    /// Bundle version.
    pub bundle_version: semver::Version,

    /// Source name (if installed from a source).
    pub source_name: Option<String>,

    /// Target endpoint.
    pub endpoint: Endpoint,

    /// Installation scope.
    pub scope: Scope,

    /// Project path (if project scope).
    pub project_path: Option<PathBuf>,

    /// Installation timestamp.
    pub installed_at: chrono::DateTime<chrono::Utc>,

    /// Files created during installation.
    pub files_created: Vec<PathBuf>,

    /// Backup directory path.
    pub backup_path: Option<PathBuf>,
}

/// Non-public type for state module.

/// Converts a scope to its string representation.
fn scope_to_string(scope: Scope) -> String {
    match scope {
        Scope::Project => "project".to_string(),
        Scope::User => "user".to_string(),
    }
}

/// Parses a scope from its string representation.
fn scope_from_string(s: &str) -> Scope {
    match s {
        "project" => Scope::Project,
        "user" => Scope::User,
        _ => Scope::Project, // Default
    }
}

fn map_source_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<SourceEntry> {
    Ok(SourceEntry {
        name: row.get(0)?,
        repo_url: row.get(1)?,
        branch: row.get(2)?,
        cache_path: PathBuf::from(row.get::<_, String>(3)?),
        ssh_key_path: row.get::<_, Option<String>>(4)?.map(PathBuf::from),
        access_token: row.get(5)?,
        last_commit: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_state_manager() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let state = StateManager::open(&db_path).await.unwrap();

        // Test registering a source
        let source = GitSource {
            name: "test".to_string(),
            repo_url: "https://github.com/test/repo.git".to_string(),
            branch: "main".to_string(),
            cache_dir: temp_dir.path().to_path_buf(),
            ssh_key_path: None,
            ssh_passphrase: None,
            access_token: Some("token".to_string()),
        };

        state.register_source(&source).await.unwrap();

        // List sources
        let sources = state.list_sources().await.unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].name, "test");
        assert_eq!(sources[0].access_token.as_deref(), Some("token"));

        // Get source
        let fetched = state.get_source("test").await.unwrap().unwrap();
        assert_eq!(fetched.repo_url, source.repo_url);

        // Update sync metadata
        state
            .update_source_sync_status("test", Some("abc123"))
            .await
            .unwrap();
        let fetched = state.get_source("test").await.unwrap().unwrap();
        assert_eq!(fetched.last_commit.as_deref(), Some("abc123"));
        assert!(fetched.updated_at.is_some());

        // Remove source
        let removed = state.remove_source("test").await.unwrap();
        assert!(removed);
        assert!(state.get_source("test").await.unwrap().is_none());
    }
}

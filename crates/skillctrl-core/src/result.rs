//! Result types for operations.

use std::collections::HashMap;
use std::path::PathBuf;
use crate::component::ComponentKind;
use crate::endpoint::Endpoint;
use crate::scope::Scope;
use serde::{Deserialize, Serialize};

/// Installation plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallPlan {
    /// Bundle being installed.
    pub bundle_id: String,
    /// Target endpoint.
    pub target: Endpoint,
    /// Installation scope.
    pub scope: Scope,
    /// Target project path (if project scope).
    pub project_path: Option<PathBuf>,
    /// Files to be created.
    pub files_to_create: Vec<InstallFile>,
    /// Files to be modified.
    pub files_to_modify: Vec<InstallFile>,
    /// Files to be backed up.
    pub files_to_backup: Vec<PathBuf>,
    /// Components to install.
    pub components: Vec<ComponentInstall>,
}

/// A file to be installed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallFile {
    /// Destination path.
    pub path: PathBuf,
    /// Content to write.
    pub content: String,
    /// Whether this is a binary file.
    #[serde(default)]
    pub binary: bool,
}

/// A component to be installed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInstall {
    /// Component ID.
    pub id: String,
    /// Component kind.
    pub kind: ComponentKind,
    /// Source path.
    pub source: PathBuf,
    /// Destination path.
    pub destination: PathBuf,
}

/// Installation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    /// Bundle ID that was installed.
    pub bundle_id: String,
    /// Target endpoint.
    pub target: Endpoint,
    /// Installation scope.
    pub scope: Scope,
    /// Files that were created.
    pub files_created: Vec<PathBuf>,
    /// Files that were modified.
    pub files_modified: Vec<PathBuf>,
    /// Backup files created.
    pub backup_files: Vec<PathBuf>,
    /// Whether installation was successful.
    pub success: bool,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl InstallResult {
    /// Creates a new successful install result.
    pub fn success(
        bundle_id: String,
        target: Endpoint,
        scope: Scope,
    ) -> Self {
        Self {
            bundle_id,
            target,
            scope,
            files_created: Vec::new(),
            files_modified: Vec::new(),
            backup_files: Vec::new(),
            success: true,
            metadata: HashMap::new(),
        }
    }

    /// Creates a new failed install result.
    pub fn failure(
        bundle_id: String,
        target: Endpoint,
        scope: Scope,
    ) -> Self {
        Self {
            bundle_id,
            target,
            scope,
            files_created: Vec::new(),
            files_modified: Vec::new(),
            backup_files: Vec::new(),
            success: false,
            metadata: HashMap::new(),
        }
    }
}

/// Uninstallation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallResult {
    /// Bundle ID that was uninstalled.
    pub bundle_id: String,
    /// Target endpoint.
    pub target: Endpoint,
    /// Files that were removed.
    pub files_removed: Vec<PathBuf>,
    /// Backup files that were restored.
    pub backups_restored: Vec<PathBuf>,
    /// Whether uninstallation was successful.
    pub success: bool,
}

/// Status result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResult {
    /// Bundle ID.
    pub bundle_id: String,
    /// Target endpoint.
    pub target: Endpoint,
    /// Installation scope.
    pub scope: Scope,
    /// Whether installed.
    pub installed: bool,
    /// Installed version (if any).
    pub version: Option<semver::Version>,
    /// Installed components.
    pub components: Vec<ComponentStatus>,
    /// Installation time (if installed).
    pub installed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Status of a single component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatus {
    /// Component ID.
    pub id: String,
    /// Component kind.
    pub kind: ComponentKind,
    /// Whether installed.
    pub installed: bool,
    /// Installation path.
    pub path: Option<PathBuf>,
}

/// Validation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed.
    pub valid: bool,
    /// Validation messages.
    pub messages: Vec<ValidationMessage>,
}

/// A validation message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMessage {
    /// Severity level.
    pub severity: crate::validation::ValidationSeverity,
    /// Message.
    pub message: String,
    /// Related path (if any).
    pub path: Option<PathBuf>,
}

/// Import plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPlan {
    /// Source endpoint.
    pub source: Endpoint,
    /// Source path.
    pub source_path: PathBuf,
    /// Detected artifacts.
    pub artifacts: Vec<ImportArtifact>,
    /// Bundle ID to create.
    pub bundle_id: String,
}

/// An artifact detected during import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportArtifact {
    /// Artifact type.
    pub kind: ComponentKind,
    /// Source path.
    pub path: PathBuf,
    /// Detected ID (if any).
    pub id: Option<String>,
    /// Whether this artifact is supported.
    pub supported: bool,
}

/// Export plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPlan {
    /// Source bundle.
    pub bundle_id: String,
    /// Target endpoint/format.
    pub target: Endpoint,
    /// Output path.
    pub output_path: PathBuf,
    /// Export format.
    pub format: ExportFormat,
}

/// Export format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExportFormat {
    /// Claude plugin format.
    ClaudePlugin,
    /// Claude marketplace.
    ClaudeMarketplace,
    /// Codex plugin.
    CodexPlugin,
    /// Cursor plugin.
    CursorPlugin,
}

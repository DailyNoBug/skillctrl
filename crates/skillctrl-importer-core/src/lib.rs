//! Core importer traits for skillctrl.
//!
//! This crate provides the foundational traits that all importers must implement.
//! Importers are responsible for scanning existing configurations from AI coding
//! assistants and converting them into skillctrl's canonical model.

use async_trait::async_trait;
use skillctrl_core::{ComponentKind, Endpoint, Error, ImportPlan, Result, ValidationReport};
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Request for scanning a source.
#[derive(Debug, Clone)]
pub struct ScanRequest {
    /// Source endpoint to scan from.
    pub from: Endpoint,

    /// Path to scan.
    pub path: PathBuf,

    /// Scan depth (how deep to recurse).
    pub depth: usize,

    /// Whether to follow symlinks.
    pub follow_symlinks: bool,

    /// Additional metadata.
    pub metadata: Metadata,
}

fn default_scan_depth() -> usize {
    10
}

/// Additional metadata for operations.
pub type Metadata = std::collections::HashMap<String, String>;

/// Artifacts detected during a scan.
#[derive(Debug, Clone)]
pub struct DetectedArtifacts {
    /// Source endpoint.
    pub source: Endpoint,

    /// Scan path.
    pub path: PathBuf,

    /// Detected artifacts.
    pub artifacts: Vec<Artifact>,

    /// Scan errors.
    pub errors: Vec<ScanError>,
}

/// An artifact detected during scanning.
#[derive(Debug, Clone)]
pub struct Artifact {
    /// Artifact type.
    pub kind: ComponentKind,

    /// Path to the artifact.
    pub path: PathBuf,

    /// Detected ID (if any).
    pub id: Option<String>,

    /// Artifact name (if any).
    pub name: Option<String>,

    /// Artifact description (if any).
    pub description: Option<String>,

    /// Whether this is a supported artifact type.
    pub supported: bool,

    /// Additional metadata.
    pub metadata: Metadata,
}

/// An error that occurred during scanning.
#[derive(Debug, Clone)]
pub struct ScanError {
    /// Path where the error occurred.
    pub path: PathBuf,

    /// Error message.
    pub message: String,

    /// Error severity.
    pub severity: ScanErrorSeverity,
}

/// Severity of a scan error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanErrorSeverity {
    /// Warning - scanning can continue.
    Warning,

    /// Error - artifact couldn't be scanned.
    Error,
}

/// Request for planning an import.
#[derive(Debug, Clone)]
pub struct ImportRequest {
    /// Source endpoint.
    pub from: Endpoint,

    /// Source path.
    pub path: PathBuf,

    /// Bundle ID to create.
    pub bundle_id: Option<String>,

    /// Bundle name.
    pub bundle_name: Option<String>,

    /// Bundle description.
    pub bundle_description: Option<String>,

    /// Whether to preserve original structure.
    pub preserve_structure: bool,

    /// Additional metadata.
    pub metadata: Metadata,
}

/// Request for applying an import.
#[derive(Debug, Clone)]
pub struct ApplyImportRequest {
    /// Import plan to apply.
    pub plan: ImportPlan,

    /// Output directory.
    pub out: PathBuf,

    /// Whether to overwrite existing files.
    pub overwrite: bool,

    /// Additional metadata.
    pub metadata: Metadata,
}

/// Result of an import operation.
#[derive(Debug, Clone)]
pub struct ImportResult {
    /// Bundle that was created.
    pub bundle_id: String,

    /// Output path.
    pub output_path: PathBuf,

    /// Files that were created.
    pub files_created: Vec<PathBuf>,

    /// Artifacts that were imported.
    pub artifacts_imported: Vec<ArtifactImport>,

    /// Warnings generated during import.
    pub warnings: Vec<String>,

    /// Whether import was successful.
    pub success: bool,
}

/// An artifact that was imported.
#[derive(Debug, Clone)]
pub struct ArtifactImport {
    /// Original artifact kind.
    pub kind: ComponentKind,

    /// Original path.
    pub source_path: PathBuf,

    /// Imported path.
    pub destination_path: PathBuf,

    /// Component ID in the bundle.
    pub component_id: String,

    /// Whether there was any loss during conversion.
    pub had_loss: bool,

    /// Loss description (if any).
    pub loss_description: Option<String>,
}

/// Core importer trait.
///
/// All importers must implement this trait.
#[async_trait]
pub trait Importer: Send + Sync + 'static {
    /// Returns the endpoint this importer handles.
    fn endpoint(&self) -> Endpoint;

    /// Returns the version of this importer.
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    /// Scans a source path for artifacts.
    ///
    /// Detects all importable artifacts at the given path.
    async fn scan(&self, req: &ScanRequest) -> Result<DetectedArtifacts>;

    /// Plans an import.
    ///
    /// Analyzes detected artifacts and creates an import plan.
    async fn plan_import(
        &self,
        req: &ImportRequest,
        artifacts: &DetectedArtifacts,
    ) -> Result<ImportPlan>;

    /// Applies an import plan.
    ///
    /// Generates bundle manifest and copies components to the output directory.
    async fn apply_import(&self, req: &ApplyImportRequest) -> Result<ImportResult>;

    /// Validates that a path can be imported.
    ///
    /// Checks that the path exists and contains valid artifacts.
    async fn validate_source(&self, path: &Path) -> Result<ValidationReport> {
        let mut report = ValidationReport::new();

        if !path.exists() {
            report.error(format!("Path does not exist: {}", path.display()));
        } else if !path.is_dir() {
            report.warning(format!("Path is not a directory: {}", path.display()));
        }

        Ok(report)
    }

    /// Estimates the bundle ID for detected artifacts.
    ///
    /// Returns a suggested bundle ID based on the detected artifacts.
    async fn estimate_bundle_id(&self, artifacts: &DetectedArtifacts) -> Result<String> {
        // Default: use directory name
        Ok(artifacts
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("imported-bundle")
            .to_string())
    }
}

/// Dynamic importer registry.
///
/// Allows registering and retrieving importers at runtime.
pub struct ImporterRegistry {
    importers: std::collections::HashMap<String, Box<dyn DynImporter>>,
}

impl ImporterRegistry {
    /// Creates a new importer registry.
    pub fn new() -> Self {
        Self {
            importers: std::collections::HashMap::new(),
        }
    }

    /// Registers an importer.
    pub fn register<I>(&mut self, importer: I)
    where
        I: Importer + 'static,
    {
        let key = importer.endpoint().to_string();
        self.importers.insert(key, Box::new(importer));
    }

    /// Gets an importer by endpoint.
    pub fn get(&self, endpoint: &Endpoint) -> Option<&dyn DynImporter> {
        self.importers.get(endpoint.as_str()).map(|i| i.as_ref())
    }

    /// Lists all registered endpoints.
    pub fn endpoints(&self) -> Vec<Endpoint> {
        self.importers
            .keys()
            .filter_map(|k| Endpoint::from_str(k).ok())
            .collect()
    }
}

impl Default for ImporterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Dynamic importer trait object.
///
/// This trait allows importers to be used as trait objects.
pub trait DynImporter: Send + Sync {
    /// Returns the endpoint.
    fn endpoint(&self) -> Endpoint;

    /// Returns the version.
    fn version(&self) -> &str;

    /// Scans for artifacts.
    fn scan<'life0, 'async_trait>(
        &'life0 self,
        req: &'life0 ScanRequest,
    ) -> core::pin::Pin<
        Box<dyn futures::Future<Output = Result<DetectedArtifacts>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait;

    /// Plans an import.
    fn plan_import<'life0, 'async_trait>(
        &'life0 self,
        req: &'life0 ImportRequest,
        artifacts: &'life0 DetectedArtifacts,
    ) -> core::pin::Pin<Box<dyn futures::Future<Output = Result<ImportPlan>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait;

    /// Applies an import.
    fn apply_import<'life0, 'async_trait>(
        &'life0 self,
        req: &'life0 ApplyImportRequest,
    ) -> core::pin::Pin<Box<dyn futures::Future<Output = Result<ImportResult>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait;
}

impl<T: Importer> DynImporter for T {
    fn endpoint(&self) -> Endpoint {
        Importer::endpoint(self)
    }

    fn version(&self) -> &str {
        Importer::version(self)
    }

    fn scan<'life0, 'async_trait>(
        &'life0 self,
        req: &'life0 ScanRequest,
    ) -> core::pin::Pin<
        Box<dyn futures::Future<Output = Result<DetectedArtifacts>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(Importer::scan(self, req))
    }

    fn plan_import<'life0, 'async_trait>(
        &'life0 self,
        req: &'life0 ImportRequest,
        artifacts: &'life0 DetectedArtifacts,
    ) -> core::pin::Pin<Box<dyn futures::Future<Output = Result<ImportPlan>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(Importer::plan_import(self, req, artifacts))
    }

    fn apply_import<'life0, 'async_trait>(
        &'life0 self,
        req: &'life0 ApplyImportRequest,
    ) -> core::pin::Pin<Box<dyn futures::Future<Output = Result<ImportResult>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(Importer::apply_import(self, req))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_request_default() {
        let req = ScanRequest {
            from: Endpoint::Known(skillctrl_core::KnownEndpoint::ClaudeCode),
            path: PathBuf::from("/test"),
            depth: default_scan_depth(),
            follow_symlinks: false,
            metadata: Metadata::new(),
        };

        assert_eq!(req.depth, 10);
        assert!(!req.follow_symlinks);
    }

    #[test]
    fn test_importer_registry() {
        let mut registry = ImporterRegistry::new();
        assert_eq!(registry.endpoints().len(), 0);
    }
}

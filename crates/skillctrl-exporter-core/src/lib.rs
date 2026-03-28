//! Core exporter traits for skillctrl.
//!
//! This crate provides the foundational traits for exporting bundles
//! to native marketplace formats.

use async_trait::async_trait;
use skillctrl_core::{BundleManifest, Endpoint, Error, Result};
use std::path::PathBuf;
use std::str::FromStr;

/// Export request.
#[derive(Debug, Clone)]
pub struct ExportRequest {
    /// Bundle manifest to export.
    pub bundle: BundleManifest,

    /// Source directory containing bundle components.
    pub source_dir: PathBuf,

    /// Output directory.
    pub output_dir: PathBuf,

    /// Export format.
    pub format: ExportFormat,

    /// Additional metadata.
    pub metadata: Metadata,
}

/// Additional metadata for export.
pub type Metadata = std::collections::HashMap<String, String>;

/// Export format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Claude plugin format (.claude-plugin)
    ClaudePlugin,

    /// Claude marketplace format
    ClaudeMarketplace,

    /// Codex plugin format
    CodexPlugin,

    /// Cursor plugin format
    CursorPlugin,
}

/// Export result.
#[derive(Debug, Clone)]
pub struct ExportResult {
    /// Export format used.
    pub format: ExportFormat,

    /// Output directory path.
    pub output_path: PathBuf,

    /// Files created.
    pub files_created: Vec<PathBuf>,

    /// Warnings generated during export.
    pub warnings: Vec<String>,

    /// Whether export was successful.
    pub success: bool,
}

/// Core exporter trait.
///
/// All exporters must implement this trait.
#[async_trait]
pub trait Exporter: Send + Sync + 'static {
    /// Returns the endpoint this exporter handles.
    fn endpoint(&self) -> Endpoint;

    /// Returns the version of this exporter.
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    /// Returns supported export formats.
    fn supported_formats(&self) -> Vec<ExportFormat>;

    /// Plans an export.
    ///
    /// Analyzes the bundle and determines what files need to be created.
    async fn plan_export(&self, req: &ExportRequest) -> Result<ExportPlan>;

    /// Applies an export plan.
    ///
    /// Creates the exported plugin/marketplace files.
    async fn apply_export(&self, plan: &ExportPlan) -> Result<ExportResult>;
}

/// Export plan.
#[derive(Debug, Clone)]
pub struct ExportPlan {
    /// Bundle being exported.
    pub bundle_id: String,

    /// Export format.
    pub format: ExportFormat,

    /// Output directory.
    pub output_dir: PathBuf,

    /// Files to create.
    pub files_to_create: Vec<ExportFile>,
}

/// A file to create during export.
#[derive(Debug, Clone)]
pub struct ExportFile {
    /// Destination path (relative to output dir).
    pub path: PathBuf,

    /// Content to write.
    pub content: String,

    /// Whether this is a binary file.
    pub binary: bool,
}

/// Dynamic exporter registry.
///
/// Allows registering and retrieving exporters at runtime.
pub struct ExporterRegistry {
    exporters: std::collections::HashMap<String, Box<dyn DynExporter>>,
}

impl ExporterRegistry {
    /// Creates a new exporter registry.
    pub fn new() -> Self {
        Self {
            exporters: std::collections::HashMap::new(),
        }
    }

    /// Registers an exporter.
    pub fn register<E>(&mut self, exporter: E)
    where
        E: Exporter + 'static,
    {
        let key = exporter.endpoint().to_string();
        self.exporters.insert(key, Box::new(exporter));
    }

    /// Gets an exporter by endpoint.
    pub fn get(&self, endpoint: &Endpoint) -> Option<&dyn DynExporter> {
        self.exporters.get(endpoint.as_str()).map(|e| e.as_ref())
    }

    /// Lists all registered endpoints.
    pub fn endpoints(&self) -> Vec<Endpoint> {
        self.exporters
            .keys()
            .filter_map(|k| Endpoint::from_str(k).ok())
            .collect()
    }
}

impl Default for ExporterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Dynamic exporter trait object.
///
/// This trait allows exporters to be used as trait objects.
pub trait DynExporter: Send + Sync {
    /// Returns the endpoint.
    fn endpoint(&self) -> Endpoint;

    /// Returns the version.
    fn version(&self) -> &str;

    /// Returns supported formats.
    fn supported_formats(&self) -> Vec<ExportFormat>;

    /// Plans an export.
    fn plan_export<'life0, 'async_trait>(
        &'life0 self,
        req: &'life0 ExportRequest,
    ) -> core::pin::Pin<Box<dyn futures::Future<Output = Result<ExportPlan>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait;

    /// Applies an export.
    fn apply_export<'life0, 'async_trait>(
        &'life0 self,
        plan: &'life0 ExportPlan,
    ) -> core::pin::Pin<Box<dyn futures::Future<Output = Result<ExportResult>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait;
}

impl<T: Exporter> DynExporter for T {
    fn endpoint(&self) -> Endpoint {
        Exporter::endpoint(self)
    }

    fn version(&self) -> &str {
        Exporter::version(self)
    }

    fn supported_formats(&self) -> Vec<ExportFormat> {
        Exporter::supported_formats(self)
    }

    fn plan_export<'life0, 'async_trait>(
        &'life0 self,
        req: &'life0 ExportRequest,
    ) -> core::pin::Pin<Box<dyn futures::Future<Output = Result<ExportPlan>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(Exporter::plan_export(self, req))
    }

    fn apply_export<'life0, 'async_trait>(
        &'life0 self,
        plan: &'life0 ExportPlan,
    ) -> core::pin::Pin<Box<dyn futures::Future<Output = Result<ExportResult>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(Exporter::apply_export(self, plan))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exporter_registry() {
        let mut registry = ExporterRegistry::new();
        assert_eq!(registry.endpoints().len(), 0);
    }
}

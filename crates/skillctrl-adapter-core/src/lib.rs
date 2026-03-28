//! Core adapter traits for skillctrl.
//!
//! This crate provides the foundational traits that all adapters must implement.
//! Adapters are responsible for translating skillctrl's canonical model into
//! the specific formats and directory structures used by different AI coding assistants.

use async_trait::async_trait;
pub use skillctrl_core::{AdapterCapabilities, InstallFile, InstallPlan, InstallResult};
use skillctrl_core::{ComponentKind, Endpoint, Error, Result, Scope, ValidationReport};
use std::path::PathBuf;
use std::str::FromStr;

/// Context for installation operations.
#[derive(Debug, Clone)]
pub struct InstallContext {
    /// Target endpoint.
    pub target: Endpoint,

    /// Installation scope.
    pub scope: Scope,

    /// Project path (if project scope).
    pub project_path: Option<PathBuf>,

    /// Dry run mode - don't make actual changes.
    pub dry_run: bool,

    /// Conflict resolution strategy.
    pub conflict_strategy: ConflictStrategy,

    /// Additional metadata.
    pub metadata: Metadata,
}

/// Additional metadata for operations.
pub type Metadata = std::collections::HashMap<String, String>;

/// Conflict resolution strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictStrategy {
    /// Skip conflicting files.
    Skip,

    /// Overwrite conflicting files.
    Overwrite,

    /// Backup then overwrite.
    BackupThenWrite,

    /// Ask the user.
    Prompt,

    /// Rename with suffix.
    Rename,
}

/// Request for uninstallation.
#[derive(Debug, Clone)]
pub struct UninstallRequest {
    /// Bundle ID to uninstall.
    pub bundle_id: String,

    /// Target endpoint.
    pub target: Endpoint,

    /// Installation scope.
    pub scope: Scope,

    /// Project path (if project scope).
    pub project_path: Option<PathBuf>,

    /// Dry run mode.
    pub dry_run: bool,

    /// Additional metadata.
    pub metadata: Metadata,
}

/// Request for status query.
#[derive(Debug, Clone)]
pub struct StatusRequest {
    /// Bundle ID to query.
    pub bundle_id: Option<String>,

    /// Target endpoint.
    pub target: Endpoint,

    /// Installation scope.
    pub scope: Scope,

    /// Project path (if project scope).
    pub project_path: Option<PathBuf>,
}

/// Status report for an endpoint or bundle.
#[derive(Debug, Clone)]
pub struct StatusReport {
    /// Target endpoint.
    pub target: Endpoint,

    /// Installation scope.
    pub scope: Scope,

    /// Installed bundles.
    pub installed_bundles: Vec<BundleStatus>,

    /// Endpoint-specific status information.
    pub endpoint_status: EndpointStatus,
}

/// Status of an installed bundle.
#[derive(Debug, Clone)]
pub struct BundleStatus {
    /// Bundle ID.
    pub bundle_id: String,

    /// Installed version.
    pub version: semver::Version,

    /// Installation time.
    pub installed_at: chrono::DateTime<chrono::Utc>,

    /// Installed components.
    pub components: Vec<ComponentStatus>,

    /// Installation path.
    pub install_path: PathBuf,
}

/// Status of a single component.
#[derive(Debug, Clone)]
pub struct ComponentStatus {
    /// Component ID.
    pub id: String,

    /// Component kind.
    pub kind: ComponentKind,

    /// Installation path.
    pub path: PathBuf,

    /// Whether component is present and valid.
    pub is_valid: bool,
}

/// Endpoint-specific status information.
#[derive(Debug, Clone)]
pub struct EndpointStatus {
    /// Whether the endpoint is available.
    pub available: bool,

    /// Endpoint version (if detectable).
    pub version: Option<String>,

    /// Config directory path.
    pub config_dir: Option<PathBuf>,

    /// Additional endpoint-specific info.
    pub info: Metadata,
}

/// Hook result from lifecycle hooks.
#[derive(Debug, Clone)]
pub struct HookResult {
    /// Whether the hook passed.
    pub success: bool,

    /// Messages from the hook.
    pub messages: Vec<String>,

    /// Additional data.
    pub data: Metadata,
}

impl HookResult {
    /// Creates a successful hook result.
    pub fn success() -> Self {
        Self {
            success: true,
            messages: Vec::new(),
            data: Metadata::new(),
        }
    }

    /// Creates a failed hook result.
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            messages: vec![message.into()],
            data: Metadata::new(),
        }
    }
}

/// Rollback result after a failed installation.
#[derive(Debug, Clone)]
pub struct RollbackResult {
    /// Whether rollback was successful.
    pub success: bool,

    /// Files that were restored.
    pub restored_files: Vec<PathBuf>,

    /// Files that were cleaned up.
    pub cleaned_files: Vec<PathBuf>,

    /// Messages from the rollback process.
    pub messages: Vec<String>,
}

/// Core adapter trait.
///
/// All adapters must implement this trait. It provides lifecycle hooks
/// and capability queries.
#[async_trait]
pub trait Adapter: Send + Sync + 'static {
    /// Returns the endpoint this adapter handles.
    fn endpoint(&self) -> Endpoint;

    /// Returns the version of this adapter.
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    /// Returns the capabilities of this adapter.
    fn capabilities(&self) -> AdapterCapabilities;

    /// Pre-installation hook.
    ///
    /// Called before installation begins. Can be used to validate
    /// the environment or prepare for installation.
    async fn pre_install(&self, _ctx: &InstallContext) -> Result<HookResult> {
        Ok(HookResult::success())
    }

    /// Post-installation hook.
    ///
    /// Called after installation completes. Can be used to perform
    /// cleanup or additional setup.
    async fn post_install(&self, _result: &InstallResult) -> Result<HookResult> {
        Ok(HookResult::success())
    }

    /// Pre-uninstallation hook.
    async fn pre_uninstall(&self, _ctx: &UninstallRequest) -> Result<HookResult> {
        Ok(HookResult::success())
    }

    /// Post-uninstallation hook.
    async fn post_uninstall(
        &self,
        _result: &skillctrl_core::UninstallResult,
    ) -> Result<HookResult> {
        Ok(HookResult::success())
    }
}

/// Installation capability trait.
///
/// Adapters that can install components must implement this trait.
#[async_trait]
pub trait InstallAdapter: Adapter {
    /// Plans an installation.
    ///
    /// Analyzes the bundle and determines what files need to be created,
    /// modified, or backed up. Returns an installation plan.
    async fn plan_install(
        &self,
        bundle: &skillctrl_core::BundleManifest,
        ctx: &InstallContext,
    ) -> Result<InstallPlan>;

    /// Applies an installation plan.
    ///
    /// Executes the installation plan, creating and modifying files as needed.
    async fn apply_install(&self, plan: &InstallPlan) -> Result<InstallResult>;

    /// Rolls back a failed installation.
    ///
    /// Attempts to restore the system to its state before installation.
    async fn rollback_install(&self, plan: &InstallPlan) -> Result<RollbackResult> {
        // Default implementation - basic cleanup
        Ok(RollbackResult {
            success: true,
            restored_files: Vec::new(),
            cleaned_files: Vec::new(),
            messages: vec!["Rollback not supported".to_string()],
        })
    }
}

/// Uninstallation capability trait.
///
/// Adapters that can uninstall components must implement this trait.
#[async_trait]
pub trait UninstallAdapter: Adapter {
    /// Plans an uninstallation.
    async fn plan_uninstall(&self, req: &UninstallRequest) -> Result<UninstallPlan>;

    /// Applies an uninstallation plan.
    async fn apply_uninstall(
        &self,
        plan: &UninstallPlan,
    ) -> Result<skillctrl_core::UninstallResult>;
}

/// Uninstallation plan.
#[derive(Debug, Clone)]
pub struct UninstallPlan {
    /// Bundle ID to uninstall.
    pub bundle_id: String,

    /// Files to remove.
    pub files_to_remove: Vec<PathBuf>,

    /// Backups to restore.
    pub backups_to_restore: Vec<PathBuf>,

    /// Components to uninstall.
    pub components: Vec<ComponentToRemove>,
}

/// A component to remove during uninstallation.
#[derive(Debug, Clone)]
pub struct ComponentToRemove {
    /// Component ID.
    pub id: String,

    /// Component kind.
    pub kind: ComponentKind,

    /// Path to remove.
    pub path: PathBuf,
}

/// Status query capability trait.
///
/// Adapters that can report installation status must implement this trait.
#[async_trait]
pub trait StatusAdapter: Adapter {
    /// Queries the status of installations.
    async fn status(&self, req: &StatusRequest) -> Result<StatusReport>;

    /// Validates an installed bundle.
    async fn validate_installation(
        &self,
        bundle_id: &str,
        scope: Scope,
        project_path: Option<PathBuf>,
    ) -> Result<ValidationReport> {
        // Default implementation - no validation
        Ok(ValidationReport::new())
    }
}

/// Dynamic adapter registry.
///
/// Allows registering and retrieving adapters at runtime.
pub struct AdapterRegistry {
    adapters: std::collections::HashMap<String, Box<dyn DynAdapter>>,
}

impl AdapterRegistry {
    /// Creates a new adapter registry.
    pub fn new() -> Self {
        Self {
            adapters: std::collections::HashMap::new(),
        }
    }

    /// Registers an adapter.
    pub fn register<A>(&mut self, adapter: A)
    where
        A: Adapter + 'static,
    {
        let key = adapter.endpoint().to_string();
        self.adapters.insert(key, Box::new(adapter));
    }

    /// Gets an adapter by endpoint.
    pub fn get(&self, endpoint: &Endpoint) -> Option<&dyn DynAdapter> {
        self.adapters.get(endpoint.as_str()).map(|a| a.as_ref())
    }

    /// Lists all registered endpoints.
    pub fn endpoints(&self) -> Vec<Endpoint> {
        self.adapters
            .keys()
            .filter_map(|k| Endpoint::from_str(k).ok())
            .collect()
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Dynamic adapter trait object.
///
/// This trait allows adapters to be used as trait objects.
pub trait DynAdapter: Send + Sync {
    /// Returns the endpoint.
    fn endpoint(&self) -> Endpoint;

    /// Returns the version.
    fn version(&self) -> &str;

    /// Returns the capabilities.
    fn capabilities(&self) -> AdapterCapabilities;

    /// Pre-install hook.
    fn pre_install<'life0, 'async_trait>(
        &'life0 self,
        ctx: &'life0 InstallContext,
    ) -> core::pin::Pin<Box<dyn futures::Future<Output = Result<HookResult>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait;

    /// Post-install hook.
    fn post_install<'life0, 'async_trait>(
        &'life0 self,
        result: &'life0 InstallResult,
    ) -> core::pin::Pin<Box<dyn futures::Future<Output = Result<HookResult>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait;
}

impl<T: Adapter> DynAdapter for T {
    fn endpoint(&self) -> Endpoint {
        Adapter::endpoint(self)
    }

    fn version(&self) -> &str {
        Adapter::version(self)
    }

    fn capabilities(&self) -> AdapterCapabilities {
        Adapter::capabilities(self)
    }

    fn pre_install<'life0, 'async_trait>(
        &'life0 self,
        ctx: &'life0 InstallContext,
    ) -> core::pin::Pin<Box<dyn futures::Future<Output = Result<HookResult>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(Adapter::pre_install(self, ctx))
    }

    fn post_install<'life0, 'async_trait>(
        &'life0 self,
        result: &'life0 InstallResult,
    ) -> core::pin::Pin<Box<dyn futures::Future<Output = Result<HookResult>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(Adapter::post_install(self, result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_result() {
        let result = HookResult::success();
        assert!(result.success);

        let result = HookResult::failure("test error");
        assert!(!result.success);
        assert_eq!(result.messages.len(), 1);
    }
}

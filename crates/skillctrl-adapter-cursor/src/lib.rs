//! Cursor adapter for skillctrl.
//!
//! This adapter handles installation of components to Cursor's
//! `.cursor` directory structure.

use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use skillctrl_adapter_core::{
    Adapter, InstallAdapter, UninstallAdapter, StatusAdapter,
    AdapterCapabilities, ConflictStrategy, HookResult, InstallContext,
    InstallPlan, InstallResult, RollbackResult, UninstallPlan, UninstallRequest,
    StatusRequest, StatusReport, BundleStatus, ComponentStatus, EndpointStatus,
    UninstallAdapter as UninstallAdapterTrait, StatusAdapter as StatusAdapterTrait,
};
use skillctrl_core::{
    BundleManifest, ComponentKind, Endpoint, Error, Result, Scope, ValidationReport,
    KnownEndpoint, InstallFile, ComponentInstall,
};

/// Cursor adapter.
pub struct CursorAdapter {
    /// Whether to enable verbose logging.
    verbose: bool,
}

impl CursorAdapter {
    /// Creates a new Cursor adapter.
    pub fn new() -> Self {
        Self { verbose: false }
    }

    /// Creates a new verbose Cursor adapter.
    pub fn verbose() -> Self {
        Self { verbose: true }
    }

    /// Returns the .cursor directory path for the given scope and project.
    fn cursor_dir(&self, scope: Scope, project_path: Option<&Path>) -> Result<PathBuf> {
        match scope {
            Scope::Project => {
                let project = project_path.ok_or_else(|| {
                    Error::InvalidInput("project path required for project scope".to_string())
                })?;
                Ok(project.join(".cursor"))
            }
            Scope::User => {
                let config = dirs::config_dir().ok_or_else(|| {
                    Error::InvalidInput("could not determine config directory".to_string())
                })?;
                Ok(config.join("cursor"))
            }
        }
    }

    /// Ensures the .cursor directory exists.
    fn ensure_cursor_dir(&self, cursor_dir: &Path) -> Result<()> {
        if !cursor_dir.exists() {
            fs::create_dir_all(cursor_dir).map_err(|e| {
                Error::Other(format!("failed to create .cursor directory: {}", e))
            })?;
        }
        Ok(())
    }

    /// Reads component content from a file.
    fn read_component_content(&self, path: &Path) -> Result<String> {
        fs::read_to_string(path).map_err(|e| {
            Error::Other(format!("failed to read component file {}: {}", path.display(), e))
        })
    }
}

impl Default for CursorAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Adapter for CursorAdapter {
    fn endpoint(&self) -> Endpoint {
        Endpoint::Known(KnownEndpoint::Cursor)
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            can_install: true,
            can_import: true,
            can_export: true,
            can_query_status: true,
            supported_scopes: vec![Scope::Project, Scope::User],
            supported_kinds: vec![
                ComponentKind::Rule,
                ComponentKind::Skill,
                ComponentKind::Resource,
            ],
            max_manifest_version: semver::Version::new(1, 0, 0),
        }
    }

    async fn pre_install(&self, ctx: &InstallContext) -> Result<HookResult> {
        let cursor_dir = self.cursor_dir(ctx.scope, ctx.project_path.as_deref())?;

        if let Some(parent) = cursor_dir.parent() {
            if !parent.exists() {
                return Ok(HookResult::failure(
                    format!("parent directory does not exist: {}", parent.display()),
                ));
            }
        }

        Ok(HookResult::success())
    }

    async fn post_install(&self, result: &InstallResult) -> Result<HookResult> {
        tracing::info!("Installation completed for bundle {}", result.bundle_id);
        Ok(HookResult::success())
    }
}

#[async_trait]
impl InstallAdapter for CursorAdapter {
    async fn plan_install(
        &self,
        bundle: &BundleManifest,
        ctx: &InstallContext,
    ) -> Result<InstallPlan> {
        let cursor_dir = self.cursor_dir(ctx.scope, ctx.project_path.as_deref())?;
        self.ensure_cursor_dir(&cursor_dir)?;

        let mut plan = InstallPlan {
            bundle_id: bundle.id.clone(),
            target: ctx.target.clone(),
            scope: ctx.scope,
            project_path: ctx.project_path.clone(),
            files_to_create: Vec::new(),
            files_to_modify: Vec::new(),
            files_to_backup: Vec::new(),
            components: Vec::new(),
        };

        let base_path = ctx
            .metadata
            .get("bundle_path")
            .map(PathBuf::from)
            .unwrap_or_default();

        for component in &bundle.components {
            let component_path = base_path.join(&component.path);

            match component.kind {
                ComponentKind::Rule => {
                    self.plan_rule_install(
                        &component,
                        &component_path,
                        &cursor_dir,
                        &mut plan,
                        ctx,
                    )?;
                }
                ComponentKind::Skill => {
                    // In Cursor, skills are typically installed as rules
                    self.plan_skill_as_rule_install(
                        &component,
                        &component_path,
                        &cursor_dir,
                        &mut plan,
                        ctx,
                    )?;
                }
                ComponentKind::Resource => {
                    self.plan_resource_install(
                        &component,
                        &component_path,
                        &cursor_dir,
                        &mut plan,
                        ctx,
                    )?;
                }
                _ => {
                    tracing::warn!("Unsupported component kind for Cursor: {:?}", component.kind);
                }
            }
        }

        Ok(plan)
    }

    async fn apply_install(&self, plan: &InstallPlan) -> Result<InstallResult> {
        let mut result = InstallResult::success(plan.bundle_id.clone(), plan.target.clone(), plan.scope);

        for file in &plan.files_to_create {
            if let Some(parent) = file.path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    Error::Other(format!("failed to create directory {}: {}", parent.display(), e))
                })?;
            }

            fs::write(&file.path, &file.content).map_err(|e| {
                Error::Other(format!("failed to write file {}: {}", file.path.display(), e))
            })?;

            result.files_created.push(file.path.clone());
        }

        for file in &plan.files_to_modify {
            fs::write(&file.path, &file.content).map_err(|e| {
                Error::Other(format!("failed to write file {}: {}", file.path.display(), e))
            })?;

            result.files_modified.push(file.path.clone());
        }

        Ok(result)
    }

    async fn rollback_install(&self, plan: &InstallPlan) -> Result<RollbackResult> {
        let mut cleaned = Vec::new();

        for file in &plan.files_to_create {
            if file.path.exists() {
                fs::remove_file(&file.path).map_err(|e| {
                    Error::Other(format!("failed to remove file {}: {}", file.path.display(), e))
                })?;
                cleaned.push(file.path.clone());
            }
        }

        Ok(RollbackResult {
            success: true,
            restored_files: Vec::new(),
            cleaned_files: cleaned,
            messages: vec!["Rollback completed".to_string()],
        })
    }
}

impl CursorAdapter {
    /// Plans installation of a rule component as .mdc file.
    fn plan_rule_install(
        &self,
        component: &skillctrl_core::ComponentRef,
        source_path: &Path,
        cursor_dir: &Path,
        plan: &mut InstallPlan,
        ctx: &InstallContext,
    ) -> Result<()> {
        let rules_dir = cursor_dir.join("rules");
        let rule_file = rules_dir.join(format!("{}.mdc", component.id));

        let content = self.read_component_content(source_path)?;

        // Convert to .mdc format with frontmatter
        let mdc_content = self.to_mdc_format(component, &content)?;

        plan.files_to_create.push(InstallFile {
            path: rule_file.clone(),
            content: mdc_content,
            binary: false,
        });

        plan.components.push(ComponentInstall {
            id: component.id.clone(),
            kind: ComponentKind::Rule,
            source: source_path.to_path_buf(),
            destination: rule_file,
        });

        Ok(())
    }

    /// Plans installation of a skill as a rule (Cursor compatibility).
    fn plan_skill_as_rule_install(
        &self,
        component: &skillctrl_core::ComponentRef,
        source_path: &Path,
        cursor_dir: &Path,
        plan: &mut InstallPlan,
        ctx: &InstallContext,
    ) -> Result<()> {
        let rules_dir = cursor_dir.join("rules");
        let rule_file = rules_dir.join(format!("{}.mdc", component.id));

        let content = self.read_component_content(source_path)?;

        // Convert skill to .mdc format
        let mdc_content = self.skill_to_mdc_format(component, &content)?;

        plan.files_to_create.push(InstallFile {
            path: rule_file.clone(),
            content: mdc_content,
            binary: false,
        });

        plan.components.push(ComponentInstall {
            id: component.id.clone(),
            kind: ComponentKind::Skill,
            source: source_path.to_path_buf(),
            destination: rule_file,
        });

        Ok(())
    }

    /// Plans installation of a resource component.
    fn plan_resource_install(
        &self,
        component: &skillctrl_core::ComponentRef,
        source_path: &Path,
        cursor_dir: &Path,
        plan: &mut InstallPlan,
        ctx: &InstallContext,
    ) -> Result<()> {
        let resource_name = component.id.clone();
        let resource_path = cursor_dir.join("resources").join(&resource_name);

        let content = self.read_component_content(source_path)?;

        plan.files_to_create.push(InstallFile {
            path: resource_path.clone(),
            content,
            binary: false,
        });

        plan.components.push(ComponentInstall {
            id: component.id.clone(),
            kind: ComponentKind::Resource,
            source: source_path.to_path_buf(),
            destination: resource_path,
        });

        Ok(())
    }

    /// Converts content to .mdc format with frontmatter.
    fn to_mdc_format(&self, component: &skillctrl_core::ComponentRef, content: &str) -> Result<String> {
        let mut mdc = String::new();

        // Frontmatter
        mdc.push_str("---\n");
        mdc.push_str(&format!("id: {}\n", component.id));
        if let Some(ref name) = component.display_name {
            mdc.push_str(&format!("name: {}\n", name));
        }
        if let Some(ref description) = component.description {
            mdc.push_str(&format!("description: {}\n", description));
        }
        mdc.push_str("---\n");
        mdc.push('\n');

        // Content
        mdc.push_str(content);

        Ok(mdc)
    }

    /// Converts a skill to .mdc format.
    fn skill_to_mdc_format(&self, component: &skillctrl_core::ComponentRef, content: &str) -> Result<String> {
        let mut mdc = String::new();

        // Frontmatter for skill
        mdc.push_str("---\n");
        mdc.push_str(&format!("id: {}\n", component.id));
        mdc.push_str("kind: skill\n");
        if let Some(ref description) = component.description {
            mdc.push_str(&format!("description: {}\n", description));
        }
        mdc.push_str("---\n");
        mdc.push('\n');

        // Content
        mdc.push_str(content);

        Ok(mdc)
    }
}

#[async_trait]
impl UninstallAdapterTrait for CursorAdapter {
    async fn plan_uninstall(&self, req: &UninstallRequest) -> Result<UninstallPlan> {
        Ok(UninstallPlan {
            bundle_id: req.bundle_id.clone(),
            files_to_remove: Vec::new(),
            backups_to_restore: Vec::new(),
            components: Vec::new(),
        })
    }

    async fn apply_uninstall(
        &self,
        _plan: &UninstallPlan,
    ) -> Result<skillctrl_core::UninstallResult> {
        Ok(skillctrl_core::UninstallResult {
            bundle_id: String::new(),
            target: self.endpoint(),
            files_removed: Vec::new(),
            backups_restored: Vec::new(),
            success: true,
        })
    }
}

#[async_trait]
impl StatusAdapterTrait for CursorAdapter {
    async fn status(&self, req: &StatusRequest) -> Result<StatusReport> {
        let cursor_dir = self.cursor_dir(req.scope, req.project_path.as_deref())?;

        let mut info = HashMap::new();
        info.insert(
            "cursor_dir".to_string(),
            cursor_dir.to_string_lossy().to_string(),
        );

        Ok(StatusReport {
            target: req.target.clone(),
            scope: req.scope,
            installed_bundles: Vec::new(),
            endpoint_status: EndpointStatus {
                available: cursor_dir.exists(),
                version: None,
                config_dir: Some(cursor_dir),
                info,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = CursorAdapter::new();
        assert_eq!(adapter.endpoint(), Endpoint::Known(KnownEndpoint::Cursor));
    }

    #[test]
    fn test_mdc_format() {
        let adapter = CursorAdapter::new();

        let component = skillctrl_core::ComponentRef {
            kind: ComponentKind::Rule,
            id: "test-rule".to_string(),
            path: PathBuf::from("test.md"),
            display_name: Some("Test Rule".to_string()),
            description: Some("A test rule".to_string()),
        };

        let content = "# Test Rule\n\nThis is a test.";
        let mdc = adapter.to_mdc_format(&component, content).unwrap();

        assert!(mdc.contains("---"));
        assert!(mdc.contains("id: test-rule"));
        assert!(mdc.contains("name: Test Rule"));
        assert!(mdc.contains("description: A test rule"));
        assert!(mdc.contains("# Test Rule"));
    }
}

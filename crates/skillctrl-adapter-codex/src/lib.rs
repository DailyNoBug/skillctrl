//! Codex adapter for skillctrl.
//!
//! This adapter handles installation of components to OpenAI's Codex
//! `.codex` directory structure and configuration.

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

/// Codex adapter.
pub struct CodexAdapter {
    /// Whether to enable verbose logging.
    verbose: bool,
}

impl CodexAdapter {
    /// Creates a new Codex adapter.
    pub fn new() -> Self {
        Self { verbose: false }
    }

    /// Creates a new verbose Codex adapter.
    pub fn verbose() -> Self {
        Self { verbose: true }
    }

    /// Returns the .codex directory path for the given scope and project.
    fn codex_dir(&self, scope: Scope, project_path: Option<&Path>) -> Result<PathBuf> {
        match scope {
            Scope::Project => {
                let project = project_path.ok_or_else(|| {
                    Error::InvalidInput("project path required for project scope".to_string())
                })?;
                Ok(project.join(".codex"))
            }
            Scope::User => {
                let config = dirs::config_dir().ok_or_else(|| {
                    Error::InvalidInput("could not determine config directory".to_string())
                })?;
                Ok(config.join("codex"))
            }
        }
    }

    /// Ensures the .codex directory exists.
    fn ensure_codex_dir(&self, codex_dir: &Path) -> Result<()> {
        if !codex_dir.exists() {
            fs::create_dir_all(codex_dir).map_err(|e| {
                Error::Other(format!("failed to create .codex directory: {}", e))
            })?;
        }
        Ok(())
    }

    /// Returns the config.toml path.
    fn config_path(&self, scope: Scope, project_path: Option<&Path>) -> Result<PathBuf> {
        let codex_dir = self.codex_dir(scope, project_path)?;
        Ok(codex_dir.join("config.toml"))
    }

    /// Reads component content from a file.
    fn read_component_content(&self, path: &Path) -> Result<String> {
        fs::read_to_string(path).map_err(|e| {
            Error::Other(format!("failed to read component file {}: {}", path.display(), e))
        })
    }

    /// Reads or creates the config.toml.
    fn read_config(&self, scope: Scope, project_path: Option<&Path>) -> Result<CodexConfig> {
        let config_path = self.config_path(scope, project_path)?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path).map_err(|e| {
                Error::Other(format!("failed to read config.toml: {}", e))
            })?;

            let config: CodexConfig = toml::from_str(&content).map_err(|e| {
                Error::Serialization(format!("failed to parse config.toml: {}", e))
            })?;

            Ok(config)
        } else {
            Ok(CodexConfig::default())
        }
    }

    /// Writes the config.toml.
    fn write_config(&self, config: &CodexConfig, scope: Scope, project_path: Option<&Path>) -> Result<()> {
        let config_path = self.config_path(scope, project_path)?;

        let toml_str = toml::to_string_pretty(config).map_err(|e| {
            Error::Serialization(format!("failed to serialize config: {}", e))
        })?;

        fs::write(&config_path, toml_str).map_err(|e| {
            Error::Other(format!("failed to write config.toml: {}", e))
        })?;

        Ok(())
    }
}

impl Default for CodexAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Adapter for CodexAdapter {
    fn endpoint(&self) -> Endpoint {
        Endpoint::Known(KnownEndpoint::Codex)
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            can_install: true,
            can_import: true,
            can_export: true,
            can_query_status: true,
            supported_scopes: vec![Scope::Project, Scope::User],
            supported_kinds: vec![
                ComponentKind::Skill,
                ComponentKind::Rule,
                ComponentKind::McpServer,
                ComponentKind::Resource,
            ],
            max_manifest_version: semver::Version::new(1, 0, 0),
        }
    }

    async fn pre_install(&self, ctx: &InstallContext) -> Result<HookResult> {
        let codex_dir = self.codex_dir(ctx.scope, ctx.project_path.as_deref())?;

        if let Some(parent) = codex_dir.parent() {
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
impl InstallAdapter for CodexAdapter {
    async fn plan_install(
        &self,
        bundle: &BundleManifest,
        ctx: &InstallContext,
    ) -> Result<InstallPlan> {
        let codex_dir = self.codex_dir(ctx.scope, ctx.project_path.as_deref())?;
        self.ensure_codex_dir(&codex_dir)?;

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
                ComponentKind::Skill => {
                    self.plan_skill_install(
                        &component,
                        &component_path,
                        &codex_dir,
                        &mut plan,
                        ctx,
                    )?;
                }
                ComponentKind::Rule => {
                    self.plan_rule_install(
                        &component,
                        &component_path,
                        &codex_dir,
                        ctx.scope,
                        ctx.project_path.as_deref(),
                        &mut plan,
                        ctx,
                    )?;
                }
                ComponentKind::McpServer => {
                    self.plan_mcp_install(
                        &component,
                        &component_path,
                        ctx.scope,
                        ctx.project_path.as_deref(),
                        &mut plan,
                        ctx,
                    )?;
                }
                ComponentKind::Resource => {
                    self.plan_resource_install(
                        &component,
                        &component_path,
                        &codex_dir,
                        &mut plan,
                        ctx,
                    )?;
                }
                _ => {
                    tracing::warn!("Unsupported component kind for Codex: {:?}", component.kind);
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

impl CodexAdapter {
    /// Plans installation of a skill component.
    fn plan_skill_install(
        &self,
        component: &skillctrl_core::ComponentRef,
        source_path: &Path,
        codex_dir: &Path,
        plan: &mut InstallPlan,
        ctx: &InstallContext,
    ) -> Result<()> {
        let skills_dir = codex_dir.join("skills");
        let skill_dir = skills_dir.join(&component.id);
        let skill_file = skill_dir.join("SKILL.md");

        let content = self.read_component_content(source_path)?;

        plan.files_to_create.push(InstallFile {
            path: skill_file.clone(),
            content,
            binary: false,
        });

        plan.components.push(ComponentInstall {
            id: component.id.clone(),
            kind: ComponentKind::Skill,
            source: source_path.to_path_buf(),
            destination: skill_file,
        });

        Ok(())
    }

    /// Plans installation of a rule component.
    /// In Codex, rules are typically added to AGENTS.md.
    fn plan_rule_install(
        &self,
        component: &skillctrl_core::ComponentRef,
        source_path: &Path,
        codex_dir: &Path,
        scope: Scope,
        project_path: Option<&Path>,
        plan: &mut InstallPlan,
        ctx: &InstallContext,
    ) -> Result<()> {
        // Rules go into AGENTS.md
        let agents_path = codex_dir.join("AGENTS.md");

        // Read existing content
        let mut existing_content = if agents_path.exists() {
            fs::read_to_string(&agents_path).unwrap_or_default()
        } else {
            String::new()
        };

        // Read new rule content
        let rule_content = self.read_component_content(source_path)?;

        // Append to AGENTS.md
        if !existing_content.is_empty() && !existing_content.ends_with('\n') {
            existing_content.push('\n');
        }
        existing_content.push_str(&format!("# {}\n\n", component.id));
        existing_content.push_str(&rule_content);
        existing_content.push('\n');

        plan.files_to_modify.push(InstallFile {
            path: agents_path.clone(),
            content: existing_content,
            binary: false,
        });

        plan.components.push(ComponentInstall {
            id: component.id.clone(),
            kind: ComponentKind::Rule,
            source: source_path.to_path_buf(),
            destination: agents_path,
        });

        Ok(())
    }

    /// Plans installation of an MCP server.
    fn plan_mcp_install(
        &self,
        component: &skillctrl_core::ComponentRef,
        source_path: &Path,
        scope: Scope,
        project_path: Option<&Path>,
        plan: &mut InstallPlan,
        ctx: &InstallContext,
    ) -> Result<()> {
        // Read component to get MCP server config
        let component_content = self.read_component_content(source_path)?;
        let mcp_config: serde_json::Value = serde_json::from_str(&component_content)
            .map_err(|e| Error::Serialization(format!("failed to parse MCP config: {}", e)))?;

        // Read existing config
        let mut config = self.read_config(scope, project_path)?;

        // Add MCP server
        config.mcp_servers.insert(component.id.clone(), toml::Value::try_from(mcp_config)
            .map_err(|e| Error::Serialization(format!("failed to convert MCP config: {}", e)))?);

        // Serialize back to TOML
        let toml_str = toml::to_string_pretty(&config)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        let config_path = self.config_path(scope, project_path)?;

        plan.files_to_modify.push(InstallFile {
            path: config_path,
            content: toml_str,
            binary: false,
        });

        Ok(())
    }

    /// Plans installation of a resource component.
    fn plan_resource_install(
        &self,
        component: &skillctrl_core::ComponentRef,
        source_path: &Path,
        codex_dir: &Path,
        plan: &mut InstallPlan,
        ctx: &InstallContext,
    ) -> Result<()> {
        // Resources can go in assets/ or references/
        let resource_name = component.id.clone();
        let asset_path = codex_dir.join("assets").join(&resource_name);

        let content = self.read_component_content(source_path)?;

        plan.files_to_create.push(InstallFile {
            path: asset_path.clone(),
            content,
            binary: false,
        });

        plan.components.push(ComponentInstall {
            id: component.id.clone(),
            kind: ComponentKind::Resource,
            source: source_path.to_path_buf(),
            destination: asset_path,
        });

        Ok(())
    }
}

#[async_trait]
impl UninstallAdapterTrait for CodexAdapter {
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
impl StatusAdapterTrait for CodexAdapter {
    async fn status(&self, req: &StatusRequest) -> Result<StatusReport> {
        let codex_dir = self.codex_dir(req.scope, req.project_path.as_deref())?;

        let mut info = HashMap::new();
        info.insert(
            "codex_dir".to_string(),
            codex_dir.to_string_lossy().to_string(),
        );

        Ok(StatusReport {
            target: req.target.clone(),
            scope: req.scope,
            installed_bundles: Vec::new(),
            endpoint_status: EndpointStatus {
                available: codex_dir.exists(),
                version: None,
                config_dir: Some(codex_dir),
                info,
            },
        })
    }
}

/// Codex config.toml structure.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CodexConfig {
    /// MCP servers configuration.
    #[serde(default, rename = "mcpServers")]
    pub mcp_servers: std::collections::HashMap<String, toml::Value>,

    /// Additional configuration.
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, toml::Value>,
}

impl Default for CodexConfig {
    fn default() -> Self {
        Self {
            mcp_servers: std::collections::HashMap::new(),
            extra: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = CodexAdapter::new();
        assert_eq!(adapter.endpoint(), Endpoint::Known(KnownEndpoint::Codex));
    }

    #[test]
    fn test_config_default() {
        let config = CodexConfig::default();
        assert!(config.mcp_servers.is_empty());
        assert!(config.extra.is_empty());
    }
}

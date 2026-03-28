//! Claude Code adapter for skillctrl.
//!
//! This adapter handles installation of components to Claude Code's
//! `.claude` directory structure.

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

/// Claude Code adapter.
pub struct ClaudeAdapter {
    /// Whether to enable verbose logging.
    verbose: bool,
}

impl ClaudeAdapter {
    /// Creates a new Claude adapter.
    pub fn new() -> Self {
        Self { verbose: false }
    }

    /// Creates a new verbose Claude adapter.
    pub fn verbose() -> Self {
        Self { verbose: true }
    }

    /// Returns the .claude directory path for the given scope and project.
    fn claude_dir(&self, scope: Scope, project_path: Option<&Path>) -> Result<PathBuf> {
        match scope {
            Scope::Project => {
                let project = project_path.ok_or_else(|| {
                    Error::InvalidInput("project path required for project scope".to_string())
                })?;
                Ok(project.join(".claude"))
            }
            Scope::User => {
                let home = dirs::home_dir().ok_or_else(|| {
                    Error::InvalidInput("could not determine home directory".to_string())
                })?;
                Ok(home.join(".claude"))
            }
        }
    }

    /// Ensures the .claude directory exists.
    fn ensure_claude_dir(&self, claude_dir: &Path) -> Result<()> {
        if !claude_dir.exists() {
            fs::create_dir_all(claude_dir).map_err(|e| {
                Error::Other(format!("failed to create .claude directory: {}", e))
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

impl Default for ClaudeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Adapter for ClaudeAdapter {
    fn endpoint(&self) -> Endpoint {
        Endpoint::Known(KnownEndpoint::ClaudeCode)
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
                ComponentKind::Command,
                ComponentKind::Rule,
                ComponentKind::McpServer,
                ComponentKind::Hook,
                ComponentKind::Agent,
                ComponentKind::Resource,
            ],
            max_manifest_version: semver::Version::new(1, 0, 0),
        }
    }

    async fn pre_install(&self, ctx: &InstallContext) -> Result<HookResult> {
        let claude_dir = self.claude_dir(ctx.scope, ctx.project_path.as_deref())?;

        // Check if directory exists or can be created
        if !claude_dir.exists() {
            if let Some(parent) = claude_dir.parent() {
                if !parent.exists() {
                    return Ok(HookResult::failure(
                        format!("parent directory does not exist: {}", parent.display()),
                    ));
                }
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
impl InstallAdapter for ClaudeAdapter {
    async fn plan_install(
        &self,
        bundle: &BundleManifest,
        ctx: &InstallContext,
    ) -> Result<InstallPlan> {
        let claude_dir = self.claude_dir(ctx.scope, ctx.project_path.as_deref())?;
        self.ensure_claude_dir(&claude_dir)?;

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

        // Get bundle directory from component paths
        // For now, assume components are relative to some base path
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
                        &claude_dir,
                        &mut plan,
                        ctx,
                    )?;
                }
                ComponentKind::Command => {
                    self.plan_command_install(
                        &component,
                        &component_path,
                        &claude_dir,
                        &mut plan,
                        ctx,
                    )?;
                }
                ComponentKind::Rule => {
                    self.plan_rule_install(
                        &component,
                        &component_path,
                        &claude_dir,
                        &mut plan,
                        ctx,
                    )?;
                }
                ComponentKind::McpServer => {
                    self.plan_mcp_install(
                        &component,
                        &component_path,
                        &claude_dir,
                        ctx.scope,
                        ctx.project_path.as_deref(),
                        &mut plan,
                        ctx,
                    )?;
                }
                ComponentKind::Hook => {
                    self.plan_hook_install(
                        &component,
                        &component_path,
                        &claude_dir,
                        &mut plan,
                        ctx,
                    )?;
                }
                ComponentKind::Agent => {
                    self.plan_agent_install(
                        &component,
                        &component_path,
                        &claude_dir,
                        &mut plan,
                        ctx,
                    )?;
                }
                ComponentKind::Resource => {
                    self.plan_resource_install(
                        &component,
                        &component_path,
                        &claude_dir,
                        &mut plan,
                        ctx,
                    )?;
                }
                _ => {
                    tracing::warn!("Unsupported component kind: {:?}", component.kind);
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
            // For now, just write
            fs::write(&file.path, &file.content).map_err(|e| {
                Error::Other(format!("failed to write file {}: {}", file.path.display(), e))
            })?;

            result.files_modified.push(file.path.clone());
        }

        Ok(result)
    }

    async fn rollback_install(&self, plan: &InstallPlan) -> Result<RollbackResult> {
        let mut restored = Vec::new();
        let mut cleaned = Vec::new();

        // Remove files that were created
        for file in &plan.files_to_create {
            if file.path.exists() {
                fs::remove_file(&file.path).map_err(|e| {
                    Error::Other(format!("failed to remove file {}: {}", file.path.display(), e))
                })?;
                cleaned.push(file.path.clone());
            }
        }

        // Clean up empty directories
        let mut dirs_to_check: Vec<PathBuf> = plan
            .files_to_create
            .iter()
            .filter_map(|f| f.path.parent())
            .map(|p| p.to_path_buf())
            .collect();
        dirs_to_check.sort();
        dirs_to_check.dedup();

        for dir in dirs_to_check {
            if dir.exists() && dir.read_dir().map(|mut i| i.next().is_none()).unwrap_or(false) {
                fs::remove_dir(&dir).ok();
            }
        }

        Ok(RollbackResult {
            success: true,
            restored_files: restored,
            cleaned_files: cleaned,
            messages: vec!["Rollback completed".to_string()],
        })
    }
}

impl ClaudeAdapter {
    /// Plans installation of a skill component.
    fn plan_skill_install(
        &self,
        component: &skillctrl_core::ComponentRef,
        source_path: &Path,
        claude_dir: &Path,
        plan: &mut InstallPlan,
        ctx: &InstallContext,
    ) -> Result<()> {
        let skills_dir = claude_dir.join("skills");
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

    /// Plans installation of a command component.
    fn plan_command_install(
        &self,
        component: &skillctrl_core::ComponentRef,
        source_path: &Path,
        claude_dir: &Path,
        plan: &mut InstallPlan,
        ctx: &InstallContext,
    ) -> Result<()> {
        // Commands are now installed as skills in Claude Code
        // But we can also install to the legacy commands directory
        let commands_dir = claude_dir.join("commands");
        let command_file = commands_dir.join(format!("{}.md", component.id));

        let content = self.read_component_content(source_path)?;

        plan.files_to_create.push(InstallFile {
            path: command_file.clone(),
            content,
            binary: false,
        });

        plan.components.push(ComponentInstall {
            id: component.id.clone(),
            kind: ComponentKind::Command,
            source: source_path.to_path_buf(),
            destination: command_file,
        });

        Ok(())
    }

    /// Plans installation of a rule component.
    fn plan_rule_install(
        &self,
        component: &skillctrl_core::ComponentRef,
        source_path: &Path,
        claude_dir: &Path,
        plan: &mut InstallPlan,
        ctx: &InstallContext,
    ) -> Result<()> {
        let rules_dir = claude_dir.join("rules");
        let rule_file = rules_dir.join(format!("{}.md", component.id));

        let content = self.read_component_content(source_path)?;

        plan.files_to_create.push(InstallFile {
            path: rule_file.clone(),
            content,
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

    /// Plans installation of an MCP server.
    fn plan_mcp_install(
        &self,
        component: &skillctrl_core::ComponentRef,
        source_path: &Path,
        claude_dir: &Path,
        scope: Scope,
        project_path: Option<&Path>,
        plan: &mut InstallPlan,
        ctx: &InstallContext,
    ) -> Result<()> {
        // MCP servers are configured in .mcp.json (project) or ~/.claude.json (user)
        let mcp_config_path = match scope {
            Scope::Project => {
                let project = project_path.ok_or_else(|| {
                    Error::InvalidInput("project path required for project scope".to_string())
                })?;
                project.join(".mcp.json")
            }
            Scope::User => {
                let home = dirs::home_dir().ok_or_else(|| {
                    Error::InvalidInput("could not determine home directory".to_string())
                })?;
                home.join(".claude.json")
            }
        };

        // Read the existing config or create a new one
        let mut mcp_config: serde_json::Value = if mcp_config_path.exists() {
            let content = fs::read_to_string(&mcp_config_path).map_err(|e| {
                Error::Other(format!("failed to read MCP config: {}", e))
            })?;
            serde_json::from_str(&content).map_err(|e| {
                Error::Serialization(format!("failed to parse MCP config: {}", e))
            })?
        } else {
            serde_json::json!({
                "mcpServers": {}
            })
        };

        // Read component to get MCP server config
        let component_content = self.read_component_content(source_path)?;
        let server_config: serde_json::Value = serde_json::from_str(&component_content)
            .map_err(|e| Error::Serialization(format!("failed to parse MCP config: {}", e)))?;

        // Add the server to the config
        if let Some(servers) = mcp_config.get_mut("mcpServers") {
            if let Some(servers_obj) = servers.as_object_mut() {
                servers_obj.insert(component.id.clone(), server_config);
            }
        }

        let content = serde_json::to_string_pretty(&mcp_config)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        plan.files_to_modify.push(InstallFile {
            path: mcp_config_path.clone(),
            content,
            binary: false,
        });

        plan.components.push(ComponentInstall {
            id: component.id.clone(),
            kind: ComponentKind::McpServer,
            source: source_path.to_path_buf(),
            destination: mcp_config_path,
        });

        Ok(())
    }

    /// Plans installation of a hook component.
    fn plan_hook_install(
        &self,
        component: &skillctrl_core::ComponentRef,
        source_path: &Path,
        claude_dir: &Path,
        plan: &mut InstallPlan,
        ctx: &InstallContext,
    ) -> Result<()> {
        // Hooks are configured in settings.json
        let settings_path = claude_dir.join("settings.json");

        let component_content = self.read_component_content(source_path)?;
        let hook_config: serde_json::Value = serde_json::from_str(&component_content)
            .map_err(|e| Error::Serialization(format!("failed to parse hook config: {}", e)))?;

        // Read existing settings or create new
        let mut settings: serde_json::Value = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path).map_err(|e| {
                Error::Other(format!("failed to read settings: {}", e))
            })?;
            serde_json::from_str(&content).map_err(|e| {
                Error::Serialization(format!("failed to parse settings: {}", e))
            })?
        } else {
            serde_json::json!({})
        };

        // Merge hooks
        if let Some(settings_obj) = settings.as_object_mut() {
            if let Some(hooks) = hook_config.get("hooks") {
                settings_obj.insert("hooks".to_string(), hooks.clone());
            }
        }

        let content = serde_json::to_string_pretty(&settings)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        plan.files_to_modify.push(InstallFile {
            path: settings_path,
            content,
            binary: false,
        });

        Ok(())
    }

    /// Plans installation of an agent component.
    fn plan_agent_install(
        &self,
        component: &skillctrl_core::ComponentRef,
        source_path: &Path,
        claude_dir: &Path,
        plan: &mut InstallPlan,
        ctx: &InstallContext,
    ) -> Result<()> {
        let agents_dir = claude_dir.join("agents");
        let agent_dir = agents_dir.join(&component.id);

        // Copy entire agent directory
        if source_path.is_dir() {
            for entry in walkdir::WalkDir::new(source_path)
                .min_depth(1)
                .max_depth(10)
            {
                let entry = entry.map_err(|e| Error::Other(format!("walk error: {}", e)))?;
                let rel_path = entry
                    .path()
                    .strip_prefix(source_path)
                    .map_err(|e| Error::Other(format!("path strip error: {}", e)))?;
                let dest_path = agent_dir.join(rel_path);

                if entry.path().is_file() {
                    let content = self.read_component_content(entry.path())?;
                    plan.files_to_create.push(InstallFile {
                        path: dest_path,
                        content,
                        binary: false,
                    });
                }
            }
        } else {
            // Single file agent
            let agent_file = agent_dir.join("agent.md");
            let content = self.read_component_content(source_path)?;
            plan.files_to_create.push(InstallFile {
                path: agent_file,
                content,
                binary: false,
            });
        }

        plan.components.push(ComponentInstall {
            id: component.id.clone(),
            kind: ComponentKind::Agent,
            source: source_path.to_path_buf(),
            destination: agent_dir,
        });

        Ok(())
    }

    /// Plans installation of a resource component.
    fn plan_resource_install(
        &self,
        component: &skillctrl_core::ComponentRef,
        source_path: &Path,
        claude_dir: &Path,
        plan: &mut InstallPlan,
        ctx: &InstallContext,
    ) -> Result<()> {
        let resources_dir = claude_dir.join("resources");
        let resource_path = resources_dir.join(&component.id);

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
}

#[async_trait]
impl UninstallAdapterTrait for ClaudeAdapter {
    async fn plan_uninstall(&self, req: &UninstallRequest) -> Result<UninstallPlan> {
        let claude_dir = self.claude_dir(req.scope, req.project_path.as_deref())?;

        // This would typically query the state database for installed files
        // For now, return a basic plan
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
impl StatusAdapterTrait for ClaudeAdapter {
    async fn status(&self, req: &StatusRequest) -> Result<StatusReport> {
        let claude_dir = self.claude_dir(req.scope, req.project_path.as_deref())?;

        let available = claude_dir.exists();

        let mut info = HashMap::new();
        info.insert(
            "claude_dir".to_string(),
            claude_dir.to_string_lossy().to_string(),
        );

        Ok(StatusReport {
            target: req.target.clone(),
            scope: req.scope,
            installed_bundles: Vec::new(),
            endpoint_status: EndpointStatus {
                available,
                version: None,
                config_dir: Some(claude_dir),
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
        let adapter = ClaudeAdapter::new();
        assert_eq!(adapter.endpoint(), Endpoint::Known(KnownEndpoint::ClaudeCode));
    }

    #[test]
    fn test_capabilities() {
        let adapter = ClaudeAdapter::new();
        let caps = adapter.capabilities();
        assert!(caps.can_install);
        assert!(caps.can_import);
        assert!(caps.can_export);
    }
}

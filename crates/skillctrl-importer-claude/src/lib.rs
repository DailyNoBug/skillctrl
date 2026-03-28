//! Claude Code importer for skillctrl.
//!
//! This importer scans existing Claude Code configurations and converts them
//! into skillctrl's canonical bundle format.

use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use skillctrl_importer_core::{
    Importer, ScanRequest, DetectedArtifacts, Artifact, ScanError, ScanErrorSeverity,
    ImportRequest, ApplyImportRequest, ImportResult, ArtifactImport,
    Metadata,
};
use skillctrl_core::{
    ComponentKind, Endpoint, Result, Error, KnownEndpoint, ValidationReport, ImportPlan,
};

/// Claude Code importer.
pub struct ClaudeImporter {
    /// Whether to enable verbose logging.
    verbose: bool,
}

impl ClaudeImporter {
    /// Creates a new Claude importer.
    pub fn new() -> Self {
        Self { verbose: false }
    }

    /// Creates a new verbose Claude importer.
    pub fn verbose() -> Self {
        Self { verbose: true }
    }

    /// Returns the .claude directory for the given path.
    fn claude_dir(&self, scan_path: &Path) -> Option<PathBuf> {
        let claude_path = scan_path.join(".claude");
        if claude_path.exists() && claude_path.is_dir() {
            Some(claude_path)
        } else {
            None
        }
    }

    /// Scans for skills in the .claude/skills directory.
    fn scan_skills(&self, claude_dir: &Path) -> Vec<Artifact> {
        let skills_dir = claude_dir.join("skills");
        let mut artifacts = Vec::new();

        if !skills_dir.exists() {
            return artifacts;
        }

        let entries = match fs::read_dir(&skills_dir) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to read skills directory: {}", e);
                return artifacts;
            }
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();

            // Check if this is a skill directory
            let skill_file = path.join("SKILL.md");
            if skill_file.exists() {
                let id = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                artifacts.push(Artifact {
                    kind: ComponentKind::Skill,
                    path: skill_file.clone(),
                    id: Some(id.clone()),
                    name: Some(id.clone()),
                    description: Self::extract_description(&skill_file),
                    supported: true,
                    metadata: Metadata::new(),
                });
            }
        }

        artifacts
    }

    /// Scans for commands in the .claude/commands directory.
    fn scan_commands(&self, claude_dir: &Path) -> Vec<Artifact> {
        let commands_dir = claude_dir.join("commands");
        let mut artifacts = Vec::new();

        if !commands_dir.exists() {
            return artifacts;
        }

        let entries = match fs::read_dir(&commands_dir) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to read commands directory: {}", e);
                return artifacts;
            }
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                let id = path
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                artifacts.push(Artifact {
                    kind: ComponentKind::Command,
                    path: path.clone(),
                    id: Some(id.clone()),
                    name: Some(id.clone()),
                    description: Self::extract_description(&path),
                    supported: true,
                    metadata: Metadata::new(),
                });
            }
        }

        artifacts
    }

    /// Scans for rules in the .claude/rules directory.
    fn scan_rules(&self, claude_dir: &Path) -> Vec<Artifact> {
        let rules_dir = claude_dir.join("rules");
        let mut artifacts = Vec::new();

        if !rules_dir.exists() {
            return artifacts;
        }

        let entries = match fs::read_dir(&rules_dir) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to read rules directory: {}", e);
                return artifacts;
            }
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                let id = path
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                artifacts.push(Artifact {
                    kind: ComponentKind::Rule,
                    path: path.clone(),
                    id: Some(id.clone()),
                    name: Some(id.clone()),
                    description: Self::extract_description(&path),
                    supported: true,
                    metadata: Metadata::new(),
                });
            }
        }

        artifacts
    }

    /// Scans for agents in the .claude/agents directory.
    fn scan_agents(&self, claude_dir: &Path) -> Vec<Artifact> {
        let agents_dir = claude_dir.join("agents");
        let mut artifacts = Vec::new();

        if !agents_dir.exists() {
            return artifacts;
        }

        let entries = match fs::read_dir(&agents_dir) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to read agents directory: {}", e);
                return artifacts;
            }
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();

            if path.is_dir() {
                // Look for agent.md or any .md file
                let agent_md = path.join("agent.md");
                let md_files: Vec<_> = if agent_md.exists() {
                    vec![agent_md]
                } else {
                    WalkDir::new(&path)
                        .max_depth(1)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            e.path()
                                .extension()
                                .and_then(|s| s.to_str())
                                == Some("md")
                        })
                        .map(|e| e.path().to_path_buf())
                        .collect()
                };

                for md_file in md_files {
                    let id = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    artifacts.push(Artifact {
                        kind: ComponentKind::Agent,
                        path: md_file.clone(),
                        id: Some(id.clone()),
                        name: Some(id.clone()),
                        description: Self::extract_description(&md_file),
                        supported: true,
                        metadata: Metadata::new(),
                    });
                }
            }
        }

        artifacts
    }

    /// Scans for hooks in settings.json.
    fn scan_hooks(&self, claude_dir: &Path) -> Vec<Artifact> {
        let settings_path = claude_dir.join("settings.json");
        let mut artifacts = Vec::new();

        if !settings_path.exists() {
            return artifacts;
        }

        match fs::read_to_string(&settings_path) {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(settings) => {
                        if settings.get("hooks").is_some() {
                            artifacts.push(Artifact {
                                kind: ComponentKind::Hook,
                                path: settings_path.clone(),
                                id: Some("hooks".to_string()),
                                name: Some("Hooks".to_string()),
                                description: Some("Claude Code hooks".to_string()),
                                supported: true,
                                metadata: Metadata::new(),
                            });
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse settings.json: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to read settings.json: {}", e);
            }
        }

        artifacts
    }

    /// Scans for MCP servers in .mcp.json or ~/.claude.json.
    fn scan_mcp_servers(&self, scan_path: &Path) -> Vec<Artifact> {
        let mut artifacts = Vec::new();

        // Check project-level .mcp.json
        let mcp_path = scan_path.join(".mcp.json");
        if !mcp_path.exists() {
            // Check user-level .claude.json
            if let Some(home) = dirs::home_dir() {
                let claude_json = home.join(".claude.json");
                if claude_json.exists() {
                    self.scan_mcp_config(&claude_json, &mut artifacts);
                }
            }
        } else {
            self.scan_mcp_config(&mcp_path, &mut artifacts);
        }

        artifacts
    }

    /// Scans an MCP config file for servers.
    fn scan_mcp_config(&self, config_path: &Path, artifacts: &mut Vec<Artifact>) {
        match fs::read_to_string(config_path) {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(config) => {
                        if let Some(servers) = config.get("mcpServers").and_then(|v| v.as_object()) {
                            for (id, _config) in servers {
                                artifacts.push(Artifact {
                                    kind: ComponentKind::McpServer,
                                    path: config_path.to_path_buf(),
                                    id: Some(id.clone()),
                                    name: Some(id.clone()),
                                    description: None,
                                    supported: true,
                                    metadata: {
                                        let mut meta = Metadata::new();
                                        meta.insert("mcp_server_id".to_string(), id.clone());
                                        meta
                                    },
                                });
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse MCP config: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to read MCP config: {}", e);
            }
        }
    }

    /// Extracts a description from a markdown file.
    fn extract_description(path: &Path) -> Option<String> {
        match fs::read_to_string(path) {
            Ok(content) => {
                // Look for the first heading or paragraph
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with('#') {
                        // Remove heading markers
                        return Some(trimmed.replacen('#', "", 1).trim().to_string());
                    } else if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    }
                }
                None
            }
            Err(_) => None,
        }
    }
}

impl Default for ClaudeImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Importer for ClaudeImporter {
    fn endpoint(&self) -> Endpoint {
        Endpoint::Known(KnownEndpoint::ClaudeCode)
    }

    async fn scan(&self, req: &ScanRequest) -> Result<DetectedArtifacts> {
        let mut artifacts = Vec::new();
        let mut errors = Vec::new();

        // Check for .claude directory
        if let Some(claude_dir) = self.claude_dir(&req.path) {
            // Scan all component types
            artifacts.extend(self.scan_skills(&claude_dir));
            artifacts.extend(self.scan_commands(&claude_dir));
            artifacts.extend(self.scan_rules(&claude_dir));
            artifacts.extend(self.scan_agents(&claude_dir));
            artifacts.extend(self.scan_hooks(&claude_dir));
        } else {
            // No .claude directory, might be a bare config
            // Scan for MCP servers anyway
            artifacts.extend(self.scan_mcp_servers(&req.path));
        }

        Ok(DetectedArtifacts {
            source: self.endpoint(),
            path: req.path.clone(),
            artifacts,
            errors,
        })
    }

    async fn plan_import(
        &self,
        req: &ImportRequest,
        artifacts: &DetectedArtifacts,
    ) -> Result<ImportPlan> {
        // Generate bundle ID from request or directory name
        let bundle_id = req.bundle_id.clone().unwrap_or_else(|| {
            req.path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("imported-bundle")
                .to_string()
        });

        Ok(ImportPlan {
            source: req.from.clone(),
            source_path: req.path.clone(),
            artifacts: artifacts
                .artifacts
                .iter()
                .map(|artifact| skillctrl_core::ImportArtifact {
                    kind: artifact.kind.clone(),
                    path: artifact.path.clone(),
                    id: artifact.id.clone(),
                    supported: artifact.supported,
                })
                .collect(),
            bundle_id,
        })
    }

    async fn apply_import(&self, req: &ApplyImportRequest) -> Result<ImportResult> {
        let mut result = ImportResult {
            bundle_id: req.plan.bundle_id.clone(),
            output_path: req.out.clone(),
            files_created: Vec::new(),
            artifacts_imported: Vec::new(),
            warnings: Vec::new(),
            success: false,
        };

        // Create output directory
        fs::create_dir_all(&req.out).map_err(|e| {
            Error::Other(format!("failed to create output directory: {}", e))
        })?;

        // Create components directory
        let components_dir = req.out.join("components");
        fs::create_dir_all(&components_dir).map_err(|e| {
            Error::Other(format!("failed to create components directory: {}", e))
        })?;

        // Create subdirectories for each component kind
        let skills_dir = components_dir.join("skills");
        let rules_dir = components_dir.join("rules");
        let commands_dir = components_dir.join("commands");
        let agents_dir = components_dir.join("agents");
        let hooks_dir = components_dir.join("hooks");
        let mcp_dir = components_dir.join("mcp");
        let resources_dir = components_dir.join("resources");

        for dir in [&skills_dir, &rules_dir, &commands_dir, &agents_dir, &hooks_dir, &mcp_dir, &resources_dir] {
            fs::create_dir_all(dir).map_err(|e| {
                Error::Other(format!("failed to create directory {}: {}", dir.display(), e))
            })?;
        }

        // Process each artifact
        for artifact in &req.plan.artifacts {
            let artifact = Artifact {
                kind: artifact.kind.clone(),
                path: artifact.path.clone(),
                id: artifact.id.clone(),
                name: artifact.id.clone(),
                description: None,
                supported: artifact.supported,
                metadata: Metadata::new(),
            };

            match artifact.kind {
                ComponentKind::Skill => {
                    self.import_skill(&artifact, &skills_dir, &mut result)?;
                }
                ComponentKind::Command => {
                    self.import_command(&artifact, &commands_dir, &mut result)?;
                }
                ComponentKind::Rule => {
                    self.import_rule(&artifact, &rules_dir, &mut result)?;
                }
                ComponentKind::Agent => {
                    self.import_agent(&artifact, &agents_dir, &mut result)?;
                }
                ComponentKind::Hook => {
                    self.import_hook(&artifact, &hooks_dir, &mut result)?;
                }
                ComponentKind::McpServer => {
                    self.import_mcp_server(&artifact, &mcp_dir, &mut result)?;
                }
                _ => {
                    result.warnings.push(format!(
                        "Unsupported component kind: {:?}",
                        artifact.kind
                    ));
                }
            }
        }

        // Generate bundle manifest
        self.generate_bundle_manifest(&req.plan, &req.out, &mut result)?;

        result.success = true;
        Ok(result)
    }
}

impl ClaudeImporter {
    fn import_skill(
        &self,
        artifact: &Artifact,
        skills_dir: &Path,
        result: &mut ImportResult,
    ) -> Result<()> {
        let skill_id = artifact.id.clone().unwrap_or_else(|| "unknown".to_string());
        let skill_dir = skills_dir.join(&skill_id);
        fs::create_dir_all(&skill_dir).map_err(|e| {
            Error::Other(format!("failed to create skill directory: {}", e))
        })?;

        let dest_file = skill_dir.join("SKILL.md");
        fs::copy(&artifact.path, &dest_file).map_err(|e| {
            Error::Other(format!("failed to copy skill file: {}", e))
        })?;

        result.files_created.push(dest_file.clone());
        result.artifacts_imported.push(ArtifactImport {
            kind: ComponentKind::Skill,
            source_path: artifact.path.clone(),
            destination_path: dest_file,
            component_id: skill_id,
            had_loss: false,
            loss_description: None,
        });

        Ok(())
    }

    fn import_command(
        &self,
        artifact: &Artifact,
        commands_dir: &Path,
        result: &mut ImportResult,
    ) -> Result<()> {
        let command_id = artifact.id.clone().unwrap_or_else(|| "unknown".to_string());
        let dest_file = commands_dir.join(format!("{}.md", command_id));
        fs::copy(&artifact.path, &dest_file).map_err(|e| {
            Error::Other(format!("failed to copy command file: {}", e))
        })?;

        result.files_created.push(dest_file.clone());
        result.artifacts_imported.push(ArtifactImport {
            kind: ComponentKind::Command,
            source_path: artifact.path.clone(),
            destination_path: dest_file,
            component_id: command_id,
            had_loss: false,
            loss_description: None,
        });

        Ok(())
    }

    fn import_rule(
        &self,
        artifact: &Artifact,
        rules_dir: &Path,
        result: &mut ImportResult,
    ) -> Result<()> {
        let rule_id = artifact.id.clone().unwrap_or_else(|| "unknown".to_string());
        let dest_file = rules_dir.join(format!("{}.md", rule_id));
        fs::copy(&artifact.path, &dest_file).map_err(|e| {
            Error::Other(format!("failed to copy rule file: {}", e))
        })?;

        result.files_created.push(dest_file.clone());
        result.artifacts_imported.push(ArtifactImport {
            kind: ComponentKind::Rule,
            source_path: artifact.path.clone(),
            destination_path: dest_file,
            component_id: rule_id,
            had_loss: false,
            loss_description: None,
        });

        Ok(())
    }

    fn import_agent(
        &self,
        artifact: &Artifact,
        agents_dir: &Path,
        result: &mut ImportResult,
    ) -> Result<()> {
        let agent_id = artifact.id.clone().unwrap_or_else(|| "unknown".to_string());
        let agent_dir = agents_dir.join(&agent_id);
        fs::create_dir_all(&agent_dir).map_err(|e| {
            Error::Other(format!("failed to create agent directory: {}", e))
        })?;

        let dest_file = agent_dir.join("agent.md");
        fs::copy(&artifact.path, &dest_file).map_err(|e| {
            Error::Other(format!("failed to copy agent file: {}", e))
        })?;

        result.files_created.push(dest_file.clone());
        result.artifacts_imported.push(ArtifactImport {
            kind: ComponentKind::Agent,
            source_path: artifact.path.clone(),
            destination_path: dest_file,
            component_id: agent_id,
            had_loss: false,
            loss_description: None,
        });

        Ok(())
    }

    fn import_hook(
        &self,
        artifact: &Artifact,
        hooks_dir: &Path,
        result: &mut ImportResult,
    ) -> Result<()> {
        let hook_id = artifact.id.clone().unwrap_or_else(|| "hooks".to_string());
        let dest_file = hooks_dir.join(format!("{}.json", hook_id));
        fs::copy(&artifact.path, &dest_file).map_err(|e| {
            Error::Other(format!("failed to copy hook file: {}", e))
        })?;

        result.files_created.push(dest_file.clone());
        result.artifacts_imported.push(ArtifactImport {
            kind: ComponentKind::Hook,
            source_path: artifact.path.clone(),
            destination_path: dest_file,
            component_id: hook_id,
            had_loss: false,
            loss_description: None,
        });

        Ok(())
    }

    fn import_mcp_server(
        &self,
        artifact: &Artifact,
        mcp_dir: &Path,
        result: &mut ImportResult,
    ) -> Result<()> {
        let mcp_id = artifact.id.clone().unwrap_or_else(|| "unknown".to_string());
        let dest_file = mcp_dir.join(format!("{}.json", mcp_id));

        // Extract the MCP server config from the source file
        match fs::read_to_string(&artifact.path) {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(config) => {
                        if let Some(servers) = config.get("mcpServers").and_then(|v| v.as_object()) {
                            if let Some(server_config) = servers.get(&mcp_id) {
                                fs::write(&dest_file, serde_json::to_string_pretty(server_config).unwrap())
                                    .map_err(|e| Error::Other(format!("failed to write MCP config: {}", e)))?;

                                result.files_created.push(dest_file.clone());
                                result.artifacts_imported.push(ArtifactImport {
                                    kind: ComponentKind::McpServer,
                                    source_path: artifact.path.clone(),
                                    destination_path: dest_file,
                                    component_id: mcp_id.clone(),
                                    had_loss: false,
                                    loss_description: None,
                                });
                            }
                        }
                    }
                    Err(e) => {
                        result.warnings.push(format!("Failed to parse MCP config: {}", e));
                    }
                }
            }
            Err(e) => {
                result.warnings.push(format!("Failed to read MCP config: {}", e));
            }
        }

        Ok(())
    }

    fn generate_bundle_manifest(
        &self,
        plan: &ImportPlan,
        out: &Path,
        result: &mut ImportResult,
    ) -> Result<()> {
        use skillctrl_core::{Author, BundleManifest, ComponentRef, KnownEndpoint};
        use std::collections::HashMap;

        let mut components = Vec::new();
        let compat: HashMap<String, skillctrl_core::CompatConfig> = HashMap::new();

        for artifact in &plan.artifacts {
            components.push(ComponentRef {
                kind: artifact.kind.clone(),
                id: artifact.id.clone().unwrap_or_else(|| {
                    artifact
                        .path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                }),
                path: PathBuf::from("components").join(artifact.kind.to_string()).join(
                    artifact.id.clone().unwrap_or_else(|| {
                        artifact
                            .path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string()
                    }),
                ),
                display_name: artifact.id.clone(),
                description: None,
            });
        }

        let manifest = BundleManifest {
            api_version: "skillctrl.dev/v1".to_string(),
            kind: "Bundle".to_string(),
            id: plan.bundle_id.clone(),
            name: plan.bundle_id.clone(),
            version: semver::Version::new(1, 0, 0),
            description: Some(format!("Imported from {}", plan.source)),
            authors: vec![Author {
                name: "skillctrl-import".to_string(),
                email: None,
                url: None,
            }],
            tags: vec!["imported".to_string()],
            targets: vec![Endpoint::Known(KnownEndpoint::ClaudeCode)],
            components,
            compat,
            provenance: Some(skillctrl_core::Provenance {
                source_type: "imported".to_string(),
                repository: None,
                branch: None,
                commit: None,
                metadata: serde_yaml::Mapping::new(),
            }),
        };

        let manifest_yaml = serde_yaml::to_string(&manifest)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        let manifest_path = out.join("bundle.yaml");
        fs::write(&manifest_path, manifest_yaml)
            .map_err(|e| Error::Other(format!("failed to write bundle manifest: {}", e)))?;

        result.files_created.push(manifest_path);

        Ok(())
    }
}

use walkdir::WalkDir;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_importer_creation() {
        let importer = ClaudeImporter::new();
        assert_eq!(
            importer.endpoint(),
            Endpoint::Known(KnownEndpoint::ClaudeCode)
        );
    }
}

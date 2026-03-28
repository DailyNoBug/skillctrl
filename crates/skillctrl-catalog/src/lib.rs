//! Catalog and bundle manifest parsing for skillctrl.

use serde::Deserialize;
use skillctrl_core::{
    Author, BundleManifest, CatalogEntry, CatalogManifest, CompatConfig, ComponentKind,
    ComponentRef, Endpoint, Error, KnownEndpoint, Provenance, Result, ValidationReport,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Catalog loader.
///
/// Handles loading and parsing traditional bundle catalogs.
pub struct CatalogLoader;

impl CatalogLoader {
    /// Loads a catalog from a file.
    pub fn load_from_file(path: &Path) -> Result<CatalogManifest> {
        let content = std::fs::read_to_string(path).map_err(|e| Error::ManifestParse {
            path: path.to_path_buf(),
            message: format!("failed to read: {}", e),
        })?;

        Self::parse(&content, path)
    }

    /// Parses a catalog manifest from YAML content.
    pub fn parse(content: &str, path: &Path) -> Result<CatalogManifest> {
        let catalog: CatalogManifest =
            serde_yaml::from_str(content).map_err(|e| Error::ManifestParse {
                path: path.to_path_buf(),
                message: format!("YAML parse error: {}", e),
            })?;

        validate_catalog(&catalog, path)?;
        Ok(catalog)
    }

    /// Loads a catalog from a git repository.
    pub async fn load_from_git(
        _repo_url: &str,
        _branch: &str,
        _path: &Path,
    ) -> Result<CatalogManifest> {
        Err(Error::Unsupported(
            "Git catalog loading not yet implemented".to_string(),
        ))
    }
}

/// Bundle loader.
///
/// Handles loading and parsing bundle manifests.
pub struct BundleLoader;

impl BundleLoader {
    /// Loads a bundle from a directory.
    ///
    /// Looks for `bundle.yaml` in the given directory.
    pub fn load_from_dir(dir: &Path) -> Result<BundleManifest> {
        let manifest_path = dir.join("bundle.yaml");

        if !manifest_path.exists() {
            let manifest_path_yml = dir.join("bundle.yml");
            if manifest_path_yml.exists() {
                return Self::load_from_file(&manifest_path_yml);
            }
            return Err(Error::NotFound(format!(
                "bundle.yaml not found in {}",
                dir.display()
            )));
        }

        Self::load_from_file(&manifest_path)
    }

    /// Loads a bundle from a file.
    pub fn load_from_file(path: &Path) -> Result<BundleManifest> {
        let content = std::fs::read_to_string(path).map_err(|e| Error::ManifestParse {
            path: path.to_path_buf(),
            message: format!("failed to read: {}", e),
        })?;

        Self::parse(&content, path)
    }

    /// Parses a bundle manifest from YAML content.
    pub fn parse(content: &str, path: &Path) -> Result<BundleManifest> {
        let bundle: BundleManifest =
            serde_yaml::from_str(content).map_err(|e| Error::ManifestParse {
                path: path.to_path_buf(),
                message: format!("YAML parse error: {}", e),
            })?;

        validate_bundle(&bundle, path)?;
        Ok(bundle)
    }

    /// Resolves component paths relative to the bundle directory.
    pub fn resolve_component_paths(
        bundle: &BundleManifest,
        bundle_dir: &Path,
    ) -> Result<Vec<ResolvedComponent>> {
        let mut resolved = Vec::new();

        for component in &bundle.components {
            let full_path = bundle_dir.join(&component.path);

            if !full_path.exists() {
                return Err(Error::NotFound(format!(
                    "Component not found: {}",
                    full_path.display()
                )));
            }

            resolved.push(ResolvedComponent {
                kind: component.kind.clone(),
                id: component.id.clone(),
                source_path: full_path,
                display_name: component.display_name.clone(),
                description: component.description.clone(),
            });
        }

        Ok(resolved)
    }
}

/// A component with resolved path.
#[derive(Debug, Clone)]
pub struct ResolvedComponent {
    /// Component kind.
    pub kind: skillctrl_core::ComponentKind,

    /// Component ID.
    pub id: String,

    /// Full path to component content.
    pub source_path: PathBuf,

    /// Display name.
    pub display_name: Option<String>,

    /// Description.
    pub description: Option<String>,
}

/// A source catalog resolved from a repository checkout.
#[derive(Debug, Clone)]
pub struct SourceCatalog {
    /// Catalog name.
    pub name: String,

    /// Catalog description.
    pub description: Option<String>,

    bundles: Vec<SourceBundle>,
}

impl SourceCatalog {
    /// Loads a source catalog from a repository root.
    pub fn load_from_dir(root: &Path) -> Result<Self> {
        let manifest_path = source_manifest_path(root)?;
        let content =
            std::fs::read_to_string(&manifest_path).map_err(|e| Error::ManifestParse {
                path: manifest_path.clone(),
                message: format!("failed to read: {}", e),
            })?;

        Self::parse(&content, &manifest_path, root)
    }

    /// Parses a source catalog from YAML content.
    pub fn parse(content: &str, manifest_path: &Path, root: &Path) -> Result<Self> {
        match detect_manifest_layout(content, manifest_path)? {
            SourceManifest::Bundles(catalog) => {
                let mut bundles = Vec::with_capacity(catalog.bundles.len());
                for entry in &catalog.bundles {
                    let bundle_root = root.join(&entry.path);
                    let manifest = BundleLoader::load_from_dir(&bundle_root)?;
                    bundles.push(SourceBundle {
                        entry: entry.clone(),
                        manifest,
                        bundle_root,
                    });
                }

                Ok(Self {
                    name: catalog.name,
                    description: catalog.description,
                    bundles,
                })
            }
            SourceManifest::Components(flat) => {
                validate_flat_catalog(&flat, manifest_path)?;

                let mut bundles = Vec::with_capacity(flat.components.len());
                for component in &flat.components {
                    let component_path = root.join(&component.path);
                    if !component_path.exists() {
                        return Err(Error::NotFound(format!(
                            "Component path not found: {}",
                            component_path.display()
                        )));
                    }

                    let targets = if component.targets.is_empty() {
                        default_targets_for_kind(&component.kind)
                    } else {
                        component.targets.clone()
                    };

                    let manifest = BundleManifest {
                        api_version: flat.api_version.clone(),
                        kind: "Bundle".to_string(),
                        id: component.id.clone(),
                        name: component
                            .display_name
                            .clone()
                            .unwrap_or_else(|| component.id.clone()),
                        version: component.version.clone(),
                        description: component
                            .description
                            .clone()
                            .or_else(|| Some(component.summary.clone())),
                        authors: Vec::<Author>::new(),
                        tags: Vec::new(),
                        targets: targets.clone(),
                        components: vec![ComponentRef {
                            kind: component.kind.clone(),
                            id: component.id.clone(),
                            path: component.path.clone(),
                            display_name: component.display_name.clone(),
                            description: component.description.clone(),
                        }],
                        compat: HashMap::<String, CompatConfig>::new(),
                        provenance: None::<Provenance>,
                    };
                    validate_bundle(&manifest, manifest_path)?;

                    bundles.push(SourceBundle {
                        entry: CatalogEntry {
                            id: component.id.clone(),
                            version: component.version.clone(),
                            path: component.path.clone(),
                            summary: component.summary.clone(),
                            targets,
                        },
                        manifest,
                        bundle_root: root.to_path_buf(),
                    });
                }

                Ok(Self {
                    name: flat.name,
                    description: flat.description,
                    bundles,
                })
            }
        }
    }

    /// Returns all available bundles.
    pub fn bundles(&self) -> &[SourceBundle] {
        &self.bundles
    }

    /// Finds a bundle by ID.
    pub fn find_bundle(&self, id: &str) -> Option<&SourceBundle> {
        self.bundles.iter().find(|bundle| bundle.entry.id == id)
    }
}

/// A bundle resolved from a source catalog.
#[derive(Debug, Clone)]
pub struct SourceBundle {
    /// Catalog entry metadata.
    pub entry: CatalogEntry,

    /// Resolved bundle manifest.
    pub manifest: BundleManifest,

    /// Base path used to resolve component files.
    pub bundle_root: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
struct FlatCatalogManifest {
    api_version: String,
    kind: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default)]
    components: Vec<FlatCatalogEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct FlatCatalogEntry {
    kind: ComponentKind,
    id: String,
    path: PathBuf,
    version: semver::Version,
    summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    targets: Vec<Endpoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

enum SourceManifest {
    Bundles(CatalogManifest),
    Components(FlatCatalogManifest),
}

fn detect_manifest_layout(content: &str, path: &Path) -> Result<SourceManifest> {
    let value: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|e| Error::ManifestParse {
            path: path.to_path_buf(),
            message: format!("YAML parse error: {}", e),
        })?;

    let bundles_key = serde_yaml::Value::String("bundles".to_string());
    let components_key = serde_yaml::Value::String("components".to_string());
    let mapping = value.as_mapping().ok_or_else(|| Error::ManifestParse {
        path: path.to_path_buf(),
        message: "manifest root must be a YAML mapping".to_string(),
    })?;

    match (
        mapping.contains_key(&bundles_key),
        mapping.contains_key(&components_key),
    ) {
        (true, false) => Ok(SourceManifest::Bundles(CatalogLoader::parse(
            content, path,
        )?)),
        (false, true) => {
            let flat: FlatCatalogManifest =
                serde_yaml::from_str(content).map_err(|e| Error::ManifestParse {
                    path: path.to_path_buf(),
                    message: format!("YAML parse error: {}", e),
                })?;
            Ok(SourceManifest::Components(flat))
        }
        (true, true) => Err(Error::Validation(
            "catalog manifest cannot define both bundles and components".to_string(),
        )),
        (false, false) => Err(Error::Validation(
            "catalog manifest must define either bundles or components".to_string(),
        )),
    }
}

fn validate_catalog(catalog: &CatalogManifest, path: &Path) -> Result<()> {
    let report = catalog.validate();
    if report.is_valid() {
        return Ok(());
    }

    let errors: Vec<_> = report.errors().iter().map(|e| e.message.clone()).collect();
    Err(Error::Validation(format!(
        "Catalog validation failed ({}): {}",
        path.display(),
        errors.join("; ")
    )))
}

fn validate_bundle(bundle: &BundleManifest, path: &Path) -> Result<()> {
    let report = bundle.validate();
    if report.is_valid() {
        return Ok(());
    }

    let errors: Vec<_> = report.errors().iter().map(|e| e.message.clone()).collect();
    Err(Error::Validation(format!(
        "Bundle validation failed ({}): {}",
        path.display(),
        errors.join("; ")
    )))
}

fn validate_flat_catalog(catalog: &FlatCatalogManifest, path: &Path) -> Result<()> {
    let mut report = ValidationReport::new();

    if catalog.api_version != "skillctrl.dev/v1" {
        report.warning(format!(
            "Unexpected API version: {}, expected skillctrl.dev/v1",
            catalog.api_version
        ));
    }

    if catalog.kind != "Catalog" {
        report.error(format!(
            "Unexpected kind: {}, expected Catalog",
            catalog.kind
        ));
    }

    let mut component_ids = std::collections::HashSet::new();
    for component in &catalog.components {
        if !component_ids.insert(&component.id) {
            report.error(format!(
                "Duplicate component ID in catalog: {}",
                component.id
            ));
        }
    }

    if report.is_valid() {
        return Ok(());
    }

    let errors: Vec<_> = report.errors().iter().map(|e| e.message.clone()).collect();
    Err(Error::Validation(format!(
        "Catalog validation failed ({}): {}",
        path.display(),
        errors.join("; ")
    )))
}

fn source_manifest_path(root: &Path) -> Result<PathBuf> {
    let yaml = root.join("catalog.yaml");
    if yaml.exists() {
        return Ok(yaml);
    }

    let yml = root.join("catalog.yml");
    if yml.exists() {
        return Ok(yml);
    }

    Err(Error::NotFound(format!(
        "catalog.yaml not found in {}",
        root.display()
    )))
}

fn default_targets_for_kind(kind: &ComponentKind) -> Vec<Endpoint> {
    match kind {
        ComponentKind::Skill | ComponentKind::Rule | ComponentKind::Resource => vec![
            Endpoint::Known(KnownEndpoint::ClaudeCode),
            Endpoint::Known(KnownEndpoint::Codex),
            Endpoint::Known(KnownEndpoint::Cursor),
        ],
        ComponentKind::McpServer => vec![
            Endpoint::Known(KnownEndpoint::ClaudeCode),
            Endpoint::Known(KnownEndpoint::Codex),
        ],
        ComponentKind::Command | ComponentKind::Hook | ComponentKind::Agent => {
            vec![Endpoint::Known(KnownEndpoint::ClaudeCode)]
        }
        ComponentKind::PluginMeta | ComponentKind::Custom(_) => Vec::new(),
    }
}

/// Catalog manager.
///
/// Manages multiple catalogs and provides unified access.
pub struct CatalogManager {
    catalogs: std::collections::HashMap<String, CatalogEntry>,
}

impl CatalogManager {
    /// Creates a new catalog manager.
    pub fn new() -> Self {
        Self {
            catalogs: std::collections::HashMap::new(),
        }
    }

    /// Adds a catalog.
    pub fn add_catalog(&mut self, _name: String, catalog: CatalogManifest) {
        for entry in catalog.bundles {
            self.catalogs.insert(entry.id.clone(), entry);
        }
    }

    /// Finds a bundle by ID.
    pub fn find_bundle(&self, id: &str) -> Option<&CatalogEntry> {
        self.catalogs.get(id)
    }

    /// Lists all bundle IDs.
    pub fn list_bundles(&self) -> Vec<&str> {
        self.catalogs.keys().map(|k| k.as_str()).collect()
    }

    /// Searches for bundles by query.
    pub fn search(&self, query: &str) -> Vec<&CatalogEntry> {
        let query_lower = query.to_lowercase();
        self.catalogs
            .values()
            .filter(|entry| {
                entry.id.to_lowercase().contains(&query_lower)
                    || entry.summary.to_lowercase().contains(&query_lower)
            })
            .collect()
    }
}

impl Default for CatalogManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Validates that a path is a valid bundle directory.
pub fn validate_bundle_dir(path: &Path) -> Result<ValidationReport> {
    let mut report = ValidationReport::new();

    if !path.exists() {
        report.error(format!("Path does not exist: {}", path.display()));
        return Ok(report);
    }

    if !path.is_dir() {
        report.error(format!("Path is not a directory: {}", path.display()));
        return Ok(report);
    }

    let manifest_path = path.join("bundle.yaml");
    if !manifest_path.exists() {
        let manifest_path_yml = path.join("bundle.yml");
        if !manifest_path_yml.exists() {
            report.error(format!("bundle.yaml not found in {}", path.display()));
        }
    }

    let components_dir = path.join("components");
    if components_dir.exists() && !components_dir.is_dir() {
        report.error("components exists but is not a directory".to_string());
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_catalog() {
        let yaml = r#"
api_version: skillctrl.dev/v1
kind: Catalog
name: test-catalog
bundles:
  - id: test-bundle
    version: 1.0.0
    path: bundles/test-bundle
    summary: A test bundle
"#;

        let catalog = CatalogLoader::parse(yaml, Path::new("test.yaml")).unwrap();
        assert_eq!(catalog.name, "test-catalog");
        assert_eq!(catalog.bundles.len(), 1);
        assert_eq!(catalog.bundles[0].id, "test-bundle");
    }

    #[test]
    fn test_parse_bundle() {
        let yaml = r#"
api_version: skillctrl.dev/v1
kind: Bundle
id: test-bundle
name: Test Bundle
version: 1.0.0
description: A test bundle
targets:
  - claude-code
components:
  - kind: skill
    id: test-skill
    path: components/skills/test-skill/SKILL.md
"#;

        let bundle = BundleLoader::parse(yaml, Path::new("test.yaml")).unwrap();
        assert_eq!(bundle.id, "test-bundle");
        assert_eq!(bundle.name, "Test Bundle");
        assert_eq!(bundle.components.len(), 1);
        assert_eq!(
            bundle.components[0].kind,
            skillctrl_core::ComponentKind::Skill
        );
    }

    #[test]
    fn test_load_flat_source_catalog() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let skill_dir = root.join("skills").join("daily-dev-assistant");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Daily Dev Assistant\n").unwrap();

        let catalog_path = root.join("catalog.yaml");
        fs::write(
            &catalog_path,
            r#"
api_version: skillctrl.dev/v1
kind: Catalog
name: local-hub
description: Root-level component catalog
components:
  - id: daily-dev-assistant
    kind: skill
    version: 0.1.0
    path: skills/daily-dev-assistant/SKILL.md
    summary: Daily planning skill
    targets:
      - claude-code
      - codex
      - cursor
    display_name: Daily Dev Assistant
"#,
        )
        .unwrap();

        let catalog = SourceCatalog::load_from_dir(root).unwrap();
        let bundle = catalog.find_bundle("daily-dev-assistant").unwrap();

        assert_eq!(catalog.name, "local-hub");
        assert_eq!(catalog.bundles().len(), 1);
        assert_eq!(bundle.manifest.id, "daily-dev-assistant");
        assert_eq!(bundle.manifest.components.len(), 1);
        assert_eq!(bundle.bundle_root, root);
    }

    #[test]
    fn test_validate_bundle_dir() {
        let temp_dir = TempDir::new().unwrap();
        let bundle_dir = temp_dir.path();

        let report = validate_bundle_dir(bundle_dir).unwrap();
        assert!(report.has_errors());

        let manifest_path = bundle_dir.join("bundle.yaml");
        fs::write(
            &manifest_path,
            r#"
api_version: skillctrl.dev/v1
kind: Bundle
id: test
name: Test
version: 1.0.0
components: []
"#,
        )
        .unwrap();

        let report = validate_bundle_dir(bundle_dir).unwrap();
        assert!(!report.has_errors());
    }

    #[test]
    fn test_catalog_manager() {
        let mut manager = CatalogManager::new();

        let yaml = r#"
api_version: skillctrl.dev/v1
kind: Catalog
name: test
bundles:
  - id: bundle1
    version: 1.0.0
    path: /fake/path
    summary: Bundle 1
  - id: bundle2
    version: 1.0.0
    path: /fake/path2
    summary: Bundle 2
"#;

        let catalog = CatalogLoader::parse(yaml, Path::new("test")).unwrap();
        manager.add_catalog("test".to_string(), catalog);

        assert_eq!(manager.list_bundles().len(), 2);
        assert!(manager.find_bundle("bundle1").is_some());
        assert!(manager.find_bundle("bundle3").is_none());

        let results = manager.search("bundle");
        assert_eq!(results.len(), 2);
    }
}

//! Catalog and bundle manifest parsing for skillctrl.

use std::path::{Path, PathBuf};
use skillctrl_core::{
    BundleManifest, CatalogManifest, CatalogEntry, Error, Result, ValidationReport,
};

/// Catalog loader.
///
/// Handles loading and parsing catalog manifests.
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
        let catalog: CatalogManifest = serde_yaml::from_str(content).map_err(|e| {
            Error::ManifestParse {
                path: path.to_path_buf(),
                message: format!("YAML parse error: {}", e),
            }
        })?;

        // Validate
        let report = catalog.validate();
        if !report.is_valid() {
            let errors: Vec<_> = report
                .errors()
                .iter()
                .map(|e| e.message.clone())
                .collect();
            return Err(Error::Validation(format!(
                "Catalog validation failed: {}",
                errors.join("; ")
            )));
        }

        Ok(catalog)
    }

    /// Loads a catalog from a git repository.
    pub async fn load_from_git(
        repo_url: &str,
        branch: &str,
        path: &Path,
    ) -> Result<CatalogManifest> {
        // This will be implemented in the git module
        // For now, return an error
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
            // Try bundle.yml as well
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
        let bundle: BundleManifest = serde_yaml::from_str(content).map_err(|e| {
            Error::ManifestParse {
                path: path.to_path_buf(),
                message: format!("YAML parse error: {}", e),
            }
        })?;

        // Validate
        let report = bundle.validate();
        if !report.is_valid() {
            let errors: Vec<_> = report
                .errors()
                .iter()
                .map(|e| e.message.clone())
                .collect();
            return Err(Error::Validation(format!(
                "Bundle validation failed: {}",
                errors.join("; ")
            )));
        }

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
    pub fn add_catalog(&mut self, name: String, catalog: CatalogManifest) {
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

    // Check if path exists
    if !path.exists() {
        report.error(format!("Path does not exist: {}", path.display()));
        return Ok(report);
    }

    // Check if it's a directory
    if !path.is_dir() {
        report.error(format!("Path is not a directory: {}", path.display()));
        return Ok(report);
    }

    // Check for bundle.yaml
    let manifest_path = path.join("bundle.yaml");
    if !manifest_path.exists() {
        let manifest_path_yml = path.join("bundle.yml");
        if !manifest_path_yml.exists() {
            report.error(format!("bundle.yaml not found in {}", path.display()));
        }
    }

    // Check for components directory
    let components_dir = path.join("components");
    if components_dir.exists() && !components_dir.is_dir() {
        report.error(format!("components exists but is not a directory"));
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
apiVersion: skillctrl.dev/v1
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
apiVersion: skillctrl.dev/v1
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
    path: components/skills/test-skill
"#;

        let bundle = BundleLoader::parse(yaml, Path::new("test.yaml")).unwrap();
        assert_eq!(bundle.id, "test-bundle");
        assert_eq!(bundle.name, "Test Bundle");
        assert_eq!(bundle.components.len(), 1);
        assert_eq!(bundle.components[0].kind, skillctrl_core::ComponentKind::Skill);
    }

    #[test]
    fn test_validate_bundle_dir() {
        let temp_dir = TempDir::new().unwrap();
        let bundle_dir = temp_dir.path();

        // Empty directory - should have errors
        let report = validate_bundle_dir(bundle_dir).unwrap();
        assert!(report.has_errors());

        // Add bundle.yaml
        let manifest_path = bundle_dir.join("bundle.yaml");
        fs::write(
            &manifest_path,
            r#"
apiVersion: skillctrl.dev/v1
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
apiVersion: skillctrl.dev/v1
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

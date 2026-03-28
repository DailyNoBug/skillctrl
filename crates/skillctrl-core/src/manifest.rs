//! Manifest types for catalogs and bundles.

use crate::component::ComponentKind;
use crate::endpoint::Endpoint;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Author information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    /// Author name.
    pub name: String,

    /// Author email (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Author URL (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// A tag.
pub type Tag = String;

/// Component reference in a bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentRef {
    /// Component kind.
    pub kind: ComponentKind,

    /// Component ID.
    pub id: String,

    /// Path to component content.
    pub path: PathBuf,

    /// Optional display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Compatibility configuration for a target endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatConfig {
    /// Installation mode for this endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_mode: Option<String>,

    /// Additional endpoint-specific configuration.
    #[serde(flatten)]
    pub extra: serde_yaml::Mapping,
}

/// Provenance information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    /// Source type.
    #[serde(rename = "type")]
    pub source_type: String,

    /// Original repository (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,

    /// Original branch (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// Original commit (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,

    /// Additional metadata.
    #[serde(default, skip_serializing_if = "serde_yaml::Mapping::is_empty")]
    pub metadata: serde_yaml::Mapping,
}

/// Bundle manifest.
///
/// Describes a single bundle of components that can be installed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    /// API version.
    pub api_version: String,

    /// Kind - should be "Bundle".
    pub kind: String,

    /// Bundle ID.
    pub id: String,

    /// Bundle name.
    pub name: String,

    /// Bundle version.
    pub version: semver::Version,

    /// Bundle description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Authors.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<Author>,

    /// Tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Tag>,

    /// Supported target endpoints.
    #[serde(default)]
    pub targets: Vec<Endpoint>,

    /// Components in this bundle.
    pub components: Vec<ComponentRef>,

    /// Endpoint-specific compatibility configuration.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub compat: std::collections::HashMap<String, CompatConfig>,

    /// Provenance information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

impl BundleManifest {
    /// Validates this manifest.
    pub fn validate(&self) -> crate::ValidationReport {
        let mut report = crate::ValidationReport::new();

        // Check API version
        if self.api_version != "skillctrl.dev/v1" {
            report.warning(format!(
                "Unexpected API version: {}, expected skillctrl.dev/v1",
                self.api_version
            ));
        }

        // Check kind
        if self.kind != "Bundle" {
            report.error(format!("Unexpected kind: {}, expected Bundle", self.kind));
        }

        // Check components
        let mut component_ids = std::collections::HashSet::new();
        for component in &self.components {
            if !component_ids.insert(&component.id) {
                report.error(format!("Duplicate component ID: {}", component.id));
            }
        }

        report
    }

    /// Returns components of a specific kind.
    pub fn components_by_kind(&self, kind: ComponentKind) -> Vec<&ComponentRef> {
        self.components.iter().filter(|c| c.kind == kind).collect()
    }

    /// Returns the compatibility config for an endpoint.
    pub fn compat_for(&self, endpoint: &Endpoint) -> Option<&CompatConfig> {
        self.compat.get(endpoint.as_str())
    }
}

/// Catalog entry for a bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    /// Bundle ID.
    pub id: String,

    /// Bundle version.
    pub version: semver::Version,

    /// Path to bundle directory.
    pub path: PathBuf,

    /// Short summary.
    pub summary: String,

    /// Supported targets (optional, inferred from bundle).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<Endpoint>,
}

/// Catalog manifest.
///
/// A catalog contains multiple bundles available for installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogManifest {
    /// API version.
    pub api_version: String,

    /// Kind - should be "Catalog".
    pub kind: String,

    /// Catalog name.
    pub name: String,

    /// Catalog description (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Bundles in this catalog.
    pub bundles: Vec<CatalogEntry>,
}

impl CatalogManifest {
    /// Validates this manifest.
    pub fn validate(&self) -> crate::ValidationReport {
        let mut report = crate::ValidationReport::new();

        // Check API version
        if self.api_version != "skillctrl.dev/v1" {
            report.warning(format!(
                "Unexpected API version: {}, expected skillctrl.dev/v1",
                self.api_version
            ));
        }

        // Check kind
        if self.kind != "Catalog" {
            report.error(format!("Unexpected kind: {}, expected Catalog", self.kind));
        }

        // Check for duplicate bundle IDs
        let mut bundle_ids = std::collections::HashSet::new();
        for entry in &self.bundles {
            if !bundle_ids.insert(&entry.id) {
                report.error(format!("Duplicate bundle ID in catalog: {}", entry.id));
            }
        }

        report
    }

    /// Finds a bundle by ID.
    pub fn find_bundle(&self, id: &str) -> Option<&CatalogEntry> {
        self.bundles.iter().find(|b| b.id == id)
    }

    /// Returns all bundle IDs.
    pub fn bundle_ids(&self) -> Vec<&str> {
        self.bundles.iter().map(|b| b.id.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_manifest_validation() {
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

        let manifest: BundleManifest = serde_yaml::from_str(yaml).unwrap();
        let report = manifest.validate();

        assert!(report.is_valid());
    }

    #[test]
    fn test_catalog_manifest() {
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

        let catalog: CatalogManifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(catalog.name, "test-catalog");
        assert_eq!(catalog.bundles.len(), 1);
    }
}

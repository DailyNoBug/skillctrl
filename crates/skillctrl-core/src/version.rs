//! Version management and compatibility.

use serde::{Deserialize, Serialize};

/// Version policy for manifest compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionPolicy {
    /// Minimum supported manifest version.
    pub min_supported: semver::Version,

    /// Maximum supported manifest version.
    pub max_supported: semver::Version,

    /// Deprecated versions that will warn but still work.
    #[serde(default)]
    pub deprecated: Vec<semver::Version>,
}

impl VersionPolicy {
    /// Creates a new version policy.
    pub fn new(min: semver::Version, max: semver::Version) -> Self {
        Self {
            min_supported: min,
            max_supported: max,
            deprecated: Vec::new(),
        }
    }

    /// Checks if a version is supported.
    pub fn is_supported(&self, version: &semver::Version) -> bool {
        version >= &self.min_supported && version <= &self.max_supported
    }

    /// Checks if a version is deprecated.
    pub fn is_deprecated(&self, version: &semver::Version) -> bool {
        self.deprecated.contains(version)
    }

    /// Returns the default version policy for skillctrl.
    pub fn default_policy() -> Self {
        Self::new(
            semver::Version::new(1, 0, 0),
            semver::Version::new(1, 0, 0),
        )
    }
}

impl Default for VersionPolicy {
    fn default() -> Self {
        Self::default_policy()
    }
}

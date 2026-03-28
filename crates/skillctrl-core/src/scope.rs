//! Installation scope types.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Installation scope.
///
/// Determines where components are installed - either to a specific project
/// or globally to the user's home directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    /// Project-specific installation.
    Project,

    /// User/global installation.
    User,
}

impl Scope {
    /// Returns the string representation of this scope.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Project => "project",
            Self::User => "user",
        }
    }

    /// Returns the config directory for this scope.
    ///
    /// For project scope, this returns the project root.
    /// For user scope, this returns the user's config directory.
    pub fn config_dir(&self) -> &'static str {
        match self {
            Self::Project => ".",
            Self::User => "~/.config/skillctrl",
        }
    }

    /// Parses a string into a Scope.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "project" => Some(Self::Project),
            "user" => Some(Self::User),
            _ => None,
        }
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Scope {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str(s).ok_or_else(|| format!("invalid scope: {}", s))
    }
}

/// Resolved scope with a concrete path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResolvedScope {
    pub scope: Scope,
    pub path: camino::Utf8PathBuf,
}

impl ResolvedScope {
    /// Creates a new resolved scope.
    pub fn new(scope: Scope, path: camino::Utf8PathBuf) -> Self {
        Self { scope, path }
    }

    /// Returns the config directory for this resolved scope.
    pub fn config_dir(&self) -> &camino::Utf8Path {
        &self.path
    }

    /// Resolves a user scope to the actual config directory.
    pub fn resolve_user() -> Result<Self, std::io::Error> {
        let config_dir = dirs::config_dir()
            .map(|p| p.join("skillctrl"))
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "could not determine user config directory",
                )
            })?;

        let config_dir = camino::Utf8PathBuf::from_path_buf(config_dir).map_err(|p| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("config path is not UTF-8: {:?}", p),
            )
        })?;

        Ok(Self::new(Scope::User, config_dir))
    }

    /// Resolves a project scope to the given project path.
    pub fn resolve_project(path: camino::Utf8PathBuf) -> Self {
        Self::new(Scope::Project, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_from_str() {
        assert_eq!(Scope::from_str("project"), Some(Scope::Project));
        assert_eq!(Scope::from_str("user"), Some(Scope::User));
        assert_eq!(Scope::from_str("invalid"), None);
    }

    #[test]
    fn test_scope_display() {
        assert_eq!(Scope::Project.to_string(), "project");
        assert_eq!(Scope::User.to_string(), "user");
    }
}

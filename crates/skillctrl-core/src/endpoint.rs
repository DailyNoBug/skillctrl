//! Endpoint types and abstractions.

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Target endpoint for installation.
///
/// Endpoints represent the different AI coding assistants that skillctrl
/// can integrate with.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Endpoint {
    /// A known, built-in endpoint.
    Known(KnownEndpoint),

    /// A custom endpoint for extensibility.
    Custom(String),
}

impl Endpoint {
    /// Returns the string representation of this endpoint.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Known(e) => e.as_str(),
            Self::Custom(s) => s.as_str(),
        }
    }

    /// Returns true if this is the Claude Code endpoint.
    pub fn is_claude_code(&self) -> bool {
        matches!(self, Self::Known(KnownEndpoint::ClaudeCode))
    }

    /// Returns true if this is the Codex endpoint.
    pub fn is_codex(&self) -> bool {
        matches!(self, Self::Known(KnownEndpoint::Codex))
    }

    /// Returns true if this is the Cursor endpoint.
    pub fn is_cursor(&self) -> bool {
        matches!(self, Self::Known(KnownEndpoint::Cursor))
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<KnownEndpoint> for Endpoint {
    fn from(e: KnownEndpoint) -> Self {
        Self::Known(e)
    }
}

impl FromStr for Endpoint {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match KnownEndpoint::from_str(s) {
            Some(known) => Ok(Self::Known(known)),
            None => Ok(Self::Custom(s.to_string())),
        }
    }
}

/// Known, built-in endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KnownEndpoint {
    /// Anthropic's Claude Code
    ClaudeCode,

    /// OpenAI's Codex
    Codex,

    /// Cursor AI
    Cursor,
}

impl KnownEndpoint {
    /// Returns the string representation of this endpoint.
    pub fn as_str(&self) -> &str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
        }
    }

    /// Returns the config directory name for this endpoint.
    pub fn config_dir(&self) -> &str {
        match self {
            Self::ClaudeCode => ".claude",
            Self::Codex => ".codex",
            Self::Cursor => ".cursor",
        }
    }

    /// Returns the project config file name for this endpoint.
    pub fn project_config_file(&self) -> Option<&str> {
        match self {
            Self::ClaudeCode => Some(".mcp.json"),
            Self::Codex => Some("config.toml"),
            Self::Cursor => None,
        }
    }

    /// Parses a string into a KnownEndpoint.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "claude-code" => Some(Self::ClaudeCode),
            "codex" => Some(Self::Codex),
            "cursor" => Some(Self::Cursor),
            _ => None,
        }
    }

    /// Returns all known endpoints.
    pub fn all() -> &'static [Self] {
        &[Self::ClaudeCode, Self::Codex, Self::Cursor]
    }
}

impl fmt::Display for KnownEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Adapter capabilities for an endpoint.
///
/// This describes what operations an adapter can perform for a given endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterCapabilities {
    /// Whether this adapter can install components.
    pub can_install: bool,

    /// Whether this adapter can import existing configurations.
    pub can_import: bool,

    /// Whether this adapter can export to native formats.
    pub can_export: bool,

    /// Whether this adapter can query installation status.
    pub can_query_status: bool,

    /// Supported scopes for this adapter.
    pub supported_scopes: Vec<crate::Scope>,

    /// Supported component kinds for this adapter.
    pub supported_kinds: Vec<crate::ComponentKind>,

    /// Maximum manifest version supported.
    pub max_manifest_version: semver::Version,
}

impl Default for AdapterCapabilities {
    fn default() -> Self {
        Self {
            can_install: true,
            can_import: true,
            can_export: true,
            can_query_status: true,
            supported_scopes: vec![crate::Scope::Project, crate::Scope::User],
            supported_kinds: vec![
                crate::ComponentKind::Skill,
                crate::ComponentKind::Rule,
                crate::ComponentKind::Command,
            ],
            max_manifest_version: semver::Version::new(1, 0, 0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_endpoint_from_str() {
        assert_eq!(
            KnownEndpoint::from_str("claude-code"),
            Some(KnownEndpoint::ClaudeCode)
        );
        assert_eq!(KnownEndpoint::from_str("codex"), Some(KnownEndpoint::Codex));
        assert_eq!(
            KnownEndpoint::from_str("cursor"),
            Some(KnownEndpoint::Cursor)
        );
        assert_eq!(KnownEndpoint::from_str("unknown"), None);
    }

    #[test]
    fn test_endpoint_from_str() {
        assert_eq!(
            Endpoint::from_str("claude-code").unwrap(),
            Endpoint::Known(KnownEndpoint::ClaudeCode)
        );
        assert_eq!(
            Endpoint::from_str("custom-ai").unwrap(),
            Endpoint::Custom("custom-ai".to_string())
        );
    }

    #[test]
    fn test_endpoint_config_dir() {
        assert_eq!(KnownEndpoint::ClaudeCode.config_dir(), ".claude");
        assert_eq!(KnownEndpoint::Codex.config_dir(), ".codex");
        assert_eq!(KnownEndpoint::Cursor.config_dir(), ".cursor");
    }
}

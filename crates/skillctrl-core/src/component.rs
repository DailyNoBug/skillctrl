//! Component types and abstractions.

use crate::dependency::ComponentDependency;
use crate::validation::ValidationReport;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt;

/// The kind of a component.
///
/// Components represent the different types of installable artifacts
/// that can be part of a bundle. Different endpoints support different
/// component kinds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentKind {
    /// A skill - reusable AI capability
    Skill,

    /// A rule - behavior constraint or guidance
    Rule,

    /// A command - slash command or CLI command
    Command,

    /// An MCP server - Model Context Protocol server
    McpServer,

    /// A hook - lifecycle event handler
    Hook,

    /// A resource - reference material or asset
    Resource,

    /// An agent - autonomous AI agent
    Agent,

    /// Plugin metadata
    PluginMeta,

    /// Custom component type for extensibility
    #[serde(untagged)]
    Custom(String),
}

impl ComponentKind {
    /// Returns the string representation of this component kind.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Skill => "skill",
            Self::Rule => "rule",
            Self::Command => "command",
            Self::McpServer => "mcp-server",
            Self::Hook => "hook",
            Self::Resource => "resource",
            Self::Agent => "agent",
            Self::PluginMeta => "plugin-meta",
            Self::Custom(s) => s.as_str(),
        }
    }

    /// Parses a string into a ComponentKind.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "skill" => Some(Self::Skill),
            "rule" => Some(Self::Rule),
            "command" => Some(Self::Command),
            "mcp-server" => Some(Self::McpServer),
            "hook" => Some(Self::Hook),
            "resource" => Some(Self::Resource),
            "agent" => Some(Self::Agent),
            "plugin-meta" => Some(Self::PluginMeta),
            _ => Some(Self::Custom(s.to_string())),
        }
    }
}

impl fmt::Display for ComponentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Base trait for all components.
///
/// Components are the fundamental units of installable content in skillctrl.
/// Each component has a kind, an ID, and can be validated.
pub trait Component: Any + Send + Sync + fmt::Debug {
    /// Returns the kind of this component.
    fn kind(&self) -> ComponentKind;

    /// Returns the unique identifier for this component.
    fn id(&self) -> &str;

    /// Validates this component, returning a report of any issues.
    fn validate(&self) -> ValidationReport {
        ValidationReport::new()
    }

    /// Returns the dependencies of this component.
    fn dependencies(&self) -> &[ComponentDependency] {
        &[]
    }

    /// Returns the file path where this component is stored (if applicable).
    fn path(&self) -> Option<&str> {
        None
    }

    /// Returns the content of this component (if applicable).
    fn content(&self) -> Option<&str> {
        None
    }
}

/// Downcast a component to its concrete type.
pub fn downcast_component<T: Component + 'static>(component: &dyn Component) -> Option<&T> {
    (component as &(dyn Any + 'static)).downcast_ref::<T>()
}

/// Macro to implement Component for simple types.
#[macro_export]
macro_rules! impl_component {
    ($ty:ty, $kind:expr, $id_field:ident) => {
        impl Component for $ty {
            fn kind(&self) -> ComponentKind {
                $kind
            }

            fn id(&self) -> &str {
                &self.$id_field
            }
        }
    };
}

/// A simple component implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleComponent {
    pub kind: ComponentKind,
    pub id: String,
    pub path: Option<String>,
    pub content: Option<String>,
    pub dependencies: Vec<ComponentDependency>,
}

impl Component for SimpleComponent {
    fn kind(&self) -> ComponentKind {
        self.kind.clone()
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn dependencies(&self) -> &[ComponentDependency] {
        &self.dependencies
    }

    fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    fn content(&self) -> Option<&str> {
        self.content.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_kind_from_str() {
        assert_eq!(ComponentKind::from_str("skill"), Some(ComponentKind::Skill));
        assert_eq!(ComponentKind::from_str("rule"), Some(ComponentKind::Rule));
        assert_eq!(
            ComponentKind::from_str("custom"),
            Some(ComponentKind::Custom("custom".to_string()))
        );
    }

    #[test]
    fn test_component_kind_display() {
        assert_eq!(ComponentKind::Skill.to_string(), "skill");
        assert_eq!(ComponentKind::McpServer.to_string(), "mcp-server");
    }
}

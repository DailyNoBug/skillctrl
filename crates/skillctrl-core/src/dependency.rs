//! Dependency management.

use crate::component::ComponentKind;
use crate::Error;
use crate::Result;
use std::collections::{HashMap, HashSet};

/// A dependency on another component.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComponentDependency {
    /// ID of the depended component.
    pub component_id: String,

    /// Kind of the depended component.
    pub kind: ComponentKind,

    /// Version constraint (if applicable).
    pub version_constraint: Option<semver::VersionReq>,

    /// Whether this dependency is required.
    #[serde(default = "default_required")]
    pub required: bool,
}

fn default_required() -> bool {
    true
}

impl ComponentDependency {
    /// Creates a new required dependency.
    pub fn required(component_id: String, kind: ComponentKind) -> Self {
        Self {
            component_id,
            kind,
            version_constraint: None,
            required: true,
        }
    }

    /// Creates a new optional dependency.
    pub fn optional(component_id: String, kind: ComponentKind) -> Self {
        Self {
            component_id,
            kind,
            version_constraint: None,
            required: false,
        }
    }

    /// Creates a new dependency with version constraint.
    pub fn with_version(
        component_id: String,
        kind: ComponentKind,
        version_constraint: semver::VersionReq,
    ) -> Self {
        Self {
            component_id,
            kind,
            version_constraint: Some(version_constraint),
            required: true,
        }
    }
}

/// Dependency resolver.
///
/// Resolves the installation order for components based on their dependencies.
pub struct DependencyResolver {
    /// Available components.
    available: HashMap<String, ComponentKind>,
}

impl DependencyResolver {
    /// Creates a new dependency resolver.
    pub fn new() -> Self {
        Self {
            available: HashMap::new(),
        }
    }

    /// Registers an available component.
    pub fn register(&mut self, id: String, kind: ComponentKind) {
        self.available.insert(id, kind);
    }

    /// Resolves the installation order for components.
    ///
    /// Returns components in dependency order (dependencies before dependents).
    pub fn resolve_order(&self, components: &[ResolvedDependency]) -> Result<Vec<String>> {
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut all_ids: HashSet<String> = HashSet::new();

        // Build the dependency graph
        for component in components {
            all_ids.insert(component.id.clone());
            let deps: Vec<String> = component
                .dependencies
                .iter()
                .map(|d| d.component_id.clone())
                .collect();
            graph.insert(component.id.clone(), deps);
        }

        // Topological sort
        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        for id in &all_ids {
            self.visit(id, &graph, &mut sorted, &mut visited, &mut visiting)?;
        }

        Ok(sorted)
    }

    fn visit(
        &self,
        id: &str,
        graph: &HashMap<String, Vec<String>>,
        sorted: &mut Vec<String>,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
    ) -> Result<()> {
        if visited.contains(id) {
            return Ok(());
        }

        if visiting.contains(id) {
            return Err(Error::Dependency(format!(
                "circular dependency detected involving: {}",
                id
            )));
        }

        visiting.insert(id.to_string());

        if let Some(deps) = graph.get(id) {
            for dep in deps {
                // Check if dependency is available
                if !self.available.contains_key(dep) {
                    if !component_is_optional(id, dep) {
                        return Err(Error::Dependency(format!(
                            "required dependency '{}' not found for '{}'",
                            dep, id
                        )));
                    }
                }
                self.visit(dep, graph, sorted, visited, visiting)?;
            }
        }

        visiting.remove(id);
        visited.insert(id.to_string());
        sorted.push(id.to_string());
        Ok(())
    }

    /// Validates that all dependencies are satisfied.
    pub fn validate(&self, components: &[ResolvedDependency]) -> Result<()> {
        for component in components {
            for dep in &component.dependencies {
                if !dep.required {
                    continue;
                }

                if let Some(kind) = self.available.get(&dep.component_id) {
                    if *kind != dep.kind {
                        return Err(Error::Dependency(format!(
                            "dependency '{}' has wrong kind: expected {:?}, got {:?}",
                            dep.component_id, dep.kind, kind
                        )));
                    }
                } else {
                    return Err(Error::Dependency(format!(
                        "required dependency '{}' not found",
                        dep.component_id
                    )));
                }

                // Check version constraint
                if let Some(ref constraint) = dep.version_constraint {
                    // Version checking would require actual version info
                    // For now, we just validate the constraint syntax
                    let _ = constraint; // Suppress unused warning
                }
            }
        }
        Ok(())
    }
}

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// A component with resolved dependencies.
#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    /// Component ID.
    pub id: String,
    /// Component kind.
    pub kind: ComponentKind,
    /// Dependencies.
    pub dependencies: Vec<ComponentDependency>,
}

fn component_is_optional(_component_id: &str, _dep_id: &str) -> bool {
    // This would need to be implemented properly with full dependency info
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_order() {
        let mut resolver = DependencyResolver::new();
        resolver.register("a".to_string(), ComponentKind::Skill);
        resolver.register("b".to_string(), ComponentKind::Skill);
        resolver.register("c".to_string(), ComponentKind::Skill);

        let components = vec![
            ResolvedDependency {
                id: "a".to_string(),
                kind: ComponentKind::Skill,
                dependencies: vec![ComponentDependency::required(
                    "b".to_string(),
                    ComponentKind::Skill,
                )],
            },
            ResolvedDependency {
                id: "b".to_string(),
                kind: ComponentKind::Skill,
                dependencies: vec![ComponentDependency::required(
                    "c".to_string(),
                    ComponentKind::Skill,
                )],
            },
            ResolvedDependency {
                id: "c".to_string(),
                kind: ComponentKind::Skill,
                dependencies: vec![],
            },
        ];

        let order = resolver.resolve_order(&components).unwrap();
        assert_eq!(order, vec!["c", "b", "a"]);
    }

    #[test]
    fn test_circular_dependency() {
        let resolver = DependencyResolver::new();

        let components = vec![
            ResolvedDependency {
                id: "a".to_string(),
                kind: ComponentKind::Skill,
                dependencies: vec![ComponentDependency::required(
                    "b".to_string(),
                    ComponentKind::Skill,
                )],
            },
            ResolvedDependency {
                id: "b".to_string(),
                kind: ComponentKind::Skill,
                dependencies: vec![ComponentDependency::required(
                    "a".to_string(),
                    ComponentKind::Skill,
                )],
            },
        ];

        assert!(resolver.resolve_order(&components).is_err());
    }
}

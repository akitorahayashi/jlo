//! Dependency resolver service.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::domain::{AppError, Component, ComponentId};
use crate::ports::ComponentCatalog;

/// Service for resolving component dependencies using topological sort.
pub struct DependencyResolver;

impl DependencyResolver {
    /// Resolve dependencies and return components in installation order.
    ///
    /// Uses Kahn's algorithm for topological sorting with cycle detection.
    /// Returns components with dependencies first.
    pub fn resolve<C: ComponentCatalog>(
        requested: &[String],
        catalog: &C,
    ) -> Result<Vec<Component>, AppError> {
        // Collect all components needed (transitive closure)
        let mut needed: BTreeMap<ComponentId, Component> = BTreeMap::new();
        let mut visiting: BTreeSet<ComponentId> = BTreeSet::new();

        for name in requested {
             // Validate ID format first
            let id = ComponentId::new(name)?;
            Self::collect_dependencies(&id, catalog, &mut needed, &mut visiting, &mut Vec::new())?;
        }

        // Build in-degree count
        // Edge A -> B means A depends on B (B must come before A)
        let mut in_degree: BTreeMap<ComponentId, usize> =
            needed.keys().map(|k| (k.clone(), 0)).collect();
        let mut dependents: BTreeMap<ComponentId, Vec<ComponentId>> =
            needed.keys().map(|k| (k.clone(), Vec::new())).collect();

        for (name, component) in &needed {
            for dep in &component.dependencies {
                if needed.contains_key(dep) {
                    *in_degree.get_mut(name).unwrap() += 1;
                    dependents.get_mut(dep).unwrap().push(name.clone());
                }
            }
        }

        // Kahn's algorithm
        let mut queue: VecDeque<ComponentId> =
            in_degree.iter().filter(|&(_, deg)| *deg == 0).map(|(k, _)| k.clone()).collect();

        // Sort for deterministic ordering
        let mut queue_vec: Vec<_> = queue.drain(..).collect();
        queue_vec.sort();
        queue = queue_vec.into_iter().collect();

        let mut result: Vec<Component> = Vec::new();

        while let Some(current) = queue.pop_front() {
            result.push(needed.remove(&current).unwrap());

            let deps = dependents.get(&current).cloned().unwrap_or_default();
            let mut next_batch = Vec::new();

            for dependent in deps {
                let deg = in_degree.get_mut(&dependent).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    next_batch.push(dependent);
                }
            }

            // Sort for deterministic ordering
            next_batch.sort();
            for name in next_batch {
                queue.push_back(name);
            }
        }

        // Check for cycle
        if result.len() != in_degree.len() {
            let remaining: Vec<_> =
                in_degree.iter().filter(|&(_, deg)| *deg > 0).map(|(k, _)| k.to_string()).collect();
            return Err(AppError::CircularDependency(remaining));
        }

        Ok(result)
    }

    fn collect_dependencies<C: ComponentCatalog>(
        id: &ComponentId,
        catalog: &C,
        collected: &mut BTreeMap<ComponentId, Component>,
        visiting: &mut BTreeSet<ComponentId>,
        path: &mut Vec<String>,
    ) -> Result<(), AppError> {
        if collected.contains_key(id) {
            return Ok(());
        }

        let name_str = id.as_str();

        if visiting.contains(id) {
            path.push(name_str.to_string());
            return Err(AppError::CircularDependency(path.clone()));
        }

        let component = catalog.get(name_str).ok_or_else(|| AppError::ComponentNotFound {
            name: name_str.to_string(),
            available: catalog.names().iter().map(|s| s.to_string()).collect(),
        })?;

        visiting.insert(id.clone());
        path.push(name_str.to_string());

        for dep in &component.dependencies {
            Self::collect_dependencies(dep, catalog, collected, visiting, path)?;
        }

        path.pop();
        visiting.remove(id);
        collected.insert(id.clone(), component.clone());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Component, ComponentId};

    struct TestCatalog {
        components: BTreeMap<String, Component>,
    }

    impl TestCatalog {
        fn new(components: Vec<Component>) -> Self {
            Self { components: components.into_iter().map(|c| (c.name.to_string(), c)).collect() }
        }
    }

    impl ComponentCatalog for TestCatalog {
        fn get(&self, name: &str) -> Option<&Component> {
            self.components.get(name)
        }

        fn list_all(&self) -> Vec<&Component> {
            self.components.values().collect()
        }

        fn names(&self) -> Vec<&str> {
            self.components.keys().map(String::as_str).collect()
        }
    }

    fn make_component(name: &str, deps: &[&str]) -> Component {
        Component {
            name: ComponentId::new(name).unwrap(),
            summary: format!("{} component", name),
            dependencies: deps.iter().map(|s| ComponentId::new(s).unwrap()).collect(),
            env: vec![],
            script_content: format!("echo {}", name),
        }
    }

    #[test]
    fn resolve_single_component() {
        let catalog = TestCatalog::new(vec![make_component("a", &[])]);

        let result = DependencyResolver::resolve(&["a".to_string()], &catalog).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name.as_str(), "a");
    }

    #[test]
    fn resolve_with_dependency() {
        let catalog = TestCatalog::new(vec![make_component("a", &[]), make_component("b", &["a"])]);

        let result = DependencyResolver::resolve(&["b".to_string()], &catalog).unwrap();

        assert_eq!(result.len(), 2);
        let names: Vec<_> = result.iter().map(|c| c.name.as_str()).collect();
        assert!(names.iter().position(|&n| n == "a") < names.iter().position(|&n| n == "b"));
    }

    #[test]
    fn resolve_chain_dependency() {
        let catalog = TestCatalog::new(vec![
            make_component("a", &[]),
            make_component("b", &["a"]),
            make_component("c", &["b"]),
        ]);

        let result = DependencyResolver::resolve(&["c".to_string()], &catalog).unwrap();

        assert_eq!(result.len(), 3);
        let names: Vec<_> = result.iter().map(|c| c.name.as_str()).collect();
        assert!(names.iter().position(|&n| n == "a") < names.iter().position(|&n| n == "b"));
        assert!(names.iter().position(|&n| n == "b") < names.iter().position(|&n| n == "c"));
    }

    #[test]
    fn detect_circular_dependency() {
        let catalog =
            TestCatalog::new(vec![make_component("x", &["y"]), make_component("y", &["x"])]);

        let result = DependencyResolver::resolve(&["x".to_string()], &catalog);

        assert!(matches!(result, Err(AppError::CircularDependency(_))));
    }

    #[test]
    fn invalid_component_id() {
        let catalog = TestCatalog::new(vec![]);
        let result = DependencyResolver::resolve(&["invalid/id".to_string()], &catalog);
        assert!(matches!(result, Err(AppError::InvalidComponentId(_))));
    }
}

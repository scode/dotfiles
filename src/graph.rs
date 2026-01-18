//! Dependency graph for feature installation ordering.
//!
//! Features can declare dependencies on other features and conditions that
//! must be met before installation. The graph ensures:
//!
//! - Dependencies are installed before dependents
//! - Dependents are uninstalled before dependencies
//! - Features with unmet conditions are skipped (along with their dependents)
//! - Features whose dependencies failed are skipped

use std::collections::{HashMap, HashSet};

use anyhow::{Result, bail};
use tracing::{debug, error, info};

use crate::condition::Condition;
use crate::features::{Feature, FeatureResult};

struct FeatureNode {
    id: String,
    feature: Box<dyn Feature>,
    depends_on: Vec<String>,
    condition: Option<Box<dyn Condition>>,
}

/// Orchestrates feature installation with dependency ordering and conditions.
///
/// # Example
///
/// ```ignore
/// let mut g = FeatureGraph::new();
/// g.add("parent-dir", ManagedDirectory::new("~/.config/app"))
///     .condition(PathExists::new("~/.config"));
/// g.add("config-file", PayloadSymlink::new("payload/config", "~/.config/app/config"))
///     .depends_on("parent-dir");
/// g.install()?;
/// ```
pub struct FeatureGraph {
    nodes: HashMap<String, FeatureNode>,
    /// Tracks insertion order for the builder pattern (last-added node).
    order: Vec<String>,
}

impl FeatureGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            order: Vec::new(),
        }
    }

    pub fn add(&mut self, id: impl Into<String>, feature: impl Feature + 'static) -> &mut Self {
        let id = id.into();
        let node = FeatureNode {
            id: id.clone(),
            feature: Box::new(feature),
            depends_on: Vec::new(),
            condition: None,
        };
        self.order.push(id.clone());
        self.nodes.insert(id, node);
        self
    }

    pub fn depends_on(&mut self, dependency_id: impl Into<String>) -> &mut Self {
        let dep = dependency_id.into();
        let id = self
            .order
            .last()
            .expect("depends_on() called before any add()");
        self.nodes
            .get_mut(id)
            .expect("internal error: node missing")
            .depends_on
            .push(dep);
        self
    }

    pub fn condition(&mut self, condition: impl Condition + 'static) -> &mut Self {
        let id = self
            .order
            .last()
            .expect("condition() called before any add()");
        self.nodes
            .get_mut(id)
            .expect("internal error: node missing")
            .condition = Some(Box::new(condition));
        self
    }

    fn topological_sort(&self) -> Result<Vec<String>> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

        for id in self.nodes.keys() {
            in_degree.insert(id.as_str(), 0);
            dependents.insert(id.as_str(), Vec::new());
        }

        for node in self.nodes.values() {
            for dep in &node.depends_on {
                if !self.nodes.contains_key(dep) {
                    bail!("feature '{}' depends on unknown feature '{}'", node.id, dep);
                }
                dependents.get_mut(dep.as_str()).unwrap().push(&node.id);
                *in_degree.get_mut(node.id.as_str()).unwrap() += 1;
            }
        }

        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|&(_, deg)| *deg == 0)
            .map(|(&id, _)| id)
            .collect();
        queue.sort();

        let mut result = Vec::new();

        while let Some(id) = queue.pop() {
            result.push(id.to_string());
            let mut next_batch = Vec::new();
            for &dependent in &dependents[id] {
                let deg = in_degree.get_mut(dependent).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    next_batch.push(dependent);
                }
            }
            next_batch.sort();
            next_batch.reverse();
            queue.extend(next_batch);
        }

        if result.len() != self.nodes.len() {
            bail!("cycle detected in feature dependencies");
        }

        Ok(result)
    }

    pub fn install(&self) -> Result<()> {
        let order = self.topological_sort()?;
        let mut skipped: HashSet<String> = HashSet::new();
        let mut failed: HashSet<String> = HashSet::new();

        for id in order {
            let node = &self.nodes[&id];

            if node
                .depends_on
                .iter()
                .any(|dep| skipped.contains(dep) || failed.contains(dep))
            {
                info!("⏭️ skipped: {} (dependency unavailable)", node.feature);
                skipped.insert(id);
                continue;
            }

            if let Some(cond) = &node.condition
                && !cond.is_met()
            {
                info!("⏭️ skipped: {} ({})", node.feature, cond);
                skipped.insert(id);
                continue;
            }

            debug!("installing feature: {:?}", node.feature);
            match node.feature.install() {
                Ok(FeatureResult::Changed) => {
                    info!("✅ changed: {}", node.feature);
                }
                Ok(FeatureResult::NoOp) => {
                    info!("⏭️ noop:    {}", node.feature);
                }
                Err(e) => {
                    error!("❌ {}: {}", node.feature, e);
                    failed.insert(id);
                }
            }
        }

        if !failed.is_empty() {
            bail!("one or more features failed");
        }
        Ok(())
    }

    pub fn uninstall(&self) -> Result<()> {
        let order = self.topological_sort()?;
        let mut failed: HashSet<String> = HashSet::new();

        for id in order.into_iter().rev() {
            let node = &self.nodes[&id];

            debug!("uninstalling feature: {:?}", node.feature);
            match node.feature.uninstall() {
                Ok(FeatureResult::Changed) => {
                    info!("✅ changed: {}", node.feature);
                }
                Ok(FeatureResult::NoOp) => {
                    info!("⏭️ noop:    {}", node.feature);
                }
                Err(e) => {
                    error!("❌ {}: {}", node.feature, e);
                    failed.insert(id);
                }
            }
        }

        if !failed.is_empty() {
            bail!("one or more features failed");
        }
        Ok(())
    }
}

impl Default for FeatureGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::PathExists;
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    #[derive(Debug)]
    struct MockFeature {
        name: String,
        install_log: Arc<Mutex<Vec<String>>>,
        should_fail: bool,
    }

    impl MockFeature {
        fn new(name: &str, install_log: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                name: name.to_string(),
                install_log,
                should_fail: false,
            }
        }

        fn failing(name: &str, install_log: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                name: name.to_string(),
                install_log,
                should_fail: true,
            }
        }
    }

    impl std::fmt::Display for MockFeature {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "mock: {}", self.name)
        }
    }

    impl Feature for MockFeature {
        fn install(&self) -> Result<FeatureResult> {
            if self.should_fail {
                bail!("mock failure");
            }
            self.install_log
                .lock()
                .unwrap()
                .push(format!("install:{}", self.name));
            Ok(FeatureResult::Changed)
        }

        fn uninstall(&self) -> Result<FeatureResult> {
            if self.should_fail {
                bail!("mock failure");
            }
            self.install_log
                .lock()
                .unwrap()
                .push(format!("uninstall:{}", self.name));
            Ok(FeatureResult::Changed)
        }
    }

    #[test]
    fn install_respects_dependency_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();

        graph.add("b", MockFeature::new("b", log.clone()));
        graph
            .add("a", MockFeature::new("a", log.clone()))
            .depends_on("b");

        graph.install().unwrap();

        let log = log.lock().unwrap();
        assert_eq!(*log, vec!["install:b", "install:a"]);
    }

    #[test]
    fn install_skips_when_condition_fails() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();

        graph
            .add("a", MockFeature::new("a", log.clone()))
            .condition(PathExists::new("/nonexistent/path/that/does/not/exist"));

        graph.install().unwrap();

        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn install_skips_dependents_when_condition_fails() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();

        graph
            .add("a", MockFeature::new("a", log.clone()))
            .condition(PathExists::new("/nonexistent/path/that/does/not/exist"));
        graph
            .add("b", MockFeature::new("b", log.clone()))
            .depends_on("a");

        graph.install().unwrap();

        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn install_skips_dependents_when_dependency_fails() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();

        graph.add("a", MockFeature::failing("a", log.clone()));
        graph
            .add("b", MockFeature::new("b", log.clone()))
            .depends_on("a");

        let result = graph.install();
        assert!(result.is_err());
        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn detect_cycle() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();

        graph
            .add("a", MockFeature::new("a", log.clone()))
            .depends_on("b");
        graph
            .add("b", MockFeature::new("b", log.clone()))
            .depends_on("a");

        let result = graph.install();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cycle"));
    }

    #[test]
    fn detect_unknown_dependency() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();

        graph
            .add("a", MockFeature::new("a", log.clone()))
            .depends_on("nonexistent");

        let result = graph.install();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown feature"));
    }

    #[test]
    fn uninstall_reverses_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();

        graph.add("b", MockFeature::new("b", log.clone()));
        graph
            .add("a", MockFeature::new("a", log.clone()))
            .depends_on("b");

        graph.uninstall().unwrap();

        let log = log.lock().unwrap();
        assert_eq!(*log, vec!["uninstall:a", "uninstall:b"]);
    }

    #[test]
    fn condition_met_allows_install() {
        let dir = tempdir().unwrap();
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();

        graph
            .add("a", MockFeature::new("a", log.clone()))
            .condition(PathExists::new(dir.path().to_string_lossy().to_string()));

        graph.install().unwrap();

        let log = log.lock().unwrap();
        assert_eq!(*log, vec!["install:a"]);
    }
}

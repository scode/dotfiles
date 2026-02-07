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
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Result, bail};
use tracing::{debug, error, info};

use crate::condition::Condition;
use crate::features::{Feature, FeatureResult};

static NEXT_GRAPH_ID: AtomicU64 = AtomicU64::new(0);

struct FeatureNode {
    id: String,
    feature: Box<dyn Feature>,
    depends_on: Vec<String>,
    condition: Option<Box<dyn Condition>>,
}

/// A handle to a registered feature, used for declaring dependencies.
///
/// Handles are lightweight (`Clone`) and can be passed to functions
/// that need to declare dependencies on a feature. Handles are scoped
/// to the graph that created them; using a handle from a different
/// graph will panic.
#[derive(Clone)]
pub struct FeatureHandle {
    id: String,
    graph_id: u64,
}

/// Builder for configuring a feature after registration.
///
/// Returned by [`FeatureGraph::add`]. The feature is registered immediately;
/// this builder allows adding dependencies and conditions before finalizing
/// with [`build()`](Self::build).
pub struct FeatureBuilder<'a> {
    graph: &'a mut FeatureGraph,
    id: String,
    graph_id: u64,
}

impl<'a> FeatureBuilder<'a> {
    /// Declares that this feature depends on another feature.
    ///
    /// The dependency will be installed before this feature, and this feature
    /// will be skipped if the dependency fails or is skipped.
    ///
    /// # Panics
    ///
    /// Panics if the handle is from a different `FeatureGraph`.
    pub fn depends_on(self, dep: &FeatureHandle) -> Self {
        assert_eq!(
            self.graph_id, dep.graph_id,
            "FeatureHandle from a different FeatureGraph"
        );
        self.graph
            .nodes
            .get_mut(&self.id)
            .expect("internal error: node missing")
            .depends_on
            .push(dep.id.clone());
        self
    }

    /// Sets a condition that must be met for this feature to be installed.
    ///
    /// If the condition is not met, this feature and all features depending
    /// on it will be skipped.
    pub fn condition(self, condition: impl Condition + 'static) -> Self {
        self.graph
            .nodes
            .get_mut(&self.id)
            .expect("internal error: node missing")
            .condition = Some(Box::new(condition));
        self
    }

    /// Finalizes the feature configuration and returns its handle.
    ///
    /// The handle can be passed to other features' [`depends_on`](Self::depends_on)
    /// to declare dependencies, or to helper functions that register related features.
    pub fn build(self) -> FeatureHandle {
        FeatureHandle {
            id: self.id,
            graph_id: self.graph_id,
        }
    }
}

/// Orchestrates feature installation with dependency ordering and conditions.
///
/// # Example
///
/// ```ignore
/// let mut g = FeatureGraph::new();
/// let parent = g.add("parent-dir", ManagedDirectory::new("~/.config/app"))
///     .condition(PathExists::new("~/.config"))
///     .build();
/// g.add("config-file", PayloadSymlink::new("payload/config", "~/.config/app/config"))
///     .depends_on(&parent)
///     .build();
/// g.install()?;
/// ```
pub struct FeatureGraph {
    nodes: HashMap<String, FeatureNode>,
    graph_id: u64,
}

impl FeatureGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            graph_id: NEXT_GRAPH_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// Adds a feature to the graph and returns a builder for configuration.
    ///
    /// The feature is registered immediately. Use the returned builder to
    /// add dependencies and conditions, then call [`FeatureBuilder::build`]
    /// to get a handle for use in other features' dependencies.
    ///
    /// # Panics
    ///
    /// Panics if `id` is already present in the graph.
    pub fn add(
        &mut self,
        id: impl Into<String>,
        feature: impl Feature + 'static,
    ) -> FeatureBuilder<'_> {
        let id = id.into();
        assert!(
            !self.nodes.contains_key(&id),
            "feature '{}' already exists",
            id
        );
        let node = FeatureNode {
            id: id.clone(),
            feature: Box::new(feature),
            depends_on: Vec::new(),
            condition: None,
        };
        let graph_id = self.graph_id;
        self.nodes.insert(id.clone(), node);
        FeatureBuilder {
            graph: self,
            id,
            graph_id,
        }
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

        let b = graph.add("b", MockFeature::new("b", log.clone())).build();
        graph
            .add("a", MockFeature::new("a", log.clone()))
            .depends_on(&b)
            .build();

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
            .condition(PathExists::new("/nonexistent/path/that/does/not/exist"))
            .build();

        graph.install().unwrap();

        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn install_skips_dependents_when_condition_fails() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();

        let a = graph
            .add("a", MockFeature::new("a", log.clone()))
            .condition(PathExists::new("/nonexistent/path/that/does/not/exist"))
            .build();
        graph
            .add("b", MockFeature::new("b", log.clone()))
            .depends_on(&a)
            .build();

        graph.install().unwrap();

        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn install_skips_dependents_when_dependency_fails() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();

        let a = graph
            .add("a", MockFeature::failing("a", log.clone()))
            .build();
        graph
            .add("b", MockFeature::new("b", log.clone()))
            .depends_on(&a)
            .build();

        let result = graph.install();
        assert!(result.is_err());
        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn detect_cycle() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();

        // The public API prevents cycles (dependencies must exist before dependents),
        // so we inject a back-edge directly to test cycle detection.
        let a = graph.add("a", MockFeature::new("a", log.clone())).build();
        graph
            .add("b", MockFeature::new("b", log.clone()))
            .depends_on(&a)
            .build();
        graph
            .nodes
            .get_mut("a")
            .unwrap()
            .depends_on
            .push("b".to_string());

        let result = graph.install();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cycle"));
    }

    #[test]
    fn uninstall_reverses_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();

        let b = graph.add("b", MockFeature::new("b", log.clone())).build();
        graph
            .add("a", MockFeature::new("a", log.clone()))
            .depends_on(&b)
            .build();

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
            .condition(PathExists::new(dir.path().to_string_lossy().to_string()))
            .build();

        graph.install().unwrap();

        let log = log.lock().unwrap();
        assert_eq!(*log, vec!["install:a"]);
    }

    #[test]
    #[should_panic(expected = "FeatureHandle from a different FeatureGraph")]
    fn cross_graph_handle_panics() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph1 = FeatureGraph::new();
        let mut graph2 = FeatureGraph::new();

        let handle_from_graph1 = graph1.add("a", MockFeature::new("a", log.clone())).build();

        // Using a handle from graph1 in graph2 should panic
        graph2
            .add("b", MockFeature::new("b", log))
            .depends_on(&handle_from_graph1)
            .build();
    }

    #[test]
    #[should_panic(expected = "feature 'a' already exists")]
    fn duplicate_feature_id_panics() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();

        graph
            .add("a", MockFeature::new("first", log.clone()))
            .build();
        graph.add("a", MockFeature::new("second", log)).build();
    }
}

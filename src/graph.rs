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
use std::fmt::Write;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Result, bail};
use tracing::{debug, error, info};

use crate::condition::Condition;
use crate::features::{Feature, FeatureResult};

/// Aggregate outcome of an install or uninstall run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStats {
    pub changed: u32,
    pub unchanged: u32,
    pub skipped: u32,
    pub failed: u32,
}

/// Counts and a complete, dependency-shaped view of one installer run.
///
/// The tree contains every registered feature, including features that did
/// nothing. A feature with multiple dependencies appears under its first
/// declared dependency; its other dependencies are named on the same line.
/// Each line includes the feature ID so those dependency references can be
/// found even when several features have the same display text.
#[derive(Debug)]
pub struct RunReport {
    /// Aggregate counts for the same run represented by `status_tree`.
    pub stats: RunStats,
    /// Every feature, ordered by its displayed dependency branch.
    pub status_tree: String,
}

/// The outcome retained for the optional end-of-run tree.
///
/// Dependency skips keep the blocked feature IDs so a node with multiple
/// prerequisites can identify a blocker outside its displayed parent branch.
enum FeatureStatus {
    Changed,
    Unchanged,
    SkippedByCondition(String),
    SkippedByDependency(Vec<String>),
    Failed(String),
}

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
/// let stats = g.install()?;
/// assert_eq!(stats.failed, 0);
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

    /// Renders every feature once, with dependencies above their dependents.
    ///
    /// A dependency graph is not always a tree. The first declared dependency
    /// owns the display branch, while any other dependencies remain visible on
    /// the child's line. Sorting each branch makes reports stable even though
    /// feature storage uses a hash map.
    fn status_tree(&self, statuses: &HashMap<String, FeatureStatus>) -> String {
        let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut roots = Vec::new();

        for node in self.nodes.values() {
            if let Some(parent) = node.depends_on.first() {
                children.entry(parent).or_default().push(&node.id);
            } else {
                roots.push(node.id.as_str());
            }
        }
        roots.sort_unstable();
        for siblings in children.values_mut() {
            siblings.sort_unstable();
        }

        let mut tree = String::from("Status tree:\n");
        for (index, root) in roots.iter().enumerate() {
            self.write_status_branch(
                &mut tree,
                root,
                "",
                index + 1 == roots.len(),
                &children,
                statuses,
            );
        }
        tree
    }

    /// Writes one display branch without duplicating shared dependencies.
    fn write_status_branch(
        &self,
        tree: &mut String,
        id: &str,
        prefix: &str,
        last: bool,
        children: &HashMap<&str, Vec<&str>>,
        statuses: &HashMap<String, FeatureStatus>,
    ) {
        let node = &self.nodes[id];
        let status = &statuses[id];
        let branch = if last { "└─" } else { "├─" };
        let label = match status {
            FeatureStatus::Changed => "✅ changed",
            FeatureStatus::Unchanged => "⏭️ unchanged",
            FeatureStatus::SkippedByCondition(_) | FeatureStatus::SkippedByDependency(_) => {
                "⏭️ skipped"
            }
            FeatureStatus::Failed(_) => "❌ failed",
        };
        write!(tree, "{prefix}{branch} {label}: {} [{id}]", node.feature).unwrap();

        match status {
            FeatureStatus::SkippedByCondition(condition) => {
                write!(tree, " (condition not met: {condition})").unwrap();
            }
            FeatureStatus::SkippedByDependency(blocked) => {
                if blocked.len() != 1 || blocked[0] != node.depends_on[0] {
                    write!(tree, " (blocked by: {})", blocked.join(", ")).unwrap();
                }
            }
            FeatureStatus::Failed(error) => {
                write!(tree, " ({error})").unwrap();
            }
            FeatureStatus::Changed | FeatureStatus::Unchanged => {}
        }

        if node.depends_on.len() > 1 {
            write!(
                tree,
                " [also depends on: {}]",
                node.depends_on[1..].join(", ")
            )
            .unwrap();
        }
        tree.push('\n');

        let child_prefix = format!("{prefix}{}", if last { "   " } else { "│  " });
        if let Some(descendants) = children.get(id) {
            for (index, child) in descendants.iter().enumerate() {
                self.write_status_branch(
                    tree,
                    child,
                    &child_prefix,
                    index + 1 == descendants.len(),
                    children,
                    statuses,
                );
            }
        }
    }

    /// Install all features in dependency order, skipping any whose
    /// conditions are unmet or whose dependencies failed.
    ///
    /// Individual feature failures are counted in [`RunStats::failed`],
    /// not returned as `Err`. `Err` is only returned for structural
    /// problems like dependency cycles.
    pub fn install(&self) -> Result<RunStats> {
        Ok(self.install_with_report()?.stats)
    }

    /// Installs all features and returns their outcomes as a complete tree.
    ///
    /// Skips are recorded for the report but not logged individually. A skipped
    /// dependent remains visible under its unavailable prerequisite, while an
    /// unmet condition is shown on the feature where it was evaluated.
    pub fn install_with_report(&self) -> Result<RunReport> {
        let order = self.topological_sort()?;
        let mut skipped: HashSet<String> = HashSet::new();
        let mut failed: HashSet<String> = HashSet::new();
        let mut statuses: HashMap<String, FeatureStatus> = HashMap::new();
        let mut changed: u32 = 0;
        let mut noop: u32 = 0;

        for id in order {
            let node = &self.nodes[&id];

            let blocked: Vec<String> = node
                .depends_on
                .iter()
                .filter(|dep| skipped.contains(*dep) || failed.contains(*dep))
                .cloned()
                .collect();
            if !blocked.is_empty() {
                statuses.insert(id.clone(), FeatureStatus::SkippedByDependency(blocked));
                skipped.insert(id);
                continue;
            }

            if let Some(cond) = &node.condition {
                match cond.is_met() {
                    Ok(true) => {}
                    Ok(false) => {
                        statuses.insert(
                            id.clone(),
                            FeatureStatus::SkippedByCondition(cond.to_string()),
                        );
                        skipped.insert(id);
                        continue;
                    }
                    Err(e) => {
                        error!("❌ {} condition failed: {}", node.feature, e);
                        statuses.insert(id.clone(), FeatureStatus::Failed(e.to_string()));
                        failed.insert(id);
                        continue;
                    }
                }
            }

            debug!("installing feature: {:?}", node.feature);
            match node.feature.install() {
                Ok(FeatureResult::Changed) => {
                    info!("✅ changed: {}", node.feature);
                    statuses.insert(id, FeatureStatus::Changed);
                    changed += 1;
                }
                Ok(FeatureResult::NoOp) => {
                    debug!("⏭️ noop:    {}", node.feature);
                    statuses.insert(id, FeatureStatus::Unchanged);
                    noop += 1;
                }
                Err(e) => {
                    error!("❌ {}: {}", node.feature, e);
                    statuses.insert(id.clone(), FeatureStatus::Failed(e.to_string()));
                    failed.insert(id);
                }
            }
        }

        Ok(RunReport {
            stats: RunStats {
                changed,
                unchanged: noop,
                skipped: skipped.len() as u32,
                failed: failed.len() as u32,
            },
            status_tree: self.status_tree(&statuses),
        })
    }

    /// Uninstall all features in reverse dependency order.
    ///
    /// Individual feature failures are counted in [`RunStats::failed`],
    /// not returned as `Err`.
    pub fn uninstall(&self) -> Result<RunStats> {
        Ok(self.uninstall_with_report()?.stats)
    }

    /// Uninstalls all features and reports each outcome in dependency order.
    ///
    /// Uninstall executes in reverse dependency order, but the report keeps
    /// prerequisites above dependents so the same tree is readable for both
    /// commands. Uninstall has no dependency skips.
    pub fn uninstall_with_report(&self) -> Result<RunReport> {
        let order = self.topological_sort()?;
        let mut failed: HashSet<String> = HashSet::new();
        let mut statuses: HashMap<String, FeatureStatus> = HashMap::new();
        let mut changed: u32 = 0;
        let mut noop: u32 = 0;

        for id in order.into_iter().rev() {
            let node = &self.nodes[&id];

            debug!("uninstalling feature: {:?}", node.feature);
            match node.feature.uninstall() {
                Ok(FeatureResult::Changed) => {
                    info!("✅ changed: {}", node.feature);
                    statuses.insert(id, FeatureStatus::Changed);
                    changed += 1;
                }
                Ok(FeatureResult::NoOp) => {
                    debug!("⏭️ noop:    {}", node.feature);
                    statuses.insert(id, FeatureStatus::Unchanged);
                    noop += 1;
                }
                Err(e) => {
                    error!("❌ {}: {}", node.feature, e);
                    statuses.insert(id.clone(), FeatureStatus::Failed(e.to_string()));
                    failed.insert(id);
                }
            }
        }

        Ok(RunReport {
            stats: RunStats {
                changed,
                unchanged: noop,
                skipped: 0,
                failed: failed.len() as u32,
            },
            status_tree: self.status_tree(&statuses),
        })
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
        result: Result<FeatureResult, &'static str>,
    }

    impl MockFeature {
        fn new(name: &str, install_log: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                name: name.to_string(),
                install_log,
                result: Ok(FeatureResult::Changed),
            }
        }

        fn noop(name: &str, install_log: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                name: name.to_string(),
                install_log,
                result: Ok(FeatureResult::NoOp),
            }
        }

        fn failing(name: &str, install_log: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                name: name.to_string(),
                install_log,
                result: Err("mock failure"),
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
            match self.result {
                Ok(r) => {
                    self.install_log
                        .lock()
                        .unwrap()
                        .push(format!("install:{}", self.name));
                    Ok(r)
                }
                Err(msg) => bail!("{msg}"),
            }
        }

        fn uninstall(&self) -> Result<FeatureResult> {
            match self.result {
                Ok(r) => {
                    self.install_log
                        .lock()
                        .unwrap()
                        .push(format!("uninstall:{}", self.name));
                    Ok(r)
                }
                Err(msg) => bail!("{msg}"),
            }
        }
    }

    #[derive(Debug)]
    struct ErrorCondition;

    impl std::fmt::Display for ErrorCondition {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "error condition")
        }
    }

    impl Condition for ErrorCondition {
        fn is_met(&self) -> Result<bool> {
            bail!("condition exploded")
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
    fn install_fails_when_condition_cannot_be_evaluated() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();

        graph
            .add("a", MockFeature::new("a", log.clone()))
            .condition(ErrorCondition)
            .build();

        let stats = graph.install().unwrap();

        assert_eq!(
            stats,
            RunStats {
                changed: 0,
                unchanged: 0,
                skipped: 0,
                failed: 1,
            }
        );
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

        let stats = graph.install().unwrap();
        assert_eq!(
            stats,
            RunStats {
                changed: 0,
                unchanged: 0,
                skipped: 1,
                failed: 1,
            }
        );
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

    #[test]
    fn install_stats_all_changed() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();
        graph.add("a", MockFeature::new("a", log.clone())).build();
        graph.add("b", MockFeature::new("b", log)).build();

        let stats = graph.install().unwrap();
        assert_eq!(
            stats,
            RunStats {
                changed: 2,
                unchanged: 0,
                skipped: 0,
                failed: 0,
            }
        );
    }

    #[test]
    fn install_stats_mixed() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();
        graph.add("a", MockFeature::new("a", log.clone())).build();
        graph.add("b", MockFeature::noop("b", log.clone())).build();
        graph
            .add("c", MockFeature::new("c", log))
            .condition(PathExists::new("/nonexistent/path/that/does/not/exist"))
            .build();

        let stats = graph.install().unwrap();
        assert_eq!(
            stats,
            RunStats {
                changed: 1,
                unchanged: 1,
                skipped: 1,
                failed: 0,
            }
        );
    }

    #[test]
    fn install_stats_with_failure() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();
        graph.add("a", MockFeature::new("a", log.clone())).build();
        let b = graph
            .add("b", MockFeature::failing("b", log.clone()))
            .build();
        // c is skipped because b failed
        graph
            .add("c", MockFeature::new("c", log))
            .depends_on(&b)
            .build();

        let stats = graph.install().unwrap();
        assert_eq!(
            stats,
            RunStats {
                changed: 1,
                unchanged: 0,
                skipped: 1,
                failed: 1,
            }
        );
    }

    /// The detailed report must account for every feature once, even when a
    /// dependent has two prerequisites and only one of them blocks it.
    #[test]
    fn install_report_shows_complete_dependency_tree() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();
        let changed = graph
            .add("changed", MockFeature::new("changed", log.clone()))
            .build();
        graph
            .add("unchanged", MockFeature::noop("unchanged", log.clone()))
            .depends_on(&changed)
            .build();
        let missing = graph
            .add("missing", MockFeature::new("absent path", log.clone()))
            .condition(PathExists::new("/nonexistent/path/that/does/not/exist"))
            .build();
        graph
            .add("blocked", MockFeature::new("blocked", log.clone()))
            .depends_on(&changed)
            .depends_on(&missing)
            .build();
        graph
            .add("failed", MockFeature::failing("failed", log))
            .build();

        let report = graph.install_with_report().unwrap();

        assert_eq!(report.stats.changed, 1);
        assert_eq!(report.stats.unchanged, 1);
        assert_eq!(report.stats.skipped, 2);
        assert_eq!(report.stats.failed, 1);
        assert_eq!(report.status_tree.lines().count(), 6);
        assert!(
            report
                .status_tree
                .contains("├─ ✅ changed: mock: changed [changed]")
        );
        assert!(report.status_tree.contains(
            "│  ├─ ⏭️ skipped: mock: blocked [blocked] (blocked by: missing) [also depends on: missing]"
        ));
        assert!(
            report
                .status_tree
                .contains("│  └─ ⏭️ unchanged: mock: unchanged [unchanged]")
        );
        assert!(
            report
                .status_tree
                .contains("├─ ❌ failed: mock: failed [failed] (mock failure)")
        );
        assert!(report.status_tree.contains("└─ ⏭️ skipped: mock: absent path [missing] (condition not met: path exists: /nonexistent/path/that/does/not/exist)"));
    }

    /// Uninstall runs from leaves to roots, but its report uses the same
    /// dependency shape as install so every feature remains easy to locate.
    #[test]
    fn uninstall_report_keeps_dependency_shape() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();
        let parent = graph
            .add("parent", MockFeature::new("parent", log.clone()))
            .build();
        graph
            .add("child", MockFeature::noop("child", log))
            .depends_on(&parent)
            .build();

        let report = graph.uninstall_with_report().unwrap();

        assert_eq!(
            report.status_tree,
            "Status tree:\n└─ ✅ changed: mock: parent [parent]\n   └─ ⏭️ unchanged: mock: child [child]\n"
        );
    }

    #[test]
    fn uninstall_stats_mixed() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut graph = FeatureGraph::new();
        graph.add("a", MockFeature::new("a", log.clone())).build();
        graph.add("b", MockFeature::noop("b", log)).build();

        let stats = graph.uninstall().unwrap();
        assert_eq!(
            stats,
            RunStats {
                changed: 1,
                unchanged: 1,
                skipped: 0,
                failed: 0,
            }
        );
    }
}

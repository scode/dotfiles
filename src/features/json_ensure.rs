use std::fmt;
use std::fs;
use std::path::Path;

use anyhow::{Result, bail};
use serde_json::{Map, Value};
use tracing::debug;

use super::{Feature, FeatureResult};
use crate::util::fs::{expand_tilde, normalize_path};

/// Ensures a regular JSON file contains a small, owned subset of values.
///
/// This feature exists for configs where symlinking a whole blob creates more
/// churn than value. It owns only the JSON paths declared in its operations.
/// Everything else in an existing regular file is preserved as-is.
///
/// Install behavior:
/// - Missing destination: create a new regular JSON object file.
/// - Existing regular file: parse it, validate the managed paths, then merge.
/// - Legacy installer symlink: replace it with a regular JSON file built from
///   the managed subset.
///
/// This does not try to "fix" malformed JSON or incompatible types. If the
/// file cannot be merged safely, install fails instead of guessing.
///
/// Uninstall is intentionally a no-op. Once the file becomes user-managed
/// state, uninstall should stop enforcing keys rather than trying to edit the
/// file back into some previous shape.
#[derive(Debug)]
pub struct JsonEnsure {
    destination: String,
    operations: Vec<JsonEnsureOperation>,
    legacy_payload_symlink_source: Option<String>,
}

#[derive(Debug)]
enum DestinationState {
    Missing,
    Regular(Value),
    LegacySymlink,
}

#[derive(Debug)]
enum JsonEnsureOperation {
    StringsInArray {
        path: Vec<String>,
        values: Vec<String>,
    },
    Value {
        path: Vec<String>,
        value: Value,
    },
    ValueIfPathExists {
        path: Vec<String>,
        condition_path: String,
        value: Value,
    },
}

impl fmt::Display for JsonEnsure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "json ensure: {}", self.destination)
    }
}

impl JsonEnsure {
    /// Creates a JSON ensure feature for the destination file.
    pub fn new(destination: impl Into<String>) -> Self {
        Self {
            destination: destination.into(),
            operations: Vec::new(),
            legacy_payload_symlink_source: None,
        }
    }

    /// Treats a symlink to the given payload path as legacy installer state.
    ///
    /// If the destination currently points there, install removes the symlink
    /// and writes a regular JSON file instead. Any other symlink target is
    /// treated as user-owned and therefore an error.
    pub fn legacy_payload_symlink_source(mut self, source: impl Into<String>) -> Self {
        self.legacy_payload_symlink_source = Some(source.into());
        self
    }

    /// Ensures an array path contains the given strings at least once each.
    pub fn ensure_strings_in_array(
        mut self,
        path: &[&str],
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.operations.push(JsonEnsureOperation::StringsInArray {
            path: to_path(path),
            values: values.into_iter().map(Into::into).collect(),
        });
        self
    }

    /// Ensures a path exists with the exact JSON value provided.
    pub fn ensure_value(mut self, path: &[&str], value: Value) -> Self {
        self.operations.push(JsonEnsureOperation::Value {
            path: to_path(path),
            value,
        });
        self
    }

    /// Ensures a value only when a filesystem path exists; otherwise removes it.
    ///
    /// This is useful when the JSON points at another installed artifact. The
    /// config should not keep advertising a command path that is absent.
    pub fn ensure_value_if_path_exists(
        mut self,
        path: &[&str],
        condition_path: impl Into<String>,
        value: Value,
    ) -> Self {
        self.operations
            .push(JsonEnsureOperation::ValueIfPathExists {
                path: to_path(path),
                condition_path: condition_path.into(),
                value,
            });
        self
    }

    fn install_with_base_dir(&self, base_dir: &Path) -> Result<FeatureResult> {
        let dest_path = expand_tilde(&self.destination)?;
        let parent = dest_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("destination has no parent directory"))?;

        if !parent.is_dir() {
            bail!(
                "destination parent directory does not exist: {}",
                parent.display()
            );
        }

        let state = self.load_destination_state(base_dir, &dest_path)?;
        let was_regular = matches!(state, DestinationState::Regular(_));
        let was_legacy_symlink = matches!(state, DestinationState::LegacySymlink);
        let mut root = match state {
            DestinationState::Missing | DestinationState::LegacySymlink => {
                Value::Object(Map::new())
            }
            DestinationState::Regular(existing) => existing,
        };

        let changed = self.apply_operations(&mut root)?;
        let needs_write = changed || !was_regular;

        if !needs_write {
            debug!(destination = %self.destination, "json file already satisfies managed subset");
            return Ok(FeatureResult::NoOp);
        }

        if was_legacy_symlink {
            fs::remove_file(&dest_path)?;
        }

        write_pretty_json(&dest_path, &root)?;
        debug!(destination = %self.destination, "wrote managed json subset");
        Ok(FeatureResult::Changed)
    }

    /// Reads the current destination and decides whether it is mergeable state.
    ///
    /// The interesting case here is the old installer symlink. That symlink was
    /// previously "owned" by the repo, so it is safe to replace. A symlink to
    /// anything else is ambiguous user state and should fail loudly.
    fn load_destination_state(
        &self,
        base_dir: &Path,
        dest_path: &Path,
    ) -> Result<DestinationState> {
        let metadata = match dest_path.symlink_metadata() {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(DestinationState::Missing);
            }
            Err(e) => return Err(e.into()),
        };

        if metadata.file_type().is_symlink() {
            let link_target = fs::read_link(dest_path)?;
            if self.is_legacy_payload_symlink(base_dir, dest_path, &link_target) {
                return Ok(DestinationState::LegacySymlink);
            }
            bail!(
                "destination is a symlink to unexpected target: {}",
                self.destination
            );
        }

        if !metadata.is_file() {
            bail!(
                "destination exists but is not a regular file: {}",
                self.destination
            );
        }

        let contents = fs::read_to_string(dest_path)?;
        let parsed: Value = serde_json::from_str(&contents)?;
        if !parsed.is_object() {
            bail!(
                "destination JSON root must be an object: {}",
                self.destination
            );
        }
        Ok(DestinationState::Regular(parsed))
    }

    fn is_legacy_payload_symlink(
        &self,
        base_dir: &Path,
        dest_path: &Path,
        link_target: &Path,
    ) -> bool {
        let Some(source) = &self.legacy_payload_symlink_source else {
            return false;
        };

        let dest_dir = dest_path.parent().unwrap_or(Path::new("/"));
        let resolved = normalize_path(&dest_dir.join(link_target));
        let legacy_target = normalize_path(&base_dir.join(source));
        resolved == legacy_target
    }

    fn apply_operations(&self, root: &mut Value) -> Result<bool> {
        let mut changed = false;
        for operation in &self.operations {
            changed |= match operation {
                JsonEnsureOperation::StringsInArray { path, values } => {
                    ensure_strings_in_array(root, path, values)?
                }
                JsonEnsureOperation::Value { path, value } => {
                    ensure_value(root, path, value.clone())?
                }
                JsonEnsureOperation::ValueIfPathExists {
                    path,
                    condition_path,
                    value,
                } => {
                    if expand_tilde(condition_path)?.exists() {
                        ensure_value(root, path, value.clone())?
                    } else {
                        remove_path(root, path)?
                    }
                }
            };
        }
        Ok(changed)
    }
}

impl Feature for JsonEnsure {
    fn install(&self) -> Result<FeatureResult> {
        self.install_with_base_dir(&std::env::current_dir()?)
    }

    fn uninstall(&self) -> Result<FeatureResult> {
        debug!(destination = %self.destination, "json ensure uninstall is intentionally a no-op");
        Ok(FeatureResult::NoOp)
    }
}

fn to_path(path: &[&str]) -> Vec<String> {
    path.iter().map(|segment| (*segment).to_string()).collect()
}

fn write_pretty_json(path: &Path, value: &Value) -> Result<()> {
    let mut contents = serde_json::to_string_pretty(value)?;
    contents.push('\n');
    fs::write(path, contents)?;
    Ok(())
}

fn ensure_value(root: &mut Value, path: &[String], value: Value) -> Result<bool> {
    let target = get_or_create_path(root, path)?;
    if *target == value {
        return Ok(false);
    }
    *target = value;
    Ok(true)
}

fn ensure_strings_in_array(root: &mut Value, path: &[String], values: &[String]) -> Result<bool> {
    let target = get_or_create_path(root, path)?;

    if target.is_null() {
        *target = Value::Array(Vec::new());
    }

    let Some(array) = target.as_array_mut() else {
        bail!("managed path '{}' must be an array", display_path(path));
    };

    if array.iter().any(|entry| !entry.is_string()) {
        bail!(
            "managed path '{}' must contain only strings",
            display_path(path)
        );
    }

    let mut changed = false;
    for value in values {
        if array.iter().any(|entry| entry.as_str() == Some(value)) {
            continue;
        }
        array.push(Value::String(value.clone()));
        changed = true;
    }

    Ok(changed)
}

fn remove_path(root: &mut Value, path: &[String]) -> Result<bool> {
    let Some((last, parents)) = path.split_last() else {
        bail!("managed path cannot be empty");
    };

    let mut current = root;
    for segment in parents {
        let Some(object) = current.as_object_mut() else {
            bail!(
                "managed path '{}' crosses a non-object value",
                display_path(path)
            );
        };

        let Some(next) = object.get_mut(segment) else {
            return Ok(false);
        };
        current = next;
    }

    let Some(object) = current.as_object_mut() else {
        bail!(
            "managed path '{}' crosses a non-object value",
            display_path(path)
        );
    };
    Ok(object.remove(last).is_some())
}

fn get_or_create_path<'a>(root: &'a mut Value, path: &[String]) -> Result<&'a mut Value> {
    if path.is_empty() {
        bail!("managed path cannot be empty");
    }

    let mut current = root;
    for (index, segment) in path.iter().enumerate() {
        let is_last = index + 1 == path.len();
        let Some(object) = current.as_object_mut() else {
            bail!(
                "managed path '{}' crosses a non-object value",
                display_path(&path[..=index])
            );
        };

        current = object.entry(segment.clone()).or_insert_with(|| {
            if is_last {
                Value::Null
            } else {
                Value::Object(Map::new())
            }
        });
    }
    Ok(current)
}

fn display_path(path: &[String]) -> String {
    path.join(".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::fs::compute_relative_path;
    use crate::util::testing::TestContext;
    use std::os::unix::fs::symlink;

    #[test]
    fn install_creates_new_regular_file_with_managed_subset() {
        let ctx = TestContext::new();
        let dest_str = ctx.dest_path_str("settings.json");

        let feature = JsonEnsure::new(dest_str)
            .ensure_strings_in_array(&["permissions", "allow"], ["A", "B"])
            .ensure_value(&["sandbox", "enabled"], Value::Bool(true));

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);

        let written: Value =
            serde_json::from_str(&fs::read_to_string(ctx.dest_path("settings.json")).unwrap())
                .unwrap();
        assert_eq!(
            written["permissions"]["allow"],
            serde_json::json!(["A", "B"])
        );
        assert_eq!(written["sandbox"]["enabled"], Value::Bool(true));
    }

    #[test]
    fn install_merges_existing_regular_file_and_preserves_unmanaged_keys() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(
            &dest,
            r#"{
  "permissions": { "allow": ["A"], "deny": ["Z"] },
  "sandbox": { "autoAllowBashIfSandboxed": true },
  "hooks": { "SessionStart": [] }
}"#,
        )
        .unwrap();

        let feature = JsonEnsure::new(ctx.dest_path_str("settings.json"))
            .ensure_strings_in_array(&["permissions", "allow"], ["A", "B"])
            .ensure_value(&["sandbox", "enabled"], Value::Bool(true));

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);

        let written: Value = serde_json::from_str(&fs::read_to_string(dest).unwrap()).unwrap();
        assert_eq!(
            written["permissions"]["allow"],
            serde_json::json!(["A", "B"])
        );
        assert_eq!(written["permissions"]["deny"], serde_json::json!(["Z"]));
        assert_eq!(
            written["sandbox"]["autoAllowBashIfSandboxed"],
            Value::Bool(true)
        );
        assert_eq!(written["sandbox"]["enabled"], Value::Bool(true));
        assert_eq!(written["hooks"]["SessionStart"], serde_json::json!([]));
    }

    #[test]
    fn install_removes_conditional_value_when_dependency_path_is_missing() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(
            &dest,
            r#"{
  "statusLine": { "type": "command", "command": "~/bin/claude-statusline.sh" }
}"#,
        )
        .unwrap();
        let missing_script = ctx.source_dir.path().join("missing-statusline.sh");

        let feature = JsonEnsure::new(ctx.dest_path_str("settings.json"))
            .ensure_value_if_path_exists(
                &["statusLine"],
                missing_script.to_string_lossy().into_owned(),
                serde_json::json!({ "type": "command", "command": "~/bin/claude-statusline.sh" }),
            );

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);

        let written: Value = serde_json::from_str(&fs::read_to_string(dest).unwrap()).unwrap();
        assert!(written.get("statusLine").is_none());
    }

    #[test]
    fn install_replaces_legacy_symlink_with_regular_file() {
        let ctx = TestContext::new();
        let legacy_source = ctx.base_dir().join("payload/dot_claude/settings.json");
        let dest = ctx.dest_path("settings.json");
        let dest_dir = dest.parent().unwrap();
        let relative_target = compute_relative_path(dest_dir, &legacy_source);
        symlink(&relative_target, &dest).unwrap();

        let feature = JsonEnsure::new(ctx.dest_path_str("settings.json"))
            .legacy_payload_symlink_source("payload/dot_claude/settings.json")
            .ensure_value(&["sandbox", "enabled"], Value::Bool(true));

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);
        assert!(!dest.is_symlink());

        let written: Value = serde_json::from_str(&fs::read_to_string(dest).unwrap()).unwrap();
        assert_eq!(
            written,
            serde_json::json!({ "sandbox": { "enabled": true } })
        );
    }

    #[test]
    fn install_errors_for_unexpected_symlink() {
        let ctx = TestContext::new();
        let external = ctx.create_source_file("elsewhere.json", "{}");
        let dest = ctx.dest_path("settings.json");
        symlink(&external, &dest).unwrap();

        let feature = JsonEnsure::new(ctx.dest_path_str("settings.json"))
            .legacy_payload_symlink_source("payload/dot_claude/settings.json")
            .ensure_value(&["sandbox", "enabled"], Value::Bool(true));

        let err = feature.install_with_base_dir(ctx.base_dir()).unwrap_err();
        assert!(err.to_string().contains("unexpected target"));
    }

    #[test]
    fn install_errors_for_invalid_json() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("settings.json"), "{ not json").unwrap();

        let feature = JsonEnsure::new(ctx.dest_path_str("settings.json"))
            .ensure_value(&["sandbox", "enabled"], Value::Bool(true));

        assert!(feature.install_with_base_dir(ctx.base_dir()).is_err());
    }

    #[test]
    fn install_errors_for_non_object_root() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("settings.json"), "[]").unwrap();

        let feature = JsonEnsure::new(ctx.dest_path_str("settings.json"))
            .ensure_value(&["sandbox", "enabled"], Value::Bool(true));

        let err = feature.install_with_base_dir(ctx.base_dir()).unwrap_err();
        assert!(err.to_string().contains("root must be an object"));
    }

    #[test]
    fn install_errors_for_non_string_permissions_entries() {
        let ctx = TestContext::new();
        fs::write(
            ctx.dest_path("settings.json"),
            r#"{"permissions":{"allow":["A", 1]}}"#,
        )
        .unwrap();

        let feature = JsonEnsure::new(ctx.dest_path_str("settings.json"))
            .ensure_strings_in_array(&["permissions", "allow"], ["B"]);

        let err = feature.install_with_base_dir(ctx.base_dir()).unwrap_err();
        assert!(err.to_string().contains("contain only strings"));
    }

    #[test]
    fn install_errors_for_non_object_path_prefix() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("settings.json"), r#"{"sandbox":true}"#).unwrap();

        let feature = JsonEnsure::new(ctx.dest_path_str("settings.json"))
            .ensure_value(&["sandbox", "enabled"], Value::Bool(true));

        let err = feature.install_with_base_dir(ctx.base_dir()).unwrap_err();
        assert!(err.to_string().contains("crosses a non-object value"));
    }

    #[test]
    fn install_errors_for_non_array_permissions_allow() {
        let ctx = TestContext::new();
        fs::write(
            ctx.dest_path("settings.json"),
            r#"{"permissions":{"allow":"Bash(cargo build)"}}"#,
        )
        .unwrap();

        let feature = JsonEnsure::new(ctx.dest_path_str("settings.json"))
            .ensure_strings_in_array(&["permissions", "allow"], ["Bash(cargo fmt:*)"]);

        let err = feature.install_with_base_dir(ctx.base_dir()).unwrap_err();
        assert!(err.to_string().contains("must be an array"));
    }

    #[test]
    fn install_is_noop_when_file_already_satisfies_managed_subset() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(
            &dest,
            r#"{
  "permissions": { "allow": ["A", "B"], "deny": ["Z"] },
  "sandbox": { "enabled": true, "autoAllowBashIfSandboxed": true }
}"#,
        )
        .unwrap();
        let before = fs::read_to_string(&dest).unwrap();

        let feature = JsonEnsure::new(ctx.dest_path_str("settings.json"))
            .ensure_strings_in_array(&["permissions", "allow"], ["A", "B"])
            .ensure_value(&["sandbox", "enabled"], Value::Bool(true));

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::NoOp);
        assert_eq!(fs::read_to_string(dest).unwrap(), before);
    }

    #[test]
    fn uninstall_is_a_noop() {
        let ctx = TestContext::new();
        let feature = JsonEnsure::new(ctx.dest_path_str("settings.json"))
            .ensure_value(&["sandbox", "enabled"], Value::Bool(true));

        let result = feature.uninstall().unwrap();
        assert_eq!(result, FeatureResult::NoOp);
    }
}

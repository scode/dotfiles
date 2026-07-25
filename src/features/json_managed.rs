use std::fmt;
use std::fs;
use std::path::Path;

use anyhow::{Result, bail};
use serde_json::{Map, Value};
use tracing::debug;

use super::{Feature, FeatureResult};
use crate::util::fs::{expand_tilde, normalize_path, write_file_atomically};

/// Manages an installer-owned subset of values inside a user-owned JSON file.
///
/// This feature exists for configs where symlinking a whole blob creates more
/// churn than value. The file belongs to the user; only the JSON paths and
/// values named by this feature's operations belong to the installer. Paths
/// not named by any operation are never touched, on install or uninstall,
/// with one carve-out: uninstall prunes parent containers that removing a
/// managed value leaves empty, since install materialized them.
///
/// Ownership contract for `managed_*` operations: declaring a path managed is
/// a claim that whatever lives there is installer state. Install writes the
/// managed value unconditionally — pre-existing user state at a managed path
/// is fair game and gets clobbered, unrecoverably (there is no pre-install
/// journal). Uninstall removes the managed footprint again: a managed value is
/// deleted when it still matches what install would write, and containers the
/// removal empties are pruned. A managed value the user has since edited is
/// treated as reclaimed by the user and left alone; that includes type
/// changes along the path (say, a scalar where install would create an
/// object), which uninstall treats as reclaimed rather than as errors.
/// Uninstall never deletes a regular file, even when nothing but `{}`
/// remains.
///
/// The `remove_*` operations are the opposite family: one-way cleanup of
/// values that older installer versions wrote. They run on install only and
/// are never inverted — uninstall must not resurrect retired values.
///
/// Install behavior:
/// - Missing destination: create a new regular JSON object file.
/// - Existing regular file: parse it, validate the managed paths, then merge.
/// - Legacy installer symlink: replace it with a regular JSON file built from
///   the managed subset.
///
/// This does not try to "fix" malformed JSON or incompatible types. If the
/// file cannot be merged safely, install fails instead of guessing, and both
/// install and uninstall fail on whole-file problems (malformed JSON, a
/// non-object root, an unrecognized symlink).
#[derive(Debug)]
pub struct JsonManaged {
    destination: String,
    operations: Vec<JsonManagedOperation>,
    legacy_payload_symlink_source: Option<String>,
}

#[derive(Debug)]
enum DestinationState {
    Missing,
    Regular(Value),
    LegacySymlink,
}

/// One declared operation. The `Managed*` variants carry the ownership claim
/// described on [`JsonManaged`] and are inverted on uninstall; the `*Absent*`
/// variants are one-way install-time cleanup.
#[derive(Debug)]
enum JsonManagedOperation {
    ManagedStringsInArray {
        path: Vec<String>,
        values: Vec<String>,
    },
    StringsAbsentFromArray {
        path: Vec<String>,
        values: Vec<String>,
    },
    ManagedValue {
        path: Vec<String>,
        value: Value,
    },
    ValuesAbsentFromArray {
        path: Vec<String>,
        values: Vec<Value>,
    },
    ValueAbsentIfEquals {
        path: Vec<String>,
        value: Value,
    },
    ManagedValueIfPathExists {
        path: Vec<String>,
        condition_path: String,
        value: Value,
    },
}

impl fmt::Display for JsonManaged {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "json managed: {}", self.destination)
    }
}

impl JsonManaged {
    /// Creates a managed-JSON feature for the destination file.
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

    /// Claims ownership of the given strings inside an array path.
    ///
    /// Install adds any that are missing, leaving neighboring user-added
    /// strings alone. Uninstall removes exactly these strings and, when that
    /// leaves the array (and then its parent objects) empty, prunes those
    /// containers too. A string absent at uninstall time means the user
    /// already removed it; that is respected, including the case where the
    /// user emptied the whole array — an already-empty array is theirs and is
    /// not pruned.
    pub fn managed_strings_in_array(
        mut self,
        path: &[&str],
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.operations
            .push(JsonManagedOperation::ManagedStringsInArray {
                path: to_path(path),
                values: values.into_iter().map(Into::into).collect(),
            });
        self
    }

    /// Removes the given strings from an existing array path, on install only.
    ///
    /// Missing paths are left alone. Existing paths still have to be arrays of
    /// strings, because this operation is meant for retiring values from a
    /// string list the installer has previously managed. This is one-way
    /// cleanup: uninstall never re-adds the removed values.
    pub fn remove_strings_from_array(
        mut self,
        path: &[&str],
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.operations
            .push(JsonManagedOperation::StringsAbsentFromArray {
                path: to_path(path),
                values: values.into_iter().map(Into::into).collect(),
            });
        self
    }

    /// Claims ownership of a path, enforcing the exact JSON value provided.
    ///
    /// Install writes the value unconditionally: a pre-existing user value at
    /// this path is clobbered and cannot be restored later. Uninstall removes
    /// the path only when it still equals this value; a user who edited the
    /// managed value has reclaimed the path, and uninstall leaves it alone.
    /// Parent objects emptied by the removal are pruned.
    pub fn managed_value(mut self, path: &[&str], value: Value) -> Self {
        self.operations.push(JsonManagedOperation::ManagedValue {
            path: to_path(path),
            value,
        });
        self
    }

    /// Removes exact JSON values from an existing array path, on install only.
    ///
    /// This is deliberately exact-match only. It is for removing values that an
    /// older installer version wrote verbatim, while leaving nearby user-added
    /// values in the same array untouched. This is one-way cleanup: uninstall
    /// never re-adds the removed values.
    pub fn remove_values_from_array(
        mut self,
        path: &[&str],
        values: impl IntoIterator<Item = Value>,
    ) -> Self {
        self.operations
            .push(JsonManagedOperation::ValuesAbsentFromArray {
                path: to_path(path),
                values: values.into_iter().collect(),
            });
        self
    }

    /// Removes a path only when its current value matches exactly.
    ///
    /// This gives cleanup operations a safe pruning step: empty containers left
    /// behind by a known removal can be dropped without deleting user-owned
    /// values that happen to live at the same path.
    pub fn remove_value_if_equals(mut self, path: &[&str], value: Value) -> Self {
        self.operations
            .push(JsonManagedOperation::ValueAbsentIfEquals {
                path: to_path(path),
                value,
            });
        self
    }

    /// Like [`Self::managed_value`], but install keys off a filesystem path.
    ///
    /// When the condition path exists, install enforces the value; when it
    /// does not, install removes it. This is for JSON that points at another
    /// installed artifact: the config should not keep advertising a command
    /// path that is absent. Uninstall ignores the condition entirely and
    /// behaves exactly like `managed_value` — the value is removed if it
    /// still matches, whether or not the artifact is installed.
    pub fn managed_value_if_path_exists(
        mut self,
        path: &[&str],
        condition_path: impl Into<String>,
        value: Value,
    ) -> Self {
        self.operations
            .push(JsonManagedOperation::ManagedValueIfPathExists {
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

        // A legacy symlink needs no removal step: write_file_atomically renames
        // over it without following it, so the migration to a regular file is a
        // single atomic replacement. An explicit remove-then-write here would
        // reopen an ENOENT window where the settings file does not exist at all.
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

    /// Removes the managed footprint from the destination file.
    ///
    /// Only a regular file is edited. A missing destination is a no-op. A
    /// legacy installer symlink is removed outright: it is repo-owned state,
    /// and SPEC.md's symlink-migration contract says uninstall removes
    /// repo-owned symlinks even when they point at previous source paths —
    /// leaving it would strand a dangling link at a payload path the repo has
    /// deleted. An unexpected symlink is an error, same as on install. A
    /// regular file is never deleted, even when reverting leaves nothing but
    /// an empty object.
    fn uninstall_with_base_dir(&self, base_dir: &Path) -> Result<FeatureResult> {
        let dest_path = expand_tilde(&self.destination)?;
        let mut root = match self.load_destination_state(base_dir, &dest_path)? {
            DestinationState::Missing => return Ok(FeatureResult::NoOp),
            DestinationState::LegacySymlink => {
                fs::remove_file(&dest_path)?;
                debug!(
                    destination = %self.destination,
                    "removed legacy repo-owned settings symlink"
                );
                return Ok(FeatureResult::Changed);
            }
            DestinationState::Regular(existing) => existing,
        };

        if !self.revert_operations(&mut root)? {
            debug!(destination = %self.destination, "no managed values present to remove");
            return Ok(FeatureResult::NoOp);
        }

        write_pretty_json(&dest_path, &root)?;
        debug!(destination = %self.destination, "removed managed json subset");
        Ok(FeatureResult::Changed)
    }

    fn apply_operations(&self, root: &mut Value) -> Result<bool> {
        let mut changed = false;
        for operation in &self.operations {
            changed |= match operation {
                JsonManagedOperation::ManagedStringsInArray { path, values } => {
                    managed_strings_in_array(root, path, values)?
                }
                JsonManagedOperation::StringsAbsentFromArray { path, values } => {
                    remove_strings_from_array(root, path, values)?
                }
                JsonManagedOperation::ManagedValue { path, value } => {
                    managed_value(root, path, value.clone())?
                }
                JsonManagedOperation::ValuesAbsentFromArray { path, values } => {
                    remove_values_from_array(root, path, values)?
                }
                JsonManagedOperation::ValueAbsentIfEquals { path, value } => {
                    remove_value_if_equals(root, path, value)?
                }
                JsonManagedOperation::ManagedValueIfPathExists {
                    path,
                    condition_path,
                    value,
                } => {
                    if expand_tilde(condition_path)?.exists() {
                        managed_value(root, path, value.clone())?
                    } else {
                        remove_path(root, path)?
                    }
                }
            };
        }
        Ok(changed)
    }

    /// Applies the uninstall inverse of every operation, in reverse
    /// declaration order.
    ///
    /// Only the `Managed*` operations have inverses. The `*Absent*` cleanup
    /// operations are one-way by contract: reverting them would resurrect
    /// values that were retired on purpose. `ManagedValueIfPathExists`
    /// deliberately ignores its condition here — uninstall removes the owned
    /// value whether or not the artifact it points at is still installed.
    fn revert_operations(&self, root: &mut Value) -> Result<bool> {
        let mut changed = false;
        for operation in self.operations.iter().rev() {
            changed |= match operation {
                JsonManagedOperation::ManagedStringsInArray { path, values } => {
                    uninstall_managed_strings_in_array(root, path, values)?
                }
                JsonManagedOperation::ManagedValue { path, value }
                | JsonManagedOperation::ManagedValueIfPathExists { path, value, .. } => {
                    uninstall_managed_value(root, path, value)?
                }
                JsonManagedOperation::StringsAbsentFromArray { .. }
                | JsonManagedOperation::ValuesAbsentFromArray { .. }
                | JsonManagedOperation::ValueAbsentIfEquals { .. } => false,
            };
        }
        Ok(changed)
    }
}

impl Feature for JsonManaged {
    fn install(&self) -> Result<FeatureResult> {
        self.install_with_base_dir(&std::env::current_dir()?)
    }

    fn uninstall(&self) -> Result<FeatureResult> {
        self.uninstall_with_base_dir(&std::env::current_dir()?)
    }
}

fn to_path(path: &[&str]) -> Vec<String> {
    path.iter().map(|segment| (*segment).to_string()).collect()
}

/// Writes the merged document, replacing the destination atomically.
///
/// A settings file is read by tools on their own schedule rather than by this
/// process, so a truncated write is discovered later, by something else, as
/// malformed JSON. Replacing by rename also preserves the mode the user's file
/// already had.
fn write_pretty_json(path: &Path, value: &Value) -> Result<()> {
    let mut contents = serde_json::to_string_pretty(value)?;
    contents.push('\n');
    write_file_atomically(path, contents.as_bytes())
}

fn managed_value(root: &mut Value, path: &[String], value: Value) -> Result<bool> {
    let target = get_or_create_path(root, path)?;
    if *target == value {
        return Ok(false);
    }
    *target = value;
    Ok(true)
}

fn managed_strings_in_array(root: &mut Value, path: &[String], values: &[String]) -> Result<bool> {
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

fn remove_strings_from_array(root: &mut Value, path: &[String], values: &[String]) -> Result<bool> {
    let Some(target) = get_existing_path_mut(root, path)? else {
        return Ok(false);
    };

    let Some(array) = target.as_array_mut() else {
        bail!("managed path '{}' must be an array", display_path(path));
    };

    if array.iter().any(|entry| !entry.is_string()) {
        bail!(
            "managed path '{}' must contain only strings",
            display_path(path)
        );
    }

    let before = array.len();
    array.retain(|entry| !values.iter().any(|value| entry.as_str() == Some(value)));
    Ok(array.len() != before)
}

fn remove_values_from_array(root: &mut Value, path: &[String], values: &[Value]) -> Result<bool> {
    let Some(target) = get_existing_path_mut(root, path)? else {
        return Ok(false);
    };

    let Some(array) = target.as_array_mut() else {
        bail!("managed path '{}' must be an array", display_path(path));
    };

    let before = array.len();
    array.retain(|entry| !values.iter().any(|value| value == entry));
    Ok(array.len() != before)
}

/// Uninstall inverse of [`managed_strings_in_array`].
///
/// Removes exactly the owned strings. Containers are pruned only when this
/// removal is what emptied them: an array that was already empty (or already
/// missing the owned strings) reflects user edits and is left alone. Shape
/// violations — a non-array at the managed path, a non-string entry in the
/// array, a scalar along the path — are user edits too, so they count as the
/// user reclaiming the path and produce a no-op, not an error.
fn uninstall_managed_strings_in_array(
    root: &mut Value,
    path: &[String],
    values: &[String],
) -> Result<bool> {
    let Some(target) = get_reclaimable_path_mut(root, path)? else {
        return Ok(false);
    };
    let Some(array) = target.as_array_mut() else {
        return Ok(false);
    };
    if array.iter().any(|entry| !entry.is_string()) {
        return Ok(false);
    }

    let before = array.len();
    array.retain(|entry| !values.iter().any(|value| entry.as_str() == Some(value)));
    if array.len() == before {
        return Ok(false);
    }

    if array.is_empty() {
        remove_path(root, path)?;
        prune_empty_ancestors(root, path)?;
    }
    Ok(true)
}

/// Uninstall inverse of [`managed_value`] (and its path-conditioned variant).
///
/// The equality guard is the ownership escape hatch: a managed value the user
/// edited no longer matches, is treated as reclaimed, and survives uninstall.
/// A type change along the path is an edit like any other and no-ops the same
/// way.
fn uninstall_managed_value(root: &mut Value, path: &[String], value: &Value) -> Result<bool> {
    if !matches!(
        get_reclaimable_path_mut(root, path)?,
        Some(existing) if *existing == *value
    ) {
        return Ok(false);
    }
    remove_path(root, path)?;
    prune_empty_ancestors(root, path)?;
    Ok(true)
}

/// Removes parent objects left empty after an owned leaf was removed.
///
/// Walks the leaf's ancestors from the inside out and stops at the first
/// non-empty one, since a container still holding user keys (or other managed
/// values) must survive. Only exactly-empty objects are pruned, so this can
/// never delete data — it only reverses the container creation that install's
/// path materialization performed.
fn prune_empty_ancestors(root: &mut Value, path: &[String]) -> Result<()> {
    for len in (1..path.len()).rev() {
        let prefix = &path[..len];
        let is_empty_object = matches!(
            get_existing_path_mut(root, prefix)?,
            Some(Value::Object(object)) if object.is_empty()
        );
        if !is_empty_object {
            break;
        }
        remove_path(root, prefix)?;
    }
    Ok(())
}

/// Uninstall-flavored variant of [`get_existing_path_mut`].
///
/// Where install's strict walk errors on an intermediate non-object ("crosses
/// a non-object value"), this reports the path absent instead: a user type
/// change along a managed path means the managed value no longer exists in
/// the shape install manages, and uninstall treats that as the user
/// reclaiming the path. Install keeps the strict variant because it has to be
/// able to write; uninstall has nothing to remove from a reshaped path.
fn get_reclaimable_path_mut<'a>(
    root: &'a mut Value,
    path: &[String],
) -> Result<Option<&'a mut Value>> {
    if path.is_empty() {
        bail!("managed path cannot be empty");
    }

    let mut current = root;
    for segment in path {
        let Some(object) = current.as_object_mut() else {
            return Ok(None);
        };
        let Some(next) = object.get_mut(segment) else {
            return Ok(None);
        };
        current = next;
    }
    Ok(Some(current))
}

fn remove_value_if_equals(root: &mut Value, path: &[String], value: &Value) -> Result<bool> {
    if !matches!(
        get_existing_path_mut(root, path)?,
        Some(existing) if *existing == *value
    ) {
        return Ok(false);
    }
    remove_path(root, path)
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

fn get_existing_path_mut<'a>(
    root: &'a mut Value,
    path: &[String],
) -> Result<Option<&'a mut Value>> {
    if path.is_empty() {
        bail!("managed path cannot be empty");
    }

    let mut current = root;
    for (index, segment) in path.iter().enumerate() {
        let Some(object) = current.as_object_mut() else {
            bail!(
                "managed path '{}' crosses a non-object value",
                display_path(&path[..=index])
            );
        };

        let Some(next) = object.get_mut(segment) else {
            return Ok(None);
        };
        current = next;
    }
    Ok(Some(current))
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

        let feature = JsonManaged::new(dest_str)
            .managed_strings_in_array(&["permissions", "allow"], ["A", "B"])
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));

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

    /// Merging into a user's settings file must not change its permissions.
    ///
    /// Worth testing at this level rather than only on the write helper: this
    /// file can hold credentials and hook commands, so a user who tightened its
    /// mode meant it, and the wiring is what decides whether that survives.
    #[test]
    fn install_preserves_the_destination_file_mode() {
        use std::os::unix::fs::PermissionsExt;

        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(&dest, "{}").unwrap();
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o600)).unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));
        feature.install_with_base_dir(ctx.base_dir()).unwrap();

        let mode = fs::metadata(&dest).unwrap().permissions().mode() & 0o7777;
        assert_eq!(mode, 0o600);
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

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_strings_in_array(&["permissions", "allow"], ["A", "B"])
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));

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
    fn install_removes_obsolete_strings_from_existing_array() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(
            &dest,
            r#"{
  "permissions": { "allow": ["A", "B", "C"] }
}"#,
        )
        .unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .remove_strings_from_array(&["permissions", "allow"], ["B", "missing"]);

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);

        let written: Value = serde_json::from_str(&fs::read_to_string(dest).unwrap()).unwrap();
        assert_eq!(
            written["permissions"]["allow"],
            serde_json::json!(["A", "C"])
        );
    }

    #[test]
    fn install_removes_exact_array_values_and_only_prunes_expected_empty_paths() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(
            &dest,
            r#"{
  "hooks": {
    "SessionStart": [
      { "hooks": [{ "type": "command", "command": "old" }] },
      { "hooks": [{ "type": "command", "command": "keep" }] }
    ],
    "SessionEnd": [
      { "hooks": [{ "type": "command", "command": "old-end" }] }
    ]
  }
}"#,
        )
        .unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .remove_values_from_array(
                &["hooks", "SessionStart"],
                [serde_json::json!({ "hooks": [{ "type": "command", "command": "old" }] })],
            )
            .remove_values_from_array(
                &["hooks", "SessionEnd"],
                [serde_json::json!({ "hooks": [{ "type": "command", "command": "old-end" }] })],
            )
            .remove_value_if_equals(&["hooks", "SessionStart"], serde_json::json!([]))
            .remove_value_if_equals(&["hooks", "SessionEnd"], serde_json::json!([]))
            .remove_value_if_equals(&["hooks"], serde_json::json!({}));

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);

        let written: Value = serde_json::from_str(&fs::read_to_string(dest).unwrap()).unwrap();
        assert_eq!(
            written["hooks"]["SessionStart"],
            serde_json::json!([{ "hooks": [{ "type": "command", "command": "keep" }] }])
        );
        assert!(written["hooks"].get("SessionEnd").is_none());
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

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_value_if_path_exists(
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

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .legacy_payload_symlink_source("payload/dot_claude/settings.json")
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));

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

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .legacy_payload_symlink_source("payload/dot_claude/settings.json")
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));

        let err = feature.install_with_base_dir(ctx.base_dir()).unwrap_err();
        assert!(err.to_string().contains("unexpected target"));
    }

    #[test]
    fn install_errors_for_invalid_json() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("settings.json"), "{ not json").unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));

        assert!(feature.install_with_base_dir(ctx.base_dir()).is_err());
    }

    #[test]
    fn install_errors_for_non_object_root() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("settings.json"), "[]").unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));

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

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_strings_in_array(&["permissions", "allow"], ["B"]);

        let err = feature.install_with_base_dir(ctx.base_dir()).unwrap_err();
        assert!(err.to_string().contains("contain only strings"));
    }

    #[test]
    fn install_errors_for_non_object_path_prefix() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("settings.json"), r#"{"sandbox":true}"#).unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));

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

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_strings_in_array(&["permissions", "allow"], ["Bash(cargo fmt:*)"]);

        let err = feature.install_with_base_dir(ctx.base_dir()).unwrap_err();
        assert!(err.to_string().contains("must be an array"));
    }

    #[test]
    fn install_errors_when_string_removal_target_is_not_an_array() {
        let ctx = TestContext::new();
        fs::write(
            ctx.dest_path("settings.json"),
            r#"{"permissions":{"allow":"A"}}"#,
        )
        .unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .remove_strings_from_array(&["permissions", "allow"], ["A"]);

        let err = feature.install_with_base_dir(ctx.base_dir()).unwrap_err();
        assert!(err.to_string().contains("must be an array"));
    }

    #[test]
    fn install_errors_when_string_removal_target_has_non_string_entries() {
        let ctx = TestContext::new();
        fs::write(
            ctx.dest_path("settings.json"),
            r#"{"permissions":{"allow":["A", 1]}}"#,
        )
        .unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .remove_strings_from_array(&["permissions", "allow"], ["A"]);

        let err = feature.install_with_base_dir(ctx.base_dir()).unwrap_err();
        assert!(err.to_string().contains("contain only strings"));
    }

    #[test]
    fn install_errors_when_value_removal_target_is_not_an_array() {
        let ctx = TestContext::new();
        fs::write(
            ctx.dest_path("settings.json"),
            r#"{"hooks":{"SessionStart":{}}}"#,
        )
        .unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .remove_values_from_array(&["hooks", "SessionStart"], [serde_json::json!({})]);

        let err = feature.install_with_base_dir(ctx.base_dir()).unwrap_err();
        assert!(err.to_string().contains("must be an array"));
    }

    #[test]
    fn install_errors_when_array_removal_path_crosses_non_object_value() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("settings.json"), r#"{"hooks":true}"#).unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .remove_strings_from_array(&["hooks", "SessionStart"], ["x"]);

        let err = feature.install_with_base_dir(ctx.base_dir()).unwrap_err();
        assert!(err.to_string().contains("crosses a non-object value"));
    }

    #[test]
    fn install_errors_when_conditional_removal_path_crosses_non_object_value() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("settings.json"), r#"{"hooks":true}"#).unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .remove_value_if_equals(&["hooks", "SessionStart"], serde_json::json!([]));

        let err = feature.install_with_base_dir(ctx.base_dir()).unwrap_err();
        assert!(err.to_string().contains("crosses a non-object value"));
    }

    /// A false `Changed` from any removal operation would make every install
    /// rewrite the destination file, since main.rs applies the removals
    /// unconditionally. This pins the no-op contract for all three removal
    /// operations at once: missing paths, values already absent, and
    /// non-matching remove_value_if_equals targets.
    #[test]
    fn install_is_noop_when_removals_have_nothing_to_remove() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(
            &dest,
            r#"{
  "permissions": { "allow": ["A"] },
  "hooks": { "SessionStart": [{ "hooks": [{ "type": "command", "command": "keep" }] }] }
}"#,
        )
        .unwrap();
        let before = fs::read_to_string(&dest).unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .remove_strings_from_array(&["permissions", "allow"], ["absent"])
            .remove_strings_from_array(&["missing", "path"], ["absent"])
            .remove_values_from_array(
                &["hooks", "SessionStart"],
                [serde_json::json!({ "hooks": [{ "type": "command", "command": "old" }] })],
            )
            .remove_values_from_array(&["hooks", "SessionEnd"], [serde_json::json!({})])
            .remove_value_if_equals(&["hooks", "SessionStart"], serde_json::json!([]))
            .remove_value_if_equals(&["hooks"], serde_json::json!({}));

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::NoOp);
        assert_eq!(fs::read_to_string(dest).unwrap(), before);
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

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_strings_in_array(&["permissions", "allow"], ["A", "B"])
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::NoOp);
        assert_eq!(fs::read_to_string(dest).unwrap(), before);
    }

    /// Documents the ownership contract's sharpest edge: install overwrites a
    /// pre-existing user value at a managed path, and nothing records the old
    /// value. Declaring a path managed makes prior user state fair game.
    #[test]
    fn install_clobbers_preexisting_user_value_at_managed_path() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(&dest, r#"{"sandbox":{"enabled":false}}"#).unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);

        let written: Value = serde_json::from_str(&fs::read_to_string(dest).unwrap()).unwrap();
        assert_eq!(written["sandbox"]["enabled"], Value::Bool(true));
    }

    /// The core round trip: install onto a missing file, then uninstall. The
    /// managed footprint disappears entirely, but the file itself survives as
    /// an empty object — uninstall never deletes the user-owned file.
    #[test]
    fn uninstall_reverts_fresh_install_to_empty_object_but_keeps_file() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_strings_in_array(&["permissions", "allow"], ["A", "B"])
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));

        feature.install_with_base_dir(ctx.base_dir()).unwrap();
        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);

        assert!(dest.is_file());
        let written: Value = serde_json::from_str(&fs::read_to_string(dest).unwrap()).unwrap();
        assert_eq!(written, serde_json::json!({}));
    }

    #[test]
    fn uninstall_removes_owned_strings_and_preserves_unmanaged_keys() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(
            &dest,
            r#"{
  "permissions": { "allow": ["A", "B"] },
  "unrelated": { "key": 1 }
}"#,
        )
        .unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_strings_in_array(&["permissions", "allow"], ["A", "B"]);

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);

        let written: Value = serde_json::from_str(&fs::read_to_string(dest).unwrap()).unwrap();
        assert_eq!(written, serde_json::json!({ "unrelated": { "key": 1 } }));
    }

    #[test]
    fn uninstall_keeps_array_and_containers_when_user_strings_remain() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(&dest, r#"{"permissions":{"allow":["A", "user-added"]}}"#).unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_strings_in_array(&["permissions", "allow"], ["A"]);

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);

        let written: Value = serde_json::from_str(&fs::read_to_string(dest).unwrap()).unwrap();
        assert_eq!(
            written,
            serde_json::json!({ "permissions": { "allow": ["user-added"] } })
        );
    }

    #[test]
    fn uninstall_prunes_emptied_array_but_stops_at_containers_with_user_keys() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(&dest, r#"{"permissions":{"allow":["A"],"deny":["Z"]}}"#).unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_strings_in_array(&["permissions", "allow"], ["A"]);

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);

        let written: Value = serde_json::from_str(&fs::read_to_string(dest).unwrap()).unwrap();
        assert_eq!(
            written,
            serde_json::json!({ "permissions": { "deny": ["Z"] } })
        );
    }

    #[test]
    fn uninstall_removes_managed_value_and_prunes_empty_ancestor_chain() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(&dest, r#"{"a":{"b":{"c":true}},"keep":1}"#).unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_value(&["a", "b", "c"], Value::Bool(true));

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);

        let written: Value = serde_json::from_str(&fs::read_to_string(dest).unwrap()).unwrap();
        assert_eq!(written, serde_json::json!({ "keep": 1 }));
    }

    /// The equality guard: a managed value the user edited is treated as
    /// reclaimed by the user, and uninstall must not delete it.
    #[test]
    fn uninstall_leaves_edited_managed_value_alone() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(&dest, r#"{"sandbox":{"enabled":false}}"#).unwrap();
        let before = fs::read_to_string(&dest).unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::NoOp);
        assert_eq!(fs::read_to_string(dest).unwrap(), before);
    }

    /// An array the user already emptied reflects a user edit, not installer
    /// state: uninstall removed nothing from it, so it must not prune it.
    #[test]
    fn uninstall_leaves_user_emptied_array_alone() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(&dest, r#"{"permissions":{"allow":[]}}"#).unwrap();
        let before = fs::read_to_string(&dest).unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_strings_in_array(&["permissions", "allow"], ["A"]);

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::NoOp);
        assert_eq!(fs::read_to_string(dest).unwrap(), before);
    }

    /// One-way cleanup contract: retired values present in the file survive
    /// uninstall untouched (they only ever get removed by install), and their
    /// absence is never "fixed" by re-adding them.
    #[test]
    fn uninstall_never_inverts_cleanup_operations() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(
            &dest,
            r#"{
  "permissions": { "allow": ["retired-value"] },
  "hooks": { "SessionStart": [{ "hooks": [] }] }
}"#,
        )
        .unwrap();
        let before = fs::read_to_string(&dest).unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .remove_strings_from_array(&["permissions", "allow"], ["retired-value", "absent"])
            .remove_values_from_array(
                &["hooks", "SessionStart"],
                [serde_json::json!({ "hooks": [] })],
            )
            .remove_value_if_equals(&["hooks"], serde_json::json!({}));

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::NoOp);
        assert_eq!(fs::read_to_string(dest).unwrap(), before);
    }

    /// Uninstall of a conditional managed value ignores the condition: the
    /// owned value is removed even when the artifact it points at is gone.
    #[test]
    fn uninstall_removes_conditional_managed_value_despite_missing_condition_path() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(
            &dest,
            r#"{"statusLine":{"type":"command","command":"~/bin/statusline.sh"}}"#,
        )
        .unwrap();
        let missing_script = ctx.source_dir.path().join("missing-statusline.sh");

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_value_if_path_exists(
                &["statusLine"],
                missing_script.to_string_lossy().into_owned(),
                serde_json::json!({ "type": "command", "command": "~/bin/statusline.sh" }),
            );

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);

        let written: Value = serde_json::from_str(&fs::read_to_string(dest).unwrap()).unwrap();
        assert_eq!(written, serde_json::json!({}));
    }

    #[test]
    fn uninstall_is_noop_on_missing_file() {
        let ctx = TestContext::new();
        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::NoOp);
        assert!(!ctx.dest_path("settings.json").exists());
    }

    /// Mirrors the PayloadSymlink uninstall contract in SPEC.md: a legacy
    /// repo-owned settings symlink is installer state and gets removed, not
    /// stranded as a dangling link at the deleted payload path.
    #[test]
    fn uninstall_removes_legacy_symlink() {
        let ctx = TestContext::new();
        let legacy_source = ctx.base_dir().join("payload/dot_claude/settings.json");
        let dest = ctx.dest_path("settings.json");
        let dest_dir = dest.parent().unwrap();
        let relative_target = compute_relative_path(dest_dir, &legacy_source);
        symlink(&relative_target, &dest).unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .legacy_payload_symlink_source("payload/dot_claude/settings.json")
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);
        assert!(
            !dest.exists() && !dest.is_symlink(),
            "legacy symlink must be removed, and no file recreated"
        );
    }

    /// A type change along a managed path is a user edit: uninstall must
    /// treat it as the user reclaiming the path (no-op), not fail the way
    /// install's strict path walk would.
    #[test]
    fn uninstall_treats_type_changed_managed_path_as_reclaimed() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(&dest, r#"{"sandbox":true,"permissions":"user-string"}"#).unwrap();
        let before = fs::read_to_string(&dest).unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_strings_in_array(&["permissions", "allow"], ["A"])
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::NoOp);
        assert_eq!(fs::read_to_string(dest).unwrap(), before);
    }

    /// A non-array value sitting at the exact managed array path is a user
    /// edit too: uninstall reclaims it (no-op) where install would error.
    #[test]
    fn uninstall_treats_non_array_at_managed_array_path_as_reclaimed() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(&dest, r#"{"permissions":{"allow":"user-string"}}"#).unwrap();
        let before = fs::read_to_string(&dest).unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_strings_in_array(&["permissions", "allow"], ["A"]);

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::NoOp);
        assert_eq!(fs::read_to_string(dest).unwrap(), before);
    }

    /// Whole-file problems are outside the reclaimed-not-failing rule: a
    /// non-object root fails uninstall exactly as it fails install.
    #[test]
    fn uninstall_errors_for_non_object_root() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("settings.json"), "[]").unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));

        let err = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap_err();
        assert!(err.to_string().contains("root must be an object"));
    }

    /// A non-string entry in a managed string array is likewise a user edit;
    /// the whole array counts as reclaimed and survives untouched, owned
    /// strings included.
    #[test]
    fn uninstall_treats_non_string_entries_in_managed_array_as_reclaimed() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(&dest, r#"{"permissions":{"allow":["A", 1]}}"#).unwrap();
        let before = fs::read_to_string(&dest).unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_strings_in_array(&["permissions", "allow"], ["A"]);

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::NoOp);
        assert_eq!(fs::read_to_string(dest).unwrap(), before);
    }

    /// Reverse declaration order is load-bearing for nested managed paths:
    /// the inner op must revert first so the outer value matches its managed
    /// shape again. Forward order would leave `{"a":{"x":1}}` behind because
    /// the outer equality guard sees the not-yet-reverted inner value.
    #[test]
    fn uninstall_reverts_nested_managed_paths_in_reverse_declaration_order() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_value(&["a"], serde_json::json!({ "x": 1 }))
            .managed_strings_in_array(&["a", "b"], ["S"]);

        feature.install_with_base_dir(ctx.base_dir()).unwrap();
        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);

        let written: Value = serde_json::from_str(&fs::read_to_string(dest).unwrap()).unwrap();
        assert_eq!(written, serde_json::json!({}));
    }

    /// Pruning also triggers when only part of the owned footprint is left:
    /// removing the one remaining owned string still empties the array, and
    /// the emptied containers go with it.
    #[test]
    fn uninstall_prunes_array_emptied_by_partial_footprint_removal() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(&dest, r#"{"permissions":{"allow":["A"]}}"#).unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_strings_in_array(&["permissions", "allow"], ["A", "B"]);

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);

        let written: Value = serde_json::from_str(&fs::read_to_string(dest).unwrap()).unwrap();
        assert_eq!(written, serde_json::json!({}));
    }

    /// The other half of "uninstall ignores the condition": the owned value
    /// is removed even when the artifact it points at is still installed.
    #[test]
    fn uninstall_removes_conditional_managed_value_when_condition_path_exists() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");
        fs::write(
            &dest,
            r#"{"statusLine":{"type":"command","command":"~/bin/statusline.sh"}}"#,
        )
        .unwrap();
        let present_script = ctx.create_source_file("statusline.sh", "#!/bin/sh\n");

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_value_if_path_exists(
                &["statusLine"],
                present_script.to_string_lossy().into_owned(),
                serde_json::json!({ "type": "command", "command": "~/bin/statusline.sh" }),
            );

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);

        let written: Value = serde_json::from_str(&fs::read_to_string(dest).unwrap()).unwrap();
        assert_eq!(written, serde_json::json!({}));
    }

    #[test]
    fn uninstall_errors_for_unexpected_symlink() {
        let ctx = TestContext::new();
        let external = ctx.create_source_file("elsewhere.json", "{}");
        let dest = ctx.dest_path("settings.json");
        symlink(&external, &dest).unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .legacy_payload_symlink_source("payload/dot_claude/settings.json")
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));

        let err = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap_err();
        assert!(err.to_string().contains("unexpected target"));
    }

    #[test]
    fn uninstall_errors_for_invalid_json() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("settings.json"), "{ not json").unwrap();

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));

        assert!(feature.uninstall_with_base_dir(ctx.base_dir()).is_err());
    }

    #[test]
    fn uninstall_is_idempotent() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("settings.json");

        let feature = JsonManaged::new(ctx.dest_path_str("settings.json"))
            .managed_strings_in_array(&["permissions", "allow"], ["A"])
            .managed_value(&["sandbox", "enabled"], Value::Bool(true));

        feature.install_with_base_dir(ctx.base_dir()).unwrap();
        let first = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(first, FeatureResult::Changed);
        let after_first = fs::read_to_string(&dest).unwrap();

        let second = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(second, FeatureResult::NoOp);
        assert_eq!(fs::read_to_string(dest).unwrap(), after_first);
    }
}

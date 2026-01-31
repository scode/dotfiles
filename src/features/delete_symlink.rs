use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Result, bail};
use tracing::debug;

use super::{Feature, FeatureResult};
use crate::util::fs::expand_tilde;

/// Lexically normalizes a path by resolving `.` and `..` components without
/// filesystem access. This is useful for checking path containment when the
/// target may not exist.
fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            c => result.push(c),
        }
    }
    result
}

/// A feature that deletes a symlink if it exists and points to a file within
/// the payload directory.
///
/// This is useful for cleaning up old symlinks that are no longer needed.
/// The deletion only proceeds if:
/// - The path exists and is a symlink
/// - The symlink target resolves (or lexically normalizes, for broken symlinks)
///   to a path within the payload directory
///
/// If the path does not exist, `install()` returns `NoOp` (idempotent).
/// If the path exists but is not a symlink, or points outside payload, an error is returned.
///
/// `uninstall()` is always a no-op since we cannot restore a deleted symlink.
#[derive(Debug)]
pub struct DeleteSymlink {
    path: String,
}

impl fmt::Display for DeleteSymlink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "delete symlink: {}", self.path)
    }
}

impl DeleteSymlink {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    fn install_with_base_dir(&self, base_dir: &Path) -> Result<FeatureResult> {
        let target_path = expand_tilde(&self.path)?;
        debug!(path = %self.path, "checking symlink for deletion");

        // If path doesn't exist, nothing to do
        if !target_path.exists() && target_path.symlink_metadata().is_err() {
            debug!(path = %self.path, "path does not exist, nothing to delete");
            return Ok(FeatureResult::NoOp);
        }

        // Check if it's a symlink
        let metadata = target_path.symlink_metadata()?;
        if !metadata.file_type().is_symlink() {
            bail!("path exists but is not a symlink: {}", self.path);
        }

        // Read the symlink target and verify it points within payload directory
        let link_target = fs::read_link(&target_path)?;
        let target_dir = target_path.parent().unwrap_or(Path::new("/"));
        let resolved = target_dir.join(&link_target);

        // Try to canonicalize the resolved path to check if it's within payload
        let payload_dir = base_dir.join("payload");

        match resolved.canonicalize() {
            Ok(resolved_canonical) => {
                // Target exists - verify it's in payload
                let payload_canonical = match payload_dir.canonicalize() {
                    Ok(p) => p,
                    Err(_) => {
                        bail!("cannot verify symlink target: payload directory does not exist");
                    }
                };

                if !resolved_canonical.starts_with(&payload_canonical) {
                    bail!(
                        "symlink points outside payload directory: {} -> {}",
                        self.path,
                        resolved_canonical.display()
                    );
                }
            }
            Err(_) => {
                // Target doesn't exist (broken symlink) - lexically normalize the
                // resolved path and check if it starts with base_dir/payload
                let link_str = link_target.to_string_lossy();
                let normalized = normalize_path(&resolved);
                let base_normalized = normalize_path(base_dir);
                let expected_prefix = base_normalized.join("payload");

                if !normalized.starts_with(&expected_prefix) {
                    bail!(
                        "broken symlink does not point to payload: {} -> {} (normalized: {})",
                        self.path,
                        link_str,
                        normalized.display()
                    );
                }
                debug!(
                    path = %self.path,
                    target = %link_str,
                    normalized = %normalized.display(),
                    "broken symlink points to payload, allowing deletion"
                );
            }
        }

        // Safe to delete
        fs::remove_file(&target_path)?;
        debug!(path = %self.path, "deleted symlink");
        Ok(FeatureResult::Changed)
    }

    fn uninstall_with_base_dir(&self, _base_dir: &Path) -> Result<FeatureResult> {
        // Cannot restore a deleted symlink - we don't know what it pointed to
        debug!(path = %self.path, "uninstall is a no-op for DeleteSymlink");
        Ok(FeatureResult::NoOp)
    }
}

impl Feature for DeleteSymlink {
    fn install(&self) -> Result<FeatureResult> {
        self.install_with_base_dir(&std::env::current_dir()?)
    }

    fn uninstall(&self) -> Result<FeatureResult> {
        self.uninstall_with_base_dir(&std::env::current_dir()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::testing::TestContext;
    use std::fs::File;
    use std::os::unix::fs::symlink;

    #[test]
    fn install_removes_symlink_to_payload() {
        let ctx = TestContext::new();
        ctx.create_source_file("payload/somefile", "content");
        let dest = ctx.dest_path("link");
        let source = ctx.base_dir().join("payload/somefile");
        symlink(&source, &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = DeleteSymlink::new(dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);
        assert!(!dest.exists());
        assert!(dest.symlink_metadata().is_err());
    }

    #[test]
    fn install_succeeds_when_already_gone() {
        let ctx = TestContext::new();
        ctx.create_source_file("payload/somefile", "content");
        let dest_str = ctx.dest_path_str("nonexistent");

        let feature = DeleteSymlink::new(dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::NoOp);
    }

    #[test]
    fn install_is_idempotent() {
        let ctx = TestContext::new();
        ctx.create_source_file("payload/somefile", "content");
        let dest = ctx.dest_path("link");
        let source = ctx.base_dir().join("payload/somefile");
        symlink(&source, &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = DeleteSymlink::new(dest_str);

        let result1 = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result1, FeatureResult::Changed);

        let result2 = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result2, FeatureResult::NoOp);
    }

    #[test]
    fn install_fails_for_regular_file() {
        let ctx = TestContext::new();
        ctx.create_source_file("payload/somefile", "content");
        let dest = ctx.dest_path("regular");
        File::create(&dest).unwrap();
        let dest_str = ctx.dest_path_str("regular");

        let feature = DeleteSymlink::new(dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a symlink"));
    }

    #[test]
    fn install_fails_for_directory() {
        let ctx = TestContext::new();
        ctx.create_source_file("payload/somefile", "content");
        let dest = ctx.dest_path("dir");
        fs::create_dir(&dest).unwrap();
        let dest_str = ctx.dest_path_str("dir");

        let feature = DeleteSymlink::new(dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a symlink"));
    }

    #[test]
    fn install_fails_for_symlink_outside_payload() {
        let ctx = TestContext::new();
        ctx.create_source_file("payload/somefile", "content");
        // Create a file outside payload
        ctx.create_source_file("outside/otherfile", "other");
        let dest = ctx.dest_path("link");
        let source = ctx.base_dir().join("outside/otherfile");
        symlink(&source, &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = DeleteSymlink::new(dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("outside payload directory")
        );
    }

    #[test]
    fn install_fails_for_broken_symlink_outside_payload() {
        let ctx = TestContext::new();
        ctx.create_source_file("payload/somefile", "content");
        let dest = ctx.dest_path("link");
        // Create symlink to non-existent target outside payload
        symlink("/nonexistent/target", &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = DeleteSymlink::new(dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("does not point to payload")
        );
    }

    #[test]
    fn install_fails_for_broken_symlink_with_payload_in_wrong_location() {
        let ctx = TestContext::new();
        ctx.create_source_file("payload/somefile", "content");
        let dest = ctx.dest_path("link");
        // Create symlink that has "payload" in path but escapes via ..
        // This tests the false-positive case: ../payload/../etc/passwd
        let tricky_target = ctx.base_dir().join("payload/../outside/file");
        symlink(&tricky_target, &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = DeleteSymlink::new(dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("does not point to payload")
        );
    }

    #[test]
    fn install_removes_broken_symlink_to_payload() {
        let ctx = TestContext::new();
        ctx.create_source_file("payload/somefile", "content");
        let dest = ctx.dest_path("link");
        // Create symlink to non-existent target that has "payload" in path
        let broken_target = ctx.base_dir().join("payload/deleted_file");
        symlink(&broken_target, &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = DeleteSymlink::new(dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);
        assert!(!dest.exists());
        assert!(dest.symlink_metadata().is_err());
    }

    #[test]
    fn uninstall_is_noop() {
        let ctx = TestContext::new();
        ctx.create_source_file("payload/somefile", "content");
        let dest_str = ctx.dest_path_str("link");

        let feature = DeleteSymlink::new(dest_str);

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::NoOp);
    }

    #[test]
    fn display_shows_path() {
        let feature = DeleteSymlink::new("~/.claude/agents/old-agent.md");
        let display = format!("{}", feature);
        assert!(display.contains("~/.claude/agents/old-agent.md"));
    }
}

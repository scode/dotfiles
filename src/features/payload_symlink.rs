use std::fmt;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use tracing::debug;

use super::{Feature, FeatureResult};
use crate::util::fs::{compute_relative_path, expand_tilde, normalize_path};

/// Creates a destination symlink to a payload path in the dotfiles repository.
///
/// The source may be a file or directory. It is relative to the project root
/// (current working directory), while the destination supports `~` expansion.
/// The symlink is created using a relative path, making it resilient to moves
/// of the entire dotfiles directory.
///
/// The parent directory of the destination must already exist; it will not be
/// created automatically. If the destination is already a symlink to another
/// path inside this repository, install treats that as an intended source move
/// and repoints the link. Uninstall removes repo-owned links for the same
/// reason: after a source move, the old installed link is still installer state.
/// Symlinks to anything outside the repository still fail rather than being
/// overwritten or removed.
///
/// Use `RawSymlink` instead when the source is an external file not managed
/// by this repository (e.g., linking to a file in another location on disk).
#[derive(Debug)]
pub struct PayloadSymlink {
    source: String,
    destination: String,
}

impl fmt::Display for PayloadSymlink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "symlink: {} -> {}", self.source, self.destination)
    }
}

impl PayloadSymlink {
    pub fn new(source: impl Into<String>, destination: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            destination: destination.into(),
        }
    }

    fn install_with_base_dir(&self, base_dir: &Path) -> Result<FeatureResult> {
        let dest_path = expand_tilde(&self.destination)?;
        let source_path = base_dir.join(&self.source);
        let base_canonical = base_dir.canonicalize()?;
        debug!(
            destination = %self.destination,
            source = %self.source,
            "installing symlink"
        );

        if !source_path.exists() {
            bail!("source file does not exist: {}", source_path.display());
        }

        let source_canonical = source_path.canonicalize()?;
        if !source_canonical.starts_with(&base_canonical) {
            bail!(
                "source file is outside repository: {}",
                source_path.display()
            );
        }

        if dest_path.exists() || dest_path.symlink_metadata().is_ok() {
            if let Ok(link_target) = fs::read_link(&dest_path) {
                let dest_dir = dest_path.parent().unwrap_or(Path::new("/"));
                let resolved = dest_dir.join(&link_target);
                match Self::repo_target(base_dir, &base_canonical, &resolved) {
                    RepoTarget::Resolved(resolved_canonical) => {
                        if resolved_canonical == source_canonical {
                            debug!(destination = %self.destination, "already installed");
                            return Ok(FeatureResult::NoOp);
                        }

                        debug!(
                            destination = %self.destination,
                            old_target = %resolved_canonical.display(),
                            "repointing repository symlink"
                        );
                        fs::remove_file(&dest_path)?;
                        return Self::create_symlink(&dest_path, &source_canonical);
                    }
                    RepoTarget::Broken(normalized) => {
                        debug!(
                            destination = %self.destination,
                            old_target = %normalized.display(),
                            "repointing broken repository symlink"
                        );
                        fs::remove_file(&dest_path)?;
                        return Self::create_symlink(&dest_path, &source_canonical);
                    }
                    RepoTarget::Outside => {
                        if normalize_path(&resolved) == normalize_path(&source_path) {
                            debug!(destination = %self.destination, "already installed");
                            return Ok(FeatureResult::NoOp);
                        }
                    }
                }
            }
            bail!("destination already exists: {}", self.destination);
        }

        Self::create_symlink(&dest_path, &source_canonical)
    }

    fn create_symlink(dest_path: &Path, source_canonical: &Path) -> Result<FeatureResult> {
        let dest_dir = dest_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("destination has no parent directory"))?
            .canonicalize()?;

        let relative_source = compute_relative_path(&dest_dir, source_canonical);

        symlink(&relative_source, dest_path)?;
        Ok(FeatureResult::Changed)
    }

    fn repo_target(base_dir: &Path, base_canonical: &Path, target: &Path) -> RepoTarget {
        if let Ok(canonical) = target.canonicalize() {
            if canonical.starts_with(base_canonical) {
                return RepoTarget::Resolved(canonical);
            }
            return RepoTarget::Outside;
        }

        let normalized = normalize_path(target);
        if !normalized.starts_with(normalize_path(base_dir)) {
            return RepoTarget::Outside;
        }

        let mut ancestor = target;
        while !ancestor.exists() {
            let Some(parent) = ancestor.parent() else {
                return RepoTarget::Outside;
            };
            ancestor = parent;
        }

        match ancestor.canonicalize() {
            Ok(canonical) if canonical.starts_with(base_canonical) => {
                RepoTarget::Broken(normalized)
            }
            _ => RepoTarget::Outside,
        }
    }

    fn uninstall_with_base_dir(&self, base_dir: &Path) -> Result<FeatureResult> {
        let dest_path = expand_tilde(&self.destination)?;
        let base_canonical = base_dir.canonicalize()?;

        if !dest_path.exists() && dest_path.symlink_metadata().is_err() {
            debug!(destination = %self.destination, "already uninstalled");
            return Ok(FeatureResult::NoOp);
        }

        if !dest_path.symlink_metadata()?.file_type().is_symlink() {
            bail!("not a symlink: {}", self.destination);
        }

        let source_path = base_dir.join(&self.source);
        let link_target = fs::read_link(&dest_path)?;
        let dest_dir = dest_path.parent().unwrap_or(Path::new("/"));
        let resolved = dest_dir.join(&link_target);

        let target_owned_by_repo = match Self::repo_target(base_dir, &base_canonical, &resolved) {
            RepoTarget::Resolved(_) | RepoTarget::Broken(_) => true,
            RepoTarget::Outside => normalize_path(&resolved) == normalize_path(&source_path),
        };

        if !target_owned_by_repo {
            bail!("symlink {} points to unexpected target", self.destination);
        }

        fs::remove_file(&dest_path)?;
        debug!(destination = %self.destination, "removed symlink");
        Ok(FeatureResult::Changed)
    }
}

enum RepoTarget {
    Resolved(PathBuf),
    Broken(PathBuf),
    Outside,
}

impl Feature for PayloadSymlink {
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
    use std::io::Write;

    #[test]
    fn install_creates_symlink() {
        let ctx = TestContext::new();
        ctx.create_source_file("testfile", "hello");
        let dest_str = ctx.dest_path_str("link");

        let feature = PayloadSymlink::new("testfile", dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);

        let dest = ctx.dest_path("link");
        assert!(dest.symlink_metadata().unwrap().file_type().is_symlink());
        let contents = fs::read_to_string(&dest).unwrap();
        assert_eq!(contents, "hello");
    }

    #[test]
    fn install_uses_relative_symlink() {
        let ctx = TestContext::new();
        ctx.create_source_file("testfile", "content");
        let dest_str = ctx.dest_path_str("link");

        let feature = PayloadSymlink::new("testfile", dest_str);

        feature.install_with_base_dir(ctx.base_dir()).unwrap();

        let link_target = fs::read_link(ctx.dest_path("link")).unwrap();
        assert!(
            link_target.is_relative(),
            "symlink should be relative, got: {:?}",
            link_target
        );
    }

    #[test]
    fn install_fails_when_source_missing() {
        let ctx = TestContext::new();
        let dest_str = ctx.dest_path_str("link");

        let feature = PayloadSymlink::new("nonexistent", dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn install_fails_when_source_symlink_resolves_outside_repo() {
        let ctx = TestContext::new();
        let outside = ctx.dest_path("outside-source");
        File::create(&outside).unwrap();
        symlink(&outside, ctx.source_dir.path().join("source-link")).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = PayloadSymlink::new("source-link", dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("outside repository")
        );
    }

    #[test]
    fn install_fails_when_dest_parent_missing() {
        let ctx = TestContext::new();
        ctx.create_source_file("source", "content");
        let dest_str = ctx.dest_path_str("nonexistent/subdir/link");

        let feature = PayloadSymlink::new("source", dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
    }

    #[test]
    fn install_fails_when_dest_is_regular_file() {
        let ctx = TestContext::new();
        ctx.create_source_file("source", "content");
        let dest = ctx.dest_path("existing");
        File::create(&dest).unwrap();
        let dest_str = ctx.dest_path_str("existing");

        let feature = PayloadSymlink::new("source", dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn install_repoints_repo_symlink_to_new_source() {
        let ctx = TestContext::new();
        let source = ctx.create_source_file("correct", "content");
        let old_source = ctx.create_source_file("wrong", "other");
        let dest = ctx.dest_path("link");
        let old_target = compute_relative_path(dest.parent().unwrap(), &old_source);
        symlink(old_target, &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = PayloadSymlink::new("correct", dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);
        assert_eq!(fs::read_to_string(&dest).unwrap(), "content");
        assert_eq!(
            dest.parent()
                .unwrap()
                .join(fs::read_link(&dest).unwrap())
                .canonicalize()
                .unwrap(),
            source
        );
    }

    #[test]
    fn install_fails_when_dest_symlink_points_outside_repo() {
        let ctx = TestContext::new();
        ctx.create_source_file("correct", "content");
        let outside = ctx.dest_path("outside-source");
        File::create(&outside).unwrap();
        let dest = ctx.dest_path("link");
        symlink(&outside, &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = PayloadSymlink::new("correct", dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
        assert_eq!(fs::read_to_string(&dest).unwrap(), "");
    }

    #[test]
    fn install_repoints_broken_repo_symlink_to_new_source() {
        let ctx = TestContext::new();
        ctx.create_source_file("correct", "content");
        let dest = ctx.dest_path("link");
        symlink(ctx.source_dir.path().join("missing-old-source"), &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = PayloadSymlink::new("correct", dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);
        assert_eq!(fs::read_to_string(&dest).unwrap(), "content");
    }

    #[test]
    fn install_fails_for_broken_target_through_symlinked_repo_directory() {
        let ctx = TestContext::new();
        ctx.create_source_file("correct", "content");
        let outside_dir = ctx.dest_path("outside-dir");
        fs::create_dir(&outside_dir).unwrap();
        symlink(&outside_dir, ctx.source_dir.path().join("repo-link")).unwrap();
        let dest = ctx.dest_path("link");
        symlink(ctx.source_dir.path().join("repo-link/missing"), &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = PayloadSymlink::new("correct", dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn install_succeeds_when_already_correct() {
        let ctx = TestContext::new();
        ctx.create_source_file("source", "content");
        let dest_str = ctx.dest_path_str("link");

        let feature = PayloadSymlink::new("source", dest_str.clone());

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);

        let feature2 = PayloadSymlink::new("source", dest_str);
        let result = feature2.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::NoOp);
    }

    #[test]
    fn uninstall_removes_symlink() {
        let ctx = TestContext::new();
        ctx.create_source_file("source", "content");
        let dest = ctx.dest_path("link");
        let dest_str = ctx.dest_path_str("link");

        let feature = PayloadSymlink::new("source", dest_str);
        feature.install_with_base_dir(ctx.base_dir()).unwrap();

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);
        assert!(!dest.exists());
        assert!(dest.symlink_metadata().is_err());
    }

    #[test]
    fn uninstall_succeeds_when_already_gone() {
        let ctx = TestContext::new();
        let dest_str = ctx.dest_path_str("nonexistent");

        let feature = PayloadSymlink::new("source", dest_str);

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::NoOp);
    }

    #[test]
    fn uninstall_fails_for_regular_file() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("regular");
        File::create(&dest).unwrap();
        let dest_str = ctx.dest_path_str("regular");

        let feature = PayloadSymlink::new("source", dest_str);

        let result = feature.uninstall_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a symlink"));
    }

    #[test]
    fn uninstall_fails_for_directory() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("dir");
        fs::create_dir(&dest).unwrap();
        let dest_str = ctx.dest_path_str("dir");

        let feature = PayloadSymlink::new("source", dest_str);

        let result = feature.uninstall_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a symlink"));
    }

    #[test]
    fn uninstall_removes_repo_symlink_to_moved_source() {
        let ctx = TestContext::new();
        ctx.create_source_file("correct", "content");
        ctx.create_source_file("wrong", "other");
        let dest = ctx.dest_path("link");
        symlink(ctx.source_dir.path().join("wrong"), &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = PayloadSymlink::new("correct", dest_str);

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);
        assert!(dest.symlink_metadata().is_err());
    }

    /// Moved-source uninstalls must still clean up old repo-owned symlinks.
    #[test]
    fn uninstall_removes_repo_symlink_when_source_missing() {
        let ctx = TestContext::new();
        ctx.create_source_file("wrong", "other");
        let dest = ctx.dest_path("link");
        symlink(ctx.source_dir.path().join("wrong"), &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = PayloadSymlink::new("missing", dest_str);

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);
        assert!(dest.symlink_metadata().is_err());
    }

    /// Regression test: lexical fallback should allow uninstall when source and
    /// destination resolve to the same non-existent payload path.
    #[test]
    fn uninstall_succeeds_when_source_missing_but_target_matches_lexically() {
        let ctx = TestContext::new();
        let source_path = ctx.source_dir.path().join("missing");
        let dest = ctx.dest_path("link");
        symlink(&source_path, &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = PayloadSymlink::new("missing", dest_str);

        let result = feature.uninstall_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);
        assert!(dest.symlink_metadata().is_err());
    }

    #[test]
    fn install_with_nested_source_path() {
        let ctx = TestContext::new();
        ctx.create_source_file("config/settings", "nested content");
        let dest_str = ctx.dest_path_str("link");

        let feature = PayloadSymlink::new("config/settings", dest_str);

        feature.install_with_base_dir(ctx.base_dir()).unwrap();

        let contents = fs::read_to_string(ctx.dest_path("link")).unwrap();
        assert_eq!(contents, "nested content");
    }

    #[test]
    fn install_succeeds_when_source_is_symlink() {
        let ctx = TestContext::new();
        ctx.create_source_file("real-source", "symlinked content");
        symlink(
            ctx.source_dir.path().join("real-source"),
            ctx.source_dir.path().join("source-link"),
        )
        .unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = PayloadSymlink::new("source-link", dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);

        let dest = ctx.dest_path("link");
        assert!(dest.symlink_metadata().unwrap().file_type().is_symlink());
        let contents = fs::read_to_string(&dest).unwrap();
        assert_eq!(contents, "symlinked content");
    }

    #[test]
    fn symlink_remains_valid_after_source_content_change() {
        let ctx = TestContext::new();
        let source_path = ctx.create_source_file("source", "original");
        let dest_str = ctx.dest_path_str("link");

        let feature = PayloadSymlink::new("source", dest_str);

        feature.install_with_base_dir(ctx.base_dir()).unwrap();

        let mut file = File::create(&source_path).unwrap();
        file.write_all(b"modified").unwrap();

        let contents = fs::read_to_string(ctx.dest_path("link")).unwrap();
        assert_eq!(contents, "modified");
    }
}

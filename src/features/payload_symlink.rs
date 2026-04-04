use std::fmt;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

use anyhow::{Result, bail};
use tracing::debug;

use super::{Feature, FeatureResult};
use crate::util::fs::{compute_relative_path, expand_tilde, normalize_path};

/// A feature that creates a symlink from a destination path to a source file
/// within the dotfiles repository (typically in the `payload/` directory).
///
/// The source path is relative to the project root (current working directory),
/// while the destination supports `~` expansion. The symlink is created using
/// a relative path, making it resilient to moves of the entire dotfiles directory.
///
/// The parent directory of the destination must already exist; it will not be
/// created automatically.
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
        debug!(
            destination = %self.destination,
            source = %self.source,
            "installing symlink"
        );

        if !source_path.exists() {
            bail!("source file does not exist: {}", source_path.display());
        }

        let source_canonical = source_path.canonicalize()?;

        if dest_path.exists() || dest_path.symlink_metadata().is_ok() {
            if let Ok(link_target) = fs::read_link(&dest_path) {
                let dest_dir = dest_path.parent().unwrap_or(Path::new("/"));
                let resolved = dest_dir.join(&link_target).canonicalize();
                if let Ok(resolved) = resolved
                    && resolved == source_canonical
                {
                    debug!(destination = %self.destination, "already installed");
                    return Ok(FeatureResult::NoOp);
                }
            }
            bail!("destination already exists: {}", self.destination);
        }

        let dest_dir = dest_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("destination has no parent directory"))?
            .canonicalize()?;

        let relative_source = compute_relative_path(&dest_dir, &source_canonical);

        symlink(&relative_source, &dest_path)?;
        Ok(FeatureResult::Changed)
    }

    fn uninstall_with_base_dir(&self, base_dir: &Path) -> Result<FeatureResult> {
        let dest_path = expand_tilde(&self.destination)?;

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

        let targets_match = match (resolved.canonicalize(), source_path.canonicalize()) {
            (Ok(resolved_canonical), Ok(source_canonical)) => {
                resolved_canonical == source_canonical
            }
            _ => normalize_path(&resolved) == normalize_path(&source_path),
        };

        if !targets_match {
            bail!("symlink {} points to unexpected target", self.destination);
        }

        fs::remove_file(&dest_path)?;
        debug!(destination = %self.destination, "removed symlink");
        Ok(FeatureResult::Changed)
    }
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
    fn install_fails_when_dest_is_wrong_symlink() {
        let ctx = TestContext::new();
        ctx.create_source_file("correct", "content");
        ctx.create_source_file("wrong", "other");
        let dest = ctx.dest_path("link");
        symlink(ctx.source_dir.path().join("wrong"), &dest).unwrap();
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
    fn uninstall_fails_for_wrong_symlink_target() {
        let ctx = TestContext::new();
        ctx.create_source_file("correct", "content");
        ctx.create_source_file("wrong", "other");
        let dest = ctx.dest_path("link");
        symlink(ctx.source_dir.path().join("wrong"), &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = PayloadSymlink::new("correct", dest_str);

        let result = feature.uninstall_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("unexpected target")
        );
    }

    /// Regression test: when payload source is missing, uninstall must fail for
    /// symlinks that point to an unexpected target.
    #[test]
    fn uninstall_fails_for_wrong_target_when_source_missing() {
        let ctx = TestContext::new();
        ctx.create_source_file("wrong", "other");
        let dest = ctx.dest_path("link");
        symlink(ctx.source_dir.path().join("wrong"), &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = PayloadSymlink::new("missing", dest_str);

        let result = feature.uninstall_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("unexpected target")
        );
        assert!(dest.symlink_metadata().unwrap().file_type().is_symlink());
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

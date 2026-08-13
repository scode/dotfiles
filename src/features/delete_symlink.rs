use std::fmt;
use std::fs;
use std::path::Path;

use anyhow::{Result, anyhow, bail};
use tracing::debug;

use super::{Feature, FeatureResult};
use crate::util::fs::{RepoTarget, expand_tilde, normalize_path, repo_target};

/// Deletes an old installer symlink if it still points into this repository.
///
/// This is useful for cleaning up old symlinks that are no longer needed.
/// The deletion only proceeds if:
/// - The path exists and is a symlink
/// - The symlink target resolves (or lexically normalizes, for broken symlinks)
///   to a path within the repository checkout (the installer's base directory)
///
/// The ownership check is "points into the repository", not "points into
/// payload/": `PayloadSymlink` installs from arbitrary repo-relative sources
/// (`payload/`, `agent-skills/`, `agent-instructions/`, ...), so a stale link
/// from any of those trees is installer-owned. An earlier version accepted only
/// `payload/`, which made rename migrations fail: renaming a skill directory
/// left the old install a broken symlink into `agent-skills/`, and the
/// migration's DeleteSymlink then refused to touch it. The check itself is the
/// shared [`repo_target`] classifier, so this feature and `PayloadSymlink`
/// agree on what installer ownership means — including refusing broken targets
/// that read as repository-internal but escape through an intermediate
/// directory symlink.
///
/// Links to both files and directories are allowed. That matters for moved
/// skill directories, where the stale installed path is a directory symlink.
/// Broken links are expected, not suspicious — a renamed or deleted source is
/// exactly what produces the stale installs this feature cleans up.
///
/// If the path does not exist, `install()` returns `NoOp` (idempotent). If the
/// path exists but is not a symlink, or points outside the repository, an
/// error is returned — those are user-owned files this feature must not touch.
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

        // Read the symlink target and verify it points within the repository.
        // The classification itself lives in repo_target so that this feature
        // and PayloadSymlink cannot drift apart on what installer ownership
        // means.
        let link_target = fs::read_link(&target_path)?;
        let target_dir = target_path.parent().unwrap_or(Path::new("/"));
        let resolved = target_dir.join(&link_target);

        let base_canonical = base_dir
            .canonicalize()
            .map_err(|_| anyhow!("cannot verify symlink target: repository root does not exist"))?;

        match repo_target(base_dir, &base_canonical, &resolved) {
            RepoTarget::Resolved(canonical) => {
                debug!(
                    path = %self.path,
                    target = %canonical.display(),
                    "symlink resolves into the repository, allowing deletion"
                );
            }
            RepoTarget::Broken(normalized) => {
                debug!(
                    path = %self.path,
                    target = %normalized.display(),
                    "broken symlink points into the repository, allowing deletion"
                );
            }
            RepoTarget::Outside => {
                bail!(
                    "symlink does not point into the repository: {} -> {} (resolved: {})",
                    self.path,
                    link_target.to_string_lossy(),
                    normalize_path(&resolved).display()
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

    /// The happy path: a live installer-owned link (target inside the repo)
    /// gets deleted and reports Changed.
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

    /// An absent path is NoOp, not an error — migration entries stay in the
    /// feature graph forever, so most runs see the link already gone.
    #[test]
    fn install_succeeds_when_already_gone() {
        let ctx = TestContext::new();
        ctx.create_source_file("payload/somefile", "content");
        let dest_str = ctx.dest_path_str("nonexistent");

        let feature = DeleteSymlink::new(dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::NoOp);
    }

    /// Running install twice must not error: first run deletes (Changed),
    /// second run finds nothing (NoOp).
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

    /// A regular file at the target path is user data, never installer state;
    /// refusing (rather than deleting) is the whole point of the symlink check.
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

    /// Same ownership rule for directories: a real directory at the path is
    /// user-owned, even where the installer once had a directory symlink.
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

    /// A live symlink to a file outside the repository must be refused: it is
    /// user-owned, and deleting it would be exactly the kind of collateral
    /// damage the ownership check exists to prevent.
    #[test]
    fn install_fails_for_symlink_outside_repo() {
        let ctx = TestContext::new();
        // dest_dir is a separate tempdir, so a target there is genuinely
        // outside the repository (base_dir).
        let outside_target = ctx.dest_path("otherfile");
        File::create(&outside_target).unwrap();
        let dest = ctx.dest_path("link");
        symlink(&outside_target, &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = DeleteSymlink::new(dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("does not point into the repository")
        );
    }

    /// A broken symlink whose target is outside the repository must be
    /// refused. Brokenness alone is not evidence of installer ownership — the
    /// user may have their own dead links lying around.
    #[test]
    fn install_fails_for_broken_symlink_outside_repo() {
        let ctx = TestContext::new();
        ctx.create_source_file("payload/somefile", "content");
        let dest = ctx.dest_path("link");
        symlink("/nonexistent/target", &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = DeleteSymlink::new(dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("does not point into the repository")
        );
    }

    /// The broken-symlink check is lexical (canonicalize cannot resolve a dead
    /// target), so it must not be fooled by a target that starts under the
    /// repo root but escapes it via `..` components.
    #[test]
    fn install_fails_for_broken_symlink_escaping_repo_via_dotdot() {
        let ctx = TestContext::new();
        ctx.create_source_file("payload/somefile", "content");
        let dest = ctx.dest_path("link");
        // Normalizes to a sibling of base_dir, i.e. outside the repository.
        let tricky_target = ctx.base_dir().join("payload/../../outside/file");
        symlink(&tricky_target, &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = DeleteSymlink::new(dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("does not point into the repository")
        );
    }

    /// Broken links into the repo are the core migration case: a renamed or
    /// removed payload source leaves the old install dangling, and DeleteSymlink
    /// must still clean it up.
    #[test]
    fn install_removes_broken_symlink_to_payload() {
        let ctx = TestContext::new();
        ctx.create_source_file("payload/somefile", "content");
        let dest = ctx.dest_path("link");
        let broken_target = ctx.base_dir().join("payload/deleted_file");
        symlink(&broken_target, &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = DeleteSymlink::new(dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);
        // exists() follows the link and was false even before deletion; only
        // symlink_metadata can prove the link itself is gone.
        assert!(dest.symlink_metadata().is_err());
    }

    /// Regression test for the scode-fable-resume → agent-resumeable rename:
    /// installed artifacts live outside payload/ (agent-skills/, agent-instructions/),
    /// and the ownership check used to accept only payload/, so the rename's
    /// DeleteSymlink migration refused to delete the stale broken link. Any
    /// broken link into the repository checkout must be deletable.
    #[test]
    fn install_removes_broken_symlink_to_agent_skills() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("link");
        // The renamed skill's directory no longer exists, so the link is broken.
        let broken_target = ctx.base_dir().join("agent-skills/renamed-away-skill");
        symlink(&broken_target, &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = DeleteSymlink::new(dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);
        assert!(dest.symlink_metadata().is_err());
    }

    /// Live targets outside payload/ are the other half of the widened
    /// ownership rule: a still-resolving link into agent-skills/ must be
    /// deletable, not just a broken one. Guards against the canonicalize
    /// branch quietly keeping the old payload/-only restriction.
    #[test]
    fn install_removes_live_symlink_to_agent_skills() {
        let ctx = TestContext::new();
        let source = ctx.create_source_file("agent-skills/some-skill/SKILL.md", "content");
        let dest = ctx.dest_path("link");
        symlink(&source, &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = DeleteSymlink::new(dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);
        assert!(dest.symlink_metadata().is_err());
    }

    /// The installer creates links with targets relative to the link's parent
    /// directory, so the ownership check must resolve relative targets from
    /// there — not from the repo root or the process working directory. A
    /// relative broken target into agent-skills/ must still be recognized as
    /// installer-owned and deleted.
    #[test]
    fn install_removes_broken_relative_symlink_to_agent_skills() {
        let ctx = TestContext::new();
        let dest = ctx.dest_path("link");
        let relative_target = crate::util::fs::compute_relative_path(
            dest.parent().unwrap(),
            &ctx.base_dir().join("agent-skills/renamed-away-skill"),
        );
        symlink(&relative_target, &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = DeleteSymlink::new(dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir()).unwrap();
        assert_eq!(result, FeatureResult::Changed);
        assert!(dest.symlink_metadata().is_err());
    }

    /// A broken target can read as repository-internal while an intermediate
    /// directory symlink carries it outside: <repo>/escape/missing where
    /// escape links to an external directory. The lexical prefix check alone
    /// accepts that path, so this pins the ancestor-canonicalization guard —
    /// the link is user-reachable state outside the repo and must survive.
    #[test]
    fn install_fails_for_broken_symlink_escaping_via_intermediate_symlink() {
        let ctx = TestContext::new();
        // An external directory (dest_dir tempdir) reachable through a
        // symlink that lives inside the repository.
        let external_dir = ctx.dest_path("external");
        fs::create_dir(&external_dir).unwrap();
        symlink(&external_dir, ctx.base_dir().join("escape")).unwrap();

        let dest = ctx.dest_path("link");
        let tricky_target = ctx.base_dir().join("escape/missing");
        symlink(&tricky_target, &dest).unwrap();
        let dest_str = ctx.dest_path_str("link");

        let feature = DeleteSymlink::new(dest_str);

        let result = feature.install_with_base_dir(ctx.base_dir());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("does not point into the repository")
        );
        // The refusal must leave the link untouched.
        assert!(dest.symlink_metadata().is_ok());
    }

    /// Uninstall cannot restore a deleted link (the target is unknown by
    /// then), so it must be a NoOp rather than an error.
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

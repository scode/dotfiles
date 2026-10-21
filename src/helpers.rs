//! Build optional executables inside skill trees without changing installation or requiring Deno on the destination.
//!
//! Compilation consumes a snapshot, not a moving checkout. The generated entrypoint carries that snapshot's digest
//! and validates the external source before importing the helper, so stale deployments cannot process commands.
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

/// Tie Rust's host names to Deno's target triples and the launcher's artifact directories in one table.
struct Target {
    triple: &'static str,
    platform: &'static str,
    host: (&'static str, &'static str),
}

/// Each platform also needs a matching branch in the POSIX launcher; compiler triples alone are not enough.
const TARGETS: &[Target] = &[
    Target {
        triple: "x86_64-unknown-linux-gnu",
        platform: "linux-x64",
        host: ("linux", "x86_64"),
    },
    Target {
        triple: "aarch64-unknown-linux-gnu",
        platform: "linux-arm64",
        host: ("linux", "aarch64"),
    },
    Target {
        triple: "x86_64-apple-darwin",
        platform: "darwin-x64",
        host: ("macos", "x86_64"),
    },
    Target {
        triple: "aarch64-apple-darwin",
        platform: "darwin-arm64",
        host: ("macos", "aarch64"),
    },
];

/// Resolve the implicit target without asking the compiler or depending on the caller's environment.
fn host_target() -> Result<&'static str> {
    TARGETS
        .iter()
        .find(|target| target.host == (std::env::consts::OS, std::env::consts::ARCH))
        .map(|target| target.triple)
        .context("this host has no helper target; pass --target with a supported POSIX Deno target")
}

/// Dispatch compilation through Deno, preserving the previous artifact on any compiler or publication failure.
pub(crate) fn build(root: &Path, target: Option<&str>) -> Result<()> {
    let target = match target {
        Some(target) => target,
        None => host_target()?,
    };
    build_with(root, target, |entry, output, target| {
        let status = Command::new("deno")
            .args([
                "compile",
                "--no-config",
                "--no-lock",
                "-A",
                "--target",
                target,
                "--output",
            ])
            .arg(output)
            .arg(entry)
            .status()
            .context("could not launch deno compile; install Deno on the build host")?;
        if !status.success() {
            bail!("deno compile failed ({status}); previous helper was preserved");
        }
        Ok(())
    })
}

/// Snapshot and publish on one filesystem. The injected compiler lets failure/race tests run without Deno or network.
fn build_with(
    root: &Path,
    target: &str,
    compile: impl FnOnce(&Path, &Path, &str) -> Result<()>,
) -> Result<()> {
    let platform = TARGETS
        .iter()
        .find(|entry| entry.triple == target)
        .map(|entry| entry.platform)
        .with_context(|| {
            format!(
                "unsupported helper target {target}; use {}",
                TARGETS
                    .iter()
                    .map(|entry| entry.triple)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
    let skill = root.join("agent-skills/persistent-review");
    let source_path = skill.join("evidence.ts");
    let source = fs::read(&source_path)
        .context("read persistent-review/evidence.ts from the checkout root")?;
    let digest = format!("{:x}", Sha256::digest(&source));
    let destination = skill.join("bin").join(platform);
    // The launcher rejects these links too: following one could bind the executable
    // to a different skill tree than the source the caller expects it to validate.
    for path in [skill.join("bin"), destination.clone()] {
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                bail!(
                    "helper output directory must not be a symlink: {}",
                    path.display()
                );
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("inspect helper output directory"),
        }
    }
    fs::create_dir_all(&destination).context("create helper output directory")?;
    // Keep interrupted-build debris out of each platform directory while retaining same-filesystem publication.
    let stage = tempfile::Builder::new()
        .prefix(".build-")
        .tempdir_in(skill.join("bin"))
        .context("create helper staging directory")?;
    fs::write(stage.path().join("evidence.ts"), &source).context("write helper source snapshot")?;
    let entry = stage.path().join("entry.ts");
    fs::write(&entry, compiled_entry(&digest)).context("write compiled helper entrypoint")?;
    let output = stage.path().join("evidence");
    compile(&entry, &output, target)?;
    if fs::read(&source_path).context("re-read helper source after compilation")? != source {
        bail!(
            "evidence.ts changed during compilation; previous helper was preserved, retry the build"
        );
    }
    if !output.is_file() {
        bail!("compiler did not produce a helper executable");
    }
    let published = destination.join("evidence");
    fs::rename(&output, &published).context("publish compiled helper")?;
    println!(
        "Built {} ({target}, source SHA-256 {digest})",
        published.display()
    );
    Ok(())
}

/// Embed the digest without escaping TypeScript syntax; compilation checks the instantiated startup program.
fn compiled_entry(digest: &str) -> String {
    let template = include_str!("helpers/entry.ts");
    assert_eq!(
        template.matches("__SOURCE_SHA256__").count(),
        1,
        "entrypoint must contain one digest placeholder"
    );
    template.replace("__SOURCE_SHA256__", digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Give each build an isolated checkout-shaped fixture; never mutate the process environment or real skills.
    fn fixture() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let skill = root.path().join("agent-skills/persistent-review");
        fs::create_dir_all(&skill).unwrap();
        let source = skill.join("evidence.ts");
        fs::write(&source, "original").unwrap();
        let binary = skill.join("bin/linux-x64/evidence");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, "previous").unwrap();
        (root, source, binary)
    }

    /// A successful build must bind the snapshotted bytes and replace the old executable only after compilation.
    #[test]
    fn publishes_snapshot_with_embedded_digest() {
        let (root, _, binary) = fixture();
        build_with(root.path(), TARGETS[0].triple, |entry, output, _| {
            assert_eq!(
                fs::read_to_string(entry.with_file_name("evidence.ts"))?,
                "original"
            );
            let wrapper = fs::read_to_string(entry)?;
            assert!(wrapper.contains(&format!("{:x}", Sha256::digest(b"original"))));
            assert_eq!(fs::read_to_string(&binary)?, "previous");
            fs::write(output, "new")?;
            Ok(())
        })
        .unwrap();
        assert_eq!(fs::read_to_string(binary).unwrap(), "new");
    }

    /// Compiler failures must retain the previous artifact, even when the compiler left partial output behind.
    #[test]
    fn compiler_failure_preserves_previous_binary() {
        let (root, _, binary) = fixture();
        let result = build_with(root.path(), TARGETS[0].triple, |_, output, _| {
            fs::write(output, "unpublished")?;
            bail!("compiler failed")
        });
        assert!(result.unwrap_err().to_string().contains("compiler failed"));
        assert_preserved(&binary);
    }

    /// A successful compiler cannot publish an artifact whose checkout source changed while it was running.
    #[test]
    fn changed_source_preserves_previous_binary() {
        let (root, source, binary) = fixture();
        let result = build_with(root.path(), TARGETS[0].triple, |_, output, _| {
            fs::write(output, "unpublished")?;
            fs::write(source, "changed")?;
            Ok(())
        });
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("changed during compilation")
        );
        assert_preserved(&binary);
    }

    /// A compiler reporting success without output must yield the actionable diagnostic, not a bare I/O error.
    #[test]
    fn missing_compiler_output_preserves_previous_binary() {
        let (root, _, binary) = fixture();
        let result = build_with(root.path(), TARGETS[0].triple, |_, _, _| Ok(()));
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("did not produce a helper executable")
        );
        assert_preserved(&binary);
    }

    /// Failure tests check both the retained artifact and cleanup at the staging directory's actual parent.
    fn assert_preserved(binary: &Path) {
        assert_eq!(fs::read_to_string(binary).unwrap(), "previous");
        assert_eq!(
            fs::read_dir(binary.parent().unwrap().parent().unwrap())
                .unwrap()
                .count(),
            1
        );
    }

    /// Reject unsupported targets before touching the skill tree or invoking a compiler.
    #[test]
    fn unsupported_target_has_no_side_effects() {
        let root = tempfile::tempdir().unwrap();
        assert!(
            build_with(root.path(), "x86_64-unknown-linux-musl", |_, _, _| panic!(
                "compiler called"
            ))
            .is_err()
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    /// A linked output directory would publish into another tree, which the launcher deliberately refuses to execute.
    #[cfg(unix)]
    #[test]
    fn linked_output_directories_are_rejected_before_compilation() {
        for relative in ["bin", "bin/linux-x64"] {
            let (root, _, _) = fixture();
            let skill = root.path().join("agent-skills/persistent-review");
            let path = skill.join(relative);
            fs::remove_dir_all(&path).unwrap();
            let elsewhere = root.path().join("other-tree");
            fs::create_dir(&elsewhere).unwrap();
            std::os::unix::fs::symlink(&elsewhere, &path).unwrap();
            let result = build_with(root.path(), TARGETS[0].triple, |_, _, _| {
                panic!("compiler called")
            });
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("must not be a symlink")
            );
            assert_eq!(fs::read_dir(elsewhere).unwrap().count(), 0);
        }
    }
}

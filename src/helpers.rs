//! Build optional executables inside skill trees without changing installation or requiring Deno on the destination.
//!
//! Compilation consumes a snapshot, not a moving checkout. The generated entrypoint carries that snapshot's digest
//! and validates the external source before importing the helper, so stale deployments cannot process commands.
//! Every helper in `HELPERS` is built the same way; each has a committed POSIX launcher in its skill directory that
//! picks the binary for the host or falls back to the source.
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

/// One compiled helper: its skill directory under `agent-skills/`, the TypeScript source there, the executable name the
/// skill's launcher looks for under `bin/<platform>/`, and the entrypoint template that checks and starts it.
struct Helper {
    skill: &'static str,
    source: &'static str,
    binary: &'static str,
    entry: &'static str,
}

/// Every helper `build-helpers` builds. Each needs a launcher in its skill directory with the same selection rules.
const HELPERS: &[Helper] = &[
    Helper {
        skill: "persistent-review",
        source: "evidence.ts",
        binary: "evidence",
        entry: include_str!("helpers/persistent-review-entry.ts"),
    },
    Helper {
        skill: "interactive-artifact",
        source: "interactive-artifact.ts",
        binary: "interactive-artifact",
        entry: include_str!("helpers/interactive-artifact-entry.ts"),
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

/// Dispatch compilation through Deno for every helper, preserving each previous artifact on any compiler or
/// publication failure. A failed helper does not stop the others; the call fails if any did.
pub(crate) fn build(root: &Path, target: Option<&str>) -> Result<()> {
    let target = match target {
        Some(target) => target,
        None => host_target()?,
    };
    build_all(root, target, &deno_compile)
}

/// Build every helper with the given compiler, continuing past a failed one. The target is checked once up front, so
/// an unsupported one is reported once rather than once per helper.
fn build_all(
    root: &Path,
    target: &str,
    compile: &dyn Fn(&Path, &Path, &str) -> Result<()>,
) -> Result<()> {
    if !TARGETS.iter().any(|entry| entry.triple == target) {
        bail!(
            "unsupported helper target {target}; use {}",
            TARGETS
                .iter()
                .map(|entry| entry.triple)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let mut failed = Vec::new();
    for helper in HELPERS {
        if let Err(error) = build_with(root, helper, target, compile) {
            eprintln!("{}: {error:#}", helper.skill);
            failed.push(helper.skill);
        }
    }
    if !failed.is_empty() {
        bail!("helper build failed for {}", failed.join(", "));
    }
    Ok(())
}

/// The real compiler. Tests inject their own instead, so failure and race cases run without Deno or network.
fn deno_compile(entry: &Path, output: &Path, target: &str) -> Result<()> {
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
}

/// Snapshot and publish on one filesystem. The injected compiler lets failure/race tests run without Deno or network.
fn build_with(
    root: &Path,
    helper: &Helper,
    target: &str,
    compile: &dyn Fn(&Path, &Path, &str) -> Result<()>,
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
    let skill = root.join("agent-skills").join(helper.skill);
    let source_path = skill.join(helper.source);
    let source = fs::read(&source_path).with_context(|| {
        format!(
            "read {}/{} from the checkout root",
            helper.skill, helper.source
        )
    })?;
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
    fs::write(stage.path().join(helper.source), &source).context("write helper source snapshot")?;
    let entry = stage.path().join("entry.ts");
    fs::write(&entry, compiled_entry(helper, &digest))
        .context("write compiled helper entrypoint")?;
    let output = stage.path().join(helper.binary);
    compile(&entry, &output, target)?;
    if fs::read(&source_path).context("re-read helper source after compilation")? != source {
        bail!(
            "{} changed during compilation; previous helper was preserved, retry the build",
            helper.source
        );
    }
    if !output.is_file() {
        bail!("compiler did not produce a helper executable");
    }
    let published = destination.join(helper.binary);
    fs::rename(&output, &published).context("publish compiled helper")?;
    println!(
        "Built {} ({target}, source SHA-256 {digest})",
        published.display()
    );
    Ok(())
}

/// Embed the digest without escaping TypeScript syntax; compilation checks the instantiated startup program. The
/// template imports the snapshot by its file name, so a table entry and its template cannot drift apart unnoticed.
fn compiled_entry(helper: &Helper, digest: &str) -> String {
    assert_eq!(
        helper.entry.matches("__SOURCE_SHA256__").count(),
        1,
        "entrypoint must contain one digest placeholder"
    );
    assert!(
        helper.entry.contains(&format!("\"./{}\"", helper.source)),
        "entrypoint for {} must import ./{}",
        helper.skill,
        helper.source
    );
    helper.entry.replace("__SOURCE_SHA256__", digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Give each build an isolated checkout-shaped fixture; never mutate the process environment or real skills.
    fn fixture(helper: &Helper) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let skill = root.path().join("agent-skills").join(helper.skill);
        fs::create_dir_all(&skill).unwrap();
        let source = skill.join(helper.source);
        fs::write(&source, "original").unwrap();
        let binary = skill.join("bin/linux-x64").join(helper.binary);
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, "previous").unwrap();
        (root, source, binary)
    }

    /// A successful build must bind the snapshotted bytes and replace the old executable only after compilation, for
    /// every helper in the table, each under its own source and binary names.
    #[test]
    fn publishes_snapshot_with_embedded_digest() {
        for helper in HELPERS {
            let (root, _, binary) = fixture(helper);
            build_with(
                root.path(),
                helper,
                TARGETS[0].triple,
                &|entry, output, _| {
                    assert_eq!(
                        fs::read_to_string(entry.with_file_name(helper.source))?,
                        "original"
                    );
                    let wrapper = fs::read_to_string(entry)?;
                    assert!(wrapper.contains(&format!("{:x}", Sha256::digest(b"original"))));
                    assert_eq!(fs::read_to_string(&binary)?, "previous");
                    fs::write(output, "new")?;
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(fs::read_to_string(binary).unwrap(), "new");
        }
    }

    /// Compiler failures must retain the previous artifact, even when the compiler left partial output behind.
    #[test]
    fn compiler_failure_preserves_previous_binary() {
        let (root, _, binary) = fixture(&HELPERS[0]);
        let result = build_with(
            root.path(),
            &HELPERS[0],
            TARGETS[0].triple,
            &|_, output, _| {
                fs::write(output, "unpublished")?;
                bail!("compiler failed")
            },
        );
        assert!(result.unwrap_err().to_string().contains("compiler failed"));
        assert_preserved(&binary);
    }

    /// A successful compiler cannot publish an artifact whose checkout source changed while it was running.
    #[test]
    fn changed_source_preserves_previous_binary() {
        let (root, source, binary) = fixture(&HELPERS[0]);
        let result = build_with(
            root.path(),
            &HELPERS[0],
            TARGETS[0].triple,
            &|_, output, _| {
                fs::write(output, "unpublished")?;
                fs::write(&source, "changed")?;
                Ok(())
            },
        );
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
        let (root, _, binary) = fixture(&HELPERS[0]);
        let result = build_with(root.path(), &HELPERS[0], TARGETS[0].triple, &|_, _, _| {
            Ok(())
        });
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

    /// One helper failing to build must not cost the others their build: the rest are still published, the failed one
    /// keeps its previous binary, and the call still fails so the caller notices.
    #[test]
    fn failed_helper_does_not_stop_the_others() {
        let root = tempfile::tempdir().unwrap();
        let mut binaries = Vec::new();
        for helper in HELPERS {
            let skill = root.path().join("agent-skills").join(helper.skill);
            let binary = skill.join("bin/linux-x64").join(helper.binary);
            fs::create_dir_all(binary.parent().unwrap()).unwrap();
            fs::write(skill.join(helper.source), "original").unwrap();
            fs::write(&binary, "previous").unwrap();
            binaries.push(binary);
        }
        let result = build_all(root.path(), TARGETS[0].triple, &|_, output, _| {
            if output.ends_with(HELPERS[0].binary) {
                bail!("compiler failed");
            }
            fs::write(output, "new")?;
            Ok(())
        });
        assert!(result.unwrap_err().to_string().contains(HELPERS[0].skill));
        assert_eq!(fs::read_to_string(&binaries[0]).unwrap(), "previous");
        assert_eq!(fs::read_to_string(&binaries[1]).unwrap(), "new");
    }

    /// Reject unsupported targets before touching the skill tree or invoking a compiler.
    #[test]
    fn unsupported_target_has_no_side_effects() {
        let root = tempfile::tempdir().unwrap();
        assert!(
            build_with(
                root.path(),
                &HELPERS[0],
                "x86_64-unknown-linux-musl",
                &|_, _, _| panic!("compiler called")
            )
            .is_err()
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    /// A linked output directory would publish into another tree, which the launcher deliberately refuses to execute.
    #[cfg(unix)]
    #[test]
    fn linked_output_directories_are_rejected_before_compilation() {
        for relative in ["bin", "bin/linux-x64"] {
            let (root, _, _) = fixture(&HELPERS[0]);
            let skill = root.path().join("agent-skills/persistent-review");
            let path = skill.join(relative);
            fs::remove_dir_all(&path).unwrap();
            let elsewhere = root.path().join("other-tree");
            fs::create_dir(&elsewhere).unwrap();
            std::os::unix::fs::symlink(&elsewhere, &path).unwrap();
            let result = build_with(root.path(), &HELPERS[0], TARGETS[0].triple, &|_, _, _| {
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

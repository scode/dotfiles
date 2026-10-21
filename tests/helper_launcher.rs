//! Exercise selection with controlled child environments; tests never change the test runner's environment.
#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tempfile::TempDir;

/// Model installed directory symlinks and spaces in paths, so selection cannot accidentally depend on cwd or splitting.
struct Fixture {
    _root: TempDir,
    skill: PathBuf,
    tools: PathBuf,
    launcher: PathBuf,
}

impl Fixture {
    /// Fake only platform discovery and the source interpreter; the launcher itself is the real installed file.
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let skill = root.path().join("checkout with spaces/persistent-review");
        let tools = root.path().join("tools");
        fs::create_dir_all(&skill).unwrap();
        let skill = fs::canonicalize(skill).unwrap();
        fs::create_dir_all(&tools).unwrap();
        fs::copy(
            "agent-skills/persistent-review/evidence",
            skill.join("evidence"),
        )
        .unwrap();
        fs::write(skill.join("evidence.ts"), "fixture source").unwrap();
        executable(
            &tools.join("uname"),
            "#!/bin/sh\nprintf 'Linux x86_64\\n'\n",
        );
        executable(&tools.join("deno"), "#!/bin/sh\nprintf '%s\\n' \"$@\"\n");
        let installed = root.path().join("installed skill");
        symlink(&skill, &installed).unwrap();
        Self {
            _root: root,
            skill,
            tools,
            launcher: installed.join("evidence"),
        }
    }

    /// Give the child only fixture tools and start elsewhere, matching a copied skill invoked by an agent.
    fn run(&self) -> Output {
        Command::new("/bin/sh")
            .arg(&self.launcher)
            .args(["read", "a path with spaces", "1", "2"])
            .current_dir("/")
            .env_clear()
            .env("PATH", &self.tools)
            .output()
            .unwrap()
    }

    /// Reserve the selected target path without fabricating an artifact at it.
    fn binary(&self) -> PathBuf {
        let path = self.skill.join("bin/linux-x64/evidence");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        path
    }
}

/// Explicit executable modes keep tests independent of the caller's umask.
fn executable(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// Without a matching artifact the source path and every caller argument must survive quoting and symlink resolution.
#[test]
fn absent_binary_uses_source_through_installed_directory_symlink() {
    let fixture = Fixture::new();
    let result = fixture.run();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        format!(
            "run\n--no-config\n--no-lock\n-A\n{}\nread\na path with spaces\n1\n2\n",
            fixture.skill.join("evidence.ts").display()
        )
    );
}

/// A selected artifact owns the invocation, including its failure status; source fallback must never rerun it.
#[test]
fn present_binary_is_preferred_and_its_status_is_preserved() {
    let fixture = Fixture::new();
    executable(
        &fixture.binary(),
        "#!/bin/sh\nprintf 'compiled\\n'\nprintf '%s\\n' \"$@\"\nexit 7\n",
    );
    let result = fixture.run();
    assert_eq!(result.status.code(), Some(7));
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "compiled\nread\na path with spaces\n1\n2\n"
    );
}

/// A missing execute bit is deployment damage, even when a working source interpreter is available.
#[test]
fn nonexecutable_artifacts_never_fall_back() {
    let fixture = Fixture::new();
    fs::write(fixture.binary(), "not executable").unwrap();
    fs::set_permissions(fixture.binary(), fs::Permissions::from_mode(0o644)).unwrap();
    let result = fixture.run();
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
}

/// Links beneath a resolved skill root must not redirect the compiled freshness check into a different tree.
#[test]
fn linked_binary_or_output_directories_never_select_another_trees_source() {
    for relative in ["bin", "bin/linux-x64", "bin/linux-x64/evidence"] {
        for dangling in [false, true] {
            let fixture = Fixture::new();
            let elsewhere = fixture.skill.parent().unwrap().join("other-tree");
            if !dangling {
                fs::create_dir(&elsewhere).unwrap();
            }
            let link = fixture.skill.join(relative);
            fs::create_dir_all(link.parent().unwrap()).unwrap();
            symlink(elsewhere, &link).unwrap();
            let result = fixture.run();
            assert_eq!(result.status.code(), Some(2));
            assert!(result.stdout.is_empty());
            assert!(String::from_utf8_lossy(&result.stderr).contains("must not be symlinks"));
        }
    }
}

/// Hosts without either execution path receive a useful failure rather than an interpreter-not-found surprise.
#[test]
fn missing_binary_and_deno_explain_how_to_recover() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.tools.join("deno")).unwrap();
    let result = fixture.run();
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("Build helpers before syncing"));
}

/// Failed platform discovery must not hide a matching stale artifact by taking the source fallback branch.
#[test]
fn platform_detection_failure_never_runs_deno() {
    let fixture = Fixture::new();
    executable(&fixture.tools.join("uname"), "#!/bin/sh\nexit 1\n");
    let result = fixture.run();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert!(String::from_utf8_lossy(&result.stderr).contains("could not determine"));
}

/// A global install-report option must not be silently accepted by an unrelated build operation.
#[test]
fn helper_build_rejects_install_status_flag_before_touching_the_tree() {
    let root = tempfile::tempdir().unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_dotfiles"))
        .args(["build-helpers", "--show-status"])
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("--show-status applies only"));
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

/// This opt-in test compiles a real native binary and proves stale/missing source fails before any CLI work.
/// It is separate from ordinary cargo tests because it needs Deno and may download a platform runtime.
#[test]
#[ignore = "requires Deno and its native compile runtime; run explicitly before shipping helper changes"]
fn compiled_helper_checks_deployed_source_without_a_deno_on_path() {
    let root = tempfile::tempdir().unwrap();
    let skill = root.path().join("agent-skills/persistent-review");
    fs::create_dir_all(&skill).unwrap();
    let source = fs::read("agent-skills/persistent-review/evidence.ts").unwrap();
    fs::write(skill.join("evidence.ts"), &source).unwrap();
    fs::copy(
        "agent-skills/persistent-review/evidence",
        skill.join("evidence"),
    )
    .unwrap();
    let build = Command::new(env!("CARGO_BIN_EXE_dotfiles"))
        .arg("build-helpers")
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let tools = root.path().join("only-uname");
    fs::create_dir(&tools).unwrap();
    symlink("/usr/bin/uname", tools.join("uname")).unwrap();
    let installed = root.path().join("installed skill with spaces");
    symlink(&skill, &installed).unwrap();
    let log = root.path().join("empty findings.jsonl");
    fs::write(&log, "").unwrap();
    let run = || {
        Command::new(installed.join("evidence"))
            .arg("findings")
            .arg(&log)
            .current_dir("/")
            .env_clear()
            .env("PATH", &tools)
            .output()
            .unwrap()
    };
    let valid = run();
    assert!(
        valid.status.success(),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&valid.stdout).unwrap()["active"],
        0
    );
    fs::write(skill.join("evidence.ts"), "changed").unwrap();
    let stale = run();
    assert_eq!(stale.status.code(), Some(2));
    assert!(stale.stdout.is_empty());
    assert!(String::from_utf8_lossy(&stale.stderr).contains("Rebuild helpers"));
    fs::remove_file(skill.join("evidence.ts")).unwrap();
    let missing = run();
    assert_eq!(missing.status.code(), Some(2));
    assert!(missing.stdout.is_empty());
    fs::write(skill.join("evidence.ts"), &source).unwrap();
    assert!(run().status.success());
}

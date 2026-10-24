//! Exercise selection with controlled child environments; tests never change the test runner's environment. Every
//! launcher test runs against each helper's launcher, since they must all follow the same selection rules.
#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tempfile::TempDir;

/// A skill with a compiled helper: its directory under `agent-skills/`, its launcher (which is also the binary's name
/// under `bin/<platform>/`), and the TypeScript source the launcher falls back to.
struct Helper {
    skill: &'static str,
    launcher: &'static str,
    source: &'static str,
}

const HELPERS: &[Helper] = &[
    Helper {
        skill: "persistent-review",
        launcher: "evidence",
        source: "evidence.ts",
    },
    Helper {
        skill: "interactive-artifact",
        launcher: "interactive-artifact",
        source: "interactive-artifact.ts",
    },
];

/// Model installed directory symlinks and spaces in paths, so selection cannot accidentally depend on cwd or splitting.
struct Fixture {
    _root: TempDir,
    skill: PathBuf,
    tools: PathBuf,
    launcher: PathBuf,
    helper: &'static Helper,
}

impl Fixture {
    /// Fake only platform discovery and the source interpreter; the launcher itself is the real installed file.
    fn new(helper: &'static Helper) -> Self {
        let root = tempfile::tempdir().unwrap();
        let skill = root.path().join("checkout with spaces").join(helper.skill);
        let tools = root.path().join("tools");
        fs::create_dir_all(&skill).unwrap();
        let skill = fs::canonicalize(skill).unwrap();
        fs::create_dir_all(&tools).unwrap();
        fs::copy(
            format!("agent-skills/{}/{}", helper.skill, helper.launcher),
            skill.join(helper.launcher),
        )
        .unwrap();
        fs::write(skill.join(helper.source), "fixture source").unwrap();
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
            launcher: installed.join(helper.launcher),
            helper,
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
        let path = self.skill.join("bin/linux-x64").join(self.helper.launcher);
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
    for helper in HELPERS {
        let fixture = Fixture::new(helper);
        let result = fixture.run();
        assert!(
            result.status.success(),
            "{}: {}",
            helper.skill,
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            String::from_utf8(result.stdout).unwrap(),
            format!(
                "run\n--no-config\n--no-lock\n-A\n{}\nread\na path with spaces\n1\n2\n",
                fixture.skill.join(helper.source).display()
            )
        );
    }
}

/// A selected artifact owns the invocation, including its failure status; source fallback must never rerun it.
#[test]
fn present_binary_is_preferred_and_its_status_is_preserved() {
    for helper in HELPERS {
        let fixture = Fixture::new(helper);
        executable(
            &fixture.binary(),
            "#!/bin/sh\nprintf 'compiled\\n'\nprintf '%s\\n' \"$@\"\nexit 7\n",
        );
        let result = fixture.run();
        assert_eq!(result.status.code(), Some(7), "{}", helper.skill);
        assert_eq!(
            String::from_utf8(result.stdout).unwrap(),
            "compiled\nread\na path with spaces\n1\n2\n"
        );
    }
}

/// A missing execute bit is deployment damage, even when a working source interpreter is available.
#[test]
fn nonexecutable_artifacts_never_fall_back() {
    for helper in HELPERS {
        let fixture = Fixture::new(helper);
        fs::write(fixture.binary(), "not executable").unwrap();
        fs::set_permissions(fixture.binary(), fs::Permissions::from_mode(0o644)).unwrap();
        let result = fixture.run();
        assert!(!result.status.success(), "{}", helper.skill);
        assert!(result.stdout.is_empty());
    }
}

/// Links beneath a resolved skill root must not redirect the compiled freshness check into a different tree.
#[test]
fn linked_binary_or_output_directories_never_select_another_trees_source() {
    for helper in HELPERS {
        let binary = format!("bin/linux-x64/{}", helper.launcher);
        for relative in ["bin", "bin/linux-x64", binary.as_str()] {
            for dangling in [false, true] {
                let fixture = Fixture::new(helper);
                let elsewhere = fixture.skill.parent().unwrap().join("other-tree");
                if !dangling {
                    fs::create_dir(&elsewhere).unwrap();
                }
                let link = fixture.skill.join(relative);
                fs::create_dir_all(link.parent().unwrap()).unwrap();
                symlink(elsewhere, &link).unwrap();
                let result = fixture.run();
                assert_eq!(result.status.code(), Some(2), "{} {relative}", helper.skill);
                assert!(result.stdout.is_empty());
                assert!(String::from_utf8_lossy(&result.stderr).contains("must not be symlinks"));
            }
        }
    }
}

/// Hosts without either execution path receive a useful failure rather than an interpreter-not-found surprise.
#[test]
fn missing_binary_and_deno_explain_how_to_recover() {
    for helper in HELPERS {
        let fixture = Fixture::new(helper);
        fs::remove_file(fixture.tools.join("deno")).unwrap();
        let result = fixture.run();
        assert_eq!(result.status.code(), Some(2), "{}", helper.skill);
        assert!(String::from_utf8_lossy(&result.stderr).contains("Build helpers before syncing"));
    }
}

/// Failed platform discovery must not hide a matching stale artifact by taking the source fallback branch.
#[test]
fn platform_detection_failure_never_runs_deno() {
    for helper in HELPERS {
        let fixture = Fixture::new(helper);
        executable(&fixture.tools.join("uname"), "#!/bin/sh\nexit 1\n");
        let result = fixture.run();
        assert_eq!(result.status.code(), Some(2), "{}", helper.skill);
        assert!(result.stdout.is_empty());
        assert!(String::from_utf8_lossy(&result.stderr).contains("could not determine"));
    }
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

/// A checkout-shaped copy of everything `build-helpers` reads and the helpers need at run time, for the opt-in compiled
/// tests. Both skills are always present because `build-helpers` builds every helper and fails if any source is missing.
fn helper_checkout() -> TempDir {
    let root = tempfile::tempdir().unwrap();
    for file in [
        "persistent-review/evidence",
        "persistent-review/evidence.ts",
        "interactive-artifact/interactive-artifact",
        "interactive-artifact/interactive-artifact.ts",
        "interactive-artifact/assets/interactive-artifact.css",
        "interactive-artifact/assets/interactive-artifact.js",
    ] {
        let to = root.path().join("agent-skills").join(file);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(format!("agent-skills/{file}"), to).unwrap();
    }
    root
}

/// This opt-in test compiles a real native binary and proves stale/missing source fails before any CLI work.
/// It is separate from ordinary cargo tests because it needs Deno and may download a platform runtime.
#[test]
#[ignore = "requires Deno and its native compile runtime; run explicitly before shipping helper changes"]
fn compiled_helper_checks_deployed_source_without_a_deno_on_path() {
    let root = helper_checkout();
    let skill = root.path().join("agent-skills/persistent-review");
    let source = fs::read("agent-skills/persistent-review/evidence.ts").unwrap();
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

/// The interactive-artifact counterpart of the test above, opt-in for the same reason. Beyond the freshness check, it
/// proves what is specific to this helper: the binary finds the skill's `assets/` beside the launcher (inside the binary,
/// `import.meta.url` names the build snapshot, so the entrypoint has to pass the skill directory in), by serving a page
/// with no Deno on PATH and fetching its stylesheet.
#[test]
#[ignore = "requires Deno and its native compile runtime; run explicitly before shipping helper changes"]
fn compiled_interactive_artifact_serves_its_assets_and_refuses_stale_source() {
    use std::io::{BufRead, BufReader, Read, Write};
    let root = helper_checkout();
    let skill = root.path().join("agent-skills/interactive-artifact");
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
    let launcher = installed.join("interactive-artifact");
    let command = |args: &[&str]| {
        let mut c = Command::new(&launcher);
        c.args(args)
            .current_dir("/")
            .env_clear()
            .env("PATH", &tools);
        c
    };
    assert_eq!(
        command(&["--help"]).output().unwrap().status.code(),
        Some(0)
    );

    // A page in a private directory, which serve insists on; the mode is set because tempdir's depends on the umask.
    let pages = tempfile::tempdir().unwrap();
    fs::set_permissions(pages.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let page = pages.path().join("page.html");
    fs::write(&page, "<h1>hi</h1>").unwrap();
    let mut server = command(&["serve", page.to_str().unwrap(), "--ttl", "1m"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut url = String::new();
    BufReader::new(server.stdout.take().unwrap())
        .read_line(&mut url)
        .unwrap();
    let rest = url.trim().strip_prefix("http://").unwrap();
    let (addr, path) = rest.split_once('/').unwrap();
    let mut stream = std::net::TcpStream::connect(addr).unwrap();
    write!(
        stream,
        "GET /{path}__interactive-artifact/interactive-artifact.css HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    server.kill().unwrap();
    server.wait().unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.contains(":root"));

    fs::write(skill.join("interactive-artifact.ts"), "changed").unwrap();
    let stale = command(&["--help"]).output().unwrap();
    assert_eq!(stale.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&stale.stderr).contains("Rebuild helpers"));
}

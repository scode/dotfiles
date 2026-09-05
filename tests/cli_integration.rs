use std::process::Command;
use tempfile::TempDir;

fn setup_fake_home() -> TempDir {
    let home = setup_fake_home_without_graphite_source();
    // Optional RawSymlink sources are present in the normal fake home, so the
    // conditional external skills install and the cross-harness comparison
    // below covers them; a source missing here would make a one-sided
    // registration of that skill invisible to the test.
    std::fs::create_dir_all(home.path().join("git/scode-graphite-skill")).unwrap();
    std::fs::create_dir_all(home.path().join("git/voice")).unwrap();
    home
}

fn setup_fake_home_without_graphite_source() -> TempDir {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".config/zed")).unwrap();
    std::fs::create_dir_all(home.path().join(".claude")).unwrap();
    std::fs::create_dir_all(home.path().join(".codex")).unwrap();
    std::fs::create_dir_all(home.path().join(".config/muse")).unwrap();
    std::fs::create_dir_all(home.path().join(".config/opencode")).unwrap();
    std::fs::create_dir_all(
        home.path()
            .join("Library/Application Support/com.mitchellh.ghostty"),
    )
    .unwrap();
    home
}

/// The skills roots of every harness the installer targets, relative to the
/// fake home. Tests that assert "the same skills everywhere" iterate this so
/// a fifth harness only has to be added here.
const SKILLS_ROOTS: &[&str] = &[
    ".claude/skills",
    ".codex/skills",
    ".config/muse/skills",
    ".config/opencode/skills",
];

/// The entries directly under a skills root as (name, resolved target),
/// sorted by name. Resolving the target is what makes a cross-harness
/// comparison meaningful: two roots can list the same names while one of
/// them points a name at the wrong source directory, and the relative link
/// text differs per root depth, so only the canonical path is comparable.
/// A dangling link fails to resolve and panics here, which is the right
/// outcome for a test that asks whether every harness has usable skills.
fn skills_root_entries(home: &TempDir, root: &str) -> Vec<(String, std::path::PathBuf)> {
    let mut entries: Vec<(String, std::path::PathBuf)> = std::fs::read_dir(home.path().join(root))
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().into_owned();
            let target = std::fs::canonicalize(entry.path())
                .unwrap_or_else(|e| panic!("{root}/{name} does not resolve: {e}"));
            (name, target)
        })
        .collect();
    entries.sort();
    entries
}

fn read_json(home: &TempDir, relative_path: &str) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(home.path().join(relative_path)).unwrap())
        .unwrap()
}

const EXPECTED_CLAUDE_ALLOW: &[&str] = &[
    "Bash(cargo build)",
    "Bash(cargo clippy:*)",
    "Bash(cargo fmt:*)",
    "Bash(codex exec*)",
    "Bash(dprint check:*)",
    "Bash(dprint fmt:*)",
    "Bash(gemini -s -p*)",
    "Bash(git diff*)",
    "Bash(gh run list*)",
    "Bash(gh run view*)",
    "Bash(gh run watch*)",
    "Bash(git show*)",
    "Bash(gt add:*)",
    "Bash(gt create:*)",
    "Bash(gt log:*)",
    "Bash(gt restack:*)",
    "Bash(gt submit:*)",
    "Bash(gt sync:*)",
    "Bash(stax:*)",
    "Bash(gh pr:*)",
    "Bash(sl:*)",
    "Skill(scode-graphite)",
    "WebFetch(domain:index.crates.io)",
];

fn assert_expected_claude_allow(settings: &serde_json::Value) {
    let allow = settings["permissions"]["allow"].as_array().unwrap();
    for expected in EXPECTED_CLAUDE_ALLOW {
        assert!(
            allow.iter().any(|entry| entry == expected),
            "expected permissions.allow to contain {expected}"
        );
    }
}

/// Extracts the text between a managed block's BEGIN and END marker lines.
///
/// `begin` and `end` are the marker keys; the BEGIN line carries trailing
/// notice text, so the body starts at the newline after the key rather than
/// immediately after it. The END line is excluded. Panics when either marker is
/// missing, which is the caller's assertion to make first.
fn managed_block_body(contents: &str, begin: &str, end: &str) -> String {
    let begin_at = contents.find(begin).expect("BEGIN marker");
    let body_start = begin_at + contents[begin_at..].find('\n').expect("BEGIN line end") + 1;
    let end_at = contents[body_start..].find(end).expect("END marker") + body_start;
    contents[body_start..end_at].to_string()
}

fn assert_no_leiter_settings(settings: &serde_json::Value) {
    let rendered = serde_json::to_string(settings).unwrap();
    assert!(
        !rendered.contains("leiter"),
        "settings should not contain leiter references: {rendered}"
    );
    assert!(
        !rendered.contains("soul.md"),
        "settings should not contain soul.md references: {rendered}"
    );
}

fn assert_skill_symlink_points_to_repo_source(home: &TempDir, link_path: &str, source_path: &str) {
    let full_path = home.path().join(link_path);
    assert!(full_path.is_symlink(), "expected {link_path} symlink");

    let target = std::fs::read_link(&full_path).unwrap();
    assert!(
        target.to_string_lossy().contains(source_path),
        "{link_path} should point at {source_path}, got: {:?}",
        target
    );
}

#[test]
fn test_install_creates_symlinks() {
    let fake_home = setup_fake_home();

    let output = Command::new(env!("CARGO_BIN_EXE_dotfiles"))
        .arg("install")
        .env("HOME", fake_home.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify the Claude markdown symlink and managed settings file were created
    let claude_md = fake_home.path().join(".claude/CLAUDE.md");
    assert!(claude_md.is_symlink(), "expected .claude/CLAUDE.md symlink");

    let claude_settings = fake_home.path().join(".claude/settings.json");
    assert!(
        claude_settings.is_file(),
        "expected .claude/settings.json file"
    );
    assert!(
        !claude_settings.is_symlink(),
        "expected .claude/settings.json to be a regular file"
    );
    let settings = read_json(&fake_home, ".claude/settings.json");
    assert_eq!(settings["sandbox"]["enabled"], serde_json::json!(true));
    assert_eq!(
        settings["statusLine"]["command"],
        serde_json::json!("~/bin/claude-statusline.sh")
    );
    assert_expected_claude_allow(&settings);

    // The managed shell blocks are the only registrations that write a plain
    // text file the installer does not own, so it is worth confirming that the
    // wiring in main.rs reaches a real destination rather than only that the
    // feature works in isolation. Both shells are checked because they are
    // separate registrations sharing one payload: a regression that drops or
    // retargets one of them would not show up in the other.
    // The markers alone only prove the registration exists; the body check is
    // what proves it was wired to the shared payload rather than to some other
    // readable file.
    let payload = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("payload/shellrc"),
    )
    .unwrap();
    for (file, id) in [(".bashrc", "bash"), (".zshrc", "zsh")] {
        let rc = fake_home.path().join(file);
        assert!(
            rc.is_file() && !rc.is_symlink(),
            "expected ~/{file} to be created as a regular file"
        );
        let contents = std::fs::read_to_string(&rc).unwrap();
        let begin = format!("# BEGIN managed-block(scode-dotfiles/{id})");
        let end = format!("# END managed-block(scode-dotfiles/{id})");
        assert!(
            contents.contains(&begin) && contents.contains(&end),
            "expected managed block markers in ~/{file}, got: {contents}"
        );
        assert_eq!(
            managed_block_body(&contents, &begin, &end),
            payload,
            "~/{file} block body should be payload/shellrc verbatim"
        );
    }

    // Verify it points to the shared agent-instructions source with a relative path.
    let target = std::fs::read_link(&claude_md).unwrap();
    assert!(
        target.ends_with("agent-instructions/AGENTS.md"),
        "symlink should point to agent-instructions/AGENTS.md"
    );
    assert!(
        target.is_relative(),
        "symlink should be relative, got: {:?}",
        target
    );

    // Verify every harness gets the same repo-owned skill symlinks.
    for skill in [
        "pre-pr-review-swarm",
        "scode-dist-rust-setup",
        "scode-modernize",
        "scode-chores",
        "scode-todo",
        "repo-swarm",
        "stax",
        "slstack",
        "jjstack",
        "sapling",
    ] {
        for root in SKILLS_ROOTS {
            assert_skill_symlink_points_to_repo_source(
                &fake_home,
                &format!("{root}/{skill}"),
                &format!("agent-skills/{skill}"),
            );
        }
    }
}

/// Every harness must end up with exactly the same set of skills, each name
/// resolving to the same source directory.
///
/// Claude and Codex list their skill entries one by one in `src/main.rs`
/// (their sections carry migration cleanups), while Muse and OpenCode derive
/// theirs from a shared list. Nothing but this test ties the two spellings
/// together, so a skill added to one and forgotten in the other would install
/// for some harnesses only and nobody would notice until a dependency load
/// failed on the harness that lacked it. Comparing resolved targets rather
/// than names also catches an explicit entry whose destination name is right
/// but whose source is a different skill.
#[test]
fn test_all_harnesses_install_the_same_skills() {
    let fake_home = setup_fake_home();

    let output = Command::new(env!("CARGO_BIN_EXE_dotfiles"))
        .arg("install")
        .env("HOME", fake_home.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let reference = skills_root_entries(&fake_home, SKILLS_ROOTS[0]);
    let names: Vec<&str> = reference.iter().map(|(name, _)| name.as_str()).collect();
    assert!(
        names.contains(&"scode-galaxy-brain"),
        "sanity: the reference root should contain a known repo skill, got {names:?}"
    );
    assert!(
        names.contains(&"scode-voice"),
        "sanity: the reference root should contain a known external skill, got {names:?}"
    );
    for root in &SKILLS_ROOTS[1..] {
        assert_eq!(
            skills_root_entries(&fake_home, root),
            reference,
            "{root} should hold exactly the same skills as {}",
            SKILLS_ROOTS[0]
        );
    }
}

#[test]
fn test_uninstall_removes_symlinks() {
    let fake_home = setup_fake_home();

    // Install first
    let install_output = Command::new(env!("CARGO_BIN_EXE_dotfiles"))
        .arg("install")
        .env("HOME", fake_home.path())
        .output()
        .unwrap();
    assert!(
        install_output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&install_output.stderr)
    );

    // Verify installed Claude artifacts exist before uninstall
    let claude_md = fake_home.path().join(".claude/CLAUDE.md");
    let claude_settings = fake_home.path().join(".claude/settings.json");
    let codex_stax_skill = fake_home.path().join(".codex/skills/stax");
    let codex_slstack_skill = fake_home.path().join(".codex/skills/slstack");
    let claude_jjstack_skill = fake_home.path().join(".claude/skills/jjstack");
    let codex_jjstack_skill = fake_home.path().join(".codex/skills/jjstack");
    let codex_dist_skill = fake_home.path().join(".codex/skills/scode-dist-rust-setup");
    let claude_repo_swarm_skill = fake_home.path().join(".claude/skills/repo-swarm");
    let codex_repo_swarm_skill = fake_home.path().join(".codex/skills/repo-swarm");
    assert!(claude_md.is_symlink(), "symlink should exist after install");
    assert!(
        claude_settings.is_file() && !claude_settings.is_symlink(),
        "settings file should exist after install"
    );
    assert!(
        codex_stax_skill.is_symlink(),
        "codex stax skill symlink should exist after install"
    );
    assert!(
        codex_slstack_skill.is_symlink(),
        "codex slstack skill symlink should exist after install"
    );
    assert!(
        claude_jjstack_skill.is_symlink(),
        "claude jjstack skill symlink should exist after install"
    );
    assert!(
        codex_jjstack_skill.is_symlink(),
        "codex jjstack skill symlink should exist after install"
    );
    assert!(
        codex_dist_skill.is_symlink(),
        "codex dist skill symlink should exist after install"
    );
    assert!(
        claude_repo_swarm_skill.is_symlink(),
        "claude repo-swarm skill symlink should exist after install"
    );
    assert!(
        codex_repo_swarm_skill.is_symlink(),
        "codex repo-swarm skill symlink should exist after install"
    );

    // Then uninstall
    let output = Command::new(env!("CARGO_BIN_EXE_dotfiles"))
        .arg("uninstall")
        .env("HOME", fake_home.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "uninstall failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify uninstall removes symlinks, and that the managed settings file
    // survives as a user-owned file with the managed footprint reverted. A
    // fresh install wrote only managed values, so reverting leaves exactly {}.
    assert!(
        !claude_md.exists() && !claude_md.is_symlink(),
        "symlink should be removed after uninstall"
    );
    assert!(
        claude_settings.is_file() && !claude_settings.is_symlink(),
        "settings file should be left in place after uninstall"
    );
    let settings_after = read_json(&fake_home, ".claude/settings.json");
    assert_eq!(
        settings_after,
        serde_json::json!({}),
        "managed settings values should be reverted on uninstall"
    );

    // Uninstall takes back the block but not the file. The file was created by
    // this installer here, but that is an accident of a fresh fake home; on a
    // real machine ~/.bashrc or ~/.zshrc is the user's, and the uninstall path
    // cannot tell the two apart.
    for (file, id) in [(".bashrc", "bash"), (".zshrc", "zsh")] {
        let rc = fake_home.path().join(file);
        assert!(
            rc.is_file(),
            "~/{file} should be left in place after uninstall"
        );
        let after = std::fs::read_to_string(&rc).unwrap();
        assert!(
            !after.contains(&format!("managed-block(scode-dotfiles/{id})")),
            "managed block should be removed from ~/{file}, got: {after}"
        );
    }
    assert!(
        !codex_stax_skill.exists() && !codex_stax_skill.is_symlink(),
        "codex stax skill symlink should be removed after uninstall"
    );
    assert!(
        !codex_slstack_skill.exists() && !codex_slstack_skill.is_symlink(),
        "codex slstack skill symlink should be removed after uninstall"
    );
    assert!(
        !claude_jjstack_skill.exists() && !claude_jjstack_skill.is_symlink(),
        "claude jjstack skill symlink should be removed after uninstall"
    );
    assert!(
        !codex_jjstack_skill.exists() && !codex_jjstack_skill.is_symlink(),
        "codex jjstack skill symlink should be removed after uninstall"
    );
    assert!(
        !codex_dist_skill.exists() && !codex_dist_skill.is_symlink(),
        "codex dist skill symlink should be removed after uninstall"
    );
    assert!(
        !claude_repo_swarm_skill.exists() && !claude_repo_swarm_skill.is_symlink(),
        "claude repo-swarm skill symlink should be removed after uninstall"
    );
    assert!(
        !codex_repo_swarm_skill.exists() && !codex_repo_swarm_skill.is_symlink(),
        "codex repo-swarm skill symlink should be removed after uninstall"
    );
}

#[test]
fn test_install_idempotent() {
    let fake_home = setup_fake_home();

    // Install twice
    for i in 0..2 {
        let output = Command::new(env!("CARGO_BIN_EXE_dotfiles"))
            .arg("install")
            .env("HOME", fake_home.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "install #{} failed: {}",
            i + 1,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Symlinks still work
    let claude_md = fake_home.path().join(".claude/CLAUDE.md");
    let claude_settings = fake_home.path().join(".claude/settings.json");
    assert!(
        claude_md.is_symlink(),
        "symlink should exist after double install"
    );
    assert!(
        claude_settings.is_file() && !claude_settings.is_symlink(),
        "settings file should remain a regular file after double install"
    );
    assert!(
        fake_home
            .path()
            .join(".codex/skills/pre-pr-review-swarm")
            .is_symlink(),
        "codex skill symlink should exist after double install"
    );
    assert!(
        fake_home.path().join(".codex/skills/stax").is_symlink(),
        "codex stax skill symlink should exist after double install"
    );
    assert!(
        fake_home.path().join(".codex/skills/slstack").is_symlink(),
        "codex slstack skill symlink should exist after double install"
    );
    assert!(
        fake_home.path().join(".claude/skills/jjstack").is_symlink(),
        "claude jjstack skill symlink should exist after double install"
    );
    assert!(
        fake_home.path().join(".codex/skills/jjstack").is_symlink(),
        "codex jjstack skill symlink should exist after double install"
    );
    assert!(
        fake_home
            .path()
            .join(".codex/skills/scode-dist-rust-setup")
            .is_symlink(),
        "codex dist skill symlink should exist after double install"
    );
    assert!(
        fake_home
            .path()
            .join(".claude/skills/repo-swarm")
            .is_symlink(),
        "claude repo-swarm skill symlink should exist after double install"
    );
    assert!(
        fake_home
            .path()
            .join(".codex/skills/repo-swarm")
            .is_symlink(),
        "codex repo-swarm skill symlink should exist after double install"
    );
}

#[test]
fn test_conditional_features_skipped_when_parent_missing() {
    let fake_home = tempfile::tempdir().unwrap();
    // Don't create .config/zed, .claude, or .codex - those features should be skipped
    // But do create ghostty's parent since it's unconditional
    std::fs::create_dir_all(
        fake_home
            .path()
            .join("Library/Application Support/com.mitchellh.ghostty"),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dotfiles"))
        .arg("install")
        .env("HOME", fake_home.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Conditional features should NOT have created symlinks
    assert!(
        !fake_home.path().join(".claude/CLAUDE.md").exists(),
        ".claude/CLAUDE.md should not exist when .claude doesn't exist"
    );
    assert!(
        !fake_home.path().join(".config/zed/keymap.json").exists(),
        ".config/zed/keymap.json should not exist when .config/zed doesn't exist"
    );
    assert!(
        !fake_home
            .path()
            .join(".codex/agents/code-review-specialist.md")
            .exists(),
        ".codex agent should not exist when .codex doesn't exist"
    );
    // Each harness's skills root is gated on that harness's own config
    // directory, never on another harness's; with no config directories at
    // all, no skills root may appear.
    for root in SKILLS_ROOTS {
        assert!(
            !fake_home.path().join(root).exists(),
            "{root} should not exist when its harness config directory doesn't exist"
        );
    }

    // Unconditional feature should still work
    let ghostty = fake_home
        .path()
        .join("Library/Application Support/com.mitchellh.ghostty/config");
    assert!(ghostty.is_symlink(), "ghostty config should be symlinked");
}

#[test]
fn test_graphite_skill_is_skipped_when_source_missing() {
    let fake_home = setup_fake_home_without_graphite_source();

    let output = Command::new(env!("CARGO_BIN_EXE_dotfiles"))
        .arg("install")
        .env("HOME", fake_home.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    for root in SKILLS_ROOTS {
        let relative_path = format!("{root}/scode-graphite");
        assert!(
            fake_home
                .path()
                .join(&relative_path)
                .symlink_metadata()
                .is_err(),
            "{relative_path} should be skipped when the source checkout is missing"
        );
    }
}

#[test]
fn test_dependency_ordering() {
    let fake_home = setup_fake_home();

    let output = Command::new(env!("CARGO_BIN_EXE_dotfiles"))
        .arg("install")
        .env("HOME", fake_home.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify directories were created only for artifacts the installer still manages.
    assert!(
        fake_home.path().join(".claude/commands").is_dir(),
        ".claude/commands should be a directory"
    );
    assert!(
        !fake_home.path().join(".claude/agents").exists(),
        ".claude/agents should not be created without installed agents"
    );
    assert!(
        fake_home.path().join(".claude/skills").is_dir(),
        ".claude/skills should be a directory"
    );
    assert!(
        !fake_home.path().join(".codex/agents").exists(),
        ".codex/agents should not be created without installed agents"
    );
    assert!(
        fake_home.path().join(".codex/skills").is_dir(),
        ".codex/skills should be a directory"
    );

    // Verify managed symlinks exist and removed agent symlinks stay absent.
    assert!(
        fake_home
            .path()
            .join(".claude/commands/review-for-quality.md")
            .is_symlink(),
        "command symlink should exist"
    );
    assert!(
        !fake_home
            .path()
            .join(".claude/agents/code-review-specialist.md")
            .exists(),
        "removed claude agent symlink should not be installed"
    );
    assert!(
        fake_home
            .path()
            .join(".claude/skills/pre-pr-review-swarm")
            .is_symlink(),
        "skill symlink should exist"
    );
    assert!(
        fake_home
            .path()
            .join(".claude/skills/scode-dist-rust-setup")
            .is_symlink(),
        "claude dist skill symlink should exist"
    );
    assert!(
        fake_home.path().join(".claude/skills/jjstack").is_symlink(),
        "claude jjstack skill symlink should exist"
    );
    assert!(
        !fake_home
            .path()
            .join(".codex/agents/code-review-specialist.md")
            .exists(),
        "removed codex agent symlink should not be installed"
    );
    assert!(
        fake_home
            .path()
            .join(".codex/skills/pre-pr-review-swarm")
            .is_symlink(),
        "codex skill symlink should exist"
    );
    assert!(
        fake_home.path().join(".codex/skills/stax").is_symlink(),
        "codex stax skill symlink should exist"
    );
    assert!(
        fake_home.path().join(".codex/skills/slstack").is_symlink(),
        "codex slstack skill symlink should exist"
    );
    assert!(
        fake_home.path().join(".codex/skills/jjstack").is_symlink(),
        "codex jjstack skill symlink should exist"
    );
    assert!(
        fake_home
            .path()
            .join(".codex/skills/scode-dist-rust-setup")
            .is_symlink(),
        "codex dist skill symlink should exist"
    );
    assert!(
        fake_home
            .path()
            .join(".claude/skills/repo-swarm")
            .is_symlink(),
        "claude repo-swarm skill symlink should exist"
    );
    assert!(
        fake_home
            .path()
            .join(".codex/skills/repo-swarm")
            .is_symlink(),
        "codex repo-swarm skill symlink should exist"
    );
}

#[test]
fn test_all_symlinks_are_relative() {
    let fake_home = setup_fake_home();

    let output = Command::new(env!("CARGO_BIN_EXE_dotfiles"))
        .arg("install")
        .env("HOME", fake_home.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check several symlinks are relative
    let symlinks_to_check = [
        ".claude/CLAUDE.md",
        ".claude/skills/pre-pr-review-swarm",
        ".claude/skills/scode-dist-rust-setup",
        ".claude/skills/jjstack",
        ".claude/skills/repo-swarm",
        ".codex/skills/pre-pr-review-swarm",
        ".codex/skills/stax",
        ".codex/skills/slstack",
        ".codex/skills/jjstack",
        ".codex/skills/scode-dist-rust-setup",
        ".codex/skills/repo-swarm",
        ".config/zed/keymap.json",
        "Library/Application Support/com.mitchellh.ghostty/config",
    ];

    for path in symlinks_to_check {
        let full_path = fake_home.path().join(path);
        if full_path.is_symlink() {
            let target = std::fs::read_link(&full_path).unwrap();
            assert!(
                target.is_relative(),
                "{} symlink should be relative, got: {:?}",
                path,
                target
            );
        }
    }
}

/// The settings fixture stacks every historical leiter shape into one file:
/// absolute-path and ~ permission spellings, plus all three hook command
/// generations. No real machine ever had them all at once, but this proves
/// each exact-match removal value in main.rs actually matches something.
#[test]
fn test_install_removes_legacy_leiter_claude_settings_from_regular_file() {
    let fake_home = setup_fake_home();
    let settings_path = fake_home.path().join(".claude/settings.json");
    std::fs::write(
        &settings_path,
        r#"{
  "permissions": {
    "allow": [
      "Bash(custom:*)",
      "Bash(leiter:*)",
      "Read(~/.leiter/soul.md)",
      "Edit(~/.leiter/soul.md)",
      "Write(~/.leiter/soul.md)",
      "Edit(/Users/scode/.leiter/soul.md)",
      "Write(/Users/scode/.leiter/soul.md)"
    ]
  },
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "leiter context"
          },
          {
            "type": "command",
            "command": "leiter nudge"
          }
        ]
      },
      {
        "hooks": [
          {
            "type": "command",
            "command": "leiter hook context"
          },
          {
            "type": "command",
            "command": "leiter hook nudge"
          }
        ]
      },
      {
        "hooks": [
          {
            "type": "command",
            "command": "leiter hook context"
          },
          {
            "type": "command",
            "command": "leiter hook nudge --auto-distill"
          }
        ]
      },
      {
        "hooks": [
          {
            "type": "command",
            "command": "custom hook"
          }
        ]
      }
    ],
    "SessionEnd": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "leiter session-end"
          }
        ]
      },
      {
        "hooks": [
          {
            "type": "command",
            "command": "leiter hook session-end"
          }
        ]
      }
    ]
  }
}"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dotfiles"))
        .arg("install")
        .env("HOME", fake_home.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let settings = read_json(&fake_home, ".claude/settings.json");
    assert_no_leiter_settings(&settings);
    assert_expected_claude_allow(&settings);
    assert!(
        settings["permissions"]["allow"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry == "Bash(custom:*)"),
        "unmanaged permissions should be preserved"
    );
    assert_eq!(
        settings["hooks"]["SessionStart"],
        serde_json::json!([
            {
                "hooks": [
                    {
                        "type": "command",
                        "command": "custom hook"
                    }
                ]
            }
        ])
    );
    assert!(settings["hooks"].get("SessionEnd").is_none());
}

/// Uninstall straight from an un-migrated legacy settings symlink, without an
/// install first. This pins two behaviors at once: the symlink is removed
/// outright (the repo-owned symlink contract), and the run succeeds even
/// though claude-settings-statusline reverts before claude-settings-base —
/// both features must recognize the legacy symlink, or the first one fails
/// the run with "unexpected target" (failed features make the binary exit
/// nonzero).
#[test]
fn test_uninstall_removes_unmigrated_legacy_claude_settings_symlink() {
    let fake_home = setup_fake_home();
    let settings_path = fake_home.path().join(".claude/settings.json");
    let settings_dir = settings_path.parent().unwrap();
    let legacy_target = dotfiles::util::fs::compute_relative_path(
        settings_dir,
        &std::env::current_dir()
            .unwrap()
            .join("payload/dot_claude/settings.json"),
    );
    std::os::unix::fs::symlink(&legacy_target, &settings_path).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dotfiles"))
        .arg("uninstall")
        .env("HOME", fake_home.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "uninstall failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !settings_path.exists() && !settings_path.is_symlink(),
        "legacy settings symlink should be removed by uninstall"
    );
}

#[test]
fn test_install_migrates_legacy_claude_settings_symlink() {
    let fake_home = setup_fake_home();
    let settings_path = fake_home.path().join(".claude/settings.json");
    let settings_dir = settings_path.parent().unwrap();
    let legacy_target = dotfiles::util::fs::compute_relative_path(
        settings_dir,
        &std::env::current_dir()
            .unwrap()
            .join("payload/dot_claude/settings.json"),
    );
    std::os::unix::fs::symlink(&legacy_target, &settings_path).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dotfiles"))
        .arg("install")
        .env("HOME", fake_home.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        settings_path.is_file() && !settings_path.is_symlink(),
        "legacy symlink should be replaced with a regular file"
    );

    let settings = read_json(&fake_home, ".claude/settings.json");
    assert!(
        settings.get("hooks").is_none(),
        "legacy unmanaged keys should be dropped"
    );
    assert!(
        settings.get("enabledPlugins").is_none(),
        "legacy unmanaged keys should be dropped"
    );
    assert_eq!(settings["sandbox"]["enabled"], serde_json::json!(true));
    assert_eq!(
        settings["statusLine"]["command"],
        serde_json::json!("~/bin/claude-statusline.sh")
    );
    assert_expected_claude_allow(&settings);
    assert_no_leiter_settings(&settings);
}

#[test]
fn test_install_repoints_legacy_agent_instruction_symlinks() {
    let fake_home = setup_fake_home();
    let legacy_links = [
        fake_home.path().join(".claude/CLAUDE.md"),
        fake_home.path().join(".codex/AGENTS.md"),
    ];

    for legacy_link in &legacy_links {
        let legacy_target = dotfiles::util::fs::compute_relative_path(
            legacy_link.parent().unwrap(),
            &std::env::current_dir()
                .unwrap()
                .join("payload/dot_claude/CLAUDE.md"),
        );
        std::os::unix::fs::symlink(&legacy_target, legacy_link).unwrap();
        assert!(
            legacy_link.is_symlink(),
            "legacy instruction symlink setup failed"
        );
    }

    let output = Command::new(env!("CARGO_BIN_EXE_dotfiles"))
        .arg("install")
        .env("HOME", fake_home.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    for legacy_link in legacy_links {
        let target = std::fs::read_link(&legacy_link).unwrap();
        assert!(
            target.ends_with("agent-instructions/AGENTS.md"),
            "{} should be repointed to agent-instructions/AGENTS.md, got: {:?}",
            legacy_link.display(),
            target
        );
        assert!(
            target.is_relative(),
            "{} symlink should remain relative, got: {:?}",
            legacy_link.display(),
            target
        );
    }
}

#[test]
fn test_install_removes_legacy_code_review_specialist_agent_symlinks() {
    let fake_home = setup_fake_home();
    let legacy_paths = [
        "code-review-specialist.md",
        "codex-code-review.md",
        "gemini-code-review.md",
    ]
    .into_iter()
    .flat_map(|agent_file| {
        [
            fake_home.path().join(".claude/agents").join(agent_file),
            fake_home.path().join(".codex/agents").join(agent_file),
        ]
    })
    .collect::<Vec<_>>();

    for legacy_path in &legacy_paths {
        std::fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
        let legacy_target = dotfiles::util::fs::compute_relative_path(
            legacy_path.parent().unwrap(),
            &std::env::current_dir()
                .unwrap()
                .join("payload/dot_claude/agents")
                .join(legacy_path.file_name().unwrap()),
        );
        std::os::unix::fs::symlink(&legacy_target, legacy_path).unwrap();
        assert!(
            legacy_path.is_symlink(),
            "legacy agent symlink setup failed"
        );
    }

    let output = Command::new(env!("CARGO_BIN_EXE_dotfiles"))
        .arg("install")
        .env("HOME", fake_home.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    for legacy_path in legacy_paths {
        assert!(
            !legacy_path.exists() && !legacy_path.is_symlink(),
            "legacy agent symlink should be removed: {}",
            legacy_path.display()
        );
    }
}

/// The shared shell payload must load cleanly in both shells it is installed
/// into.
///
/// `payload/shellrc` goes into `~/.bashrc` and `~/.zshrc` unconditionally, so a
/// construct only one shell accepts would break the other's every interactive
/// startup while the Rust suite stays green. Sourcing the payload in each shell
/// with errors fatal is the cheapest check that catches that. zsh is sourced
/// only where it is installed; the skip is printed rather than silent so a
/// machine without zsh knows it ran half the check.
#[test]
fn test_shell_payload_loads_in_bash_and_zsh() {
    let payload = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("payload/shellrc");
    let script = format!(". '{}'", payload.display());
    for shell in ["bash", "zsh"] {
        let output = match Command::new(shell).args(["-eu", "-c", &script]).output() {
            Ok(output) => output,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("{shell} not installed; skipping payload load check for it");
                continue;
            }
            Err(error) => panic!("failed to run {shell}: {error}"),
        };
        assert!(
            output.status.success(),
            "payload/shellrc failed to load in {shell}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

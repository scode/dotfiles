use std::process::Command;
use tempfile::TempDir;

fn setup_fake_home() -> TempDir {
    let home = setup_fake_home_without_graphite_source();
    // Optional RawSymlink sources are present in the normal fake home.
    std::fs::create_dir_all(home.path().join("git/scode-graphite-skill")).unwrap();
    home
}

fn setup_fake_home_without_graphite_source() -> TempDir {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".config/zed")).unwrap();
    std::fs::create_dir_all(home.path().join(".claude")).unwrap();
    std::fs::create_dir_all(home.path().join(".codex")).unwrap();
    std::fs::create_dir_all(
        home.path()
            .join("Library/Application Support/com.mitchellh.ghostty"),
    )
    .unwrap();
    home
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
    "Bash(leiter:*)",
    "Read(~/.leiter/soul.md)",
    "Edit(~/.leiter/soul.md)",
    "Write(~/.leiter/soul.md)",
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

    // Verify it points to payload/ with a relative path
    let target = std::fs::read_link(&claude_md).unwrap();
    assert!(
        target.to_string_lossy().contains("payload/"),
        "symlink should point to payload/"
    );
    assert!(
        target.is_relative(),
        "symlink should be relative, got: {:?}",
        target
    );

    // Verify codex gets the same skill symlinks as claude.
    for skill in [
        "pre-pr-review-swarm",
        "scode-dist-rust-setup",
        "scode-modernize",
        "scode-todo",
        "repo-swarm",
        "stax",
        "slstack",
        "jjstack",
        "sapling",
    ] {
        assert_skill_symlink_points_to_repo_source(
            &fake_home,
            &format!(".claude/skills/{skill}"),
            &format!("agent-skills/{skill}"),
        );
        assert_skill_symlink_points_to_repo_source(
            &fake_home,
            &format!(".codex/skills/{skill}"),
            &format!("agent-skills/{skill}"),
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

    // Verify uninstall removes symlinks but leaves the managed settings file
    assert!(
        !claude_md.exists() && !claude_md.is_symlink(),
        "symlink should be removed after uninstall"
    );
    assert!(
        claude_settings.is_file() && !claude_settings.is_symlink(),
        "settings file should be left intact after uninstall"
    );
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
    for (relative_path, label) in [
        (".claude/skills/scode-graphite", "claude"),
        (".codex/skills/scode-graphite", "codex"),
    ] {
        assert!(
            fake_home
                .path()
                .join(relative_path)
                .symlink_metadata()
                .is_err(),
            "{label} graphite skill should be skipped when the source checkout is missing"
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

use std::process::Command;
use tempfile::TempDir;

fn setup_fake_home() -> TempDir {
    let home = tempfile::tempdir().unwrap();
    // Create parent directories that conditions check for
    std::fs::create_dir_all(home.path().join(".config/zed")).unwrap();
    std::fs::create_dir_all(home.path().join(".claude")).unwrap();
    std::fs::create_dir_all(home.path().join(".codex")).unwrap();
    std::fs::create_dir_all(
        home.path()
            .join("Library/Application Support/com.mitchellh.ghostty"),
    )
    .unwrap();
    // For RawSymlink source (will create broken symlink, but tests the mechanism)
    std::fs::create_dir_all(home.path().join("git/scode-graphite-skill")).unwrap();
    home
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

    // Verify symlinks created
    let claude_md = fake_home.path().join(".claude/CLAUDE.md");
    assert!(claude_md.is_symlink(), "expected .claude/CLAUDE.md symlink");

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

    // Verify codex gets the same skill/agent symlinks as claude
    let codex_agent = fake_home
        .path()
        .join(".codex/agents/code-review-specialist.md");
    assert!(
        codex_agent.is_symlink(),
        "expected .codex/agents/code-review-specialist.md symlink"
    );

    let codex_skill = fake_home.path().join(".codex/skills/pre-pr-review-swarm");
    assert!(
        codex_skill.is_symlink(),
        "expected .codex/skills/pre-pr-review-swarm symlink"
    );
    let codex_stax_skill = fake_home.path().join(".codex/skills/stax");
    assert!(
        codex_stax_skill.is_symlink(),
        "expected .codex/skills/stax symlink"
    );
    let codex_slstack_skill = fake_home.path().join(".codex/skills/slstack");
    assert!(
        codex_slstack_skill.is_symlink(),
        "expected .codex/skills/slstack symlink"
    );
    let claude_dist_skill = fake_home
        .path()
        .join(".claude/skills/scode-dist-rust-setup");
    assert!(
        claude_dist_skill.is_symlink(),
        "expected .claude/skills/scode-dist-rust-setup symlink"
    );
    let codex_dist_skill = fake_home.path().join(".codex/skills/scode-dist-rust-setup");
    assert!(
        codex_dist_skill.is_symlink(),
        "expected .codex/skills/scode-dist-rust-setup symlink"
    );
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

    // Verify symlink exists before uninstall
    let claude_md = fake_home.path().join(".claude/CLAUDE.md");
    let codex_agent = fake_home
        .path()
        .join(".codex/agents/code-review-specialist.md");
    let codex_stax_skill = fake_home.path().join(".codex/skills/stax");
    let codex_slstack_skill = fake_home.path().join(".codex/skills/slstack");
    let codex_dist_skill = fake_home.path().join(".codex/skills/scode-dist-rust-setup");
    assert!(claude_md.is_symlink(), "symlink should exist after install");
    assert!(
        codex_agent.is_symlink(),
        "codex symlink should exist after install"
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
        codex_dist_skill.is_symlink(),
        "codex dist skill symlink should exist after install"
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

    // Verify symlinks removed
    assert!(
        !claude_md.exists() && !claude_md.is_symlink(),
        "symlink should be removed after uninstall"
    );
    assert!(
        !codex_agent.exists() && !codex_agent.is_symlink(),
        "codex symlink should be removed after uninstall"
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
        !codex_dist_skill.exists() && !codex_dist_skill.is_symlink(),
        "codex dist skill symlink should be removed after uninstall"
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
    assert!(
        claude_md.is_symlink(),
        "symlink should exist after double install"
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
        fake_home
            .path()
            .join(".codex/skills/scode-dist-rust-setup")
            .is_symlink(),
        "codex dist skill symlink should exist after double install"
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

    // Verify directories were created (via ManagedDirectory)
    assert!(
        fake_home.path().join(".claude/commands").is_dir(),
        ".claude/commands should be a directory"
    );
    assert!(
        fake_home.path().join(".claude/agents").is_dir(),
        ".claude/agents should be a directory"
    );
    assert!(
        fake_home.path().join(".claude/skills").is_dir(),
        ".claude/skills should be a directory"
    );
    assert!(
        fake_home.path().join(".codex/agents").is_dir(),
        ".codex/agents should be a directory"
    );
    assert!(
        fake_home.path().join(".codex/skills").is_dir(),
        ".codex/skills should be a directory"
    );

    // Verify symlinks that depend on those directories exist
    assert!(
        fake_home
            .path()
            .join(".claude/commands/review-for-quality.md")
            .is_symlink(),
        "command symlink should exist"
    );
    assert!(
        fake_home
            .path()
            .join(".claude/agents/code-review-specialist.md")
            .is_symlink(),
        "agent symlink should exist"
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
        fake_home
            .path()
            .join(".codex/agents/code-review-specialist.md")
            .is_symlink(),
        "codex agent symlink should exist"
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
        fake_home
            .path()
            .join(".codex/skills/scode-dist-rust-setup")
            .is_symlink(),
        "codex dist skill symlink should exist"
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
        ".claude/settings.json",
        ".claude/skills/pre-pr-review-swarm",
        ".claude/skills/scode-dist-rust-setup",
        ".codex/agents/code-review-specialist.md",
        ".codex/skills/pre-pr-review-swarm",
        ".codex/skills/stax",
        ".codex/skills/slstack",
        ".codex/skills/scode-dist-rust-setup",
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

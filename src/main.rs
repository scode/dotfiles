use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use dotfiles::{
    DeleteSymlink, FeatureGraph, FeatureHandle, JsonEnsure, ManagedDirectory, PathExists,
    PayloadSymlink, RawSymlink,
};

const CLAUDE_PERMISSIONS_ALLOW: &[&str] = &[
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

const LEGACY_AGENT_FILES: &[&str] = &[
    "code-review-specialist.md",
    "codex-code-review.md",
    "gemini-code-review.md",
];

/// Registers cleanup for agent symlinks that may exist from older installs.
///
/// These cleanup features are conditioned on the agents directory already
/// existing. That lets real upgrades remove repo-owned legacy symlinks without
/// making fresh installs create an empty agents directory.
fn add_agent_cleanup_features(g: &mut FeatureGraph, agent_owner: &str, agents_dir: &str) {
    for file_name in LEGACY_AGENT_FILES {
        let agent_name = file_name.strip_suffix(".md").unwrap_or(file_name);
        g.add(
            format!("delete-{agent_owner}-agent-{agent_name}"),
            DeleteSymlink::new(format!("{agents_dir}/{file_name}")),
        )
        .condition(PathExists::new(agents_dir.to_owned()))
        .build();
    }
}

fn add_zed_features(g: &mut FeatureGraph) {
    g.add(
        "zed-keymap",
        PayloadSymlink::new(
            "payload/dot_config/zed/keymap.json",
            "~/.config/zed/keymap.json",
        ),
    )
    .condition(PathExists::new("~/.config/zed"))
    .build();
    g.add(
        "zed-tasks",
        PayloadSymlink::new(
            "payload/dot_config/zed/tasks.json",
            "~/.config/zed/tasks.json",
        ),
    )
    .condition(PathExists::new("~/.config/zed"))
    .build();
    let zed_scripts_dir = g
        .add(
            "zed-scripts-dir",
            ManagedDirectory::new("~/.config/zed/scripts"),
        )
        .condition(PathExists::new("~/.config/zed"))
        .build();
    g.add(
        "zed-scripts-claude-ctx",
        PayloadSymlink::new(
            "payload/dot_config/zed/scripts/zed_claude_ctx.sh",
            "~/.config/zed/scripts/zed_claude_ctx.sh",
        ),
    )
    .depends_on(&zed_scripts_dir)
    .build();
}

fn add_ghostty_features(g: &mut FeatureGraph) {
    let ghostty_parent_dir = g
        .add(
            "ghostty-parent-dir",
            ManagedDirectory::new("~/Library/Application Support/com.mitchellh.ghostty"),
        )
        .condition(PathExists::new("~/Library/Application Support"))
        .build();
    g.add(
        "ghostty-config",
        PayloadSymlink::new(
            "payload/Library/Application Support/com.mitchellh.ghostty/config",
            "~/Library/Application Support/com.mitchellh.ghostty/config",
        ),
    )
    .depends_on(&ghostty_parent_dir)
    .build();
}

fn add_herdr_features(g: &mut FeatureGraph) {
    let herdr_config_dir = g
        .add("herdr-config-dir", ManagedDirectory::new("~/.config/herdr"))
        .condition(PathExists::new("~/.config"))
        .build();
    g.add(
        "herdr-config",
        PayloadSymlink::new(
            "payload/dot_config/herdr/config.toml",
            "~/.config/herdr/config.toml",
        ),
    )
    .depends_on(&herdr_config_dir)
    .build();
}

fn add_claude_features(g: &mut FeatureGraph, claude_statusline: &FeatureHandle) {
    g.add(
        "claude-md",
        PayloadSymlink::new("agent-instructions/AGENTS.md", "~/.claude/CLAUDE.md"),
    )
    .condition(PathExists::new("~/.claude"))
    .build();
    let claude_settings_base = g
        .add(
            "claude-settings-base",
            JsonEnsure::new("~/.claude/settings.json")
                // This deleted payload path is still the marker for legacy
                // installer-owned symlinks that need in-place migration.
                .legacy_payload_symlink_source("payload/dot_claude/settings.json")
                .ensure_strings_in_array(
                    &["permissions", "allow"],
                    CLAUDE_PERMISSIONS_ALLOW.iter().copied(),
                )
                .ensure_value(&["sandbox", "enabled"], json!(true)),
        )
        .condition(PathExists::new("~/.claude"))
        .build();

    // Statusline ordering matters, but it is not a prerequisite for the rest
    // of the settings merge. Keep it as a separate feature so a bin install
    // problem does not suppress permissions/sandbox management.
    g.add(
        "claude-settings-statusline",
        JsonEnsure::new("~/.claude/settings.json").ensure_value_if_path_exists(
            &["statusLine"],
            "~/bin/claude-statusline.sh",
            json!({
                "type": "command",
                "command": "~/bin/claude-statusline.sh"
            }),
        ),
    )
    .depends_on(&claude_settings_base)
    .depends_on(claude_statusline)
    .build();

    // Commands directory + files
    let claude_commands_dir = g
        .add(
            "claude-commands-dir",
            ManagedDirectory::new("~/.claude/commands"),
        )
        .condition(PathExists::new("~/.claude"))
        .build();
    g.add(
        "claude-cmd-review-for-quality",
        PayloadSymlink::new(
            "payload/dot_claude/commands/review-for-quality.md",
            "~/.claude/commands/review-for-quality.md",
        ),
    )
    .depends_on(&claude_commands_dir)
    .build();
    g.add(
        "claude-cmd-tasks-to-prs",
        PayloadSymlink::new(
            "payload/dot_claude/commands/tasks-to-prs.md",
            "~/.claude/commands/tasks-to-prs.md",
        ),
    )
    .depends_on(&claude_commands_dir)
    .build();
    g.add(
        "claude-cmd-gt-new",
        PayloadSymlink::new(
            "payload/dot_claude/commands/gt-new.md",
            "~/.claude/commands/gt-new.md",
        ),
    )
    .depends_on(&claude_commands_dir)
    .build();
    g.add(
        "claude-cmd-gt-update",
        PayloadSymlink::new(
            "payload/dot_claude/commands/gt-update.md",
            "~/.claude/commands/gt-update.md",
        ),
    )
    .depends_on(&claude_commands_dir)
    .build();

    add_agent_cleanup_features(g, "claude", "~/.claude/agents");

    // Skills directory + files
    let claude_skills_dir = g
        .add(
            "claude-skills-dir",
            ManagedDirectory::new("~/.claude/skills"),
        )
        .condition(PathExists::new("~/.claude"))
        .build();
    g.add(
        "claude-skill-pre-pr-review-swarm",
        PayloadSymlink::new(
            "agent-skills/pre-pr-review-swarm",
            "~/.claude/skills/pre-pr-review-swarm",
        ),
    )
    .depends_on(&claude_skills_dir)
    .build();
    g.add(
        "claude-skill-scode-dist-rust-setup",
        PayloadSymlink::new(
            "agent-skills/scode-dist-rust-setup",
            "~/.claude/skills/scode-dist-rust-setup",
        ),
    )
    .depends_on(&claude_skills_dir)
    .build();
    g.add(
        "claude-skill-scode-modernize",
        PayloadSymlink::new(
            "agent-skills/scode-modernize",
            "~/.claude/skills/scode-modernize",
        ),
    )
    .depends_on(&claude_skills_dir)
    .build();
    g.add(
        "claude-skill-scode-todo",
        PayloadSymlink::new("agent-skills/scode-todo", "~/.claude/skills/scode-todo"),
    )
    .depends_on(&claude_skills_dir)
    .build();
    g.add(
        "claude-skill-repo-swarm",
        PayloadSymlink::new("agent-skills/repo-swarm", "~/.claude/skills/repo-swarm"),
    )
    .depends_on(&claude_skills_dir)
    .build();
    g.add(
        "claude-skill-stax",
        PayloadSymlink::new("agent-skills/stax", "~/.claude/skills/stax"),
    )
    .depends_on(&claude_skills_dir)
    .build();
    g.add(
        "claude-skill-slstack",
        PayloadSymlink::new("agent-skills/slstack", "~/.claude/skills/slstack"),
    )
    .depends_on(&claude_skills_dir)
    .build();
    g.add(
        "claude-skill-jjstack",
        PayloadSymlink::new("agent-skills/jjstack", "~/.claude/skills/jjstack"),
    )
    .depends_on(&claude_skills_dir)
    .build();
    g.add(
        "claude-skill-sapling",
        PayloadSymlink::new("agent-skills/sapling", "~/.claude/skills/sapling"),
    )
    .depends_on(&claude_skills_dir)
    .build();
    g.add(
        "delete-claude-skill-scode-stax",
        DeleteSymlink::new("~/.claude/skills/scode-stax"),
    )
    .depends_on(&claude_skills_dir)
    .build();
    g.add(
        "claude-skill-scode-graphite",
        RawSymlink::new(
            "~/git/scode-graphite-skill",
            "~/.claude/skills/scode-graphite",
        ),
    )
    .depends_on(&claude_skills_dir)
    .condition(PathExists::new("~/git/scode-graphite-skill"))
    .build();
    g.add(
        "claude-skill-scode-voice",
        RawSymlink::new("~/git/voice", "~/.claude/skills/scode-voice"),
    )
    .depends_on(&claude_skills_dir)
    .condition(PathExists::new("~/git/voice"))
    .build();
}

fn add_bin_features(g: &mut FeatureGraph) -> FeatureHandle {
    let bin_dir = g.add("bin-dir", ManagedDirectory::new("~/bin")).build();
    g.add(
        "bin-claude-statusline",
        PayloadSymlink::new(
            "payload/bin/claude-statusline.sh",
            "~/bin/claude-statusline.sh",
        ),
    )
    .depends_on(&bin_dir)
    .build()
}

fn add_codex_features(g: &mut FeatureGraph) {
    g.add(
        "codex-md",
        PayloadSymlink::new("agent-instructions/AGENTS.md", "~/.codex/AGENTS.md"),
    )
    .condition(PathExists::new("~/.codex"))
    .build();

    add_agent_cleanup_features(g, "codex", "~/.codex/agents");

    // Skills directory + files
    let codex_skills_dir = g
        .add("codex-skills-dir", ManagedDirectory::new("~/.codex/skills"))
        .condition(PathExists::new("~/.codex"))
        .build();
    g.add(
        "codex-skill-pre-pr-review-swarm",
        PayloadSymlink::new(
            "agent-skills/pre-pr-review-swarm",
            "~/.codex/skills/pre-pr-review-swarm",
        ),
    )
    .depends_on(&codex_skills_dir)
    .build();
    g.add(
        "codex-skill-scode-dist-rust-setup",
        PayloadSymlink::new(
            "agent-skills/scode-dist-rust-setup",
            "~/.codex/skills/scode-dist-rust-setup",
        ),
    )
    .depends_on(&codex_skills_dir)
    .build();
    g.add(
        "codex-skill-scode-modernize",
        PayloadSymlink::new(
            "agent-skills/scode-modernize",
            "~/.codex/skills/scode-modernize",
        ),
    )
    .depends_on(&codex_skills_dir)
    .build();
    g.add(
        "codex-skill-scode-todo",
        PayloadSymlink::new("agent-skills/scode-todo", "~/.codex/skills/scode-todo"),
    )
    .depends_on(&codex_skills_dir)
    .build();
    g.add(
        "codex-skill-repo-swarm",
        PayloadSymlink::new("agent-skills/repo-swarm", "~/.codex/skills/repo-swarm"),
    )
    .depends_on(&codex_skills_dir)
    .build();
    g.add(
        "codex-skill-stax",
        PayloadSymlink::new("agent-skills/stax", "~/.codex/skills/stax"),
    )
    .depends_on(&codex_skills_dir)
    .build();
    g.add(
        "codex-skill-slstack",
        PayloadSymlink::new("agent-skills/slstack", "~/.codex/skills/slstack"),
    )
    .depends_on(&codex_skills_dir)
    .build();
    g.add(
        "codex-skill-jjstack",
        PayloadSymlink::new("agent-skills/jjstack", "~/.codex/skills/jjstack"),
    )
    .depends_on(&codex_skills_dir)
    .build();
    g.add(
        "codex-skill-sapling",
        PayloadSymlink::new("agent-skills/sapling", "~/.codex/skills/sapling"),
    )
    .depends_on(&codex_skills_dir)
    .build();
    g.add(
        "codex-skill-scode-graphite",
        RawSymlink::new(
            "~/git/scode-graphite-skill",
            "~/.codex/skills/scode-graphite",
        ),
    )
    .depends_on(&codex_skills_dir)
    .condition(PathExists::new("~/git/scode-graphite-skill"))
    .build();
    g.add(
        "codex-skill-scode-voice",
        RawSymlink::new("~/git/voice", "~/.codex/skills/scode-voice"),
    )
    .depends_on(&codex_skills_dir)
    .condition(PathExists::new("~/git/voice"))
    .build();
}

fn features() -> FeatureGraph {
    let mut g = FeatureGraph::new();
    add_zed_features(&mut g);
    add_ghostty_features(&mut g);
    add_herdr_features(&mut g);
    let claude_statusline = add_bin_features(&mut g);
    add_claude_features(&mut g, &claude_statusline);
    add_codex_features(&mut g);
    g
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Install,
    Uninstall,
}

fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let graph = features();

    let stats = match cli.command {
        Command::Install => graph.install(),
        Command::Uninstall => graph.uninstall(),
    }?;

    info!(
        "{} changed, {} unchanged, {} skipped, {} failed",
        stats.changed, stats.unchanged, stats.skipped, stats.failed
    );

    if stats.failed > 0 {
        anyhow::bail!("one or more features failed");
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        error!(error = %e, "fatal error");
        std::process::exit(1);
    }
}

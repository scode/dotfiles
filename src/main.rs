use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use dotfiles::{
    DeleteSymlink, FeatureGraph, FeatureHandle, JsonManaged, ManagedBlock, ManagedDirectory,
    MissingDestination, PathExists, PayloadSymlink, RawSymlink,
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
];

/// Every leiter permission string a previous payload version wrote. Removal
/// is exact-match, so each historical spelling must be listed verbatim: the
/// earliest payloads used absolute /Users/scode paths before switching to ~.
const LEGACY_LEITER_PERMISSIONS_ALLOW: &[&str] = &[
    "Bash(leiter:*)",
    "Read(~/.leiter/soul.md)",
    "Edit(~/.leiter/soul.md)",
    "Write(~/.leiter/soul.md)",
    "Edit(/Users/scode/.leiter/soul.md)",
    "Write(/Users/scode/.leiter/soul.md)",
];

/// Builds one SessionStart/SessionEnd hook group exactly as an old payload
/// wrote it. Hook cleanup is exact-match against whole groups, so every
/// historical command spelling needs its own group value; the call sites
/// below enumerate all shapes the payload ever carried.
fn leiter_hook_group(commands: &[&str]) -> serde_json::Value {
    let hooks: Vec<_> = commands
        .iter()
        .map(|command| json!({ "type": "command", "command": command }))
        .collect();
    json!({ "hooks": hooks })
}

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
            JsonManaged::new("~/.claude/settings.json")
                // This deleted payload path is still the marker for legacy
                // installer-owned symlinks that need in-place migration.
                .legacy_payload_symlink_source("payload/dot_claude/settings.json")
                .managed_strings_in_array(
                    &["permissions", "allow"],
                    CLAUDE_PERMISSIONS_ALLOW.iter().copied(),
                )
                .remove_strings_from_array(
                    &["permissions", "allow"],
                    LEGACY_LEITER_PERMISSIONS_ALLOW.iter().copied(),
                )
                .remove_values_from_array(
                    &["hooks", "SessionStart"],
                    [
                        leiter_hook_group(&["leiter context", "leiter nudge"]),
                        leiter_hook_group(&["leiter hook context", "leiter hook nudge"]),
                        leiter_hook_group(&[
                            "leiter hook context",
                            "leiter hook nudge --auto-distill",
                        ]),
                    ],
                )
                .remove_values_from_array(
                    &["hooks", "SessionEnd"],
                    [
                        leiter_hook_group(&["leiter session-end"]),
                        leiter_hook_group(&["leiter hook session-end"]),
                    ],
                )
                .remove_value_if_equals(&["hooks", "SessionStart"], json!([]))
                .remove_value_if_equals(&["hooks", "SessionEnd"], json!([]))
                .remove_value_if_equals(&["hooks"], json!({}))
                .managed_value(&["sandbox", "enabled"], json!(true)),
        )
        .condition(PathExists::new("~/.claude"))
        .build();

    // Statusline ordering matters, but it is not a prerequisite for the rest
    // of the settings merge. Keep it as a separate feature so a bin install
    // problem does not suppress permissions/sandbox management.
    g.add(
        "claude-settings-statusline",
        JsonManaged::new("~/.claude/settings.json")
            // Uninstall runs this feature before claude-settings-base (reverse
            // dependency order), so it can be the first to encounter an
            // un-migrated legacy symlink. Without this marker that symlink is
            // "unexpected" and uninstall fails on machines that never ran a
            // post-migration install.
            .legacy_payload_symlink_source("payload/dot_claude/settings.json")
            .managed_value_if_path_exists(
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
    // The tasks-to-prs command was retired; its review step pointed at an
    // agent that no longer exists, and the goal-file workflow replaced it.
    // Clean up installs that still carry the symlink.
    g.add(
        "delete-claude-cmd-tasks-to-prs",
        DeleteSymlink::new("~/.claude/commands/tasks-to-prs.md"),
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
        "claude-skill-scode-chores",
        PayloadSymlink::new("agent-skills/scode-chores", "~/.claude/skills/scode-chores"),
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
        "claude-skill-swarm-triage",
        PayloadSymlink::new("agent-skills/swarm-triage", "~/.claude/skills/swarm-triage"),
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
        "claude-skill-scode-galaxy-brain",
        PayloadSymlink::new(
            "agent-skills/scode-galaxy-brain",
            "~/.claude/skills/scode-galaxy-brain",
        ),
    )
    .depends_on(&claude_skills_dir)
    .build();
    g.add(
        "claude-skill-scode-ssh-delegate",
        PayloadSymlink::new(
            "agent-skills/scode-ssh-delegate",
            "~/.claude/skills/scode-ssh-delegate",
        ),
    )
    .depends_on(&claude_skills_dir)
    .build();
    g.add(
        "claude-skill-scode-commit-msg-reviewer",
        PayloadSymlink::new(
            "agent-skills/scode-commit-msg-reviewer",
            "~/.claude/skills/scode-commit-msg-reviewer",
        ),
    )
    .depends_on(&claude_skills_dir)
    .build();
    g.add(
        "claude-skill-agent-resumeable",
        PayloadSymlink::new(
            "agent-skills/agent-resumeable",
            "~/.claude/skills/agent-resumeable",
        ),
    )
    .depends_on(&claude_skills_dir)
    .build();
    // The skill was previously named scode-fable-resume; clean up installs
    // from before the rename.
    g.add(
        "delete-claude-skill-scode-fable-resume",
        DeleteSymlink::new("~/.claude/skills/scode-fable-resume"),
    )
    .depends_on(&claude_skills_dir)
    .build();
    g.add(
        "claude-skill-scode-build-goal",
        PayloadSymlink::new(
            "agent-skills/scode-build-goal",
            "~/.claude/skills/scode-build-goal",
        ),
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

/// Registers the installer-owned region of each interactive shell startup file.
///
/// `~/.bashrc` and `~/.zshrc` are shared with every tool that ever wants a line
/// in the user's shell, so the installer owns a marked block rather than the
/// file. Both shells get the same body from `payload/shellrc`: it holds only
/// aliases and comments, which the two shells parse identically, so one source
/// is deliberately shared rather than forked. Anything shell-specific that
/// lands there later — bash-only `shopt`, zsh-only `setopt`, completion hooks —
/// must split the payload instead of being guarded inline, because the block is
/// installed unconditionally into both files. Creation is requested explicitly
/// for both because a machine without the file is an unconfigured one, not one
/// that has opted out; for zsh in particular, a stock macOS account has no
/// `~/.zshrc` until something writes one.
///
/// The ids and the default `Append` position are all effectively frozen once
/// this has run anywhere, for different reasons. Renaming an id installs a
/// fresh block everywhere and orphans the old one, so it needs a
/// `DeleteManagedBlock` for the old id alongside it. Changing the position is
/// worse: the marker is unchanged, so install leaves the block where it already
/// sits and the new position applies only to machines that never installed. A
/// `DeleteManagedBlock` does not help, even ordered ahead of the install with
/// `depends_on` — it names the same marker the install writes back, so the pair
/// would delete and re-insert the block on every run forever. Moving an
/// installed block means renaming its id.
fn add_shell_features(g: &mut FeatureGraph) {
    g.add(
        "bashrc-block",
        ManagedBlock::new("payload/shellrc", "~/.bashrc", "bash")
            .missing_destination(MissingDestination::Create),
    )
    .build();
    g.add(
        "zshrc-block",
        ManagedBlock::new("payload/shellrc", "~/.zshrc", "zsh")
            .missing_destination(MissingDestination::Create),
    )
    .build();
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
        "codex-skill-scode-chores",
        PayloadSymlink::new("agent-skills/scode-chores", "~/.codex/skills/scode-chores"),
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
        "codex-skill-swarm-triage",
        PayloadSymlink::new("agent-skills/swarm-triage", "~/.codex/skills/swarm-triage"),
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
        "codex-skill-scode-galaxy-brain",
        PayloadSymlink::new(
            "agent-skills/scode-galaxy-brain",
            "~/.codex/skills/scode-galaxy-brain",
        ),
    )
    .depends_on(&codex_skills_dir)
    .build();
    g.add(
        "codex-skill-scode-ssh-delegate",
        PayloadSymlink::new(
            "agent-skills/scode-ssh-delegate",
            "~/.codex/skills/scode-ssh-delegate",
        ),
    )
    .depends_on(&codex_skills_dir)
    .build();
    g.add(
        "codex-skill-scode-commit-msg-reviewer",
        PayloadSymlink::new(
            "agent-skills/scode-commit-msg-reviewer",
            "~/.codex/skills/scode-commit-msg-reviewer",
        ),
    )
    .depends_on(&codex_skills_dir)
    .build();
    g.add(
        "codex-skill-agent-resumeable",
        PayloadSymlink::new(
            "agent-skills/agent-resumeable",
            "~/.codex/skills/agent-resumeable",
        ),
    )
    .depends_on(&codex_skills_dir)
    .build();
    // The skill was previously named scode-fable-resume; clean up installs
    // from before the rename.
    g.add(
        "delete-codex-skill-scode-fable-resume",
        DeleteSymlink::new("~/.codex/skills/scode-fable-resume"),
    )
    .depends_on(&codex_skills_dir)
    .build();
    g.add(
        "codex-skill-scode-build-goal",
        PayloadSymlink::new(
            "agent-skills/scode-build-goal",
            "~/.codex/skills/scode-build-goal",
        ),
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
    add_shell_features(&mut g);
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

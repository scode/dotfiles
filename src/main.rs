use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::error;
use tracing_subscriber::EnvFilter;

use dotfiles::{
    DeleteSymlink, FeatureGraph, ManagedDirectory, PathExists, PayloadSymlink, RawSymlink,
};

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

fn add_claude_features(g: &mut FeatureGraph) {
    g.add(
        "claude-md",
        PayloadSymlink::new("payload/dot_claude/CLAUDE.md", "~/.claude/CLAUDE.md"),
    )
    .condition(PathExists::new("~/.claude"))
    .build();
    g.add(
        "claude-settings",
        PayloadSymlink::new(
            "payload/dot_claude/settings.json",
            "~/.claude/settings.json",
        ),
    )
    .condition(PathExists::new("~/.claude"))
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

    // Agents directory + files
    let claude_agents_dir = g
        .add(
            "claude-agents-dir",
            ManagedDirectory::new("~/.claude/agents"),
        )
        .condition(PathExists::new("~/.claude"))
        .build();
    g.add(
        "claude-agent-code-review-specialist",
        PayloadSymlink::new(
            "payload/dot_claude/agents/code-review-specialist.md",
            "~/.claude/agents/code-review-specialist.md",
        ),
    )
    .depends_on(&claude_agents_dir)
    .build();

    // Delete old agent symlinks that have been moved into the plugin
    g.add(
        "delete-claude-agent-codex-code-review",
        DeleteSymlink::new("~/.claude/agents/codex-code-review.md"),
    )
    .depends_on(&claude_agents_dir)
    .build();
    g.add(
        "delete-claude-agent-gemini-code-review",
        DeleteSymlink::new("~/.claude/agents/gemini-code-review.md"),
    )
    .depends_on(&claude_agents_dir)
    .build();

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
            "payload/dot_claude/skills/pre-pr-review-swarm",
            "~/.claude/skills/pre-pr-review-swarm",
        ),
    )
    .depends_on(&claude_skills_dir)
    .build();
    g.add(
        "claude-skill-scode-dist-rust-setup",
        PayloadSymlink::new(
            "payload/dot_claude/skills/scode-dist-rust-setup",
            "~/.claude/skills/scode-dist-rust-setup",
        ),
    )
    .depends_on(&claude_skills_dir)
    .build();
    g.add(
        "claude-skill-scode-modernize",
        PayloadSymlink::new(
            "payload/dot_claude/skills/scode-modernize",
            "~/.claude/skills/scode-modernize",
        ),
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
    .build();
    g.add(
        "claude-skill-scode-voice",
        RawSymlink::new("~/git/voice", "~/.claude/skills/scode-voice"),
    )
    .depends_on(&claude_skills_dir)
    .condition(PathExists::new("~/git/voice"))
    .build();
}

fn add_bin_features(g: &mut FeatureGraph) {
    let bin_dir = g.add("bin-dir", ManagedDirectory::new("~/bin")).build();
    g.add(
        "bin-claude-statusline",
        PayloadSymlink::new(
            "payload/bin/claude-statusline.sh",
            "~/bin/claude-statusline.sh",
        ),
    )
    .depends_on(&bin_dir)
    .build();
}

fn add_codex_features(g: &mut FeatureGraph) {
    // Agents directory + files
    let codex_agents_dir = g
        .add("codex-agents-dir", ManagedDirectory::new("~/.codex/agents"))
        .condition(PathExists::new("~/.codex"))
        .build();
    g.add(
        "codex-agent-code-review-specialist",
        PayloadSymlink::new(
            "payload/dot_claude/agents/code-review-specialist.md",
            "~/.codex/agents/code-review-specialist.md",
        ),
    )
    .depends_on(&codex_agents_dir)
    .build();

    // Delete old agent symlinks that have been moved into the plugin
    g.add(
        "delete-codex-agent-codex-code-review",
        DeleteSymlink::new("~/.codex/agents/codex-code-review.md"),
    )
    .depends_on(&codex_agents_dir)
    .build();
    g.add(
        "delete-codex-agent-gemini-code-review",
        DeleteSymlink::new("~/.codex/agents/gemini-code-review.md"),
    )
    .depends_on(&codex_agents_dir)
    .build();

    // Skills directory + files
    let codex_skills_dir = g
        .add("codex-skills-dir", ManagedDirectory::new("~/.codex/skills"))
        .condition(PathExists::new("~/.codex"))
        .build();
    g.add(
        "codex-skill-pre-pr-review-swarm",
        PayloadSymlink::new(
            "payload/dot_claude/skills/pre-pr-review-swarm",
            "~/.codex/skills/pre-pr-review-swarm",
        ),
    )
    .depends_on(&codex_skills_dir)
    .build();
    g.add(
        "codex-skill-scode-dist-rust-setup",
        PayloadSymlink::new(
            "payload/dot_claude/skills/scode-dist-rust-setup",
            "~/.codex/skills/scode-dist-rust-setup",
        ),
    )
    .depends_on(&codex_skills_dir)
    .build();
    g.add(
        "codex-skill-scode-modernize",
        PayloadSymlink::new(
            "payload/dot_claude/skills/scode-modernize",
            "~/.codex/skills/scode-modernize",
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
    add_bin_features(&mut g);
    add_claude_features(&mut g);
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

    match cli.command {
        Command::Install => graph.install(),
        Command::Uninstall => graph.uninstall(),
    }
}

fn main() {
    if let Err(e) = run() {
        error!(error = %e, "fatal error");
        std::process::exit(1);
    }
}

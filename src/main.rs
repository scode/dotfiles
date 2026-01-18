use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::error;
use tracing_subscriber::EnvFilter;

use dotfiles::{FeatureGraph, ManagedDirectory, PathExists, PayloadSymlink, RawSymlink};

fn features() -> FeatureGraph {
    let mut g = FeatureGraph::new();

    // === Zed features (conditional on ~/.config/zed existing) ===
    g.add(
        "zed-keymap",
        PayloadSymlink::new(
            "payload/dot_config/zed/keymap.json",
            "~/.config/zed/keymap.json",
        ),
    )
    .condition(PathExists::new("~/.config/zed"));
    g.add(
        "zed-tasks",
        PayloadSymlink::new(
            "payload/dot_config/zed/tasks.json",
            "~/.config/zed/tasks.json",
        ),
    )
    .condition(PathExists::new("~/.config/zed"));
    g.add(
        "zed-scripts-dir",
        ManagedDirectory::new("~/.config/zed/scripts"),
    )
    .condition(PathExists::new("~/.config/zed"));
    g.add(
        "zed-scripts-claude-ctx",
        PayloadSymlink::new(
            "payload/dot_config/zed/scripts/zed_claude_ctx.sh",
            "~/.config/zed/scripts/zed_claude_ctx.sh",
        ),
    )
    .depends_on("zed-scripts-dir");

    // === Ghostty (unconditional) ===
    g.add(
        "ghostty-config",
        PayloadSymlink::new(
            "payload/Library/Application Support/com.mitchellh.ghostty/config",
            "~/Library/Application Support/com.mitchellh.ghostty/config",
        ),
    );

    // === Claude features (conditional on ~/.claude existing) ===
    g.add(
        "claude-md",
        PayloadSymlink::new("payload/dot_claude/CLAUDE.md", "~/.claude/CLAUDE.md"),
    )
    .condition(PathExists::new("~/.claude"));
    g.add(
        "claude-settings",
        PayloadSymlink::new(
            "payload/dot_claude/settings.json",
            "~/.claude/settings.json",
        ),
    )
    .condition(PathExists::new("~/.claude"));

    // Claude commands directory + files
    g.add(
        "claude-commands-dir",
        ManagedDirectory::new("~/.claude/commands"),
    )
    .condition(PathExists::new("~/.claude"));
    g.add(
        "claude-cmd-review-for-quality",
        PayloadSymlink::new(
            "payload/dot_claude/commands/review-for-quality.md",
            "~/.claude/commands/review-for-quality.md",
        ),
    )
    .depends_on("claude-commands-dir");
    g.add(
        "claude-cmd-tasks-to-prs",
        PayloadSymlink::new(
            "payload/dot_claude/commands/tasks-to-prs.md",
            "~/.claude/commands/tasks-to-prs.md",
        ),
    )
    .depends_on("claude-commands-dir");
    g.add(
        "claude-cmd-gt-new",
        PayloadSymlink::new(
            "payload/dot_claude/commands/gt-new.md",
            "~/.claude/commands/gt-new.md",
        ),
    )
    .depends_on("claude-commands-dir");
    g.add(
        "claude-cmd-gt-update",
        PayloadSymlink::new(
            "payload/dot_claude/commands/gt-update.md",
            "~/.claude/commands/gt-update.md",
        ),
    )
    .depends_on("claude-commands-dir");

    // Claude agents directory + files
    g.add(
        "claude-agents-dir",
        ManagedDirectory::new("~/.claude/agents"),
    )
    .condition(PathExists::new("~/.claude"));
    g.add(
        "claude-agent-code-review-specialist",
        PayloadSymlink::new(
            "payload/dot_claude/agents/code-review-specialist.md",
            "~/.claude/agents/code-review-specialist.md",
        ),
    )
    .depends_on("claude-agents-dir");

    // Claude skills directory + files
    g.add(
        "claude-skills-dir",
        ManagedDirectory::new("~/.claude/skills"),
    )
    .condition(PathExists::new("~/.claude"));
    g.add(
        "claude-skill-scode-graphite",
        RawSymlink::new(
            "~/git/scode-graphite-skill",
            "~/.claude/skills/scode-graphite",
        ),
    )
    .depends_on("claude-skills-dir");

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

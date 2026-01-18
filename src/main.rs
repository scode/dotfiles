use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::error;
use tracing_subscriber::EnvFilter;

use dotfiles::{FeatureGraph, PayloadSymlink, RawSymlink};

fn features() -> FeatureGraph {
    let mut g = FeatureGraph::new();

    g.add(
        "zed-keymap",
        PayloadSymlink::new(
            "payload/dot_config/zed/keymap.json",
            "~/.config/zed/keymap.json",
        ),
    );
    g.add(
        "zed-tasks",
        PayloadSymlink::new(
            "payload/dot_config/zed/tasks.json",
            "~/.config/zed/tasks.json",
        ),
    );
    g.add(
        "zed-scripts-claude-ctx",
        PayloadSymlink::new(
            "payload/dot_config/zed/scripts/zed_claude_ctx.sh",
            "~/.config/zed/scripts/zed_claude_ctx.sh",
        ),
    );
    g.add(
        "ghostty-config",
        PayloadSymlink::new(
            "payload/Library/Application Support/com.mitchellh.ghostty/config",
            "~/Library/Application Support/com.mitchellh.ghostty/config",
        ),
    );
    g.add(
        "claude-md",
        PayloadSymlink::new("payload/dot_claude/CLAUDE.md", "~/.claude/CLAUDE.md"),
    );
    g.add(
        "claude-settings",
        PayloadSymlink::new(
            "payload/dot_claude/settings.json",
            "~/.claude/settings.json",
        ),
    );
    g.add(
        "claude-cmd-review-for-quality",
        PayloadSymlink::new(
            "payload/dot_claude/commands/review-for-quality.md",
            "~/.claude/commands/review-for-quality.md",
        ),
    );
    g.add(
        "claude-cmd-tasks-to-prs",
        PayloadSymlink::new(
            "payload/dot_claude/commands/tasks-to-prs.md",
            "~/.claude/commands/tasks-to-prs.md",
        ),
    );
    g.add(
        "claude-cmd-gt-new",
        PayloadSymlink::new(
            "payload/dot_claude/commands/gt-new.md",
            "~/.claude/commands/gt-new.md",
        ),
    );
    g.add(
        "claude-cmd-gt-update",
        PayloadSymlink::new(
            "payload/dot_claude/commands/gt-update.md",
            "~/.claude/commands/gt-update.md",
        ),
    );
    g.add(
        "claude-agent-code-review-specialist",
        PayloadSymlink::new(
            "payload/dot_claude/agents/code-review-specialist.md",
            "~/.claude/agents/code-review-specialist.md",
        ),
    );
    g.add(
        "claude-skill-scode-graphite",
        RawSymlink::new(
            "~/git/scode-graphite-skill",
            "~/.claude/skills/scode-graphite",
        ),
    );

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

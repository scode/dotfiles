mod features;
pub mod util;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

use features::{Feature, FeatureResult, FileSymlink};

fn features() -> Vec<FileSymlink> {
    vec![
        FileSymlink::new(
            "payload/dot_config/zed/keymap.json",
            "~/.config/zed/keymap.json",
        ),
        FileSymlink::new(
            "payload/dot_config/zed/tasks.json",
            "~/.config/zed/tasks.json",
        ),
        FileSymlink::new(
            "payload/dot_config/zed/scripts/zed_claude_ctx.sh",
            "~/.config/zed/scripts/zed_claude_ctx.sh",
        ),
    ]
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
    let features = features();

    let mut had_errors = false;

    match cli.command {
        Command::Install => {
            for feature in &features {
                debug!("installing feature: {:?}", feature);
                match feature.install() {
                    Ok(FeatureResult::Changed) => {
                        info!("✅ changed: {}", feature);
                    }
                    Ok(FeatureResult::NoOp) => {
                        info!("⏭️ noop:    {}", feature);
                    }
                    Err(e) => {
                        error!("❌ {}: {}", feature, e);
                        had_errors = true;
                    }
                }
            }
        }
        Command::Uninstall => {
            for feature in &features {
                debug!("uninstalling feature: {:?}", feature);
                match feature.uninstall() {
                    Ok(FeatureResult::Changed) => {
                        info!("✅ changed: {}", feature);
                    }
                    Ok(FeatureResult::NoOp) => {
                        info!("⏭️ noop:    {}", feature);
                    }
                    Err(e) => {
                        error!("❌ {}: {}", feature, e);
                        had_errors = true;
                    }
                }
            }
        }
    }

    if had_errors {
        bail!("one or more features failed");
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        error!(error = %e, "fatal error");
        std::process::exit(1);
    }
}

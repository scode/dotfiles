use anyhow::Result;
use clap::{Parser, Subcommand};

fn main() -> Result<()> {
    let Command::Eval(eval) = Cli::parse().command;
    eval.run()
}

#[derive(Debug, Parser)]
#[command(author, version, about = "Repository maintenance tasks")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Eval(xtask::eval::EvalCommand),
}

use anyhow::Result;
use clap::Parser;

mod cmd;
mod config;

/// specmate — CLI companion for document-driven AI coding
#[derive(Parser)]
#[command(name = "specmate", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: cmd::Commands,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    cmd::run(cli.command)
}

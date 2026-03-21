use anyhow::Result;
use clap::Subcommand;

pub mod check;
pub mod init;

/// All specmate subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Validate the managed document system
    Check(check::CheckArgs),
    /// Initialise a new repo with the specmate document structure
    Init(init::InitArgs),
}

/// Dispatch a command to its handler.
pub fn run(command: Commands) -> Result<()> {
    match command {
        Commands::Check(args) => check::run(args),
        Commands::Init(args) => init::run(args),
    }
}

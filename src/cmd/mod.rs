use anyhow::Result;
use clap::Subcommand;

pub mod init;

/// All specmate subcommands.
#[derive(Subcommand)]
pub enum Commands {
    /// Initialise a new repo with the specmate document structure
    Init(init::InitArgs),
}

/// Dispatch a command to its handler.
pub fn run(command: Commands) -> Result<()> {
    match command {
        Commands::Init(args) => init::run(args),
    }
}

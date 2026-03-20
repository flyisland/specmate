use anyhow::Result;
use clap::Subcommand;

pub mod init;

// Stubs — to be implemented in subsequent tasks
pub mod check;
pub mod move_;
pub mod new;
pub mod run;
pub mod status;

/// All specmate subcommands.
#[derive(Subcommand)]
pub enum Commands {
    /// Initialise a new repo with the specmate document structure
    Init(init::InitArgs),

    /// Create a new document with an auto-assigned ID
    New(new::NewArgs),

    /// Transition a document to a new status
    Move(move_::MoveArgs),

    /// Run mechanical validation checks
    Check(check::CheckArgs),

    /// Drive the agent development loop for a Task Spec
    Run(run::RunArgs),

    /// Show document status overview
    Status(status::StatusArgs),
}

/// Dispatch a command to its handler.
pub fn run(command: Commands) -> Result<()> {
    match command {
        Commands::Init(args) => init::run(args),
        Commands::New(args) => new::run(args),
        Commands::Move(args) => move_::run(args),
        Commands::Check(args) => check::run(args),
        Commands::Run(args) => run::run(args),
        Commands::Status(args) => status::run(args),
    }
}

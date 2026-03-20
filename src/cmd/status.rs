use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct StatusArgs {
    /// Subcommand: plan <id>, design <id>, stale
    pub subcommand: Option<String>,
    /// Document ID for plan or design subcommands
    pub doc_id: Option<String>,
}

pub fn run(_args: StatusArgs) -> Result<()> {
    anyhow::bail!("specmate status is not yet implemented")
}

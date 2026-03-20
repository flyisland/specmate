use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct MoveArgs {
    /// Document ID (e.g. task-0001, design-003)
    pub doc_id: String,
    /// Target status
    pub status: String,
}

pub fn run(_args: MoveArgs) -> Result<()> {
    anyhow::bail!("specmate move is not yet implemented")
}

use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct RunArgs {
    /// Task Spec ID to run (e.g. task-0001)
    pub task_id: String,
    /// Skip review agent, run coding agent only
    #[arg(long)]
    pub code_only: bool,
    /// Run review agent on current state without re-coding
    #[arg(long)]
    pub review_only: bool,
}

pub fn run(_args: RunArgs) -> Result<()> {
    anyhow::bail!("specmate run is not yet implemented")
}

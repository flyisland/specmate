use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct CheckArgs {
    /// Specific check to run (names, frontmatter, status, refs, boundaries, conflicts)
    pub check: Option<String>,
    /// Task Spec ID for boundary check
    pub task_id: Option<String>,
}

pub fn run(_args: CheckArgs) -> Result<()> {
    anyhow::bail!("specmate check is not yet implemented")
}

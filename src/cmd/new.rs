use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct NewArgs {
    /// Document type (prd, design, plan, task, patch)
    pub doc_type: String,
    /// Slug for the new document
    pub slug: Option<String>,
}

pub fn run(_args: NewArgs) -> Result<()> {
    anyhow::bail!("specmate new is not yet implemented")
}

use anyhow::Result;
use clap::Args;

/// Arguments for `specmate init`.
#[derive(Args)]
pub struct InitArgs {
    /// Language for generated document content
    #[arg(long, default_value = "en", value_parser = ["en", "zh"])]
    pub lang: String,

    /// Print planned operations without writing any files
    #[arg(long)]
    pub dry_run: bool,

    /// Merge into existing repo: overwrite specmate-owned files,
    /// skip user-owned files, create missing structure
    #[arg(long)]
    pub merge: bool,
}

/// Run `specmate init`.
///
/// Deploys the full directory structure and self-documentation into the repo.
/// This is the onboarding command — the first thing a team runs in a new repo.
pub fn run(_args: InitArgs) -> Result<()> {
    // TODO: implement in task-0001
    anyhow::bail!("specmate init is not yet implemented")
}

use crate::check::{render_reports, run_all, run_boundaries, run_named, CheckName};
use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use std::io::Write;
use std::path::Path;

/// Arguments for `specmate check`.
#[derive(Args, Debug, Clone)]
#[command(
    after_help = "Examples:\n  specmate check\n  specmate check names\n  specmate check boundaries task-0001"
)]
pub struct CheckArgs {
    #[command(subcommand)]
    pub command: Option<CheckCommand>,
}

/// Supported `specmate check` subcommands.
#[derive(Subcommand, Debug, Clone)]
pub enum CheckCommand {
    /// Validate managed document filenames and managed locations
    Names,
    /// Validate frontmatter fields and frontmatter-level rules
    Frontmatter,
    /// Validate document directory placement against status
    Status,
    /// Validate cross-document references
    Refs,
    /// Validate task boundary overlap
    Conflicts,
    /// Validate changed files against a Task Spec boundary
    Boundaries(BoundariesArgs),
}

/// Arguments for `specmate check boundaries`.
#[derive(Args, Debug, Clone)]
pub struct BoundariesArgs {
    /// Task Spec id such as `task-0001`
    pub task_id: String,
}

/// Run `specmate check`.
pub fn run(args: CheckArgs) -> Result<()> {
    let repo_root = std::env::current_dir().context("reading current working directory")?;
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    run_in_repo(&repo_root, args, &mut stdout, &mut stderr)
}

fn run_in_repo<W: Write, E: Write>(
    repo_root: &Path,
    args: CheckArgs,
    stdout: &mut W,
    stderr: &mut E,
) -> Result<()> {
    let is_boundaries = matches!(args.command, Some(CheckCommand::Boundaries(_)));
    let reports = match args.command {
        None => run_all(repo_root),
        Some(CheckCommand::Names) => {
            run_named(repo_root, CheckName::Names).map(|report| vec![report])
        }
        Some(CheckCommand::Frontmatter) => {
            run_named(repo_root, CheckName::Frontmatter).map(|report| vec![report])
        }
        Some(CheckCommand::Status) => {
            run_named(repo_root, CheckName::Status).map(|report| vec![report])
        }
        Some(CheckCommand::Refs) => {
            run_named(repo_root, CheckName::Refs).map(|report| vec![report])
        }
        Some(CheckCommand::Conflicts) => {
            run_named(repo_root, CheckName::Conflicts).map(|report| vec![report])
        }
        Some(CheckCommand::Boundaries(args)) => {
            run_boundaries(repo_root, &args.task_id).map(|report| vec![report])
        }
    };
    let reports = match reports {
        Ok(reports) => reports,
        Err(error) => {
            writeln!(stderr, "[fail] check")?;
            writeln!(stderr, "       {}", error)?;
            writeln!(
                stderr,
                "       -> Fix the reported issue and re-run specmate check."
            )?;
            bail!("specmate check failed");
        }
    };

    write!(stdout, "{}", render_reports(&reports))?;

    if reports.iter().any(|report| !report.passed()) {
        bail!("specmate check failed");
    }

    if is_boundaries {
        return Ok(());
    }

    stderr.flush()?;
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/cmd/check_support.rs"]
mod check_support;

#[cfg(test)]
#[path = "../../tests/cmd/check_cli_test.rs"]
mod check_cli_test;

#[cfg(test)]
#[path = "../../tests/cmd/check_index_test.rs"]
mod check_index_test;

#[cfg(test)]
#[path = "../../tests/cmd/check_boundaries_test.rs"]
mod check_boundaries_test;

use anyhow::{Context, Result};
use git2::{Repository, StatusOptions};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Returns the repository-relative paths currently changed against `HEAD`.
pub fn changed_paths(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let repo = Repository::discover(repo_root)
        .with_context(|| format!("opening git repository at {}", repo_root.display()))?;
    let workdir = repo
        .workdir()
        .context("git repository does not have a working tree")?;

    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .include_unmodified(false)
        .include_ignored(false)
        .recurse_untracked_dirs(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true);

    let statuses = repo
        .statuses(Some(&mut options))
        .context("reading changed files from git status")?;

    let mut paths = BTreeSet::new();
    for entry in statuses.iter() {
        if let Some(path) = entry.path() {
            paths.insert(PathBuf::from(path));
            continue;
        }

        if let Some(delta) = entry.head_to_index() {
            if let Some(path) = delta.new_file().path().or_else(|| delta.old_file().path()) {
                paths.insert(make_repo_relative(workdir, path));
            }
        }
        if let Some(delta) = entry.index_to_workdir() {
            if let Some(path) = delta.new_file().path().or_else(|| delta.old_file().path()) {
                paths.insert(make_repo_relative(workdir, path));
            }
        }
    }

    Ok(paths.into_iter().collect())
}

fn make_repo_relative(workdir: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(workdir)
        .map(PathBuf::from)
        .unwrap_or_else(|_| path.to_path_buf())
}

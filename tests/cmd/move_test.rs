use super::check_support::{temp_repo, write_file};
use super::{run_in_repo, MoveArgs};
use clap::{CommandFactory, Parser};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
struct RootCli {
    #[command(subcommand)]
    command: crate::cmd::Commands,
}

fn args(doc_id: &str, to_status: &str, dry_run: bool) -> MoveArgs {
    MoveArgs {
        doc_id: doc_id.to_string(),
        to_status: to_status.to_string(),
        dry_run,
    }
}

fn run_move(dir: &tempfile::TempDir, move_args: MoveArgs) -> (anyhow::Result<()>, String, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let result = run_in_repo(dir.path(), move_args, &mut stdout, &mut stderr);
    (
        result,
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        String::from_utf8(stderr).expect("stderr should be utf-8"),
    )
}

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn write_repo_basics(root: &Path) {
    for relative in [
        "docs/design-docs/draft",
        "docs/design-docs/candidate",
        "docs/design-docs/implemented",
        "docs/design-docs/obsolete",
        "docs/exec-plans/draft",
        "docs/exec-plans/active",
        "docs/exec-plans/archived",
        "docs/prd/draft",
        "docs/prd/approved",
        "docs/prd/obsolete",
        "docs/guidelines",
        "specs/active",
        "specs/archived",
    ] {
        fs::create_dir_all(root.join(relative)).unwrap_or_else(|error| {
            panic!(
                "failed to create {}: {error}",
                root.join(relative).display()
            )
        });
    }
    write_file(
        root,
        "specs/project.md",
        "---\nid: project\nstatus: active\n---\n\n# Project\n",
    );
    write_file(
        root,
        "docs/guidelines/specmate.md",
        "---\ntitle: \"Specmate\"\n---\n\n# Guideline\n",
    );
}

fn write_active_exec_repo(root: &Path) {
    write_repo_basics(root);
    write_file(
        root,
        "docs/prd/approved/prd-001-core-checks.md",
        "---\nid: prd-001\ntitle: \"Core Checks\"\nstatus: approved\n---\n\n# PRD\n",
    );
    write_file(
        root,
        "docs/design-docs/implemented/design-001-check-engine.md",
        "---\nid: design-001\ntitle: \"Check Engine\"\nstatus: implemented\nprd: prd-001\n---\n\n# Design\n",
    );
    write_file(
        root,
        "docs/exec-plans/active/exec-001-build-check-engine.md",
        "---\nid: exec-001\ntitle: \"Build Check Engine\"\nstatus: active\ndesign-doc: design-001\n---\n\n# Exec Plan\n",
    );
}

fn write_active_task_repo(root: &Path) -> PathBuf {
    write_active_exec_repo(root);
    let task = root.join("specs/active/task-0001-implement-check-engine.md");
    write_file(
        root,
        "specs/active/task-0001-implement-check-engine.md",
        "---\nid: task-0001\ntitle: \"Implement check engine\"\nstatus: active\nexec-plan: exec-001\nguidelines:\n  - docs/guidelines/specmate.md\nboundaries:\n  allowed:\n    - \"src/lib.rs\"\n  forbidden_patterns:\n    - \"specs/**\"\ncompletion_criteria:\n  - id: \"cc-001\"\n    scenario: \"task passes\"\n    test: \"test_task\"\n---\n\n# Task\n",
    );
    task
}

fn write_draft_task_repo(root: &Path) -> PathBuf {
    write_active_exec_repo(root);
    let task = root.join("specs/active/task-0001-implement-check-engine.md");
    write_file(
        root,
        "specs/active/task-0001-implement-check-engine.md",
        "---\nid: task-0001\ntitle: \"Implement check engine\"\nstatus: draft\nexec-plan: exec-001\nguidelines:\n  - docs/guidelines/specmate.md\nboundaries:\n  allowed:\n    - \"src/lib.rs\"\n  forbidden_patterns:\n    - \"specs/**\"\ncompletion_criteria:\n  - id: \"cc-001\"\n    scenario: \"task passes\"\n    test: \"test_task\"\n---\n\n# Task\n",
    );
    task
}

fn write_patch_repo(root: &Path, with_merged_into: bool) -> PathBuf {
    write_repo_basics(root);
    write_file(
        root,
        "docs/design-docs/implemented/design-001-check-engine.md",
        "---\nid: design-001\ntitle: \"Check Engine\"\nstatus: implemented\n---\n\n# Design\n",
    );

    let patch = root.join("docs/design-docs/implemented/design-001-patch-01-fix-links.md");
    let merged_into = if with_merged_into {
        "merged-into: design-001\n"
    } else {
        ""
    };
    write_file(
        root,
        "docs/design-docs/implemented/design-001-patch-01-fix-links.md",
        &format!(
            "---\nid: design-001-patch-01\ntitle: \"Fix links\"\nstatus: implemented\nparent: design-001\n{merged_into}---\n\n# Patch\n"
        ),
    );
    patch
}

#[test]
fn test_move_help_describes_command_surface() {
    let mut command = RootCli::command();
    let move_cmd = command
        .find_subcommand_mut("move")
        .expect("move subcommand should exist");
    let mut help = Vec::new();
    move_cmd
        .write_long_help(&mut help)
        .expect("help should render");
    let help = String::from_utf8(help).expect("help should be utf-8");

    assert!(help.contains("specmate move"));
    assert!(help.contains("--dry-run"));
    assert!(help.contains("task-0007 completed"));
}

#[test]
fn test_move_dry_run_reports_operations_without_writing_files() {
    let dir = temp_repo();
    let source = write_active_task_repo(dir.path());
    let destination = dir
        .path()
        .join("specs/archived/task-0001-implement-check-engine.md");

    let before = read(&source);
    let (result, stdout, stderr) = run_move(&dir, args("task-0001", "completed", true));

    assert!(result.is_ok(), "dry-run failed: {stderr}");
    assert!(stdout.contains("Planned operations (no files will be written):"));
    assert!(stdout.contains("[user] UPDATE"));
    assert!(stdout.contains("[user] MOVE"));
    assert!(stdout
        .trim_end()
        .ends_with("Run without --dry-run to apply."));
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    assert_eq!(read(&source), before, "dry-run should not modify source");
    assert!(
        !destination.exists(),
        "dry-run should not create the destination file"
    );
}

#[test]
fn test_move_applies_status_update_and_relocates_file() {
    let dir = temp_repo();
    let source = write_active_task_repo(dir.path());
    let destination = dir
        .path()
        .join("specs/archived/task-0001-implement-check-engine.md");

    let (result, stdout, stderr) = run_move(&dir, args("task-0001", "completed", false));

    assert!(result.is_ok(), "move failed: {stderr}");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    assert!(stdout.contains("[user] UPDATE"));
    assert!(stdout.contains("[user] MOVE"));
    assert!(!source.exists(), "source should be removed after move");
    assert!(destination.exists(), "destination should exist after move");
    let updated = read(&destination);
    assert!(updated.contains("status: completed"));
    assert!(
        destination.ends_with("task-0001-implement-check-engine.md"),
        "filename should be preserved"
    );
}

#[test]
fn test_move_updates_in_place_when_directory_does_not_change() {
    let dir = temp_repo();
    let task = write_draft_task_repo(dir.path());

    let (result, stdout, stderr) = run_move(&dir, args("task-0001", "active", false));

    assert!(result.is_ok(), "move failed: {stderr}");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    assert!(stdout.contains("[user] UPDATE"));
    assert!(!stdout.contains("[user] MOVE"));
    let updated = read(&task);
    assert!(updated.contains("status: active"));
}

#[test]
fn test_move_rejects_invalid_targets_and_illegal_transitions() {
    let dir = temp_repo();
    let task = write_active_task_repo(dir.path());

    let (result, stdout, stderr) = run_move(&dir, args("task-0001", "active", false));
    assert!(result.is_err(), "same-status move should fail");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.contains("[fail] move"));
    assert!(stderr.contains("already active"));
    assert!(stderr.contains("Choose a different target status."));

    let (result, stdout, stderr) = run_move(&dir, args("project", "active", false));
    assert!(result.is_err(), "project move should fail");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.contains("does not support status transitions"));
    assert!(stderr.contains("Choose a PRD, Design Doc, Design Patch, Exec Plan, or Task Spec."));

    let (result, stdout, stderr) = run_move(&dir, args("task-0001", "xxx", false));
    assert!(result.is_err(), "invalid target status should fail");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.contains("status xxx is not valid for TaskSpec currently in status active"));
    assert!(stderr.contains("Choose one of: completed, cancelled."));

    let (result, stdout, stderr) = run_move(&dir, args("task-0001", "draft", false));
    assert!(result.is_err(), "illegal move should fail");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.contains(task.to_string_lossy().as_ref()) || stderr.contains("task-0001"));
    assert!(stderr.contains("illegal transition"));
    assert!(
        stderr.contains("Fix the blocking transition rule or choose a different target status.")
    );
}

#[test]
fn test_move_fails_before_writing_on_preflight_or_preview_validation_errors() {
    let invalid_repo = temp_repo();
    write_active_task_repo(invalid_repo.path());
    write_file(
        invalid_repo.path(),
        "docs/design-docs/draft/design-002-patch-01-bad.md",
        "---\nid: design-002-patch-01\ntitle: \"Bad patch\"\nstatus: draft\n---\n\n# Broken\n",
    );
    let (result, stdout, stderr) = run_move(&invalid_repo, args("task-0001", "completed", false));
    assert!(result.is_err(), "invalid repository should fail");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.contains("repository document state is invalid"));

    let preview_repo = temp_repo();
    write_active_task_repo(preview_repo.path());
    let source = preview_repo
        .path()
        .join("docs/exec-plans/active/exec-001-build-check-engine.md");
    let before = read(&source);
    let (result, stdout, stderr) = run_move(&preview_repo, args("exec-001", "abandoned", false));
    assert!(result.is_err(), "blocked move should fail");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.contains("cannot transition to abandoned"));
    assert_eq!(read(&source), before, "failed move must not modify source");

    let merged_repo = temp_repo();
    let patch = write_patch_repo(merged_repo.path(), false);
    let before = read(&patch);
    let (result, stdout, stderr) = run_move(
        &merged_repo,
        args("design-001-patch-01", "obsolete:merged", false),
    );
    assert!(result.is_err(), "missing merged-into should fail");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.contains("merged-into"));
    assert_eq!(read(&patch), before, "failed move must not modify patch");
}

#[test]
fn test_move_applies_design_patch_merge_transition() {
    let dir = temp_repo();
    let source = write_patch_repo(dir.path(), true);
    let destination = dir
        .path()
        .join("docs/design-docs/obsolete/design-001-patch-01-fix-links.md");

    let (result, stdout, stderr) =
        run_move(&dir, args("design-001-patch-01", "obsolete:merged", false));

    assert!(result.is_ok(), "move failed: {stderr}");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    assert!(stdout.contains("[user] UPDATE"));
    assert!(stdout.contains("[user] MOVE"));
    assert!(!source.exists(), "source patch should be removed");
    let updated = read(&destination);
    assert!(updated.contains("status: obsolete:merged"));
    assert!(updated.contains("merged-into: design-001"));
}

#[test]
fn test_move_rejects_destination_collisions_without_writing() {
    let dir = temp_repo();
    let source = write_active_task_repo(dir.path());
    let destination = dir
        .path()
        .join("specs/archived/task-0001-implement-check-engine.md");
    fs::create_dir_all(&destination)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", destination.display()));

    let before = read(&source);
    let (result, stdout, stderr) = run_move(&dir, args("task-0001", "completed", false));

    assert!(result.is_err(), "collision should fail");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.contains("destination path"));
    assert!(stderr.contains("already exists"));
    assert_eq!(read(&source), before, "source should remain unchanged");
}

#[test]
fn test_move_locates_repo_root_from_a_subdirectory() {
    let dir = temp_repo();
    write_active_task_repo(dir.path());
    let nested = dir.path().join("src/nested/deeper");
    fs::create_dir_all(&nested)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", nested.display()));

    let (result, stdout, stderr) = run_move(&dir, args("task-0001", "completed", true));
    assert!(result.is_ok(), "sanity check failed: {stderr}");
    assert!(stdout.contains("[user] MOVE"));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let result = run_in_repo(
        &nested,
        args("task-0001", "completed", true),
        &mut stdout,
        &mut stderr,
    );
    let stdout = String::from_utf8(stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8(stderr).expect("stderr should be utf-8");

    assert!(result.is_ok(), "move from subdirectory failed: {stderr}");
    assert!(stdout.contains("[user] MOVE"));
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
}

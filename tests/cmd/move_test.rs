use super::check_support::{temp_repo, write_file};
use super::{run_in_repo, MoveArgs};
use clap::{CommandFactory, Parser};
use std::fs;
use std::path::Path;

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

fn run_move(dir: &Path, move_args: MoveArgs) -> (anyhow::Result<()>, String, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let result = run_in_repo(dir, move_args, &mut stdout, &mut stderr);
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
        "docs/specs",
        "docs/guidelines",
        "docs/prd/draft",
        "docs/prd/approved",
        "docs/prd/obsolete",
        "docs/design/draft",
        "docs/design/candidate",
        "docs/design/implemented",
        "docs/design/obsolete",
        "docs/exec-plans/exec-auth-rollout",
        "src",
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
        "docs/specs/project.md",
        "---\nid: project\nstatus: active\n---\n\n# Project\n",
    );
    write_file(
        root,
        "docs/specs/org.md",
        "---\nid: org\nstatus: active\n---\n\n# Org\n",
    );
    write_file(
        root,
        "docs/guidelines/specmate.md",
        "---\ntitle: \"Specmate\"\n---\n\n# Guideline\n",
    );
    write_file(
        root,
        "docs/prd/approved/prd-auth.md",
        "---\nid: prd-auth\ntitle: \"Auth\"\nstatus: approved\ncreated: 2026-03-25\n---\n\n# PRD\n",
    );
}

fn write_design_repo(root: &Path) -> std::path::PathBuf {
    write_repo_basics(root);
    let source = root.join("docs/design/candidate/design-auth-system.md");
    write_file(
        root,
        "docs/design/candidate/design-auth-system.md",
        "---\nid: design-auth-system\ntitle: \"Auth System\"\nstatus: candidate\ncreated: 2026-03-25\nprd: prd-auth\n---\n\n# Design\n",
    );
    source
}

fn write_exec_repo(root: &Path) -> std::path::PathBuf {
    write_design_repo(root);
    let source = root.join("docs/exec-plans/exec-auth-rollout/plan.md");
    write_file(
        root,
        "docs/exec-plans/exec-auth-rollout/plan.md",
        "---\nid: exec-auth-rollout\ntitle: \"Auth rollout\"\nstatus: candidate\ncreated: 2026-03-25\ndesign-docs:\n  - design-auth-system\n---\n\n# Exec Plan\n",
    );
    source
}

fn write_task_repo(root: &Path) -> std::path::PathBuf {
    write_exec_repo(root);
    let source = root.join("docs/exec-plans/exec-auth-rollout/task-01-implement-login.md");
    write_file(
        root,
        "docs/exec-plans/exec-auth-rollout/task-01-implement-login.md",
        "---\nid: task-01\ntitle: \"Implement login\"\nstatus: candidate\ncreated: 2026-03-25\nexec-plan: exec-auth-rollout\nboundaries:\n  allowed:\n    - \"src/lib.rs\"\n  forbidden_patterns:\n    - \"docs/prd/**\"\n    - \"docs/design/**\"\n    - \"docs/guidelines/**\"\n    - \"docs/specs/**\"\n    - \"docs/exec-plans/**\"\ncompletion_criteria:\n  - id: \"cc-001\"\n    scenario: \"task passes\"\n    test: \"test_task\"\n---\n\n# Task\n",
    );
    write_file(root, "src/lib.rs", "pub fn login() {}\n");
    source
}

fn write_patch_repo(root: &Path, with_merged_into: bool) -> std::path::PathBuf {
    write_repo_basics(root);
    write_file(
        root,
        "docs/design/implemented/design-auth-system.md",
        "---\nid: design-auth-system\ntitle: \"Auth System\"\nstatus: implemented\ncreated: 2026-03-25\nprd: prd-auth\n---\n\n# Design\n",
    );

    let source = root.join("docs/design/implemented/design-auth-system-patch-01-fix-links.md");
    let merged_into = if with_merged_into {
        "merged-into: design-auth-system\n"
    } else {
        ""
    };
    write_file(
        root,
        "docs/design/implemented/design-auth-system-patch-01-fix-links.md",
        &format!(
            "---\nid: design-auth-system-patch-01-fix-links\ntitle: \"Fix links\"\nstatus: implemented\ncreated: 2026-03-25\nparent: design-auth-system\n{merged_into}---\n\n# Patch\n"
        ),
    );
    source
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
    assert!(help.contains("exec-auth-add-oauth/task-01 closed"));
}

#[test]
fn test_move_dry_run_reports_in_place_task_close_without_writing() {
    let dir = temp_repo();
    let source = write_task_repo(dir.path());
    let before = read(&source);

    let (result, stdout, stderr) = run_move(
        dir.path(),
        args("exec-auth-rollout/task-01", "closed", true),
    );

    assert!(result.is_ok(), "dry-run failed: {stderr}");
    assert!(stdout.contains("Planned operations (no files will be written):"));
    assert!(stdout.contains("[user] UPDATE"));
    assert!(!stdout.contains("[user] MOVE"));
    assert!(stdout
        .trim_end()
        .ends_with("Run without --dry-run to apply."));
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    assert_eq!(read(&source), before, "dry-run should not modify source");
}

#[test]
fn test_move_applies_status_update_and_relocates_design_doc() {
    let dir = temp_repo();
    let source = write_design_repo(dir.path());
    let destination = dir
        .path()
        .join("docs/design/implemented/design-auth-system.md");

    let (result, stdout, stderr) =
        run_move(dir.path(), args("design-auth-system", "implemented", false));

    assert!(result.is_ok(), "move failed: {stderr}");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    assert!(stdout.contains("[user] UPDATE"));
    assert!(stdout.contains("[user] MOVE"));
    assert!(!source.exists(), "source should be removed after move");
    assert!(destination.exists(), "destination should exist after move");
    let updated = read(&destination);
    assert!(updated.contains("status: implemented"));
}

#[test]
fn test_move_updates_task_in_place_and_adds_closed_date() {
    let dir = temp_repo();
    let task = write_task_repo(dir.path());

    let (result, stdout, stderr) = run_move(
        dir.path(),
        args("exec-auth-rollout/task-01", "closed", false),
    );

    assert!(result.is_ok(), "move failed: {stderr}");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    assert!(stdout.contains("[user] UPDATE"));
    assert!(!stdout.contains("[user] MOVE"));
    let updated = read(&task);
    assert!(updated.contains("status: closed"));
    assert!(updated.contains("closed: "));
}

#[test]
fn test_move_rejects_invalid_targets_and_illegal_transitions() {
    let dir = temp_repo();
    let task = write_task_repo(dir.path());

    let (result, stdout, stderr) = run_move(
        dir.path(),
        args("exec-auth-rollout/task-01", "candidate", false),
    );
    assert!(result.is_err(), "same-status move should fail");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.contains("[fail] move"));
    assert!(stderr.contains("already candidate"));

    let (result, stdout, stderr) = run_move(dir.path(), args("project", "active", false));
    assert!(result.is_err(), "project move should fail");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.contains("does not support status transitions"));

    let (result, stdout, stderr) = run_move(
        dir.path(),
        args("exec-auth-rollout/task-01", "implemented", false),
    );
    assert!(result.is_err(), "invalid target status should fail");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.contains("status implemented is not valid"));

    let (result, stdout, stderr) = run_move(
        dir.path(),
        args("exec-auth-rollout/task-01", "draft", false),
    );
    assert!(
        result.is_ok(),
        "candidate -> draft should be legal: {stderr}"
    );
    assert!(stdout.contains("[user] UPDATE"));
    let updated = read(&task);
    assert!(updated.contains("status: draft"));
}

#[test]
fn test_move_fails_before_writing_on_preflight_or_preview_validation_errors() {
    let invalid_repo = temp_repo();
    write_task_repo(invalid_repo.path());
    write_file(
        invalid_repo.path(),
        "docs/design/draft/design-bad-patch-patch-01-missing-parent.md",
        "---\nid: design-bad-patch-patch-01-missing-parent\ntitle: \"Bad patch\"\nstatus: draft\ncreated: 2026-03-25\n---\n\n# Broken\n",
    );
    let (result, stdout, stderr) = run_move(
        invalid_repo.path(),
        args("exec-auth-rollout/task-01", "closed", false),
    );
    assert!(result.is_err(), "invalid repository should fail");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.contains("repository document state is invalid"));

    let preview_repo = temp_repo();
    write_design_repo(preview_repo.path());
    write_file(
        preview_repo.path(),
        "docs/exec-plans/exec-auth-rollout/plan.md",
        "---\nid: exec-auth-rollout\ntitle: \"Auth rollout\"\nstatus: candidate\ncreated: 2026-03-25\ndesign-docs:\n  - design-auth-system\n---\n\n# Exec Plan\n",
    );
    let source = preview_repo
        .path()
        .join("docs/design/candidate/design-auth-system.md");
    let before = read(&source);
    let (result, stdout, stderr) = run_move(
        preview_repo.path(),
        args("design-auth-system", "implemented", false),
    );
    assert!(result.is_err(), "blocked move should fail");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.contains("cannot transition to implemented"));
    assert_eq!(read(&source), before, "failed move must not modify source");

    let merged_repo = temp_repo();
    let patch = write_patch_repo(merged_repo.path(), false);
    let before = read(&patch);
    let (result, stdout, stderr) = run_move(
        merged_repo.path(),
        args(
            "design-auth-system-patch-01-fix-links",
            "obsolete:merged",
            false,
        ),
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
        .join("docs/design/obsolete/design-auth-system-patch-01-fix-links.md");

    let (result, stdout, stderr) = run_move(
        dir.path(),
        args(
            "design-auth-system-patch-01-fix-links",
            "obsolete:merged",
            false,
        ),
    );

    assert!(result.is_ok(), "move failed: {stderr}");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    assert!(stdout.contains("[user] UPDATE"));
    assert!(stdout.contains("[user] MOVE"));
    assert!(!source.exists(), "source patch should be removed");
    let updated = read(&destination);
    assert!(updated.contains("status: obsolete:merged"));
    assert!(updated.contains("merged-into: design-auth-system"));
}

#[test]
fn test_move_locates_repo_root_from_a_subdirectory() {
    let dir = temp_repo();
    write_task_repo(dir.path());
    let nested = dir.path().join("src");

    let (result, stdout, stderr) =
        run_move(&nested, args("exec-auth-rollout/task-01", "closed", true));

    assert!(
        result.is_ok(),
        "move from subdirectory should work: {stderr}"
    );
    assert!(stdout.contains("[user] UPDATE"));
}

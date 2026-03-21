use super::check_support::{create_compliant_repo, init_git_repo, temp_repo, write_file};
use super::{run_in_repo, BoundariesArgs, CheckArgs, CheckCommand};

fn run_boundaries(dir: &tempfile::TempDir, task_id: &str) -> (anyhow::Result<()>, String, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let result = run_in_repo(
        dir.path(),
        CheckArgs {
            command: Some(CheckCommand::Boundaries(BoundariesArgs {
                task_id: task_id.to_string(),
            })),
        },
        &mut stdout,
        &mut stderr,
    );
    (
        result,
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        String::from_utf8(stderr).expect("stderr should be utf-8"),
    )
}

#[test]
fn test_check_boundaries_passes_for_allowed_changes() {
    let dir = temp_repo();
    create_compliant_repo(dir.path());
    init_git_repo(dir.path());

    std::fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn check_engine() { let _ = 1; }\n",
    )
    .expect("failed to modify allowed file");

    let (result, stdout, stderr) = run_boundaries(&dir, "task-0001");

    assert!(result.is_ok(), "boundaries should pass: {stderr}");
    assert!(stdout.contains("[pass] check boundaries task-0001"));
}

#[test]
fn test_check_boundaries_reports_files_outside_allowed_patterns() {
    let dir = temp_repo();
    create_compliant_repo(dir.path());
    init_git_repo(dir.path());

    std::fs::write(
        dir.path().join("src/main.rs"),
        "fn main() { println!(\"hi\"); }\n",
    )
    .expect("failed to modify disallowed file");

    let (result, stdout, _) = run_boundaries(&dir, "task-0001");

    assert!(result.is_err(), "boundaries should fail");
    assert!(stdout.contains("src/main.rs"));
    assert!(stdout.contains("Allowed: src/lib.rs"));
}

#[test]
fn test_check_boundaries_reports_forbidden_pattern_matches() {
    let dir = temp_repo();
    create_compliant_repo(dir.path());
    write_file(
        dir.path(),
        "specs/active/task-0001-implement-check-engine.md",
        "---\nid: task-0001\ntitle: \"Implement check engine\"\nstatus: active\nexec-plan: exec-001\nguidelines:\n  - docs/guidelines/specmate.md\nboundaries:\n  allowed:\n    - \"**/*.md\"\n  forbidden_patterns:\n    - \"specs/**\"\ncompletion_criteria:\n  - id: \"cc-001\"\n    scenario: \"task passes\"\n    test: \"test_task\"\n---\n\n# Task\n",
    );
    init_git_repo(dir.path());

    std::fs::write(
        dir.path().join("specs/project.md"),
        "---\nid: project\nstatus: active\n---\n\n# Changed\n",
    )
    .expect("failed to modify forbidden file");

    let (result, stdout, _) = run_boundaries(&dir, "task-0001");

    assert!(result.is_err(), "boundaries should fail");
    assert!(stdout.contains("specs/project.md"));
    assert!(stdout.contains("forbidden"));
}

#[test]
fn test_check_boundaries_rejects_missing_or_invalid_task_id() {
    let dir = temp_repo();
    create_compliant_repo(dir.path());
    init_git_repo(dir.path());

    let (result, stdout, stderr) = run_boundaries(&dir, "task-9999");

    assert!(result.is_err(), "boundaries should fail");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.contains("[fail] check"));
    assert!(stderr.contains("task spec task-9999 does not exist"));
}

#[test]
fn test_check_boundaries_ignores_unrelated_invalid_managed_docs() {
    let dir = temp_repo();
    create_compliant_repo(dir.path());
    write_file(
        dir.path(),
        "docs/prd/approved/prd-01-invalid.md",
        "---\nid: prd-01\ntitle: \"Bad\"\nstatus: approved\n---\n\n# Bad\n",
    );
    init_git_repo(dir.path());

    std::fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn check_engine() { let _ = 1; }\n",
    )
    .expect("failed to modify allowed file");

    let (result, stdout, stderr) = run_boundaries(&dir, "task-0001");

    assert!(result.is_ok(), "boundaries should still pass: {stderr}");
    assert!(stdout.contains("[pass] check boundaries task-0001"));
}

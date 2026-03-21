use super::check_support::{create_compliant_repo, temp_repo, write_file};
use super::{run_in_repo, CheckArgs, CheckCommand};

fn run_check(
    dir: &tempfile::TempDir,
    command: Option<CheckCommand>,
) -> (anyhow::Result<()>, String, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let result = run_in_repo(dir.path(), CheckArgs { command }, &mut stdout, &mut stderr);
    (
        result,
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        String::from_utf8(stderr).expect("stderr should be utf-8"),
    )
}

#[test]
fn test_check_names_reports_invalid_managed_filenames() {
    let dir = temp_repo();
    create_compliant_repo(dir.path());
    write_file(
        dir.path(),
        "docs/prd/approved/prd-01-invalid.md",
        "---\nid: prd-01\ntitle: \"Bad\"\nstatus: approved\n---\n\n# Bad\n",
    );

    let (result, stdout, _) = run_check(&dir, Some(CheckCommand::Names));

    assert!(result.is_err(), "names check should fail");
    assert!(stdout.contains("[fail] check names"));
    assert!(stdout.contains("docs/prd/approved/prd-01-invalid.md"));
}

#[test]
fn test_check_frontmatter_reports_invalid_frontmatter() {
    let dir = temp_repo();
    create_compliant_repo(dir.path());
    write_file(
        dir.path(),
        "docs/prd/approved/prd-002-bad-frontmatter.md",
        "---\nid: prd-002\nstatus: approved\n---\n\n# PRD\n",
    );

    let (result, stdout, _) = run_check(&dir, Some(CheckCommand::Frontmatter));

    assert!(result.is_err(), "frontmatter check should fail");
    assert!(stdout.contains("[fail] check frontmatter"));
    assert!(stdout.contains("missing field `title`"));
}

#[test]
fn test_check_status_reports_directory_mismatches() {
    let dir = temp_repo();
    create_compliant_repo(dir.path());
    write_file(
        dir.path(),
        "docs/exec-plans/active/exec-002-completed-plan.md",
        "---\nid: exec-002\ntitle: \"Completed plan\"\nstatus: completed\ndesign-doc: design-001\n---\n\n# Exec Plan\n",
    );

    let (result, stdout, _) = run_check(&dir, Some(CheckCommand::Status));

    assert!(result.is_err(), "status check should fail");
    assert!(stdout.contains("[fail] check status"));
    assert!(stdout.contains("expected docs/exec-plans/archived"));
}

#[test]
fn test_check_refs_reports_stale_references() {
    let dir = temp_repo();
    create_compliant_repo(dir.path());
    write_file(
        dir.path(),
        "docs/prd/obsolete/prd-002-obsolete-checks.md",
        "---\nid: prd-002\ntitle: \"Obsolete\"\nstatus: obsolete\n---\n\n# PRD\n",
    );
    write_file(
        dir.path(),
        "docs/design-docs/candidate/design-002-stale-checks.md",
        "---\nid: design-002\ntitle: \"Stale\"\nstatus: candidate\nprd: prd-002\n---\n\n# Design\n",
    );

    let (result, stdout, _) = run_check(&dir, Some(CheckCommand::Refs));

    assert!(result.is_err(), "refs check should fail");
    assert!(stdout.contains("[fail] check refs"));
    assert!(stdout.contains("prd prd-002 is obsolete"));
}

#[test]
fn test_check_conflicts_reports_overlapping_boundaries() {
    let dir = temp_repo();
    create_compliant_repo(dir.path());
    write_file(
        dir.path(),
        "specs/active/task-0002-overlapping-boundaries.md",
        "---\nid: task-0002\ntitle: \"Overlapping task\"\nstatus: draft\nexec-plan: exec-001\nguidelines:\n  - docs/guidelines/specmate.md\nboundaries:\n  allowed:\n    - \"src/**/*.rs\"\n  forbidden_patterns:\n    - \"specs/**\"\ncompletion_criteria:\n  - id: \"cc-001\"\n    scenario: \"draft\"\n    test: \"test_draft\"\n---\n\n# Task\n",
    );

    let (result, stdout, _) = run_check(&dir, Some(CheckCommand::Conflicts));

    assert!(result.is_err(), "conflicts check should fail");
    assert!(stdout.contains("[fail] check conflicts"));
    assert!(stdout.contains("task-0001 <-> task-0002"));
}

#[test]
fn test_check_conflicts_reports_pattern_overlap_without_existing_files() {
    let dir = temp_repo();
    create_compliant_repo(dir.path());
    write_file(
        dir.path(),
        "specs/active/task-0002-future-overlap.md",
        "---\nid: task-0002\ntitle: \"Future overlap\"\nstatus: draft\nexec-plan: exec-001\nguidelines:\n  - docs/guidelines/specmate.md\nboundaries:\n  allowed:\n    - \"src/new/**/*.rs\"\n  forbidden_patterns:\n    - \"specs/**\"\ncompletion_criteria:\n  - id: \"cc-001\"\n    scenario: \"draft\"\n    test: \"test_draft\"\n---\n\n# Task\n",
    );
    write_file(
        dir.path(),
        "specs/active/task-0001-implement-check-engine.md",
        "---\nid: task-0001\ntitle: \"Implement check engine\"\nstatus: active\nexec-plan: exec-001\nguidelines:\n  - docs/guidelines/specmate.md\nboundaries:\n  allowed:\n    - \"src/**/*.rs\"\n  forbidden_patterns:\n    - \"specs/**\"\ncompletion_criteria:\n  - id: \"cc-001\"\n    scenario: \"task passes\"\n    test: \"test_task\"\n---\n\n# Task\n",
    );

    let (result, stdout, _) = run_check(&dir, Some(CheckCommand::Conflicts));

    assert!(result.is_err(), "conflicts check should fail");
    assert!(stdout.contains("[fail] check conflicts"));
    assert!(stdout.contains("'src/**/*.rs' overlaps 'src/new/**/*.rs'"));
}

#[test]
fn test_check_aggregates_index_check_results() {
    let dir = temp_repo();
    create_compliant_repo(dir.path());
    write_file(
        dir.path(),
        "docs/exec-plans/active/exec-002-completed-plan.md",
        "---\nid: exec-002\ntitle: \"Completed plan\"\nstatus: completed\ndesign-doc: design-001\n---\n\n# Exec Plan\n",
    );

    let (result, stdout, _) = run_check(&dir, None);

    assert!(result.is_err(), "aggregate check should fail");
    assert!(stdout.contains("[pass] check names"));
    assert!(stdout.contains("[fail] check status"));
    assert!(stdout.contains("1 check failed."));
}

#[test]
fn test_check_refs_distinguishes_steady_state_validity_from_transition_gates() {
    let valid_dir = temp_repo();
    create_compliant_repo(valid_dir.path());

    let (valid_result, valid_stdout, _) = run_check(&valid_dir, Some(CheckCommand::Refs));

    assert!(
        valid_result.is_ok(),
        "steady-state valid repo should pass refs"
    );
    assert!(valid_stdout.contains("[pass] check refs"));

    let invalid_dir = temp_repo();
    create_compliant_repo(invalid_dir.path());
    write_file(
        invalid_dir.path(),
        "docs/design-docs/obsolete/design-002-stale-design.md",
        "---\nid: design-002\ntitle: \"Stale design\"\nstatus: obsolete\nprd: prd-001\n---\n\n# Design\n",
    );
    write_file(
        invalid_dir.path(),
        "docs/exec-plans/active/exec-002-stale-exec.md",
        "---\nid: exec-002\ntitle: \"Stale exec\"\nstatus: active\ndesign-doc: design-002\n---\n\n# Exec\n",
    );

    let (invalid_result, invalid_stdout, _) = run_check(&invalid_dir, Some(CheckCommand::Refs));

    assert!(
        invalid_result.is_err(),
        "live ref to obsolete design should fail"
    );
    assert!(invalid_stdout.contains("[fail] check refs"));
    assert!(invalid_stdout.contains("design-doc design-002 is obsolete"));
}

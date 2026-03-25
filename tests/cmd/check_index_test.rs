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
        "docs/exec-plans/exec-build-check-engine/task-02-bad-frontmatter.md",
        "---\nid: task-02\ntitle: \"Bad frontmatter\"\nstatus: candidate\ncreated: 2026-03-25\nexec-plan: exec-build-check-engine\nboundaries:\n  allowed:\n    - src/other.rs\ncompletion_criteria:\n  - id: cc-001\n    scenario: broken\n    test: test_broken\n---\n\n# Task\n",
    );

    let (result, stdout, _) = run_check(&dir, Some(CheckCommand::Frontmatter));

    assert!(result.is_err(), "frontmatter check should fail");
    assert!(stdout.contains("[fail] check frontmatter"));
    assert!(stdout.contains("must include"));
    assert!(stdout.contains("docs/"));
}

#[test]
fn test_check_status_reports_directory_mismatches() {
    let dir = temp_repo();
    create_compliant_repo(dir.path());
    write_file(
        dir.path(),
        "docs/design/draft/design-misplaced.md",
        "---\nid: design-misplaced\ntitle: \"Misplaced\"\nstatus: candidate\ncreated: 2026-03-25\n---\n\n# Design\n",
    );

    let (result, stdout, _) = run_check(&dir, Some(CheckCommand::Status));

    assert!(result.is_err(), "status check should fail");
    assert!(stdout.contains("[fail] check status"));
    assert!(stdout.contains("expected docs/design/candidate"));
}

#[test]
fn test_check_refs_reports_stale_references() {
    let dir = temp_repo();
    create_compliant_repo(dir.path());
    write_file(
        dir.path(),
        "docs/prd/obsolete/prd-stale.md",
        "---\nid: prd-stale\ntitle: \"Obsolete\"\nstatus: obsolete\ncreated: 2026-03-25\n---\n\n# PRD\n",
    );
    write_file(
        dir.path(),
        "docs/design/candidate/design-stale.md",
        "---\nid: design-stale\ntitle: \"Stale\"\nstatus: candidate\ncreated: 2026-03-25\nprd: prd-stale\n---\n\n# Design\n",
    );

    let (result, stdout, _) = run_check(&dir, Some(CheckCommand::Refs));

    assert!(result.is_err(), "refs check should fail");
    assert!(stdout.contains("[fail] check refs"));
    assert!(stdout.contains("prd prd-stale is obsolete"));
}

#[test]
fn test_check_conflicts_reports_overlapping_boundaries() {
    let dir = temp_repo();
    create_compliant_repo(dir.path());
    write_file(
        dir.path(),
        "docs/exec-plans/exec-build-check-engine/task-02-overlap.md",
        "---\nid: task-02\ntitle: \"Overlap\"\nstatus: candidate\ncreated: 2026-03-25\nexec-plan: exec-build-check-engine\nboundaries:\n  allowed:\n    - \"src/**/*.rs\"\n  forbidden_patterns:\n    - \"docs/prd/**\"\n    - \"docs/design/**\"\n    - \"docs/guidelines/**\"\n    - \"docs/specs/**\"\n    - \"docs/exec-plans/**\"\ncompletion_criteria:\n  - id: \"cc-001\"\n    scenario: \"overlap\"\n    test: \"test_overlap\"\n---\n\n# Task\n",
    );

    let (result, stdout, _) = run_check(&dir, Some(CheckCommand::Conflicts));

    assert!(result.is_err(), "conflicts check should fail");
    assert!(stdout.contains("[fail] check conflicts"));
    assert!(stdout.contains("exec-build-check-engine/task-01"));
    assert!(stdout.contains("exec-build-check-engine/task-02"));
}

#[test]
fn test_check_conflicts_reports_pattern_overlap_without_existing_files() {
    let dir = temp_repo();
    create_compliant_repo(dir.path());
    write_file(
        dir.path(),
        "docs/exec-plans/exec-build-check-engine/task-02-future-overlap.md",
        "---\nid: task-02\ntitle: \"Future overlap\"\nstatus: candidate\ncreated: 2026-03-25\nexec-plan: exec-build-check-engine\nboundaries:\n  allowed:\n    - \"src/**/*.rs\"\n  forbidden_patterns:\n    - \"docs/prd/**\"\n    - \"docs/design/**\"\n    - \"docs/guidelines/**\"\n    - \"docs/specs/**\"\n    - \"docs/exec-plans/**\"\ncompletion_criteria:\n  - id: \"cc-001\"\n    scenario: \"future\"\n    test: \"test_future\"\n---\n\n# Task\n",
    );

    let (result, stdout, _) = run_check(&dir, Some(CheckCommand::Conflicts));

    assert!(result.is_err(), "conflicts check should fail");
    assert!(stdout.contains("[fail] check conflicts"));
    assert!(
        stdout.contains("'src/lib.rs' overlaps 'src/**/*.rs'")
            || stdout.contains("'src/**/*.rs' overlaps 'src/lib.rs'")
    );
}

#[test]
fn test_check_aggregates_index_check_results() {
    let dir = temp_repo();
    create_compliant_repo(dir.path());
    write_file(
        dir.path(),
        "docs/design/draft/design-misplaced.md",
        "---\nid: design-misplaced\ntitle: \"Misplaced\"\nstatus: candidate\ncreated: 2026-03-25\n---\n\n# Design\n",
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
        "docs/design/obsolete/design-retired.md",
        "---\nid: design-retired\ntitle: \"Retired\"\nstatus: obsolete\ncreated: 2026-03-25\nprd: prd-core-checks\n---\n\n# Design\n",
    );
    write_file(
        invalid_dir.path(),
        "docs/exec-plans/exec-retired/plan.md",
        "---\nid: exec-retired\ntitle: \"Retired\"\nstatus: candidate\ncreated: 2026-03-25\ndesign-docs:\n  - design-retired\n---\n\n# Exec\n",
    );

    let (invalid_result, invalid_stdout, _) = run_check(&invalid_dir, Some(CheckCommand::Refs));

    assert!(
        invalid_result.is_err(),
        "live ref to obsolete design should fail"
    );
    assert!(invalid_stdout.contains("[fail] check refs"));
    assert!(
        invalid_stdout.contains("design-docs reference design-retired has invalid status obsolete")
    );
}

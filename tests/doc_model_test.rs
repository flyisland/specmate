use specmate::doc::{
    build_index, expected_directory, next_id, next_patch_number, validate_index,
    validate_transition, DocId, DocType, Status,
};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn temp_repo() -> TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

fn write_markdown(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("failed to create parent directories");
    }
    fs::write(path, contents).expect("failed to write markdown file");
}

fn valid_repo(dir: &TempDir) {
    write_markdown(
        dir.path(),
        "specs/project.md",
        "---\nid: project\nstatus: active\n---\n\n# Project\n",
    );
    write_markdown(
        dir.path(),
        "specs/org.md",
        "---\nid: org\nstatus: active\n---\n\n# Org\n",
    );
    write_markdown(
        dir.path(),
        "docs/guidelines/reliability.md",
        "---\ntitle: \"Reliability\"\n---\n\n# Reliability\n",
    );
    write_markdown(
        dir.path(),
        "docs/prd/approved/prd-001-user-auth.md",
        "---\nid: prd-001\ntitle: \"User Auth\"\nstatus: approved\n---\n\n# PRD\n",
    );
    write_markdown(
        dir.path(),
        "docs/design-docs/candidate/design-001-auth-system.md",
        "---\nid: design-001\ntitle: \"Auth System\"\nstatus: candidate\nprd: prd-001\n---\n\n# Design\n",
    );
    write_markdown(
        dir.path(),
        "docs/design-docs/obsolete/design-001-patch-01-drop-username.md",
        "---\nid: design-001-patch-01\ntitle: \"Drop username\"\nstatus: obsolete\nparent: design-001\n---\n\n# Patch\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/active/exec-001-auth-rollout.md",
        "---\nid: exec-001\ntitle: \"Auth rollout\"\nstatus: active\n---\n\n# Exec\n",
    );
    write_markdown(
        dir.path(),
        "specs/active/task-0001-implement-auth.md",
        "---\nid: task-0001\ntitle: \"Implement auth\"\nstatus: active\nexec-plan: exec-001\nguidelines:\n  - docs/guidelines/reliability.md\nboundaries:\n  allowed:\n    - src/**/*.rs\n  forbidden_patterns:\n    - specs/**\ncompletion_criteria:\n  - id: cc-001\n    scenario: auth compiles\n    test: test_auth\n---\n\n# Task\n",
    );
}

#[test]
fn build_index_loads_valid_documents() {
    let dir = temp_repo();
    valid_repo(&dir);

    let index = build_index(dir.path()).expect("index should load");

    assert!(
        index.invalid_entries.is_empty(),
        "{:#?}",
        index.invalid_entries
    );
    assert_eq!(index.documents.len(), 8);
    assert_eq!(
        index
            .documents
            .get(&DocId::Guideline("reliability".to_string()))
            .map(|doc| doc.status),
        Some(Status::Active)
    );
    assert_eq!(
        index
            .documents
            .get(&DocId::ProjectSpec)
            .and_then(|doc| doc.title.clone()),
        None
    );
}

#[test]
fn build_index_reports_invalid_managed_filename() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "specs/active/not-a-task.md",
        "---\nid: task-9999\ntitle: \"Wrong\"\nstatus: draft\n---\n",
    );

    let index = build_index(dir.path()).expect("index should load");

    assert!(index
        .invalid_entries
        .iter()
        .any(|entry| entry.path.ends_with("specs/active/not-a-task.md")));
}

#[test]
fn build_index_ignores_unmanaged_markdown() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(dir.path(), "notes.md", "# Scratch\n");

    let index = build_index(dir.path()).expect("index should load");

    assert!(index
        .ignored_paths
        .iter()
        .any(|path| path.ends_with("notes.md")));
}

#[test]
fn build_index_rejects_id_mismatch() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/prd/approved/prd-002-user-auth.md",
        "---\nid: prd-999\ntitle: \"User Auth\"\nstatus: approved\n---\n",
    );

    let index = build_index(dir.path()).expect("index should load");

    assert!(index
        .invalid_entries
        .iter()
        .any(|entry| entry.reason.contains("id mismatch")));
}

#[test]
fn validate_index_requires_guideline_paths_to_resolve() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "specs/active/task-0002-bad-guideline.md",
        "---\nid: task-0002\ntitle: \"Bad guideline\"\nstatus: active\nguidelines:\n  - docs/guidelines/missing.md\nboundaries:\n  allowed:\n    - src/**/*.rs\n  forbidden_patterns:\n    - specs/**\ncompletion_criteria:\n  - id: cc-001\n    scenario: check\n    test: test_check\n---\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let violations = validate_index(&index);

    assert!(violations.iter().any(|violation| violation
        .message
        .contains("does not resolve to a Guideline")));
}

#[test]
fn expected_directory_covers_patch_obsolete() {
    assert_eq!(
        expected_directory(DocType::DesignPatch, Status::Obsolete),
        Some("docs/design-docs/obsolete")
    );
}

#[test]
fn validate_transition_allows_and_rejects_expected_moves() {
    assert!(validate_transition(DocType::DesignPatch, Status::Draft, Status::Obsolete).is_ok());
    assert!(validate_transition(
        DocType::DesignPatch,
        Status::Implemented,
        Status::ObsoleteMerged
    )
    .is_ok());
    assert!(validate_transition(DocType::Guideline, Status::Active, Status::Obsolete).is_err());
}

#[test]
fn next_id_uses_max_across_status_buckets() {
    let dir = temp_repo();
    write_markdown(
        dir.path(),
        "docs/prd/approved/prd-001-one.md",
        "---\nid: prd-001\ntitle: \"One\"\nstatus: approved\n---\n",
    );
    write_markdown(
        dir.path(),
        "docs/prd/obsolete/prd-005-five.md",
        "---\nid: prd-005\ntitle: \"Five\"\nstatus: obsolete\n---\n",
    );

    let next = next_id(dir.path(), DocType::Prd).expect("next id should resolve");

    assert_eq!(next, 6);
}

#[test]
fn next_patch_number_is_scoped_to_parent() {
    let dir = temp_repo();
    write_markdown(
        dir.path(),
        "docs/design-docs/candidate/design-001-auth.md",
        "---\nid: design-001\ntitle: \"Auth\"\nstatus: candidate\n---\n",
    );
    write_markdown(
        dir.path(),
        "docs/design-docs/candidate/design-002-billing.md",
        "---\nid: design-002\ntitle: \"Billing\"\nstatus: candidate\n---\n",
    );
    write_markdown(
        dir.path(),
        "docs/design-docs/obsolete/design-001-patch-01-first-change.md",
        "---\nid: design-001-patch-01\ntitle: \"First\"\nstatus: obsolete\nparent: design-001\n---\n",
    );
    write_markdown(
        dir.path(),
        "docs/design-docs/implemented/design-002-patch-03-billing-change.md",
        "---\nid: design-002-patch-03\ntitle: \"Third\"\nstatus: implemented\nparent: design-002\n---\n",
    );

    let next = next_patch_number(dir.path(), 1).expect("next patch should resolve");

    assert_eq!(next, 2);
}

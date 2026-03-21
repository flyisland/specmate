use specmate::doc::{
    association_summaries, build_compliant_index, build_index, ensure_index_compliant,
    expected_directory, next_id, next_patch_number, preview_transition, validate_index,
    validate_preview, validate_transition, AssociationKind, DocId, DocType, Status,
};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
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
fn build_compliant_index_loads_valid_repository() {
    let dir = temp_repo();
    valid_repo(&dir);

    let index = build_compliant_index(dir.path()).expect("repository should be compliant");

    assert_eq!(index.documents.len(), 8);
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
fn build_index_rejects_fixed_path_id_mismatch() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "specs/project.md",
        "---\nid: wrong-project\nstatus: active\n---\n\n# Project\n",
    );
    write_markdown(
        dir.path(),
        "specs/org.md",
        "---\nid: wrong-org\nstatus: active\n---\n\n# Org\n",
    );

    let index = build_index(dir.path()).expect("index should load");

    assert!(index.invalid_entries.iter().any(|entry| {
        entry.path.ends_with("specs/project.md") && entry.reason.contains("id mismatch")
    }));
    assert!(index.invalid_entries.iter().any(|entry| {
        entry.path.ends_with("specs/org.md") && entry.reason.contains("id mismatch")
    }));
}

#[test]
fn build_index_rejects_patch_missing_parent() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/design-docs/candidate/design-002-patch-01-missing-parent.md",
        "---\nid: design-002-patch-01\ntitle: \"Missing parent\"\nstatus: candidate\n---\n\n# Patch\n",
    );

    let index = build_index(dir.path()).expect("index should load");

    assert!(index.invalid_entries.iter().any(|entry| {
        entry
            .path
            .ends_with("docs/design-docs/candidate/design-002-patch-01-missing-parent.md")
            && entry.reason.contains("missing field `parent`")
    }));
}

#[test]
fn build_index_rejects_obsolete_merged_patch_without_merged_into() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/design-docs/candidate/design-002-billing.md",
        "---\nid: design-002\ntitle: \"Billing\"\nstatus: candidate\n---\n\n# Design\n",
    );
    write_markdown(
        dir.path(),
        "docs/design-docs/obsolete/design-002-patch-01-merged.md",
        "---\nid: design-002-patch-01\ntitle: \"Merged patch\"\nstatus: obsolete:merged\nparent: design-002\n---\n\n# Patch\n",
    );

    let index = build_index(dir.path()).expect("index should load");

    assert!(index.invalid_entries.iter().any(|entry| {
        entry
            .path
            .ends_with("docs/design-docs/obsolete/design-002-patch-01-merged.md")
            && entry.reason.contains("missing field `merged-into`")
    }));
}

#[test]
fn ensure_index_compliant_rejects_invalid_managed_entries() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/prd/approved/prd-002-bad.md",
        "---\nid: prd-002\nstatus: approved\n---\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let error = ensure_index_compliant(&index).expect_err("index should be invalid");

    assert!(error.to_string().contains("invalid managed entr"));
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
fn expected_directory_covers_all_valid_doc_type_status_pairs() {
    let cases = [
        (DocType::Prd, Status::Draft, Some("docs/prd/draft")),
        (DocType::Prd, Status::Approved, Some("docs/prd/approved")),
        (DocType::Prd, Status::Obsolete, Some("docs/prd/obsolete")),
        (
            DocType::DesignDoc,
            Status::Draft,
            Some("docs/design-docs/draft"),
        ),
        (
            DocType::DesignDoc,
            Status::Candidate,
            Some("docs/design-docs/candidate"),
        ),
        (
            DocType::DesignDoc,
            Status::Implemented,
            Some("docs/design-docs/implemented"),
        ),
        (
            DocType::DesignDoc,
            Status::Obsolete,
            Some("docs/design-docs/obsolete"),
        ),
        (
            DocType::DesignPatch,
            Status::Draft,
            Some("docs/design-docs/draft"),
        ),
        (
            DocType::DesignPatch,
            Status::Candidate,
            Some("docs/design-docs/candidate"),
        ),
        (
            DocType::DesignPatch,
            Status::Implemented,
            Some("docs/design-docs/implemented"),
        ),
        (
            DocType::DesignPatch,
            Status::Obsolete,
            Some("docs/design-docs/obsolete"),
        ),
        (
            DocType::DesignPatch,
            Status::ObsoleteMerged,
            Some("docs/design-docs/obsolete"),
        ),
        (
            DocType::ExecPlan,
            Status::Draft,
            Some("docs/exec-plans/draft"),
        ),
        (
            DocType::ExecPlan,
            Status::Active,
            Some("docs/exec-plans/active"),
        ),
        (
            DocType::ExecPlan,
            Status::Completed,
            Some("docs/exec-plans/archived"),
        ),
        (
            DocType::ExecPlan,
            Status::Abandoned,
            Some("docs/exec-plans/archived"),
        ),
        (DocType::TaskSpec, Status::Draft, Some("specs/active")),
        (DocType::TaskSpec, Status::Active, Some("specs/active")),
        (DocType::TaskSpec, Status::Completed, Some("specs/archived")),
        (DocType::TaskSpec, Status::Cancelled, Some("specs/archived")),
        (DocType::Guideline, Status::Active, Some("docs/guidelines")),
        (DocType::ProjectSpec, Status::Active, Some("specs")),
        (DocType::OrgSpec, Status::Active, Some("specs")),
    ];

    for (doc_type, status, expected) in cases {
        assert_eq!(expected_directory(doc_type, status), expected);
    }
}

#[test]
fn validate_transition_allows_and_rejects_expected_moves() {
    let dir = temp_repo();
    valid_repo(&dir);

    let index = build_index(dir.path()).expect("index should load");
    let patch = index
        .documents
        .get(&DocId::DesignPatch(1, 1))
        .expect("patch should exist");
    let design = index
        .documents
        .get(&DocId::DesignDoc(1))
        .expect("design should exist");
    let guideline = index
        .documents
        .get(&DocId::Guideline("reliability".to_string()))
        .expect("guideline should exist");

    assert!(validate_transition(&index, design, Status::Implemented).is_ok());
    assert!(validate_transition(&index, patch, Status::Obsolete).is_err());
    assert!(validate_transition(&index, patch, Status::ObsoleteMerged).is_err());
    assert!(validate_transition(&index, guideline, Status::Obsolete).is_err());
}

#[test]
fn validate_transition_allows_legal_design_patch_moves() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/design-docs/draft/design-002-patch-01-draft-change.md",
        "---\nid: design-002-patch-01\ntitle: \"Draft change\"\nstatus: draft\nparent: design-001\n---\n\n# Patch\n",
    );
    write_markdown(
        dir.path(),
        "docs/design-docs/candidate/design-002-patch-02-candidate-change.md",
        "---\nid: design-002-patch-02\ntitle: \"Candidate change\"\nstatus: candidate\nparent: design-001\n---\n\n# Patch\n",
    );
    write_markdown(
        dir.path(),
        "docs/design-docs/implemented/design-002-patch-03-implemented-change.md",
        "---\nid: design-002-patch-03\ntitle: \"Implemented change\"\nstatus: implemented\nparent: design-001\nmerged-into: design-001\n---\n\n# Patch\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let draft_patch = index
        .documents
        .get(&DocId::DesignPatch(2, 1))
        .expect("draft patch should exist");
    let candidate_patch = index
        .documents
        .get(&DocId::DesignPatch(2, 2))
        .expect("candidate patch should exist");
    let implemented_patch = index
        .documents
        .get(&DocId::DesignPatch(2, 3))
        .expect("implemented patch should exist");

    assert!(validate_transition(&index, draft_patch, Status::Candidate).is_ok());
    assert!(validate_transition(&index, draft_patch, Status::Obsolete).is_ok());
    assert!(validate_transition(&index, candidate_patch, Status::Implemented).is_ok());
    assert!(validate_transition(&index, candidate_patch, Status::Obsolete).is_ok());
    assert!(validate_transition(&index, implemented_patch, Status::ObsoleteMerged).is_ok());
}

#[test]
fn validate_transition_covers_prd_exec_plan_and_task_spec_lifecycles() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/prd/draft/prd-002-draft.md",
        "---\nid: prd-002\ntitle: \"Draft PRD\"\nstatus: draft\n---\n\n# PRD\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/draft/exec-002-auth-draft.md",
        "---\nid: exec-002\ntitle: \"Draft Exec\"\nstatus: draft\n---\n\n# Exec\n",
    );
    write_markdown(
        dir.path(),
        "docs/prd/approved/prd-003-standalone.md",
        "---\nid: prd-003\ntitle: \"Standalone PRD\"\nstatus: approved\n---\n\n# PRD\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/active/exec-003-free-exec.md",
        "---\nid: exec-003\ntitle: \"Free Exec\"\nstatus: active\n---\n\n# Exec\n",
    );
    write_markdown(
        dir.path(),
        "specs/active/task-0002-draft-task.md",
        "---\nid: task-0002\ntitle: \"Draft task\"\nstatus: draft\n---\n\n# Task\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let approved_prd = index
        .documents
        .get(&DocId::Prd(3))
        .expect("approved prd should exist");
    let draft_prd = index
        .documents
        .get(&DocId::Prd(2))
        .expect("draft prd should exist");
    let active_exec = index
        .documents
        .get(&DocId::ExecPlan(3))
        .expect("active exec should exist");
    let draft_exec = index
        .documents
        .get(&DocId::ExecPlan(2))
        .expect("draft exec should exist");
    let active_task = index
        .documents
        .get(&DocId::TaskSpec(1))
        .expect("active task should exist");
    let draft_task = index
        .documents
        .get(&DocId::TaskSpec(2))
        .expect("draft task should exist");

    assert!(validate_transition(&index, draft_prd, Status::Approved).is_ok());
    assert!(validate_transition(&index, approved_prd, Status::Obsolete).is_ok());
    assert!(validate_transition(&index, approved_prd, Status::Draft).is_err());

    assert!(validate_transition(&index, draft_exec, Status::Active).is_ok());
    assert!(validate_transition(&index, active_exec, Status::Completed).is_ok());
    assert!(validate_transition(&index, active_exec, Status::Abandoned).is_ok());
    assert!(validate_transition(&index, active_exec, Status::Draft).is_err());

    assert!(validate_transition(&index, draft_task, Status::Active).is_ok());
    assert!(validate_transition(&index, draft_task, Status::Cancelled).is_ok());
    assert!(validate_transition(&index, active_task, Status::Completed).is_ok());
    assert!(validate_transition(&index, active_task, Status::Cancelled).is_ok());
    assert!(validate_transition(&index, active_task, Status::Draft).is_err());
}

#[test]
fn validate_transition_blocks_design_implementation_until_exec_plans_complete() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/exec-plans/active/exec-002-auth-followup.md",
        "---\nid: exec-002\ntitle: \"Auth followup\"\nstatus: active\ndesign-doc: design-001\n---\n\n# Exec\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let design = index
        .documents
        .get(&DocId::DesignDoc(1))
        .expect("design should exist");

    assert!(validate_transition(&index, design, Status::Implemented).is_err());
}

#[test]
fn validate_transition_allows_design_implementation_when_exec_plans_are_completed() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/exec-plans/archived/exec-002-auth-followup.md",
        "---\nid: exec-002\ntitle: \"Auth followup\"\nstatus: completed\ndesign-doc: design-001\n---\n\n# Exec\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let design = index
        .documents
        .get(&DocId::DesignDoc(1))
        .expect("design should exist");

    assert!(validate_transition(&index, design, Status::Implemented).is_ok());
}

#[test]
fn validate_transition_blocks_design_implementation_with_abandoned_exec_plan() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/exec-plans/archived/exec-002-auth-followup.md",
        "---\nid: exec-002\ntitle: \"Auth followup\"\nstatus: abandoned\ndesign-doc: design-001\n---\n\n# Exec\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let design = index
        .documents
        .get(&DocId::DesignDoc(1))
        .expect("design should exist");

    assert!(validate_transition(&index, design, Status::Implemented).is_err());
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
fn next_id_rejects_invalid_managed_documents() {
    let dir = temp_repo();
    write_markdown(
        dir.path(),
        "docs/prd/approved/prd-001-one.md",
        "---\nid: prd-001\ntitle: \"One\"\nstatus: approved\n---\n",
    );
    write_markdown(
        dir.path(),
        "docs/prd/approved/prd-007-bad.md",
        "---\nid: prd-007\nstatus: approved\n---\n",
    );

    let error = next_id(dir.path(), DocType::Prd).expect_err("next id should fail");

    assert!(error.to_string().contains("invalid managed entr"));
}

#[test]
fn next_id_counts_design_doc_slug_starting_with_patch() {
    let dir = temp_repo();
    write_markdown(
        dir.path(),
        "docs/design-docs/candidate/design-007-patch-routing.md",
        "---\nid: design-007\ntitle: \"Patch routing\"\nstatus: candidate\n---\n",
    );

    let next = next_id(dir.path(), DocType::DesignDoc).expect("next id should resolve");

    assert_eq!(next, 8);
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

#[test]
fn next_patch_number_rejects_invalid_managed_documents() {
    let dir = temp_repo();
    write_markdown(
        dir.path(),
        "docs/design-docs/candidate/design-001-auth.md",
        "---\nid: design-001\ntitle: \"Auth\"\nstatus: candidate\n---\n",
    );
    write_markdown(
        dir.path(),
        "docs/design-docs/obsolete/design-001-patch-01-first-change.md",
        "---\nid: design-001-patch-01\ntitle: \"First\"\nstatus: obsolete\nparent: design-001\n---\n",
    );
    write_markdown(
        dir.path(),
        "docs/design-docs/obsolete/design-001-patch-05-bad.md",
        "---\nid: design-001-patch-05\nstatus: obsolete\nparent: design-001\n---\n",
    );

    let error = next_patch_number(dir.path(), 1).expect_err("next patch should fail");

    assert!(error.to_string().contains("invalid managed entr"));
}

#[cfg(unix)]
#[test]
fn next_id_reports_traversal_errors() {
    let dir = temp_repo();
    write_markdown(
        dir.path(),
        "docs/prd/approved/prd-001-one.md",
        "---\nid: prd-001\ntitle: \"One\"\nstatus: approved\n---\n",
    );

    let unreadable = dir.path().join("docs/prd/approved/blocked");
    fs::create_dir_all(&unreadable).expect("blocked directory should exist");
    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000))
        .expect("permissions should update");

    let result = next_id(dir.path(), DocType::Prd);

    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o755))
        .expect("permissions should restore");

    assert!(result.is_err(), "expected traversal error, got {result:?}");
}

#[test]
fn validate_index_requires_superseded_by_target_to_exist() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/design-docs/obsolete/design-002-auth-v2.md",
        "---\nid: design-002\ntitle: \"Auth v2\"\nstatus: obsolete\nsuperseded-by: design-999\n---\n\n# Design\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let violations = validate_index(&index);

    assert!(violations.iter().any(|violation| violation
        .message
        .contains("superseded-by design-999 does not exist")));
}

#[test]
fn next_id_rejects_repository_validation_violations() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/design-docs/obsolete/design-002-auth-v2.md",
        "---\nid: design-002\ntitle: \"Auth v2\"\nstatus: obsolete\nsuperseded-by: design-999\n---\n\n# Design\n",
    );

    let error = next_id(dir.path(), DocType::Prd).expect_err("next id should fail");

    assert!(error
        .to_string()
        .contains("repository-level validation violation"));
}

#[test]
fn build_index_rejects_invalid_completion_criterion_id_format() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "specs/active/task-0002-bad-criterion.md",
        "---\nid: task-0002\ntitle: \"Bad criterion\"\nstatus: active\nboundaries:\n  allowed:\n    - src/**/*.rs\n  forbidden_patterns:\n    - specs/**\ncompletion_criteria:\n  - id: login\n    scenario: check\n    test: test_check\n---\n",
    );

    let index = build_index(dir.path()).expect("index should load");

    assert!(index.invalid_entries.iter().any(|entry| {
        entry
            .path
            .ends_with("specs/active/task-0002-bad-criterion.md")
            && entry.reason.contains("cc-NNN")
    }));
}

#[test]
fn test_validate_transition_rejects_blocked_association_aware_moves() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/design-docs/draft/design-002-prd-draft.md",
        "---\nid: design-002\ntitle: \"PRD draft\"\nstatus: draft\nprd: prd-001\n---\n\n# Design\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/active/exec-002-design-active.md",
        "---\nid: exec-002\ntitle: \"Active exec\"\nstatus: active\ndesign-doc: design-001\n---\n\n# Exec\n",
    );
    write_markdown(
        dir.path(),
        "docs/design-docs/implemented/design-003-patch-01-implemented.md",
        "---\nid: design-003-patch-01\ntitle: \"Implemented patch\"\nstatus: implemented\nparent: design-001\n---\n\n# Patch\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/active/exec-003-release.md",
        "---\nid: exec-003\ntitle: \"Release\"\nstatus: active\n---\n\n# Exec\n",
    );
    write_markdown(
        dir.path(),
        "specs/active/task-0003-release-draft.md",
        "---\nid: task-0003\ntitle: \"Release draft\"\nstatus: draft\nexec-plan: exec-003\n---\n\n# Task\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/active/exec-004-abandon.md",
        "---\nid: exec-004\ntitle: \"Abandon\"\nstatus: active\n---\n\n# Exec\n",
    );
    write_markdown(
        dir.path(),
        "specs/active/task-0004-abandon-active.md",
        "---\nid: task-0004\ntitle: \"Abandon active\"\nstatus: active\nexec-plan: exec-004\nguidelines:\n  - docs/guidelines/reliability.md\nboundaries:\n  allowed:\n    - src/**/*.rs\n  forbidden_patterns:\n    - specs/**\ncompletion_criteria:\n  - id: cc-001\n    scenario: task\n    test: test_task\n---\n\n# Task\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let prd = index
        .documents
        .get(&DocId::Prd(1))
        .expect("prd should exist");
    let design = index
        .documents
        .get(&DocId::DesignDoc(1))
        .expect("design should exist");
    let patch = index
        .documents
        .get(&DocId::DesignPatch(3, 1))
        .expect("patch should exist");
    let release_exec = index
        .documents
        .get(&DocId::ExecPlan(3))
        .expect("release exec should exist");
    let abandon_exec = index
        .documents
        .get(&DocId::ExecPlan(4))
        .expect("abandon exec should exist");

    assert!(validate_transition(&index, prd, Status::Obsolete).is_err());
    assert!(validate_transition(&index, design, Status::Implemented).is_err());
    assert!(validate_transition(&index, design, Status::Obsolete).is_err());
    assert!(validate_transition(&index, patch, Status::ObsoleteMerged).is_err());
    assert!(validate_transition(&index, release_exec, Status::Completed).is_err());
    assert!(validate_transition(&index, abandon_exec, Status::Abandoned).is_err());
}

#[test]
fn test_validate_index_allows_later_bugfix_work_for_implemented_design() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/design-docs/implemented/design-001-auth-system.md",
        "---\nid: design-001\ntitle: \"Auth System\"\nstatus: implemented\nprd: prd-001\n---\n\n# Design\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/active/exec-001-auth-rollout.md",
        "---\nid: exec-001\ntitle: \"Auth rollout\"\nstatus: active\ndesign-doc: design-001\n---\n\n# Exec\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let violations = validate_index(&index);

    assert!(
        violations
            .iter()
            .all(|violation| !violation.message.contains("design-doc")),
        "{violations:#?}"
    );
    assert!(
        violations
            .iter()
            .all(|violation| !violation.message.contains("exec-plan")),
        "{violations:#?}"
    );
}

#[test]
fn test_validate_index_preserves_historical_association_links() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/design-docs/obsolete/design-001-auth-system.md",
        "---\nid: design-001\ntitle: \"Auth System\"\nstatus: obsolete\nprd: prd-001\n---\n\n# Design\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/archived/exec-001-auth-rollout.md",
        "---\nid: exec-001\ntitle: \"Auth rollout\"\nstatus: abandoned\ndesign-doc: design-001\n---\n\n# Exec\n",
    );
    write_markdown(
        dir.path(),
        "specs/archived/task-0001-implement-auth.md",
        "---\nid: task-0001\ntitle: \"Implement auth\"\nstatus: completed\nexec-plan: exec-001\nguidelines:\n  - docs/guidelines/reliability.md\nboundaries:\n  allowed:\n    - src/**/*.rs\n  forbidden_patterns:\n    - specs/**\ncompletion_criteria:\n  - id: cc-001\n    scenario: auth compiles\n    test: test_auth\n---\n\n# Task\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let violations = validate_index(&index);

    assert!(
        violations
            .iter()
            .all(|violation| !violation.message.contains("design-doc")),
        "{violations:#?}"
    );
    assert!(
        violations
            .iter()
            .all(|violation| !violation.message.contains("exec-plan")),
        "{violations:#?}"
    );
}

#[test]
fn test_validate_index_rejects_stale_associated_references() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/prd/obsolete/prd-001-user-auth.md",
        "---\nid: prd-001\ntitle: \"User Auth\"\nstatus: obsolete\n---\n\n# PRD\n",
    );
    write_markdown(
        dir.path(),
        "docs/design-docs/obsolete/design-002-old-auth.md",
        "---\nid: design-002\ntitle: \"Old Auth\"\nstatus: obsolete\nprd: prd-001\n---\n\n# Design\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/active/exec-001-auth-rollout.md",
        "---\nid: exec-001\ntitle: \"Auth rollout\"\nstatus: active\ndesign-doc: design-002\n---\n\n# Exec\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/archived/exec-002-abandoned.md",
        "---\nid: exec-002\ntitle: \"Abandoned\"\nstatus: abandoned\n---\n\n# Exec\n",
    );
    write_markdown(
        dir.path(),
        "specs/active/task-0001-implement-auth.md",
        "---\nid: task-0001\ntitle: \"Implement auth\"\nstatus: active\nexec-plan: exec-002\nguidelines:\n  - docs/guidelines/reliability.md\nboundaries:\n  allowed:\n    - src/**/*.rs\n  forbidden_patterns:\n    - specs/**\ncompletion_criteria:\n  - id: cc-001\n    scenario: auth compiles\n    test: test_auth\n---\n\n# Task\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let violations = validate_index(&index);

    assert!(violations
        .iter()
        .any(|violation| violation.message.contains("prd prd-001 is obsolete")));
    assert!(violations.iter().any(|violation| violation
        .message
        .contains("design-doc design-002 is obsolete")));
    assert!(violations.iter().any(|violation| violation
        .message
        .contains("exec-plan exec-002 has invalid status abandoned")));
}

#[test]
fn test_validate_preview_rejects_post_transition_repository_violation() {
    let dir = temp_repo();
    valid_repo(&dir);

    let index = build_index(dir.path()).expect("index should load");
    let exec = index
        .documents
        .get(&DocId::ExecPlan(1))
        .expect("exec should exist");
    let preview =
        preview_transition(&index, exec, Status::Abandoned).expect("preview should build");
    let violations = validate_preview(&preview);

    assert!(violations.iter().any(|violation| violation
        .message
        .contains("exec-plan exec-001 has invalid status abandoned")));
}

#[test]
fn test_validate_preview_accepts_satisfied_association_aware_move() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/exec-plans/archived/exec-002-auth-complete.md",
        "---\nid: exec-002\ntitle: \"Auth complete\"\nstatus: completed\ndesign-doc: design-001\n---\n\n# Exec\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let design = index
        .documents
        .get(&DocId::DesignDoc(1))
        .expect("design should exist");
    let preview =
        preview_transition(&index, design, Status::Implemented).expect("preview should build");

    assert!(validate_preview(&preview).is_empty());
}

#[test]
fn test_preview_transition_rejects_document_missing_from_index() {
    let dir = temp_repo();
    valid_repo(&dir);

    let index = build_index(dir.path()).expect("index should load");
    let missing = index
        .documents
        .get(&DocId::ExecPlan(1))
        .expect("exec should exist")
        .clone();
    let missing = specmate::doc::Document {
        id: DocId::ExecPlan(999),
        path: dir
            .path()
            .join("docs/exec-plans/active/exec-999-auth-rollout.md"),
        ..missing
    };

    let error =
        preview_transition(&index, &missing, Status::Completed).expect_err("preview should fail");
    let message = format!("{error:#}");

    assert!(message.contains("document exec-999 is not present in the current index"));
}

#[test]
fn test_association_summaries_report_related_documents_target_statuses_and_terminal_states() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/design-docs/obsolete/design-002-auth-history.md",
        "---\nid: design-002\ntitle: \"Auth history\"\nstatus: obsolete\nprd: prd-001\n---\n\n# Design\n",
    );
    write_markdown(
        dir.path(),
        "docs/design-docs/obsolete/design-001-patch-02-cleanup.md",
        "---\nid: design-001-patch-02\ntitle: \"Cleanup\"\nstatus: obsolete\nparent: design-001\n---\n\n# Patch\n",
    );
    write_markdown(
        dir.path(),
        "docs/design-docs/obsolete/design-001-patch-03-merged.md",
        "---\nid: design-001-patch-03\ntitle: \"Merged\"\nstatus: obsolete:merged\nparent: design-001\nmerged-into: design-001\n---\n\n# Patch\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/archived/exec-002-auth-complete.md",
        "---\nid: exec-002\ntitle: \"Auth complete\"\nstatus: completed\ndesign-doc: design-001\n---\n\n# Exec\n",
    );
    write_markdown(
        dir.path(),
        "specs/archived/task-0002-auth-complete.md",
        "---\nid: task-0002\ntitle: \"Auth complete\"\nstatus: completed\nexec-plan: exec-002\nguidelines:\n  - docs/guidelines/reliability.md\nboundaries:\n  allowed:\n    - src/**/*.rs\n  forbidden_patterns:\n    - specs/**\ncompletion_criteria:\n  - id: cc-001\n    scenario: auth compiles\n    test: test_auth\n---\n\n# Task\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let prd = index
        .documents
        .get(&DocId::Prd(1))
        .expect("prd should exist");
    let design = index
        .documents
        .get(&DocId::DesignDoc(1))
        .expect("design should exist");
    let exec = index
        .documents
        .get(&DocId::ExecPlan(2))
        .expect("exec should exist");

    let prd_summary = association_summaries(&index, prd)
        .into_iter()
        .find(|summary| summary.kind == AssociationKind::PrdDesignDocs)
        .expect("prd summary should exist");
    assert_eq!(prd_summary.related.len(), 2);
    assert!(!prd_summary.all_terminal());

    let design_summaries = association_summaries(&index, design);
    let patch_summary = design_summaries
        .iter()
        .find(|summary| summary.kind == AssociationKind::DesignDocPatches)
        .expect("patch summary should exist");
    assert_eq!(patch_summary.related.len(), 3);
    assert!(patch_summary.all_terminal());
    let exec_summary = design_summaries
        .iter()
        .find(|summary| summary.kind == AssociationKind::DesignDocExecPlans)
        .expect("exec summary should exist");
    assert!(exec_summary.all_in_status(Status::Completed));

    let task_summary = association_summaries(&index, exec)
        .into_iter()
        .find(|summary| summary.kind == AssociationKind::ExecPlanTasks)
        .expect("task summary should exist");
    assert!(task_summary.all_in_status(Status::Completed));
    assert!(task_summary.all_terminal());
}

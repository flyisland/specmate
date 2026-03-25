use specmate::doc::{
    association_summaries, build_compliant_index, build_index, ensure_index_compliant,
    ensure_unique_slug, expected_directory, next_patch_number, next_task_sequence,
    preview_transition, validate_index, validate_preview, validate_transition, AssociationKind,
    DocId, DocType, Status,
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
        "docs/specs/project.md",
        "---\nid: project\nstatus: active\n---\n\n# Project\n",
    );
    write_markdown(
        dir.path(),
        "docs/specs/org.md",
        "---\nid: org\nstatus: active\n---\n\n# Org\n",
    );
    write_markdown(
        dir.path(),
        "docs/guidelines/reliability.md",
        "---\ntitle: \"Reliability\"\n---\n\n# Reliability\n",
    );
    write_markdown(
        dir.path(),
        "docs/prd/approved/prd-user-auth.md",
        "---\nid: prd-user-auth\ntitle: \"User Auth\"\nstatus: approved\ncreated: 2026-03-25\n---\n\n# PRD\n",
    );
    write_markdown(
        dir.path(),
        "docs/design/candidate/design-auth-system.md",
        "---\nid: design-auth-system\ntitle: \"Auth System\"\nstatus: candidate\ncreated: 2026-03-25\nprd: prd-user-auth\n---\n\n# Design\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/exec-auth-rollout/plan.md",
        "---\nid: exec-auth-rollout\ntitle: \"Auth rollout\"\nstatus: candidate\ncreated: 2026-03-25\ndesign-docs:\n  - design-auth-system\n---\n\n# Exec\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/exec-auth-rollout/task-01-implement-login.md",
        "---\nid: task-01\ntitle: \"Implement login\"\nstatus: candidate\ncreated: 2026-03-25\nexec-plan: exec-auth-rollout\nguidelines:\n  - docs/guidelines/reliability.md\nboundaries:\n  allowed:\n    - src/**/*.rs\n  forbidden_patterns:\n    - docs/prd/**\n    - docs/design/**\n    - docs/guidelines/**\n    - docs/specs/**\n    - docs/exec-plans/**\ncompletion_criteria:\n  - id: cc-001\n    scenario: login compiles\n    test: test_login\n---\n\n# Task\n",
    );
}

fn closed_task_repo(dir: &TempDir) {
    write_markdown(
        dir.path(),
        "docs/specs/project.md",
        "---\nid: project\nstatus: active\n---\n\n# Project\n",
    );
    write_markdown(
        dir.path(),
        "docs/specs/org.md",
        "---\nid: org\nstatus: active\n---\n\n# Org\n",
    );
    write_markdown(
        dir.path(),
        "docs/guidelines/reliability.md",
        "---\ntitle: \"Reliability\"\n---\n\n# Reliability\n",
    );
    write_markdown(
        dir.path(),
        "docs/prd/approved/prd-user-auth.md",
        "---\nid: prd-user-auth\ntitle: \"User Auth\"\nstatus: approved\ncreated: 2026-03-25\n---\n\n# PRD\n",
    );
    write_markdown(
        dir.path(),
        "docs/design/candidate/design-auth-system.md",
        "---\nid: design-auth-system\ntitle: \"Auth System\"\nstatus: candidate\ncreated: 2026-03-25\nprd: prd-user-auth\n---\n\n# Design\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/exec-auth-rollout/plan.md",
        "---\nid: exec-auth-rollout\ntitle: \"Auth rollout\"\nstatus: candidate\ncreated: 2026-03-25\ndesign-docs:\n  - design-auth-system\n---\n\n# Exec\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/exec-auth-rollout/task-01-implement-login.md",
        "---\nid: task-01\ntitle: \"Implement login\"\nstatus: closed\ncreated: 2026-03-25\nclosed: 2026-03-26\nexec-plan: exec-auth-rollout\nguidelines:\n  - docs/guidelines/reliability.md\nboundaries:\n  allowed:\n    - src/**/*.rs\n  forbidden_patterns:\n    - docs/prd/**\n    - docs/design/**\n    - docs/guidelines/**\n    - docs/specs/**\n    - docs/exec-plans/**\ncompletion_criteria:\n  - id: cc-001\n    scenario: login compiles\n    test: test_login\n---\n\n# Task\n",
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
    assert_eq!(index.documents.len(), 7);
    assert!(index.documents.contains_key(&DocId::ProjectSpec));
    assert!(index.documents.contains_key(&DocId::OrgSpec));
    assert!(index
        .documents
        .contains_key(&DocId::Guideline("reliability".to_string())));
    assert!(index
        .documents
        .contains_key(&DocId::Prd("user-auth".to_string())));
    assert!(index
        .documents
        .contains_key(&DocId::DesignDoc("auth-system".to_string())));
    assert!(index
        .documents
        .contains_key(&DocId::ExecPlan("auth-rollout".to_string())));
    assert!(index.documents.contains_key(&DocId::TaskSpec {
        exec_slug: "auth-rollout".to_string(),
        sequence: 1,
    }));
}

#[test]
fn build_compliant_index_accepts_valid_repository() {
    let dir = temp_repo();
    valid_repo(&dir);

    let index = build_compliant_index(dir.path()).expect("repository should be compliant");

    assert_eq!(index.documents.len(), 7);
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
fn build_index_rejects_unsupported_managed_location() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/exec-plans/task-01-orphan.md",
        "---\nid: task-01\ntitle: \"Orphan\"\nstatus: draft\ncreated: 2026-03-25\nexec-plan: exec-auth-rollout\n---\n",
    );

    let index = build_index(dir.path()).expect("index should load");

    assert!(index.invalid_entries.iter().any(|entry| {
        entry.path.ends_with("docs/exec-plans/task-01-orphan.md")
            && entry.reason.contains("unsupported markdown location")
    }));
}

#[test]
fn build_index_rejects_patch_missing_parent() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/design/candidate/design-auth-system-patch-01-cleanup.md",
        "---\nid: design-auth-system-patch-01-cleanup\ntitle: \"Cleanup\"\nstatus: candidate\ncreated: 2026-03-25\n---\n\n# Patch\n",
    );

    let index = build_index(dir.path()).expect("index should load");

    assert!(index.invalid_entries.iter().any(|entry| {
        entry
            .path
            .ends_with("docs/design/candidate/design-auth-system-patch-01-cleanup.md")
            && entry.reason.contains("missing field `parent`")
    }));
}

#[test]
fn build_index_rejects_task_with_legacy_design_doc_field() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/exec-plans/exec-auth-rollout/task-02-bad-link.md",
        "---\nid: task-02\ntitle: \"Bad link\"\nstatus: draft\ncreated: 2026-03-25\nexec-plan: exec-auth-rollout\ndesign-doc: design-auth-system\n---\n\n# Task\n",
    );

    let index = build_index(dir.path()).expect("index should load");

    assert!(index.invalid_entries.iter().any(|entry| {
        entry
            .path
            .ends_with("docs/exec-plans/exec-auth-rollout/task-02-bad-link.md")
            && entry.reason.contains("must not declare design-doc")
    }));
}

#[test]
fn validate_index_requires_patch_parent_when_exec_references_patch() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/design/candidate/design-auth-system-patch-01-cleanup.md",
        "---\nid: design-auth-system-patch-01-cleanup\ntitle: \"Cleanup\"\nstatus: candidate\ncreated: 2026-03-25\nparent: design-auth-system\n---\n\n# Patch\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/exec-auth-rollout/plan.md",
        "---\nid: exec-auth-rollout\ntitle: \"Auth rollout\"\nstatus: candidate\ncreated: 2026-03-25\ndesign-docs:\n  - design-auth-system-patch-01-cleanup\n---\n\n# Exec\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let violations = validate_index(&index);

    assert!(violations.iter().any(|violation| {
        violation
            .message
            .contains("requires parent design design-auth-system in design-docs")
    }));
}

#[test]
fn validate_index_requires_guideline_paths_to_resolve() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/exec-plans/exec-auth-rollout/task-02-bad-guideline.md",
        "---\nid: task-02\ntitle: \"Bad guideline\"\nstatus: draft\ncreated: 2026-03-25\nexec-plan: exec-auth-rollout\nguidelines:\n  - docs/guidelines/missing.md\n---\n\n# Task\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let violations = validate_index(&index);

    assert!(violations.iter().any(|violation| {
        violation
            .message
            .contains("guideline docs/guidelines/missing.md does not resolve to a Guideline")
    }));
}

#[test]
fn expected_directory_covers_new_directory_mappings() {
    let cases = [
        (DocType::Prd, Status::Draft, Some("docs/prd/draft")),
        (DocType::Prd, Status::Approved, Some("docs/prd/approved")),
        (DocType::Prd, Status::Obsolete, Some("docs/prd/obsolete")),
        (DocType::DesignDoc, Status::Draft, Some("docs/design/draft")),
        (
            DocType::DesignDoc,
            Status::Candidate,
            Some("docs/design/candidate"),
        ),
        (
            DocType::DesignDoc,
            Status::Implemented,
            Some("docs/design/implemented"),
        ),
        (
            DocType::DesignDoc,
            Status::Obsolete,
            Some("docs/design/obsolete"),
        ),
        (
            DocType::DesignPatch,
            Status::Draft,
            Some("docs/design/draft"),
        ),
        (
            DocType::DesignPatch,
            Status::Candidate,
            Some("docs/design/candidate"),
        ),
        (
            DocType::DesignPatch,
            Status::Implemented,
            Some("docs/design/implemented"),
        ),
        (
            DocType::DesignPatch,
            Status::Obsolete,
            Some("docs/design/obsolete"),
        ),
        (
            DocType::DesignPatch,
            Status::ObsoleteMerged,
            Some("docs/design/obsolete"),
        ),
        (DocType::ProjectSpec, Status::Active, Some("docs/specs")),
        (DocType::OrgSpec, Status::Active, Some("docs/specs")),
        (DocType::Guideline, Status::Active, Some("docs/guidelines")),
        (DocType::ExecPlan, Status::Draft, None),
        (DocType::ExecPlan, Status::Candidate, None),
        (DocType::ExecPlan, Status::Closed, None),
        (DocType::TaskSpec, Status::Draft, None),
        (DocType::TaskSpec, Status::Candidate, None),
        (DocType::TaskSpec, Status::Closed, None),
    ];

    for (doc_type, status, expected) in cases {
        assert_eq!(expected_directory(doc_type, status), expected);
    }
}

#[test]
fn validate_transition_uses_new_lifecycles() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/prd/draft/prd-billing.md",
        "---\nid: prd-billing\ntitle: \"Billing\"\nstatus: draft\ncreated: 2026-03-25\n---\n\n# PRD\n",
    );
    write_markdown(
        dir.path(),
        "docs/design/draft/design-billing.md",
        "---\nid: design-billing\ntitle: \"Billing\"\nstatus: draft\ncreated: 2026-03-25\nprd: prd-billing\n---\n\n# Design\n",
    );
    write_markdown(
        dir.path(),
        "docs/design/candidate/design-auth-system-patch-01-cleanup.md",
        "---\nid: design-auth-system-patch-01-cleanup\ntitle: \"Cleanup\"\nstatus: candidate\ncreated: 2026-03-25\nparent: design-auth-system\n---\n\n# Patch\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/exec-billing/plan.md",
        "---\nid: exec-billing\ntitle: \"Billing\"\nstatus: draft\ncreated: 2026-03-25\ndesign-docs:\n  - design-billing\n---\n\n# Exec\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/exec-billing/task-01-add-invoice.md",
        "---\nid: task-01\ntitle: \"Add invoice\"\nstatus: draft\ncreated: 2026-03-25\nexec-plan: exec-billing\n---\n\n# Task\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let draft_prd = index
        .documents
        .get(&DocId::Prd("billing".to_string()))
        .expect("draft prd should exist");
    let draft_design = index
        .documents
        .get(&DocId::DesignDoc("billing".to_string()))
        .expect("draft design should exist");
    let patch = index
        .documents
        .get(&DocId::DesignPatch {
            parent_slug: "auth-system".to_string(),
            sequence: 1,
            patch_slug: "cleanup".to_string(),
        })
        .expect("patch should exist");
    let draft_exec = index
        .documents
        .get(&DocId::ExecPlan("billing".to_string()))
        .expect("draft exec should exist");
    let draft_task = index
        .documents
        .get(&DocId::TaskSpec {
            exec_slug: "billing".to_string(),
            sequence: 1,
        })
        .expect("draft task should exist");

    assert!(validate_transition(&index, draft_prd, Status::Approved).is_ok());
    assert!(validate_transition(&index, draft_design, Status::Candidate).is_ok());
    assert!(validate_transition(&index, patch, Status::Implemented).is_ok());
    assert!(validate_transition(&index, draft_exec, Status::Candidate).is_ok());
    assert!(validate_transition(&index, draft_task, Status::Candidate).is_ok());
    assert!(validate_transition(&index, draft_task, Status::Closed).is_ok());
}

#[test]
fn validate_transition_blocks_design_implementation_until_exec_is_closed() {
    let dir = temp_repo();
    valid_repo(&dir);

    let index = build_index(dir.path()).expect("index should load");
    let design = index
        .documents
        .get(&DocId::DesignDoc("auth-system".to_string()))
        .expect("design should exist");

    assert!(validate_transition(&index, design, Status::Implemented).is_err());
}

#[test]
fn validate_transition_blocks_exec_close_until_tasks_are_closed() {
    let dir = temp_repo();
    valid_repo(&dir);

    let index = build_index(dir.path()).expect("index should load");
    let exec = index
        .documents
        .get(&DocId::ExecPlan("auth-rollout".to_string()))
        .expect("exec should exist");

    assert!(validate_transition(&index, exec, Status::Closed).is_err());
}

#[test]
fn validate_transition_allows_exec_close_when_tasks_are_closed() {
    let dir = temp_repo();
    closed_task_repo(&dir);

    let index = build_index(dir.path()).expect("index should load");
    let exec = index
        .documents
        .get(&DocId::ExecPlan("auth-rollout".to_string()))
        .expect("exec should exist");

    assert!(validate_transition(&index, exec, Status::Closed).is_ok());
}

#[test]
fn next_patch_number_is_scoped_to_parent_design() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/design/candidate/design-auth-system-patch-01-cleanup.md",
        "---\nid: design-auth-system-patch-01-cleanup\ntitle: \"Cleanup\"\nstatus: candidate\ncreated: 2026-03-25\nparent: design-auth-system\n---\n\n# Patch\n",
    );
    write_markdown(
        dir.path(),
        "docs/design/implemented/design-payments.md",
        "---\nid: design-payments\ntitle: \"Payments\"\nstatus: implemented\ncreated: 2026-03-25\n---\n\n# Design\n",
    );
    write_markdown(
        dir.path(),
        "docs/design/candidate/design-payments-patch-02-reconcile.md",
        "---\nid: design-payments-patch-02-reconcile\ntitle: \"Reconcile\"\nstatus: candidate\ncreated: 2026-03-25\nparent: design-payments\n---\n\n# Patch\n",
    );

    let next = next_patch_number(dir.path(), "auth-system").expect("next patch should resolve");

    assert_eq!(next, 2);
}

#[test]
fn next_task_sequence_is_scoped_to_exec_plan() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/design/candidate/design-payments.md",
        "---\nid: design-payments\ntitle: \"Payments\"\nstatus: candidate\ncreated: 2026-03-25\n---\n\n# Design\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/exec-payments/plan.md",
        "---\nid: exec-payments\ntitle: \"Payments\"\nstatus: candidate\ncreated: 2026-03-25\ndesign-docs:\n  - design-payments\n---\n\n# Exec\n",
    );
    write_markdown(
        dir.path(),
        "docs/exec-plans/exec-payments/task-03-reconcile-ledger.md",
        "---\nid: task-03\ntitle: \"Reconcile ledger\"\nstatus: draft\ncreated: 2026-03-25\nexec-plan: exec-payments\n---\n\n# Task\n",
    );

    let next =
        next_task_sequence(dir.path(), "auth-rollout").expect("next task sequence should resolve");

    assert_eq!(next, 2);
}

#[test]
fn ensure_unique_slug_rejects_existing_slug() {
    let dir = temp_repo();
    valid_repo(&dir);

    let error = ensure_unique_slug(dir.path(), DocType::DesignDoc, "auth-system")
        .expect_err("slug should already exist");

    assert!(error
        .to_string()
        .contains("slug `auth-system` is already in use"));
}

#[test]
fn ensure_index_compliant_rejects_repository_level_violations() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/design/obsolete/design-retired-auth.md",
        "---\nid: design-retired-auth\ntitle: \"Retired Auth\"\nstatus: obsolete\ncreated: 2026-03-25\nsuperseded-by: design-missing\n---\n\n# Design\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let error = ensure_index_compliant(&index).expect_err("index should be invalid");

    assert!(error
        .to_string()
        .contains("repository-level validation violation"));
}

#[test]
fn preview_transition_rejects_documents_missing_from_index() {
    let dir = temp_repo();
    valid_repo(&dir);

    let index = build_index(dir.path()).expect("index should load");
    let existing = index
        .documents
        .get(&DocId::ExecPlan("auth-rollout".to_string()))
        .expect("exec should exist")
        .clone();
    let missing = specmate::doc::Document {
        id: DocId::ExecPlan("missing".to_string()),
        path: dir.path().join("docs/exec-plans/exec-missing/plan.md"),
        ..existing
    };

    let error =
        preview_transition(&index, &missing, Status::Closed).expect_err("preview should fail");

    assert!(format!("{error:#}").contains("document exec-missing is not present"));
}

#[test]
fn validate_preview_reports_post_transition_violations() {
    let dir = temp_repo();
    valid_repo(&dir);

    let index = build_index(dir.path()).expect("index should load");
    let exec = index
        .documents
        .get(&DocId::ExecPlan("auth-rollout".to_string()))
        .expect("exec should exist");
    let preview = preview_transition(&index, exec, Status::Closed).expect("preview should build");
    let violations = validate_preview(&preview);

    assert!(violations.iter().any(|violation| {
        violation
            .message
            .contains("exec-plan exec-auth-rollout has invalid status closed")
    }));
}

#[test]
fn association_summaries_reflect_related_documents() {
    let dir = temp_repo();
    valid_repo(&dir);
    write_markdown(
        dir.path(),
        "docs/design/candidate/design-auth-system-patch-01-cleanup.md",
        "---\nid: design-auth-system-patch-01-cleanup\ntitle: \"Cleanup\"\nstatus: candidate\ncreated: 2026-03-25\nparent: design-auth-system\n---\n\n# Patch\n",
    );

    let index = build_index(dir.path()).expect("index should load");
    let prd = index
        .documents
        .get(&DocId::Prd("user-auth".to_string()))
        .expect("prd should exist");
    let design = index
        .documents
        .get(&DocId::DesignDoc("auth-system".to_string()))
        .expect("design should exist");
    let exec = index
        .documents
        .get(&DocId::ExecPlan("auth-rollout".to_string()))
        .expect("exec should exist");

    let prd_summary = association_summaries(&index, prd)
        .into_iter()
        .find(|summary| summary.kind == AssociationKind::PrdDesignDocs)
        .expect("prd summary should exist");
    assert_eq!(prd_summary.related.len(), 1);

    let design_summaries = association_summaries(&index, design);
    let patch_summary = design_summaries
        .iter()
        .find(|summary| summary.kind == AssociationKind::DesignDocPatches)
        .expect("patch summary should exist");
    assert_eq!(patch_summary.related.len(), 1);
    let exec_summary = design_summaries
        .iter()
        .find(|summary| summary.kind == AssociationKind::DesignDocExecPlans)
        .expect("exec summary should exist");
    assert_eq!(exec_summary.related.len(), 1);

    let task_summary = association_summaries(&index, exec)
        .into_iter()
        .find(|summary| summary.kind == AssociationKind::ExecPlanTasks)
        .expect("task summary should exist");
    assert_eq!(task_summary.related.len(), 1);
    assert!(task_summary.all_in_status(Status::Candidate));
}

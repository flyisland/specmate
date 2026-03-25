//! Shared document-model utilities used by specmate commands.

mod frontmatter;
mod id;
mod types;

pub use id::{ensure_unique_slug, next_patch_number, next_task_sequence};
pub use types::{
    AssociatedDocument, AssociationKind, AssociationSummary, Boundaries, CompletionCriterion,
    DocId, DocType, Document, DocumentIndex, Frontmatter, InvalidManagedEntry, Status,
    ValidationViolation,
};

use crate::error::DocumentModelError;
use anyhow::{Context, Result};
use frontmatter::parse_frontmatter;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

/// Builds a repository-wide index of managed documents.
pub fn build_index(repo_root: &Path) -> Result<DocumentIndex> {
    let repo_root = fs::canonicalize(repo_root)
        .with_context(|| format!("canonicalising {}", repo_root.display()))?;
    let mut index = DocumentIndex {
        repo_root: repo_root.clone(),
        ..DocumentIndex::default()
    };

    for entry in WalkDir::new(&repo_root)
        .into_iter()
        .filter_entry(should_visit)
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        let relative = entry
            .path()
            .strip_prefix(&repo_root)
            .with_context(|| format!("stripping repo root from {}", entry.path().display()))?;

        match classify_path(relative) {
            PathDisposition::Ignored => index.ignored_paths.push(entry.path().to_path_buf()),
            PathDisposition::InvalidManaged(reason) => {
                index.invalid_entries.push(InvalidManagedEntry {
                    path: entry.path().to_path_buf(),
                    reason,
                });
            }
            disposition => match load_document_from_path(&repo_root, relative, disposition) {
                Ok(document) => {
                    if let Some(existing) = index.documents.get(&document.id) {
                        index.invalid_entries.push(InvalidManagedEntry {
                            path: document.path.clone(),
                            reason: format!(
                                "duplicate canonical id {} already used by {}",
                                document.id,
                                existing.path.display()
                            ),
                        });
                    } else {
                        index.documents.insert(document.id.clone(), document);
                    }
                }
                Err(error) => index.invalid_entries.push(InvalidManagedEntry {
                    path: entry.path().to_path_buf(),
                    reason: error.to_string(),
                }),
            },
        }
    }

    Ok(index)
}

/// Builds a repository index and fails if the managed document system is not compliant.
pub fn build_compliant_index(repo_root: &Path) -> Result<DocumentIndex> {
    let index = build_index(repo_root).context("building document index")?;
    ensure_index_compliant(&index)?;
    Ok(index)
}

/// Resolves the expected relative directory for a `(DocType, Status)` pair when directory=status applies.
pub fn expected_directory(doc_type: DocType, status: Status) -> Option<&'static str> {
    match (doc_type, status) {
        (DocType::Prd, Status::Draft) => Some("docs/prd/draft"),
        (DocType::Prd, Status::Approved) => Some("docs/prd/approved"),
        (DocType::Prd, Status::Obsolete) => Some("docs/prd/obsolete"),
        (DocType::DesignDoc, Status::Draft) | (DocType::DesignPatch, Status::Draft) => {
            Some("docs/design/draft")
        }
        (DocType::DesignDoc, Status::Candidate) | (DocType::DesignPatch, Status::Candidate) => {
            Some("docs/design/candidate")
        }
        (DocType::DesignDoc, Status::Implemented) | (DocType::DesignPatch, Status::Implemented) => {
            Some("docs/design/implemented")
        }
        (DocType::DesignDoc, Status::Obsolete)
        | (DocType::DesignPatch, Status::Obsolete)
        | (DocType::DesignPatch, Status::ObsoleteMerged) => Some("docs/design/obsolete"),
        (DocType::ProjectSpec, Status::Active) | (DocType::OrgSpec, Status::Active) => {
            Some("docs/specs")
        }
        (DocType::Guideline, Status::Active) => Some("docs/guidelines"),
        _ => None,
    }
}

/// Returns whether a status is terminal for the given managed document type.
pub fn is_terminal_status(doc_type: DocType, status: Status) -> bool {
    match doc_type {
        DocType::Prd => status == Status::Obsolete,
        DocType::DesignDoc => matches!(status, Status::Implemented | Status::Obsolete),
        DocType::DesignPatch => matches!(
            status,
            Status::Implemented | Status::Obsolete | Status::ObsoleteMerged
        ),
        DocType::ExecPlan | DocType::TaskSpec => status == Status::Closed,
        DocType::ProjectSpec | DocType::OrgSpec | DocType::Guideline => status == Status::Active,
    }
}

/// Returns whether a status is considered live for association-aware validation.
pub fn is_live_status(doc_type: DocType, status: Status) -> bool {
    !is_terminal_status(doc_type, status)
}

impl AssociationSummary {
    /// Returns whether this association set is empty.
    pub fn is_empty(&self) -> bool {
        self.related.is_empty()
    }

    /// Returns whether every related document is in the provided status.
    pub fn all_in_status(&self, status: Status) -> bool {
        !self.related.is_empty()
            && self
                .related
                .iter()
                .all(|document| document.status == status)
    }

    /// Returns whether every related document is terminal for its own type.
    pub fn all_terminal(&self) -> bool {
        !self.related.is_empty()
            && self
                .related
                .iter()
                .all(|document| is_terminal_status(document.doc_type, document.status))
    }
}

/// Builds direct-association summaries for the provided document.
pub fn association_summaries(
    index: &DocumentIndex,
    document: &Document,
) -> Vec<AssociationSummary> {
    let owner = document.id.clone();
    let owner_id = owner.as_string();

    match document.doc_type {
        DocType::Prd => vec![AssociationSummary {
            kind: AssociationKind::PrdDesignDocs,
            owner,
            related: index
                .documents
                .values()
                .filter(|candidate| candidate.doc_type == DocType::DesignDoc)
                .filter(|candidate| candidate.frontmatter.prd.as_deref() == Some(owner_id.as_str()))
                .map(associated_document)
                .collect(),
        }],
        DocType::DesignDoc => vec![
            AssociationSummary {
                kind: AssociationKind::DesignDocPatches,
                owner: document.id.clone(),
                related: index
                    .documents
                    .values()
                    .filter(|candidate| candidate.doc_type == DocType::DesignPatch)
                    .filter(|candidate| {
                        candidate.frontmatter.parent.as_deref() == Some(owner_id.as_str())
                    })
                    .map(associated_document)
                    .collect(),
            },
            AssociationSummary {
                kind: AssociationKind::DesignDocExecPlans,
                owner: owner.clone(),
                related: index
                    .documents
                    .values()
                    .filter(|candidate| candidate.doc_type == DocType::ExecPlan)
                    .filter(|candidate| candidate.frontmatter.design_docs.contains(&owner_id))
                    .map(associated_document)
                    .collect(),
            },
            AssociationSummary {
                kind: AssociationKind::DesignDocTasks,
                owner,
                related: Vec::new(),
            },
        ],
        DocType::ExecPlan => vec![AssociationSummary {
            kind: AssociationKind::ExecPlanTasks,
            owner,
            related: index
                .documents
                .values()
                .filter(|candidate| candidate.doc_type == DocType::TaskSpec)
                .filter(|candidate| {
                    candidate.frontmatter.exec_plan.as_deref() == Some(owner_id.as_str())
                })
                .map(associated_document)
                .collect(),
        }],
        _ => Vec::new(),
    }
}

/// Builds a predicted repository index for a proposed status transition.
pub fn preview_transition(
    index: &DocumentIndex,
    document: &Document,
    to: Status,
) -> Result<DocumentIndex> {
    let mut preview = index.clone();
    let entry = preview.documents.get_mut(&document.id).ok_or_else(|| {
        DocumentModelError::InvalidField {
            path: document.path.clone(),
            field: "id",
            message: format!(
                "document {} is not present in the current index",
                document.id
            ),
        }
    })?;

    entry.status = to;
    entry.path = expected_path_for_document(&preview.repo_root, entry, to)?;
    Ok(preview)
}

/// Performs repository-level validation for a predicted post-transition index.
pub fn validate_preview(index: &DocumentIndex) -> Vec<ValidationViolation> {
    validate_index(index)
}

/// Validates whether a status transition is legal for a loaded document.
pub fn validate_transition(index: &DocumentIndex, document: &Document, to: Status) -> Result<()> {
    let legal = match document.doc_type {
        DocType::Prd => matches!(
            (document.status, to),
            (Status::Draft, Status::Approved)
                | (Status::Approved, Status::Obsolete)
                | (Status::Draft, Status::Obsolete)
        ),
        DocType::DesignDoc => matches!(
            (document.status, to),
            (Status::Draft, Status::Candidate)
                | (Status::Candidate, Status::Implemented)
                | (Status::Candidate, Status::Obsolete)
                | (Status::Implemented, Status::Obsolete)
        ),
        DocType::DesignPatch => matches!(
            (document.status, to),
            (Status::Draft, Status::Candidate)
                | (Status::Draft, Status::Obsolete)
                | (Status::Candidate, Status::Implemented)
                | (Status::Candidate, Status::Obsolete)
                | (Status::Implemented, Status::ObsoleteMerged)
        ),
        DocType::ExecPlan => matches!(
            (document.status, to),
            (Status::Draft, Status::Closed)
                | (Status::Draft, Status::Candidate)
                | (Status::Candidate, Status::Draft)
                | (Status::Candidate, Status::Closed)
        ),
        DocType::TaskSpec => matches!(
            (document.status, to),
            (Status::Draft, Status::Closed)
                | (Status::Draft, Status::Candidate)
                | (Status::Candidate, Status::Draft)
                | (Status::Candidate, Status::Closed)
        ),
        DocType::ProjectSpec | DocType::OrgSpec | DocType::Guideline => false,
    };

    if !legal {
        return Err(DocumentModelError::IllegalTransition {
            doc_type: document.doc_type.as_str(),
            from: document.status.to_string(),
            to: to.to_string(),
        }
        .into());
    }

    validate_transition_gate(index, document, to)
}

/// Performs repository-level validation that depends on multiple loaded documents.
pub fn validate_index(index: &DocumentIndex) -> Vec<ValidationViolation> {
    let mut violations = Vec::new();

    for document in index.documents.values() {
        let frontmatter = &document.frontmatter;

        if document.doc_type == DocType::DesignPatch {
            match frontmatter.parent.as_deref().and_then(parse_reference_id) {
                Some(reference @ DocId::DesignDoc(_)) => {
                    if !index.documents.contains_key(&reference) {
                        violations.push(ValidationViolation {
                            path: document.path.clone(),
                            message: format!("parent {} does not exist", reference),
                        });
                    }
                }
                Some(other) => violations.push(ValidationViolation {
                    path: document.path.clone(),
                    message: format!("parent must reference a Design Doc, found {other}"),
                }),
                None => {}
            }
        }

        if let Some(merged_into) = frontmatter.merged_into.as_deref() {
            match parse_reference_id(merged_into) {
                Some(reference @ DocId::DesignDoc(_)) => {
                    if !index.documents.contains_key(&reference) {
                        violations.push(ValidationViolation {
                            path: document.path.clone(),
                            message: format!("merged-into {} does not exist", reference),
                        });
                    }
                }
                _ => violations.push(ValidationViolation {
                    path: document.path.clone(),
                    message: format!(
                        "merged-into must reference a Design Doc, found {merged_into}"
                    ),
                }),
            }
        }

        if let Some(superseded_by) = frontmatter.superseded_by.as_deref() {
            match parse_reference_id(superseded_by) {
                Some(reference @ DocId::DesignDoc(_)) => {
                    if !index.documents.contains_key(&reference) {
                        violations.push(ValidationViolation {
                            path: document.path.clone(),
                            message: format!("superseded-by {} does not exist", reference),
                        });
                    }
                }
                _ => violations.push(ValidationViolation {
                    path: document.path.clone(),
                    message: format!(
                        "superseded-by must reference a Design Doc, found {superseded_by}"
                    ),
                }),
            }
        }

        validate_relationships(index, document, &mut violations);

        if document.doc_type == DocType::TaskSpec {
            let mut seen = BTreeSet::new();
            for criterion in &frontmatter.completion_criteria {
                if !is_completion_criterion_id(&criterion.id) {
                    violations.push(ValidationViolation {
                        path: document.path.clone(),
                        message: format!(
                            "completion criterion id {} must use cc-NNN format",
                            criterion.id
                        ),
                    });
                }
                if !seen.insert(criterion.id.clone()) {
                    violations.push(ValidationViolation {
                        path: document.path.clone(),
                        message: format!("duplicate completion criterion id {}", criterion.id),
                    });
                }
            }
        }
    }

    violations
}

/// Fails if the loaded repository index contains invalid entries or validation violations.
pub fn ensure_index_compliant(index: &DocumentIndex) -> Result<()> {
    if let Some(entry) = index.invalid_entries.first() {
        return Err(DocumentModelError::InvalidRepositoryState {
            path: index.repo_root.clone(),
            message: format!(
                "found {} invalid managed entr{}; first violation: {} ({})",
                index.invalid_entries.len(),
                if index.invalid_entries.len() == 1 {
                    "y"
                } else {
                    "ies"
                },
                entry.path.display(),
                entry.reason
            ),
        }
        .into());
    }

    let violations = validate_index(index);
    if let Some(violation) = violations.first() {
        return Err(DocumentModelError::InvalidRepositoryState {
            path: index.repo_root.clone(),
            message: format!(
                "found {} repository-level validation violation{}; first violation: {} ({})",
                violations.len(),
                if violations.len() == 1 { "" } else { "s" },
                violation.path.display(),
                violation.message
            ),
        }
        .into());
    }

    Ok(())
}

fn should_visit(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    name != ".git" && name != "target"
}

fn associated_document(document: &Document) -> AssociatedDocument {
    AssociatedDocument {
        id: document.id.clone(),
        doc_type: document.doc_type,
        status: document.status,
    }
}

fn validate_transition_gate(index: &DocumentIndex, document: &Document, to: Status) -> Result<()> {
    match (document.doc_type, to) {
        (DocType::Prd, Status::Obsolete) => {
            let prd_id = document.id.as_string();
            if let Some(blocking_design) = index.documents.values().find(|candidate| {
                candidate.doc_type == DocType::DesignDoc
                    && is_live_status(candidate.doc_type, candidate.status)
                    && candidate.frontmatter.prd.as_deref() == Some(prd_id.as_str())
            }) {
                return invalid_transition_field(
                    document,
                    format!(
                        "cannot transition to obsolete while {} is {}",
                        blocking_design.id, blocking_design.status
                    ),
                );
            }
        }
        (DocType::DesignDoc, Status::Implemented) => {
            let design_id = document.id.as_string();
            if let Some(blocking_plan) = index.documents.values().find(|candidate| {
                candidate.doc_type == DocType::ExecPlan
                    && candidate.frontmatter.design_docs.contains(&design_id)
                    && candidate.status != Status::Closed
            }) {
                return invalid_transition_field(
                    document,
                    format!(
                        "cannot transition to implemented while {} is {}",
                        blocking_plan.id, blocking_plan.status
                    ),
                );
            }
        }
        (DocType::DesignDoc, Status::Obsolete) => {
            let design_id = document.id.as_string();
            if let Some(blocking_plan) = index.documents.values().find(|candidate| {
                candidate.doc_type == DocType::ExecPlan
                    && is_live_status(candidate.doc_type, candidate.status)
                    && candidate.frontmatter.design_docs.contains(&design_id)
            }) {
                return invalid_transition_field(
                    document,
                    format!(
                        "cannot transition to obsolete while {} is {}",
                        blocking_plan.id, blocking_plan.status
                    ),
                );
            }
        }
        (DocType::DesignPatch, Status::ObsoleteMerged) => {
            match document
                .frontmatter
                .merged_into
                .as_deref()
                .and_then(parse_reference_id)
            {
                Some(reference @ DocId::DesignDoc(_)) => {
                    if !index.documents.contains_key(&reference) {
                        return invalid_transition_field(
                        document,
                        format!(
                            "cannot transition to obsolete:merged because merged-into {reference} does not exist"
                        ),
                    );
                    }
                }
                _ => return invalid_transition_field(
                    document,
                    "cannot transition to obsolete:merged without a valid merged-into Design Doc"
                        .to_string(),
                ),
            }
        }
        (DocType::ExecPlan, Status::Closed) => {
            let exec_id = document.id.as_string();
            if let Some(blocking_task) = index.documents.values().find(|candidate| {
                candidate.doc_type == DocType::TaskSpec
                    && candidate.frontmatter.exec_plan.as_deref() == Some(exec_id.as_str())
                    && candidate.status != Status::Closed
            }) {
                return invalid_transition_field(
                    document,
                    format!(
                        "cannot transition to closed while {} is {}",
                        blocking_task.id, blocking_task.status
                    ),
                );
            }
        }
        _ => {}
    }

    Ok(())
}

fn invalid_transition_field(document: &Document, message: String) -> Result<()> {
    Err(DocumentModelError::InvalidField {
        path: document.path.clone(),
        field: "status",
        message,
    }
    .into())
}

fn validate_relationships(
    index: &DocumentIndex,
    document: &Document,
    violations: &mut Vec<ValidationViolation>,
) {
    let frontmatter = &document.frontmatter;

    if let Some(prd) = frontmatter.prd.as_deref() {
        match parse_reference_id(prd) {
            Some(reference @ DocId::Prd(_)) => match index.documents.get(&reference) {
                Some(target)
                    if !(is_live_status(document.doc_type, document.status)
                        && target.status == Status::Obsolete) => {}
                Some(_) => violations.push(ValidationViolation {
                    path: document.path.clone(),
                    message: format!("prd {} is obsolete", reference),
                }),
                None => violations.push(ValidationViolation {
                    path: document.path.clone(),
                    message: format!("prd {} does not exist", reference),
                }),
            },
            _ => violations.push(ValidationViolation {
                path: document.path.clone(),
                message: format!("prd must reference a PRD, found {prd}"),
            }),
        }
    }

    if let Some(exec_plan) = frontmatter.exec_plan.as_deref() {
        match parse_reference_id(exec_plan) {
            Some(reference @ DocId::ExecPlan(_)) => match index.documents.get(&reference) {
                Some(target)
                    if !(is_live_status(document.doc_type, document.status)
                        && target.status == Status::Closed) => {}
                Some(_) => violations.push(ValidationViolation {
                    path: document.path.clone(),
                    message: format!("exec-plan {} has invalid status closed", reference),
                }),
                None => violations.push(ValidationViolation {
                    path: document.path.clone(),
                    message: format!("exec-plan {} does not exist", reference),
                }),
            },
            _ => violations.push(ValidationViolation {
                path: document.path.clone(),
                message: format!("exec-plan must reference an Exec Plan, found {exec_plan}"),
            }),
        }
    }

    for guideline in &frontmatter.guidelines {
        match parse_guideline_reference(guideline) {
            Some(reference) => {
                let guideline_id = DocId::Guideline(reference);
                if !index.documents.contains_key(&guideline_id) {
                    violations.push(ValidationViolation {
                        path: document.path.clone(),
                        message: format!(
                            "guideline {} does not resolve to a Guideline",
                            guideline.trim()
                        ),
                    });
                }
            }
            None => violations.push(ValidationViolation {
                path: document.path.clone(),
                message: format!(
                    "guideline {} does not resolve to a Guideline",
                    guideline.trim()
                ),
            }),
        }
    }

    if document.doc_type == DocType::ExecPlan {
        if frontmatter.design_docs.is_empty() {
            violations.push(ValidationViolation {
                path: document.path.clone(),
                message: "design-docs must contain at least one reference".to_string(),
            });
        }

        let mut refs = BTreeSet::new();
        for raw in &frontmatter.design_docs {
            if !refs.insert(raw.clone()) {
                violations.push(ValidationViolation {
                    path: document.path.clone(),
                    message: format!("duplicate design-docs reference {}", raw),
                });
                continue;
            }

            match parse_reference_id(raw) {
                Some(reference @ DocId::DesignDoc(_)) => match index.documents.get(&reference) {
                    Some(target)
                        if target.status == Status::Candidate
                            || target.status == Status::Implemented => {}
                    Some(target) => violations.push(ValidationViolation {
                        path: document.path.clone(),
                        message: format!(
                            "design-docs reference {} has invalid status {}",
                            reference, target.status
                        ),
                    }),
                    None => violations.push(ValidationViolation {
                        path: document.path.clone(),
                        message: format!("design-docs reference {} does not exist", reference),
                    }),
                },
                Some(reference @ DocId::DesignPatch { .. }) => {
                    match index.documents.get(&reference) {
                        Some(target)
                            if target.status == Status::Candidate
                                || target.status == Status::Implemented => {}
                        Some(target) => violations.push(ValidationViolation {
                            path: document.path.clone(),
                            message: format!(
                                "design-docs reference {} has invalid status {}",
                                reference, target.status
                            ),
                        }),
                        None => violations.push(ValidationViolation {
                            path: document.path.clone(),
                            message: format!("design-docs reference {} does not exist", reference),
                        }),
                    }
                }
                _ => violations.push(ValidationViolation {
                    path: document.path.clone(),
                    message: format!(
                        "design-docs entry {} must reference a Design Doc or Design Patch",
                        raw
                    ),
                }),
            }
        }

        for raw in &frontmatter.design_docs {
            if let Some(DocId::DesignPatch { parent_slug, .. }) = parse_reference_id(raw) {
                let parent = DocId::DesignDoc(parent_slug.clone()).as_string();
                if !frontmatter.design_docs.contains(&parent) {
                    violations.push(ValidationViolation {
                        path: document.path.clone(),
                        message: format!(
                            "design patch {} requires parent design {} in design-docs",
                            raw, parent
                        ),
                    });
                }
            }

            if let Some(DocId::DesignDoc(slug)) = parse_reference_id(raw) {
                if slug.starts_with("design-principles-") {
                    // unreachable because parse_reference_id strips prefix, handled below
                    let _ = slug;
                }
            }
        }
    }

    if document.doc_type == DocType::TaskSpec && frontmatter.exec_plan.is_none() {
        violations.push(ValidationViolation {
            path: document.path.clone(),
            message: "TaskSpec must belong to an Exec Plan".to_string(),
        });
    }
}

#[derive(Debug, Clone)]
enum PathDisposition {
    Prd,
    Design,
    ExecPlan { exec_slug: String },
    Task { exec_slug: String },
    FixedManaged(DocType),
    GuidelineManaged(String),
    InvalidManaged(String),
    Ignored,
}

fn classify_path(relative: &Path) -> PathDisposition {
    let parts: Vec<String> = relative
        .iter()
        .map(|part| part.to_string_lossy().into_owned())
        .collect();
    if parts.is_empty() {
        return PathDisposition::Ignored;
    }

    let file_name = parts.last().map(String::as_str).unwrap_or_default();
    if file_name == "README.md" {
        return PathDisposition::Ignored;
    }

    match parts.as_slice() {
        [docs, specs, file] if docs == "docs" && specs == "specs" && file == "project.md" => {
            PathDisposition::FixedManaged(DocType::ProjectSpec)
        }
        [docs, specs, file] if docs == "docs" && specs == "specs" && file == "org.md" => {
            PathDisposition::FixedManaged(DocType::OrgSpec)
        }
        [docs, guidelines, file]
            if docs == "docs" && guidelines == "guidelines" && file.ends_with(".md") =>
        {
            PathDisposition::GuidelineManaged(file.trim_end_matches(".md").to_string())
        }
        [docs, guidelines, obsolete, file]
            if docs == "docs"
                && guidelines == "guidelines"
                && obsolete == "obsolete"
                && file.ends_with(".md") =>
        {
            PathDisposition::GuidelineManaged(format!("obsolete/{}", file.trim_end_matches(".md")))
        }
        [docs, prd, bucket, file]
            if docs == "docs"
                && prd == "prd"
                && matches!(bucket.as_str(), "draft" | "approved" | "obsolete")
                && file.ends_with(".md") =>
        {
            PathDisposition::Prd
        }
        [docs, design, bucket, file]
            if docs == "docs"
                && design == "design"
                && matches!(
                    bucket.as_str(),
                    "draft" | "candidate" | "implemented" | "obsolete"
                )
                && file.ends_with(".md") =>
        {
            PathDisposition::Design
        }
        [docs, exec_plans, exec_dir, file]
            if docs == "docs"
                && exec_plans == "exec-plans"
                && file == "plan.md"
                && exec_dir.starts_with("exec-") =>
        {
            PathDisposition::ExecPlan {
                exec_slug: exec_dir.trim_start_matches("exec-").to_string(),
            }
        }
        [docs, exec_plans, exec_dir, file]
            if docs == "docs"
                && exec_plans == "exec-plans"
                && exec_dir.starts_with("exec-")
                && file.ends_with(".md")
                && file != "plan.md"
                && !file.ends_with("-report.md") =>
        {
            PathDisposition::Task {
                exec_slug: exec_dir.trim_start_matches("exec-").to_string(),
            }
        }
        [docs, exec_plans, exec_dir, file]
            if docs == "docs"
                && exec_plans == "exec-plans"
                && exec_dir.starts_with("exec-")
                && file.ends_with("-report.md") =>
        {
            PathDisposition::Ignored
        }
        _ if file_name.ends_with(".md") && is_under_managed_root(&parts) => {
            PathDisposition::InvalidManaged(format!(
                "unsupported markdown location {}",
                relative.display()
            ))
        }
        _ if file_name.ends_with(".md") => PathDisposition::Ignored,
        _ => PathDisposition::Ignored,
    }
}

fn is_under_managed_root(parts: &[String]) -> bool {
    if parts.is_empty() {
        return false;
    }
    parts.len() >= 2
        && parts[0] == "docs"
        && matches!(
            parts[1].as_str(),
            "guidelines" | "specs" | "prd" | "design" | "exec-plans"
        )
}

fn load_document_from_path(
    repo_root: &Path,
    relative: &Path,
    disposition: PathDisposition,
) -> Result<Document> {
    let absolute = repo_root.join(relative);
    let raw = fs::read_to_string(&absolute)
        .with_context(|| format!("reading managed document {}", absolute.display()))?;
    let frontmatter = parse_frontmatter(relative, &raw)?;

    let (doc_type, canonical_id) = match disposition {
        PathDisposition::Prd => parse_prd_id(relative)?,
        PathDisposition::Design => parse_design_id(relative)?,
        PathDisposition::ExecPlan { exec_slug } => (DocType::ExecPlan, DocId::ExecPlan(exec_slug)),
        PathDisposition::Task { exec_slug } => parse_task_id(relative, &exec_slug)?,
        PathDisposition::FixedManaged(doc_type) => {
            let id = match doc_type {
                DocType::ProjectSpec => DocId::ProjectSpec,
                DocType::OrgSpec => DocId::OrgSpec,
                _ => {
                    return Err(DocumentModelError::InvalidManagedPath {
                        path: relative.to_path_buf(),
                    }
                    .into())
                }
            };
            (doc_type, id)
        }
        PathDisposition::GuidelineManaged(relative_id) => {
            (DocType::Guideline, DocId::Guideline(relative_id))
        }
        PathDisposition::InvalidManaged(reason) => {
            return Err(DocumentModelError::InvalidField {
                path: relative.to_path_buf(),
                field: "path",
                message: reason,
            }
            .into())
        }
        PathDisposition::Ignored => {
            return Err(DocumentModelError::InvalidManagedPath {
                path: relative.to_path_buf(),
            }
            .into())
        }
    };

    validate_frontmatter(relative, doc_type, &canonical_id, &frontmatter)?;
    let status = parse_status(relative, doc_type, &frontmatter)?;
    validate_directory(relative, doc_type, &canonical_id, status)?;

    Ok(Document {
        id: canonical_id,
        doc_type,
        status,
        title: frontmatter
            .title
            .clone()
            .map(|title| title.trim().to_string()),
        path: absolute,
        frontmatter,
        raw,
    })
}

fn parse_prd_id(relative: &Path) -> Result<(DocType, DocId)> {
    let stem = file_stem(relative)?;
    let slug = stem
        .strip_prefix("prd-")
        .filter(|slug| valid_slug(slug))
        .ok_or_else(|| DocumentModelError::InvalidFilename {
            path: relative.to_path_buf(),
            doc_type: DocType::Prd.as_str(),
        })?;
    Ok((DocType::Prd, DocId::Prd(slug.to_string())))
}

fn parse_design_id(relative: &Path) -> Result<(DocType, DocId)> {
    let stem = file_stem(relative)?;
    let remainder =
        stem.strip_prefix("design-")
            .ok_or_else(|| DocumentModelError::InvalidFilename {
                path: relative.to_path_buf(),
                doc_type: "DesignDoc/DesignPatch",
            })?;

    if let Some((parent_slug, rest)) = remainder.split_once("-patch-") {
        let (sequence_raw, patch_slug) =
            rest.split_once('-')
                .ok_or_else(|| DocumentModelError::InvalidFilename {
                    path: relative.to_path_buf(),
                    doc_type: DocType::DesignPatch.as_str(),
                })?;
        let sequence = parse_local_sequence(sequence_raw).ok_or_else(|| {
            DocumentModelError::InvalidFilename {
                path: relative.to_path_buf(),
                doc_type: DocType::DesignPatch.as_str(),
            }
        })?;
        if !valid_slug(parent_slug) || !valid_slug(patch_slug) {
            return Err(DocumentModelError::InvalidFilename {
                path: relative.to_path_buf(),
                doc_type: DocType::DesignPatch.as_str(),
            }
            .into());
        }
        return Ok((
            DocType::DesignPatch,
            DocId::DesignPatch {
                parent_slug: parent_slug.to_string(),
                sequence,
                patch_slug: patch_slug.to_string(),
            },
        ));
    }

    if !valid_slug(remainder) {
        return Err(DocumentModelError::InvalidFilename {
            path: relative.to_path_buf(),
            doc_type: DocType::DesignDoc.as_str(),
        }
        .into());
    }

    Ok((DocType::DesignDoc, DocId::DesignDoc(remainder.to_string())))
}

fn parse_task_id(relative: &Path, exec_slug: &str) -> Result<(DocType, DocId)> {
    let stem = file_stem(relative)?;
    let remainder =
        stem.strip_prefix("task-")
            .ok_or_else(|| DocumentModelError::InvalidFilename {
                path: relative.to_path_buf(),
                doc_type: DocType::TaskSpec.as_str(),
            })?;
    let (sequence_raw, task_slug) =
        remainder
            .split_once('-')
            .ok_or_else(|| DocumentModelError::InvalidFilename {
                path: relative.to_path_buf(),
                doc_type: DocType::TaskSpec.as_str(),
            })?;
    let sequence =
        parse_local_sequence(sequence_raw).ok_or_else(|| DocumentModelError::InvalidFilename {
            path: relative.to_path_buf(),
            doc_type: DocType::TaskSpec.as_str(),
        })?;
    if !valid_slug(task_slug) {
        return Err(DocumentModelError::InvalidFilename {
            path: relative.to_path_buf(),
            doc_type: DocType::TaskSpec.as_str(),
        }
        .into());
    }

    Ok((
        DocType::TaskSpec,
        DocId::TaskSpec {
            exec_slug: exec_slug.to_string(),
            sequence,
        },
    ))
}

fn validate_frontmatter(
    relative: &Path,
    doc_type: DocType,
    canonical_id: &DocId,
    frontmatter: &Frontmatter,
) -> Result<()> {
    match doc_type {
        DocType::Guideline => {
            require_non_empty(relative, "title", frontmatter.title.as_deref())?;
            if frontmatter.id.is_some() {
                return Err(DocumentModelError::InvalidField {
                    path: relative.to_path_buf(),
                    field: "id",
                    message: "Guidelines do not declare id".to_string(),
                }
                .into());
            }
            if frontmatter.status.is_some() {
                return Err(DocumentModelError::InvalidField {
                    path: relative.to_path_buf(),
                    field: "status",
                    message: "Guidelines do not declare status".to_string(),
                }
                .into());
            }
        }
        DocType::ProjectSpec | DocType::OrgSpec => {
            let found = require_field(relative, "id", frontmatter.id.as_deref())?;
            if found != canonical_id.frontmatter_id() {
                return Err(DocumentModelError::IdMismatch {
                    path: relative.to_path_buf(),
                    expected: canonical_id.frontmatter_id(),
                    found: found.to_string(),
                }
                .into());
            }
        }
        _ => {
            let found = require_field(relative, "id", frontmatter.id.as_deref())?;
            if found != canonical_id.frontmatter_id() {
                return Err(DocumentModelError::IdMismatch {
                    path: relative.to_path_buf(),
                    expected: canonical_id.frontmatter_id(),
                    found: found.to_string(),
                }
                .into());
            }
            require_non_empty(relative, "title", frontmatter.title.as_deref())?;
            require_non_empty(relative, "created", frontmatter.created.as_deref())?;
            validate_date_field(relative, "created", frontmatter.created.as_deref())?;
        }
    }

    if matches!(doc_type, DocType::ExecPlan | DocType::TaskSpec) {
        if frontmatter.closed.is_some() {
            validate_date_field(relative, "closed", frontmatter.closed.as_deref())?;
        }
    } else if frontmatter.closed.is_some() {
        return Err(DocumentModelError::InvalidField {
            path: relative.to_path_buf(),
            field: "closed",
            message: "closed is only allowed on ExecPlan and TaskSpec".to_string(),
        }
        .into());
    }

    if doc_type == DocType::DesignPatch {
        require_field(relative, "parent", frontmatter.parent.as_deref())?;
    }

    if matches!(doc_type, DocType::ExecPlan)
        && !frontmatter.design_docs.is_empty()
        && frontmatter.design_doc.is_some()
    {
        return Err(DocumentModelError::InvalidField {
            path: relative.to_path_buf(),
            field: "design-docs",
            message: "ExecPlan must not declare both design-doc and design-docs".to_string(),
        }
        .into());
    }

    if doc_type == DocType::TaskSpec && frontmatter.exec_plan.is_none() {
        return Err(DocumentModelError::MissingField {
            path: relative.to_path_buf(),
            field: "exec-plan",
        }
        .into());
    }

    if doc_type == DocType::TaskSpec && frontmatter.design_doc.is_some() {
        return Err(DocumentModelError::InvalidField {
            path: relative.to_path_buf(),
            field: "design-doc",
            message: "TaskSpec must not declare design-doc in the new model".to_string(),
        }
        .into());
    }

    if doc_type == DocType::TaskSpec && frontmatter.status.as_deref() == Some("candidate") {
        let boundaries =
            frontmatter
                .boundaries
                .as_ref()
                .ok_or_else(|| DocumentModelError::MissingField {
                    path: relative.to_path_buf(),
                    field: "boundaries",
                })?;
        if boundaries.allowed.is_empty() {
            return Err(DocumentModelError::InvalidField {
                path: relative.to_path_buf(),
                field: "boundaries.allowed",
                message: "must contain at least one pattern".to_string(),
            }
            .into());
        }
        for required in [
            "docs/prd/**",
            "docs/design/**",
            "docs/guidelines/**",
            "docs/specs/**",
            "docs/exec-plans/**",
        ] {
            if !boundaries
                .forbidden_patterns
                .iter()
                .any(|pattern| pattern == required)
            {
                return Err(DocumentModelError::InvalidField {
                    path: relative.to_path_buf(),
                    field: "boundaries.forbidden_patterns",
                    message: format!("must include {required}"),
                }
                .into());
            }
        }
        if frontmatter.completion_criteria.is_empty() {
            return Err(DocumentModelError::InvalidField {
                path: relative.to_path_buf(),
                field: "completion_criteria",
                message: "must contain at least one criterion".to_string(),
            }
            .into());
        }
        for criterion in &frontmatter.completion_criteria {
            if criterion.id.trim().is_empty()
                || criterion.scenario.trim().is_empty()
                || criterion.test.trim().is_empty()
            {
                return Err(DocumentModelError::InvalidField {
                    path: relative.to_path_buf(),
                    field: "completion_criteria",
                    message: "each criterion must include non-empty id, scenario, and test"
                        .to_string(),
                }
                .into());
            }
            if !is_completion_criterion_id(&criterion.id) {
                return Err(DocumentModelError::InvalidField {
                    path: relative.to_path_buf(),
                    field: "completion_criteria.id",
                    message: format!("{} must use cc-NNN format", criterion.id.trim()),
                }
                .into());
            }
        }
    }

    if matches!(doc_type, DocType::ExecPlan | DocType::TaskSpec)
        && frontmatter.status.as_deref() == Some("closed")
        && frontmatter.closed.is_none()
    {
        return Err(DocumentModelError::MissingField {
            path: relative.to_path_buf(),
            field: "closed",
        }
        .into());
    }

    if matches!(doc_type, DocType::ExecPlan | DocType::TaskSpec)
        && frontmatter.status.as_deref() != Some("closed")
        && frontmatter.closed.is_some()
    {
        return Err(DocumentModelError::InvalidField {
            path: relative.to_path_buf(),
            field: "closed",
            message: "must be absent unless status is closed".to_string(),
        }
        .into());
    }

    if matches!(doc_type, DocType::DesignPatch)
        && frontmatter.status.as_deref() == Some("obsolete:merged")
    {
        require_field(relative, "merged-into", frontmatter.merged_into.as_deref())?;
    }

    Ok(())
}

fn parse_status(relative: &Path, doc_type: DocType, frontmatter: &Frontmatter) -> Result<Status> {
    if doc_type == DocType::Guideline {
        return Ok(Status::Active);
    }

    let raw = require_field(relative, "status", frontmatter.status.as_deref())?;
    match (doc_type, raw) {
        (DocType::Prd, "draft") => Ok(Status::Draft),
        (DocType::Prd, "approved") => Ok(Status::Approved),
        (DocType::Prd, "obsolete") => Ok(Status::Obsolete),
        (DocType::DesignDoc, "draft") => Ok(Status::Draft),
        (DocType::DesignDoc, "candidate") => Ok(Status::Candidate),
        (DocType::DesignDoc, "implemented") => Ok(Status::Implemented),
        (DocType::DesignDoc, "obsolete") => Ok(Status::Obsolete),
        (DocType::DesignPatch, "draft") => Ok(Status::Draft),
        (DocType::DesignPatch, "candidate") => Ok(Status::Candidate),
        (DocType::DesignPatch, "implemented") => Ok(Status::Implemented),
        (DocType::DesignPatch, "obsolete") => Ok(Status::Obsolete),
        (DocType::DesignPatch, "obsolete:merged") => Ok(Status::ObsoleteMerged),
        (DocType::ExecPlan, "draft") => Ok(Status::Draft),
        (DocType::ExecPlan, "candidate") => Ok(Status::Candidate),
        (DocType::ExecPlan, "closed") => Ok(Status::Closed),
        (DocType::TaskSpec, "draft") => Ok(Status::Draft),
        (DocType::TaskSpec, "candidate") => Ok(Status::Candidate),
        (DocType::TaskSpec, "closed") => Ok(Status::Closed),
        (DocType::ProjectSpec | DocType::OrgSpec, "active") => Ok(Status::Active),
        _ => Err(DocumentModelError::InvalidStatus {
            path: relative.to_path_buf(),
            doc_type: doc_type.as_str(),
            status: raw.to_string(),
        }
        .into()),
    }
}

fn validate_directory(
    relative: &Path,
    doc_type: DocType,
    canonical_id: &DocId,
    status: Status,
) -> Result<()> {
    match doc_type {
        DocType::ExecPlan => {
            let parent = relative.parent().unwrap_or_else(|| Path::new(""));
            let expected = format!("docs/exec-plans/{}", canonical_id.as_string());
            let actual = path_to_unix(parent);
            if actual != expected {
                return Err(DocumentModelError::InvalidField {
                    path: relative.to_path_buf(),
                    field: "path",
                    message: format!("expected directory {expected}, found {actual}"),
                }
                .into());
            }
        }
        DocType::TaskSpec => {
            let parent = relative.parent().unwrap_or_else(|| Path::new(""));
            let exec_dir = canonical_id
                .exec_slug()
                .map(|slug| format!("docs/exec-plans/exec-{slug}"))
                .ok_or_else(|| DocumentModelError::InvalidField {
                    path: relative.to_path_buf(),
                    field: "path",
                    message: "task id is missing exec slug".to_string(),
                })?;
            let actual = path_to_unix(parent);
            if actual != exec_dir {
                return Err(DocumentModelError::InvalidField {
                    path: relative.to_path_buf(),
                    field: "path",
                    message: format!("expected directory {exec_dir}, found {actual}"),
                }
                .into());
            }
        }
        _ => {
            let parent = relative.parent().unwrap_or_else(|| Path::new(""));
            let expected = expected_directory(doc_type, status).ok_or_else(|| {
                DocumentModelError::InvalidField {
                    path: relative.to_path_buf(),
                    field: "status",
                    message: format!("no directory mapping for {} {}", doc_type, status),
                }
            })?;
            let actual = path_to_unix(parent);
            if actual != expected {
                return Err(DocumentModelError::InvalidField {
                    path: relative.to_path_buf(),
                    field: "path",
                    message: format!("expected directory {expected}, found {actual}"),
                }
                .into());
            }
        }
    }
    Ok(())
}

fn expected_path_for_document(
    repo_root: &Path,
    document: &Document,
    to: Status,
) -> Result<PathBuf> {
    let file_name = document
        .path
        .file_name()
        .ok_or_else(|| DocumentModelError::InvalidField {
            path: document.path.clone(),
            field: "path",
            message: "missing file name".to_string(),
        })?;

    match document.doc_type {
        DocType::ExecPlan | DocType::TaskSpec => Ok(document.path.clone()),
        _ => {
            let expected = expected_directory(document.doc_type, to).ok_or_else(|| {
                DocumentModelError::InvalidField {
                    path: document.path.clone(),
                    field: "status",
                    message: format!("no directory mapping for {} {}", document.doc_type, to),
                }
            })?;
            Ok(repo_root.join(expected).join(file_name))
        }
    }
}

fn require_field<'a>(path: &Path, field: &'static str, value: Option<&'a str>) -> Result<&'a str> {
    let value = value.ok_or_else(|| DocumentModelError::MissingField {
        path: path.to_path_buf(),
        field,
    })?;
    if value.trim().is_empty() {
        return Err(DocumentModelError::InvalidField {
            path: path.to_path_buf(),
            field,
            message: "must be non-empty".to_string(),
        }
        .into());
    }
    Ok(value.trim())
}

fn require_non_empty(path: &Path, field: &'static str, value: Option<&str>) -> Result<()> {
    require_field(path, field, value).map(|_| ())
}

fn validate_date_field(path: &Path, field: &'static str, value: Option<&str>) -> Result<()> {
    let value = require_field(path, field, value)?;
    if !valid_date(value) {
        return Err(DocumentModelError::InvalidField {
            path: path.to_path_buf(),
            field,
            message: format!("{value} must use valid YYYY-MM-DD form"),
        }
        .into());
    }
    Ok(())
}

fn valid_date(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    if parts.len() != 3
        || parts[0].len() != 4
        || parts[1].len() != 2
        || parts[2].len() != 2
        || !parts
            .iter()
            .all(|part| part.chars().all(|ch| ch.is_ascii_digit()))
    {
        return false;
    }

    let year = parts[0].parse::<u32>().ok();
    let month = parts[1].parse::<u32>().ok();
    let day = parts[2].parse::<u32>().ok();
    let (Some(year), Some(month), Some(day)) = (year, month, day) else {
        return false;
    };

    if !(1..=12).contains(&month) || day == 0 {
        return false;
    }

    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => return false,
    };

    day <= days_in_month
}

fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn file_stem(path: &Path) -> Result<&str> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| {
            DocumentModelError::InvalidFilename {
                path: path.to_path_buf(),
                doc_type: "unknown",
            }
            .into()
        })
}

fn valid_slug(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    !parts.is_empty()
        && parts.iter().all(|part| {
            let mut chars = part.chars();
            matches!(chars.next(), Some(first) if first.is_ascii_lowercase())
                && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
        })
}

fn parse_local_sequence(value: &str) -> Option<u32> {
    if value.len() < 2 || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    value.parse::<u32>().ok().filter(|number| *number > 0)
}

fn is_completion_criterion_id(value: &str) -> bool {
    let parts: Vec<&str> = value.trim().split('-').collect();
    matches!(parts.as_slice(), ["cc", digits] if digits.len() == 3 && digits.chars().all(|ch| ch.is_ascii_digit()))
}

fn parse_reference_id(raw: &str) -> Option<DocId> {
    let raw = raw.trim();
    if raw == "project" {
        return Some(DocId::ProjectSpec);
    }
    if raw == "org" {
        return Some(DocId::OrgSpec);
    }

    if let Some(exec_part) = raw.strip_prefix("exec-") {
        if let Some((exec_slug, task_part)) = exec_part.split_once("/task-") {
            let sequence = parse_local_sequence(task_part)?;
            return Some(DocId::TaskSpec {
                exec_slug: exec_slug.to_string(),
                sequence,
            });
        }
        if valid_slug(exec_part) {
            return Some(DocId::ExecPlan(exec_part.to_string()));
        }
    }

    if let Some(prd_slug) = raw.strip_prefix("prd-") {
        return valid_slug(prd_slug).then(|| DocId::Prd(prd_slug.to_string()));
    }

    if let Some(design_remainder) = raw.strip_prefix("design-") {
        if let Some((parent_slug, rest)) = design_remainder.split_once("-patch-") {
            let (sequence_raw, patch_slug) = rest.split_once('-')?;
            let sequence = parse_local_sequence(sequence_raw)?;
            if valid_slug(parent_slug) && valid_slug(patch_slug) {
                return Some(DocId::DesignPatch {
                    parent_slug: parent_slug.to_string(),
                    sequence,
                    patch_slug: patch_slug.to_string(),
                });
            }
            return None;
        }
        return valid_slug(design_remainder)
            .then(|| DocId::DesignDoc(design_remainder.to_string()));
    }

    if let Some(task_remainder) = raw.strip_prefix("task-") {
        let sequence = parse_local_sequence(task_remainder)?;
        return Some(DocId::TaskSpec {
            exec_slug: String::new(),
            sequence,
        });
    }

    None
}

fn parse_guideline_reference(raw: &str) -> Option<String> {
    let raw = raw.trim().trim_start_matches("./");
    let relative = raw.strip_prefix("docs/guidelines/")?;
    let stem = relative.strip_suffix(".md")?;
    if stem.is_empty() {
        return None;
    }
    Some(stem.to_string())
}

fn path_to_unix(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

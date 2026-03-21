//! Shared document-model utilities used by specmate commands.

mod frontmatter;
mod id;
mod types;

#[allow(unused_imports)]
pub use id::{next_id, next_patch_number};
#[allow(unused_imports)]
pub use types::{
    Boundaries, CompletionCriterion, DocId, DocType, Document, DocumentIndex, Frontmatter,
    InvalidManagedEntry, Status, ValidationViolation,
};

use crate::error::DocumentModelError;
use anyhow::{Context, Result};
use frontmatter::parse_frontmatter;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Clone, Copy)]
enum FilenameFamily {
    Prd,
    Design,
    Exec,
    Task,
}

#[derive(Debug, Clone)]
enum PathDisposition {
    FilenameManaged(FilenameFamily),
    FixedManaged(DocType),
    GuidelineManaged(String),
    InvalidManaged(String),
    Ignored,
}

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
                })
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

/// Resolves the expected relative directory for a `(DocType, Status)` pair.
pub fn expected_directory(doc_type: DocType, status: Status) -> Option<&'static str> {
    match (doc_type, status) {
        (DocType::Prd, Status::Draft) => Some("docs/prd/draft"),
        (DocType::Prd, Status::Approved) => Some("docs/prd/approved"),
        (DocType::Prd, Status::Obsolete) => Some("docs/prd/obsolete"),
        (DocType::DesignDoc, Status::Draft) => Some("docs/design-docs/draft"),
        (DocType::DesignDoc, Status::Candidate) => Some("docs/design-docs/candidate"),
        (DocType::DesignDoc, Status::Implemented) => Some("docs/design-docs/implemented"),
        (DocType::DesignDoc, Status::Obsolete) => Some("docs/design-docs/obsolete"),
        (DocType::DesignPatch, Status::Draft) => Some("docs/design-docs/draft"),
        (DocType::DesignPatch, Status::Candidate) => Some("docs/design-docs/candidate"),
        (DocType::DesignPatch, Status::Implemented) => Some("docs/design-docs/implemented"),
        (DocType::DesignPatch, Status::Obsolete) => Some("docs/design-docs/obsolete"),
        (DocType::DesignPatch, Status::ObsoleteMerged) => Some("docs/design-docs/obsolete"),
        (DocType::ExecPlan, Status::Draft) => Some("docs/exec-plans/draft"),
        (DocType::ExecPlan, Status::Active) => Some("docs/exec-plans/active"),
        (DocType::ExecPlan, Status::Completed) => Some("docs/exec-plans/archived"),
        (DocType::ExecPlan, Status::Abandoned) => Some("docs/exec-plans/archived"),
        (DocType::TaskSpec, Status::Draft) => Some("specs/active"),
        (DocType::TaskSpec, Status::Active) => Some("specs/active"),
        (DocType::TaskSpec, Status::Completed) => Some("specs/archived"),
        (DocType::TaskSpec, Status::Cancelled) => Some("specs/archived"),
        (DocType::Guideline, Status::Active) => Some("docs/guidelines"),
        (DocType::ProjectSpec, Status::Active) | (DocType::OrgSpec, Status::Active) => {
            Some("specs")
        }
        _ => None,
    }
}

/// Validates whether a status transition is legal for a loaded document.
pub fn validate_transition(index: &DocumentIndex, document: &Document, to: Status) -> Result<()> {
    let doc_type = document.doc_type;
    let from = document.status;
    let legal = match doc_type {
        DocType::Prd => matches!(
            (from, to),
            (Status::Draft, Status::Approved)
                | (Status::Approved, Status::Obsolete)
                | (Status::Draft, Status::Obsolete)
        ),
        DocType::DesignDoc => matches!(
            (from, to),
            (Status::Draft, Status::Candidate)
                | (Status::Candidate, Status::Implemented)
                | (Status::Candidate, Status::Obsolete)
                | (Status::Implemented, Status::Obsolete)
        ),
        DocType::DesignPatch => matches!(
            (from, to),
            (Status::Draft, Status::Candidate)
                | (Status::Draft, Status::Obsolete)
                | (Status::Candidate, Status::Implemented)
                | (Status::Candidate, Status::Obsolete)
                | (Status::Implemented, Status::ObsoleteMerged)
        ),
        DocType::ExecPlan => matches!(
            (from, to),
            (Status::Draft, Status::Active)
                | (Status::Active, Status::Completed)
                | (Status::Active, Status::Abandoned)
        ),
        DocType::TaskSpec => matches!(
            (from, to),
            (Status::Draft, Status::Active)
                | (Status::Active, Status::Completed)
                | (Status::Active, Status::Cancelled)
                | (Status::Draft, Status::Cancelled)
        ),
        DocType::ProjectSpec | DocType::OrgSpec | DocType::Guideline => false,
    };

    if legal {
        if matches!(doc_type, DocType::DesignDoc)
            && from == Status::Candidate
            && to == Status::Implemented
        {
            let design_id = document.id.as_string();
            if let Some(blocking_plan) = index.documents.values().find(|candidate| {
                candidate.doc_type == DocType::ExecPlan
                    && candidate.frontmatter.design_doc.as_deref() == Some(design_id.as_str())
                    && candidate.status != Status::Completed
            }) {
                return Err(DocumentModelError::InvalidField {
                    path: document.path.clone(),
                    field: "status",
                    message: format!(
                        "cannot transition to implemented while {} is {}",
                        blocking_plan.id, blocking_plan.status
                    ),
                }
                .into());
            }
        }

        Ok(())
    } else {
        Err(DocumentModelError::IllegalTransition {
            doc_type: doc_type.as_str(),
            from: from.to_string(),
            to: to.to_string(),
        }
        .into())
    }
}

/// Performs repository-level validation that depends on multiple loaded documents.
pub fn validate_index(index: &DocumentIndex) -> Vec<ValidationViolation> {
    let mut violations = Vec::new();

    for document in index.documents.values() {
        let frontmatter = &document.frontmatter;

        if let DocType::DesignPatch = document.doc_type {
            match frontmatter.parent.as_deref().and_then(parse_reference_id) {
                Some(parent_id @ DocId::DesignDoc(_)) => {
                    if !index.documents.contains_key(&parent_id) {
                        violations.push(ValidationViolation {
                            path: document.path.clone(),
                            message: format!("parent {} does not exist", parent_id),
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

        if let Some(design_doc) = frontmatter.design_doc.as_deref() {
            match parse_reference_id(design_doc) {
                Some(reference @ DocId::DesignDoc(_)) => match index.documents.get(&reference) {
                    Some(target) if target.status != Status::Obsolete => {}
                    Some(_) => violations.push(ValidationViolation {
                        path: document.path.clone(),
                        message: format!("design-doc {} is obsolete", reference),
                    }),
                    None => violations.push(ValidationViolation {
                        path: document.path.clone(),
                        message: format!("design-doc {} does not exist", reference),
                    }),
                },
                _ => violations.push(ValidationViolation {
                    path: document.path.clone(),
                    message: format!("design-doc must reference a Design Doc, found {design_doc}"),
                }),
            }
        }

        if let Some(exec_plan) = frontmatter.exec_plan.as_deref() {
            match parse_reference_id(exec_plan) {
                Some(reference @ DocId::ExecPlan(_)) => match index.documents.get(&reference) {
                    Some(target)
                        if target.status != Status::Abandoned
                            && target.status != Status::ObsoleteMerged
                            && target.status != Status::Obsolete => {}
                    Some(target) => violations.push(ValidationViolation {
                        path: document.path.clone(),
                        message: format!(
                            "exec-plan {} has invalid status {}",
                            reference, target.status
                        ),
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

        if matches!(document.doc_type, DocType::DesignDoc)
            && matches!(document.status, Status::Candidate | Status::Implemented)
        {
            if let Some(prd) = frontmatter.prd.as_deref() {
                match parse_reference_id(prd) {
                    Some(reference @ DocId::Prd(_)) => match index.documents.get(&reference) {
                        Some(target) if target.status != Status::Obsolete => {}
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
        }

        if matches!(document.doc_type, DocType::TaskSpec) {
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
            for guideline in &frontmatter.guidelines {
                let absolute = index.repo_root.join(guideline.trim());
                let matching = index
                    .documents
                    .values()
                    .find(|candidate| candidate.path == absolute);
                if !matches!(matching.map(|doc| doc.doc_type), Some(DocType::Guideline)) {
                    violations.push(ValidationViolation {
                        path: document.path.clone(),
                        message: format!(
                            "guideline path {} does not resolve to a Guideline",
                            guideline
                        ),
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
        [specs, file] if specs == "specs" && file == "project.md" => {
            PathDisposition::FixedManaged(DocType::ProjectSpec)
        }
        [specs, file] if specs == "specs" && file == "org.md" => {
            PathDisposition::FixedManaged(DocType::OrgSpec)
        }
        [specs, bucket, file]
            if specs == "specs"
                && matches!(bucket.as_str(), "active" | "archived")
                && file.ends_with(".md") =>
        {
            PathDisposition::FilenameManaged(FilenameFamily::Task)
        }
        [docs, guidelines, file]
            if docs == "docs" && guidelines == "guidelines" && file.ends_with(".md") =>
        {
            let slug = file.trim_end_matches(".md").to_string();
            PathDisposition::GuidelineManaged(slug)
        }
        [docs, prd, bucket, file]
            if docs == "docs"
                && prd == "prd"
                && matches!(bucket.as_str(), "draft" | "approved" | "obsolete")
                && file.ends_with(".md") =>
        {
            PathDisposition::FilenameManaged(FilenameFamily::Prd)
        }
        [docs, design_docs, bucket, file]
            if docs == "docs"
                && design_docs == "design-docs"
                && matches!(
                    bucket.as_str(),
                    "draft" | "candidate" | "implemented" | "obsolete"
                )
                && file.ends_with(".md") =>
        {
            PathDisposition::FilenameManaged(FilenameFamily::Design)
        }
        [docs, exec_plans, bucket, file]
            if docs == "docs"
                && exec_plans == "exec-plans"
                && matches!(bucket.as_str(), "draft" | "active" | "archived")
                && file.ends_with(".md") =>
        {
            PathDisposition::FilenameManaged(FilenameFamily::Exec)
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
    if parts[0] == "specs" {
        return true;
    }
    parts.len() >= 2
        && parts[0] == "docs"
        && matches!(
            parts[1].as_str(),
            "guidelines" | "prd" | "design-docs" | "exec-plans"
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
        PathDisposition::FilenameManaged(family) => parse_filename_managed(relative, family)?,
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
        PathDisposition::GuidelineManaged(slug) => (DocType::Guideline, DocId::Guideline(slug)),
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
    validate_directory(relative, doc_type, status)?;

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

fn parse_filename_managed(relative: &Path, family: FilenameFamily) -> Result<(DocType, DocId)> {
    let stem = relative
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| DocumentModelError::InvalidFilename {
            path: relative.to_path_buf(),
            doc_type: "unknown",
        })?;
    let parts: Vec<&str> = stem.split('-').collect();
    match family {
        FilenameFamily::Prd => {
            if parts.len() < 3
                || parts[0] != "prd"
                || !is_fixed_digits(parts[1], 3)
                || !valid_slug(&parts[2..])
            {
                return Err(DocumentModelError::InvalidFilename {
                    path: relative.to_path_buf(),
                    doc_type: DocType::Prd.as_str(),
                }
                .into());
            }
            let id = parts[1].parse::<u32>()?;
            Ok((DocType::Prd, DocId::Prd(id)))
        }
        FilenameFamily::Exec => {
            if parts.len() < 3
                || parts[0] != "exec"
                || !is_fixed_digits(parts[1], 3)
                || !valid_slug(&parts[2..])
            {
                return Err(DocumentModelError::InvalidFilename {
                    path: relative.to_path_buf(),
                    doc_type: DocType::ExecPlan.as_str(),
                }
                .into());
            }
            let id = parts[1].parse::<u32>()?;
            Ok((DocType::ExecPlan, DocId::ExecPlan(id)))
        }
        FilenameFamily::Task => {
            if parts.len() < 3
                || parts[0] != "task"
                || !is_fixed_digits(parts[1], 4)
                || !valid_slug(&parts[2..])
            {
                return Err(DocumentModelError::InvalidFilename {
                    path: relative.to_path_buf(),
                    doc_type: DocType::TaskSpec.as_str(),
                }
                .into());
            }
            let id = parts[1].parse::<u32>()?;
            Ok((DocType::TaskSpec, DocId::TaskSpec(id)))
        }
        FilenameFamily::Design => {
            if parts.len() >= 5
                && parts[0] == "design"
                && is_fixed_digits(parts[1], 3)
                && parts[2] == "patch"
                && is_fixed_digits(parts[3], 2)
                && valid_slug(&parts[4..])
            {
                let id = parts[1].parse::<u32>()?;
                let patch = parts[3].parse::<u8>()?;
                Ok((DocType::DesignPatch, DocId::DesignPatch(id, patch)))
            } else if parts.len() >= 3
                && parts[0] == "design"
                && is_fixed_digits(parts[1], 3)
                && valid_slug(&parts[2..])
            {
                let id = parts[1].parse::<u32>()?;
                Ok((DocType::DesignDoc, DocId::DesignDoc(id)))
            } else {
                Err(DocumentModelError::InvalidFilename {
                    path: relative.to_path_buf(),
                    doc_type: "DesignDoc/DesignPatch",
                }
                .into())
            }
        }
    }
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
            if found != canonical_id.as_string() {
                return Err(DocumentModelError::IdMismatch {
                    path: relative.to_path_buf(),
                    expected: canonical_id.as_string(),
                    found: found.to_string(),
                }
                .into());
            }
        }
        _ => {
            let found = require_field(relative, "id", frontmatter.id.as_deref())?;
            if found != canonical_id.as_string() {
                return Err(DocumentModelError::IdMismatch {
                    path: relative.to_path_buf(),
                    expected: canonical_id.as_string(),
                    found: found.to_string(),
                }
                .into());
            }
            require_non_empty(relative, "title", frontmatter.title.as_deref())?;
        }
    }

    match doc_type {
        DocType::DesignPatch => {
            require_field(relative, "parent", frontmatter.parent.as_deref())?;
        }
        DocType::TaskSpec if frontmatter.status.as_deref() == Some("active") => {
            let boundaries = frontmatter.boundaries.as_ref().ok_or_else(|| {
                DocumentModelError::MissingField {
                    path: relative.to_path_buf(),
                    field: "boundaries",
                }
            })?;
            if boundaries.allowed.is_empty() {
                return Err(DocumentModelError::InvalidField {
                    path: relative.to_path_buf(),
                    field: "boundaries.allowed",
                    message: "must contain at least one pattern".to_string(),
                }
                .into());
            }
            if !boundaries
                .forbidden_patterns
                .iter()
                .any(|pattern| pattern == "specs/**")
            {
                return Err(DocumentModelError::InvalidField {
                    path: relative.to_path_buf(),
                    field: "boundaries.forbidden_patterns",
                    message: "must include specs/**".to_string(),
                }
                .into());
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
        _ => {}
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
        (DocType::ExecPlan, "active") => Ok(Status::Active),
        (DocType::ExecPlan, "completed") => Ok(Status::Completed),
        (DocType::ExecPlan, "abandoned") => Ok(Status::Abandoned),
        (DocType::TaskSpec, "draft") => Ok(Status::Draft),
        (DocType::TaskSpec, "active") => Ok(Status::Active),
        (DocType::TaskSpec, "completed") => Ok(Status::Completed),
        (DocType::TaskSpec, "cancelled") => Ok(Status::Cancelled),
        (DocType::ProjectSpec | DocType::OrgSpec, "active") => Ok(Status::Active),
        _ => Err(DocumentModelError::InvalidStatus {
            path: relative.to_path_buf(),
            doc_type: doc_type.as_str(),
            status: raw.to_string(),
        }
        .into()),
    }
}

fn validate_directory(relative: &Path, doc_type: DocType, status: Status) -> Result<()> {
    let parent = relative.parent().unwrap_or_else(|| Path::new(""));
    let expected =
        expected_directory(doc_type, status).ok_or_else(|| DocumentModelError::InvalidField {
            path: relative.to_path_buf(),
            field: "status",
            message: format!("no directory mapping for {} {}", doc_type, status),
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
    Ok(())
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

fn valid_slug(parts: &[&str]) -> bool {
    if parts.is_empty() || parts.len() > 5 {
        return false;
    }
    parts.iter().all(|part| {
        let mut chars = part.chars();
        matches!(chars.next(), Some(first) if first.is_ascii_lowercase())
            && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
    })
}

fn is_fixed_digits(value: &str, digits: usize) -> bool {
    value.len() == digits && value.chars().all(|ch| ch.is_ascii_digit())
}

fn is_completion_criterion_id(value: &str) -> bool {
    let parts: Vec<&str> = value.trim().split('-').collect();
    matches!(parts.as_slice(), ["cc", digits] if is_fixed_digits(digits, 3))
}

fn parse_reference_id(raw: &str) -> Option<DocId> {
    let parts: Vec<&str> = raw.split('-').collect();
    match parts.as_slice() {
        ["prd", id] if is_fixed_digits(id, 3) => id.parse().ok().map(DocId::Prd),
        ["design", id] if is_fixed_digits(id, 3) => id.parse().ok().map(DocId::DesignDoc),
        ["design", id, "patch", patch] if is_fixed_digits(id, 3) && is_fixed_digits(patch, 2) => {
            Some(DocId::DesignPatch(id.parse().ok()?, patch.parse().ok()?))
        }
        ["exec", id] if is_fixed_digits(id, 3) => id.parse().ok().map(DocId::ExecPlan),
        ["task", id] if is_fixed_digits(id, 4) => id.parse().ok().map(DocId::TaskSpec),
        ["project"] => Some(DocId::ProjectSpec),
        ["org"] => Some(DocId::OrgSpec),
        _ => None,
    }
}

fn path_to_unix(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

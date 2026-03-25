use crate::doc::{build_compliant_index, DocId, DocType};
use crate::error::DocumentModelError;
use anyhow::Result;
use std::path::Path;

/// Ensures a proposed slug is not already occupied for the given document type.
pub fn ensure_unique_slug(repo_root: &Path, doc_type: DocType, slug: &str) -> Result<()> {
    let index = build_compliant_index(repo_root)?;
    let occupied = index.documents.keys().any(|id| match (doc_type, id) {
        (DocType::Prd, DocId::Prd(existing)) => existing == slug,
        (DocType::DesignDoc, DocId::DesignDoc(existing)) => existing == slug,
        (DocType::ExecPlan, DocId::ExecPlan(existing)) => existing == slug,
        _ => false,
    });

    if occupied {
        return Err(DocumentModelError::InvalidField {
            path: index.repo_root,
            field: "id",
            message: format!("slug `{slug}` is already in use for {doc_type}"),
        }
        .into());
    }

    Ok(())
}

/// Allocates the next patch sequence number for a parent design slug.
pub fn next_patch_number(repo_root: &Path, parent_slug: &str) -> Result<u32> {
    let index = build_compliant_index(repo_root)?;
    let max_patch = index
        .documents
        .keys()
        .filter_map(|id| match id {
            DocId::DesignPatch {
                parent_slug: existing,
                sequence,
                ..
            } if existing == parent_slug => Some(*sequence),
            _ => None,
        })
        .max()
        .unwrap_or(0);

    Ok(max_patch + 1)
}

/// Allocates the next task sequence number for an exec plan slug.
pub fn next_task_sequence(repo_root: &Path, exec_slug: &str) -> Result<u32> {
    let index = build_compliant_index(repo_root)?;
    let max_task = index
        .documents
        .keys()
        .filter_map(|id| match id {
            DocId::TaskSpec {
                exec_slug: existing,
                sequence,
            } if existing == exec_slug => Some(*sequence),
            _ => None,
        })
        .max()
        .unwrap_or(0);

    Ok(max_task + 1)
}

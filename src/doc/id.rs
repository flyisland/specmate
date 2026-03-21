use crate::doc::{build_compliant_index, DocId, DocType};
use crate::error::DocumentModelError;
use anyhow::Result;
use std::path::Path;

/// Allocates the next numeric ID for a document type.
pub fn next_id(repo_root: &Path, doc_type: DocType) -> Result<u32> {
    match doc_type {
        DocType::Prd | DocType::DesignDoc | DocType::ExecPlan | DocType::TaskSpec => {
            let index = build_compliant_index(repo_root)?;
            let max_id = index
                .documents
                .keys()
                .filter_map(|id| match (doc_type, id) {
                    (DocType::Prd, DocId::Prd(value)) => Some(*value),
                    (DocType::DesignDoc, DocId::DesignDoc(value)) => Some(*value),
                    (DocType::ExecPlan, DocId::ExecPlan(value)) => Some(*value),
                    (DocType::TaskSpec, DocId::TaskSpec(value)) => Some(*value),
                    _ => None,
                })
                .max();
            Ok(max_id.unwrap_or(0) + 1)
        }
        _ => Err(DocumentModelError::UnsupportedIdAllocation {
            doc_type: doc_type.as_str(),
        }
        .into()),
    }
}

/// Allocates the next patch sequence number for a parent design document.
pub fn next_patch_number(repo_root: &Path, parent_id: u32) -> Result<u8> {
    let index = build_compliant_index(repo_root)?;
    let max_patch = index
        .documents
        .keys()
        .filter_map(|id| match id {
            DocId::DesignPatch(parent, patch) if *parent == parent_id => Some(*patch),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    max_patch
        .checked_add(1)
        .ok_or_else(|| DocumentModelError::InvalidField {
            path: index.repo_root,
            field: "patch",
            message: format!("patch sequence overflow for design-{parent_id:03}"),
        })
        .map_err(Into::into)
}

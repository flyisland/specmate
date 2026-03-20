use crate::doc::DocType;
use std::fmt;

/// The identifier for a document, derived from its filename.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DocId {
    Prd(u32),
    DesignDoc(u32),
    DesignPatch { parent: u32, sequence: u8 },
    ExecPlan(u32),
    TaskSpec(u32),
    ProjectSpec,
    OrgSpec,
    Guideline(String),
}

impl fmt::Display for DocId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocId::Prd(n) => write!(f, "prd-{:03}", n),
            DocId::DesignDoc(n) => write!(f, "design-{:03}", n),
            DocId::DesignPatch { parent, sequence } => {
                write!(f, "design-{:03}-patch-{:02}", parent, sequence)
            }
            DocId::ExecPlan(n) => write!(f, "exec-{:03}", n),
            DocId::TaskSpec(n) => write!(f, "task-{:04}", n),
            DocId::ProjectSpec => write!(f, "project"),
            DocId::OrgSpec => write!(f, "org"),
            DocId::Guideline(slug) => write!(f, "{}", slug),
        }
    }
}

/// Allocate the next available ID for a given DocType by scanning existing files.
pub fn next_id(doc_type: &DocType, repo_root: &std::path::Path) -> u32 {
    let dirs = doc_dirs(doc_type, repo_root);
    let mut max = 0u32;

    for dir in dirs {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if let Some(n) = extract_id(doc_type, &name) {
                    if n > max {
                        max = n;
                    }
                }
            }
        }
    }

    max + 1
}

/// Allocate the next patch sequence number for a given parent design doc ID.
pub fn next_patch_number(parent_id: u32, repo_root: &std::path::Path) -> u8 {
    let dirs = doc_dirs(&DocType::DesignPatch, repo_root);
    let prefix = format!("design-{:03}-patch-", parent_id);
    let mut max = 0u8;

    for dir in dirs {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with(&prefix) {
                    // design-001-patch-01-slug.md
                    let rest = &name[prefix.len()..];
                    if let Some(seq_str) = rest.split('-').next() {
                        if let Ok(seq) = seq_str.parse::<u8>() {
                            if seq > max {
                                max = seq;
                            }
                        }
                    }
                }
            }
        }
    }

    max + 1
}

fn doc_dirs(doc_type: &DocType, repo_root: &std::path::Path) -> Vec<std::path::PathBuf> {
    match doc_type {
        DocType::Prd => vec![
            repo_root.join("docs/prd/draft"),
            repo_root.join("docs/prd/approved"),
            repo_root.join("docs/prd/obsolete"),
        ],
        DocType::DesignDoc | DocType::DesignPatch => vec![
            repo_root.join("docs/design-docs/draft"),
            repo_root.join("docs/design-docs/candidate"),
            repo_root.join("docs/design-docs/implemented"),
            repo_root.join("docs/design-docs/obsolete"),
        ],
        DocType::ExecPlan => vec![
            repo_root.join("docs/exec-plans/draft"),
            repo_root.join("docs/exec-plans/active"),
            repo_root.join("docs/exec-plans/archived"),
        ],
        DocType::TaskSpec => vec![
            repo_root.join("specs/active"),
            repo_root.join("specs/archived"),
        ],
        _ => vec![],
    }
}

fn extract_id(doc_type: &DocType, filename: &str) -> Option<u32> {
    let prefix = match doc_type {
        DocType::Prd => "prd-",
        DocType::DesignDoc => "design-",
        DocType::ExecPlan => "exec-",
        DocType::TaskSpec => "task-",
        _ => return None,
    };
    if !filename.starts_with(prefix) {
        return None;
    }
    let rest = &filename[prefix.len()..];
    rest.split('-').next()?.parse().ok()
}

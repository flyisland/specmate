use std::path::PathBuf;
use thiserror::Error;

/// Domain errors raised by the document model.
#[derive(Debug, Error)]
pub enum DocumentModelError {
    /// The path is inside the managed document space but does not map to a valid document shape.
    #[error("invalid managed document path: {path}")]
    InvalidManagedPath { path: PathBuf },

    /// The filename does not match the required naming convention for the inferred document type.
    #[error("invalid filename for {doc_type} at {path}")]
    InvalidFilename {
        path: PathBuf,
        doc_type: &'static str,
    },

    /// The file is missing the leading YAML frontmatter block.
    #[error("missing frontmatter in {path}")]
    MissingFrontmatter { path: PathBuf },

    /// The frontmatter block could not be parsed as YAML.
    #[error("invalid frontmatter in {path}: {message}")]
    InvalidFrontmatter { path: PathBuf, message: String },

    /// A required frontmatter field is absent.
    #[error("missing field `{field}` in {path}")]
    MissingField { path: PathBuf, field: &'static str },

    /// A frontmatter field is present but invalid for the current document type.
    #[error("invalid field `{field}` in {path}: {message}")]
    InvalidField {
        path: PathBuf,
        field: &'static str,
        message: String,
    },

    /// The frontmatter `id` does not match the canonical ID derived from the path.
    #[error("id mismatch in {path}: expected `{expected}`, found `{found}`")]
    IdMismatch {
        path: PathBuf,
        expected: String,
        found: String,
    },

    /// The status value is not valid for the current document type.
    #[error("invalid status `{status}` for {doc_type} in {path}")]
    InvalidStatus {
        path: PathBuf,
        doc_type: &'static str,
        status: String,
    },

    /// The requested status transition is not allowed.
    #[error("illegal transition for {doc_type}: {from} -> {to}")]
    IllegalTransition {
        doc_type: &'static str,
        from: String,
        to: String,
    },

    /// The caller requested ID allocation for a document type that does not support numeric allocation.
    #[error("unsupported id allocation for {doc_type}")]
    UnsupportedIdAllocation { doc_type: &'static str },
}

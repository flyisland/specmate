use thiserror::Error;

/// Top-level specmate error types.
///
/// Each variant corresponds to a distinct failure mode. Use `anyhow::Error`
/// for propagation; use these types when you need to match on a specific error.
#[derive(Error, Debug)]
pub enum SpecmateError {
    /// The repo already has a specmate structure and --merge was not passed.
    #[error("specmate structure already exists. Use --merge to update it.")]
    AlreadyInitialised,

    /// A file that should exist was not found.
    #[error("file not found: {0}")]
    FileNotFound(String),

    /// A document's frontmatter is missing or malformed.
    #[error("invalid frontmatter in {path}: {reason}")]
    InvalidFrontmatter { path: String, reason: String },

    /// A document's status does not match its directory location.
    #[error("status mismatch in {path}: status is '{status}' but file is in '{directory}'")]
    StatusDirectoryMismatch {
        path: String,
        status: String,
        directory: String,
    },

    /// A document references a non-existent or obsolete document.
    #[error("stale reference in {source}: {field} points to '{target}' which {reason}")]
    StaleReference {
        source: String,
        field: String,
        target: String,
        reason: String,
    },

    /// A Task Spec boundary was violated.
    #[error("boundary violation: {path} is not in boundaries.allowed for {spec}")]
    BoundaryViolation { path: String, spec: String },

    /// Two active Task Specs have overlapping boundaries.
    #[error("boundary conflict between {spec_a} and {spec_b}: '{pattern}' overlaps")]
    BoundaryConflict {
        spec_a: String,
        spec_b: String,
        pattern: String,
    },

    /// An invalid status transition was attempted.
    #[error("invalid transition for {doc_type}: {from} -> {to}")]
    InvalidTransition {
        doc_type: String,
        from: String,
        to: String,
    },

    /// A required frontmatter field for a status transition is missing.
    #[error("missing required field '{field}' for status '{status}' in {path}")]
    MissingRequiredField {
        path: String,
        field: String,
        status: String,
    },
}

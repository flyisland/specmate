use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

/// Managed document types recognised by specmate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DocType {
    /// Product requirements document.
    Prd,
    /// Design document for a module or subsystem.
    DesignDoc,
    /// Patch against an existing design document.
    DesignPatch,
    /// Execution plan document.
    ExecPlan,
    /// Task specification document.
    TaskSpec,
    /// Fixed-path `specs/project.md`.
    ProjectSpec,
    /// Fixed-path `specs/org.md`.
    OrgSpec,
    /// Guideline under `docs/guidelines/`.
    Guideline,
}

impl DocType {
    /// Returns the stable display name for the document type.
    pub fn as_str(self) -> &'static str {
        match self {
            DocType::Prd => "PRD",
            DocType::DesignDoc => "DesignDoc",
            DocType::DesignPatch => "DesignPatch",
            DocType::ExecPlan => "ExecPlan",
            DocType::TaskSpec => "TaskSpec",
            DocType::ProjectSpec => "ProjectSpec",
            DocType::OrgSpec => "OrgSpec",
            DocType::Guideline => "Guideline",
        }
    }
}

impl fmt::Display for DocType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical lifecycle states used by managed documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Status {
    /// Draft state.
    Draft,
    /// Approved state.
    Approved,
    /// Candidate state.
    Candidate,
    /// Implemented state.
    Implemented,
    /// Obsolete state.
    Obsolete,
    /// Obsolete merged state used by design patches.
    ObsoleteMerged,
    /// Active state.
    Active,
    /// Completed state.
    Completed,
    /// Abandoned state.
    Abandoned,
    /// Cancelled state.
    Cancelled,
}

impl Status {
    /// Returns the canonical lowercase string form used in frontmatter.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Draft => "draft",
            Status::Approved => "approved",
            Status::Candidate => "candidate",
            Status::Implemented => "implemented",
            Status::Obsolete => "obsolete",
            Status::ObsoleteMerged => "obsolete:merged",
            Status::Active => "active",
            Status::Completed => "completed",
            Status::Abandoned => "abandoned",
            Status::Cancelled => "cancelled",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical ID for a managed document.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DocId {
    /// `prd-001`
    Prd(u32),
    /// `design-001`
    DesignDoc(u32),
    /// `design-001-patch-01`
    DesignPatch(u32, u8),
    /// `exec-001`
    ExecPlan(u32),
    /// `task-0001`
    TaskSpec(u32),
    /// `project`
    ProjectSpec,
    /// `org`
    OrgSpec,
    /// Guideline slug from `docs/guidelines/<slug>.md`
    Guideline(String),
}

impl DocId {
    /// Returns the canonical serialized identifier used by frontmatter references.
    pub fn as_string(&self) -> String {
        match self {
            DocId::Prd(id) => format!("prd-{id:03}"),
            DocId::DesignDoc(id) => format!("design-{id:03}"),
            DocId::DesignPatch(id, patch) => format!("design-{id:03}-patch-{patch:02}"),
            DocId::ExecPlan(id) => format!("exec-{id:03}"),
            DocId::TaskSpec(id) => format!("task-{id:04}"),
            DocId::ProjectSpec => "project".to_string(),
            DocId::OrgSpec => "org".to_string(),
            DocId::Guideline(slug) => slug.clone(),
        }
    }

    /// Returns the top-level document type for this identifier.
    pub fn doc_type(&self) -> DocType {
        match self {
            DocId::Prd(_) => DocType::Prd,
            DocId::DesignDoc(_) => DocType::DesignDoc,
            DocId::DesignPatch(_, _) => DocType::DesignPatch,
            DocId::ExecPlan(_) => DocType::ExecPlan,
            DocId::TaskSpec(_) => DocType::TaskSpec,
            DocId::ProjectSpec => DocType::ProjectSpec,
            DocId::OrgSpec => DocType::OrgSpec,
            DocId::Guideline(_) => DocType::Guideline,
        }
    }
}

impl fmt::Display for DocId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_string())
    }
}

/// Task-spec boundary rules.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Boundaries {
    /// Paths the agent may modify.
    pub allowed: Vec<String>,
    /// Paths the agent must never modify.
    pub forbidden_patterns: Vec<String>,
}

/// One completion criterion in a Task Spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionCriterion {
    /// Stable criterion ID such as `cc-001`.
    pub id: String,
    /// Human-readable scenario.
    pub scenario: String,
    /// Exact test name.
    pub test: String,
}

/// Typed frontmatter fields shared across document types.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Frontmatter {
    /// Optional frontmatter `id`.
    pub id: Option<String>,
    /// Optional frontmatter `title`.
    pub title: Option<String>,
    /// Optional raw status string.
    pub status: Option<String>,
    /// Optional module name.
    pub module: Option<String>,
    /// Optional PRD reference.
    pub prd: Option<String>,
    /// Optional patch parent reference.
    pub parent: Option<String>,
    /// Optional merged-into reference.
    pub merged_into: Option<String>,
    /// Optional superseded-by reference.
    pub superseded_by: Option<String>,
    /// Optional design-doc reference.
    pub design_doc: Option<String>,
    /// Optional exec-plan reference.
    pub exec_plan: Option<String>,
    /// Optional guideline paths.
    pub guidelines: Vec<String>,
    /// Optional boundaries section.
    pub boundaries: Option<Boundaries>,
    /// Optional completion criteria.
    pub completion_criteria: Vec<CompletionCriterion>,
}

/// Parsed, validated document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// Canonical ID.
    pub id: DocId,
    /// Document type.
    pub doc_type: DocType,
    /// Effective status. Guideline status is implicit `Active`.
    pub status: Status,
    /// Optional title.
    pub title: Option<String>,
    /// Absolute path.
    pub path: PathBuf,
    /// Parsed frontmatter.
    pub frontmatter: Frontmatter,
    /// Original file contents.
    pub raw: String,
}

/// Invalid markdown discovered in the managed document space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidManagedEntry {
    /// Absolute path to the invalid file.
    pub path: PathBuf,
    /// Human-readable reason.
    pub reason: String,
}

/// Repository-wide index of managed documents.
#[derive(Debug, Clone, Default)]
pub struct DocumentIndex {
    /// Repository root used to build the index.
    pub repo_root: PathBuf,
    /// Valid managed documents indexed by canonical ID.
    pub documents: BTreeMap<DocId, Document>,
    /// Invalid markdown found in managed locations.
    pub invalid_entries: Vec<InvalidManagedEntry>,
    /// Markdown ignored because it lives outside the managed document system or is support material.
    pub ignored_paths: Vec<PathBuf>,
}

/// Validation issue discovered after loading documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationViolation {
    /// Path associated with the violation.
    pub path: PathBuf,
    /// Human-readable explanation.
    pub message: String,
}

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

/// Managed document types recognised by specmate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DocType {
    /// Product requirements document.
    Prd,
    /// Design document for a module, subsystem, or cross-cutting principle.
    DesignDoc,
    /// Patch against an existing design document.
    DesignPatch,
    /// Execution plan document.
    ExecPlan,
    /// Task specification document.
    TaskSpec,
    /// Fixed-path `docs/specs/project.md`.
    ProjectSpec,
    /// Fixed-path `docs/specs/org.md`.
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
    /// Closed historical state.
    Closed,
    /// Obsolete state.
    Obsolete,
    /// Obsolete merged state used by design patches.
    ObsoleteMerged,
    /// Fixed-path always-active state for non-lifecycle docs.
    Active,
}

impl Status {
    /// Returns the canonical lowercase string form used in frontmatter.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Draft => "draft",
            Status::Approved => "approved",
            Status::Candidate => "candidate",
            Status::Implemented => "implemented",
            Status::Closed => "closed",
            Status::Obsolete => "obsolete",
            Status::ObsoleteMerged => "obsolete:merged",
            Status::Active => "active",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

fn render_local_sequence(value: u32) -> String {
    format!("{value:02}")
}

/// Canonical ID for a managed document.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DocId {
    /// `prd-<slug>`
    Prd(String),
    /// `design-<slug>`
    DesignDoc(String),
    /// `design-<parent-slug>-patch-<nn>-<patch-slug>`
    DesignPatch {
        parent_slug: String,
        sequence: u32,
        patch_slug: String,
    },
    /// `exec-<slug>`
    ExecPlan(String),
    /// `<exec-id>/task-<nn>`
    TaskSpec { exec_slug: String, sequence: u32 },
    /// `project`
    ProjectSpec,
    /// `org`
    OrgSpec,
    /// Relative guideline identifier such as `error-handling` or `obsolete/error-handling`.
    Guideline(String),
}

impl DocId {
    /// Returns the canonical serialized identifier used by references and CLI lookup.
    pub fn as_string(&self) -> String {
        match self {
            DocId::Prd(slug) => format!("prd-{slug}"),
            DocId::DesignDoc(slug) => format!("design-{slug}"),
            DocId::DesignPatch {
                parent_slug,
                sequence,
                patch_slug,
            } => format!(
                "design-{parent_slug}-patch-{}-{patch_slug}",
                render_local_sequence(*sequence)
            ),
            DocId::ExecPlan(slug) => format!("exec-{slug}"),
            DocId::TaskSpec {
                exec_slug,
                sequence,
            } => format!("exec-{exec_slug}/task-{}", render_local_sequence(*sequence)),
            DocId::ProjectSpec => "project".to_string(),
            DocId::OrgSpec => "org".to_string(),
            DocId::Guideline(relative) => relative.clone(),
        }
    }

    /// Returns the top-level document type for this identifier.
    pub fn doc_type(&self) -> DocType {
        match self {
            DocId::Prd(_) => DocType::Prd,
            DocId::DesignDoc(_) => DocType::DesignDoc,
            DocId::DesignPatch { .. } => DocType::DesignPatch,
            DocId::ExecPlan(_) => DocType::ExecPlan,
            DocId::TaskSpec { .. } => DocType::TaskSpec,
            DocId::ProjectSpec => DocType::ProjectSpec,
            DocId::OrgSpec => DocType::OrgSpec,
            DocId::Guideline(_) => DocType::Guideline,
        }
    }

    /// Returns the frontmatter `id` value for this document.
    pub fn frontmatter_id(&self) -> String {
        match self {
            DocId::TaskSpec { sequence, .. } => {
                format!("task-{}", render_local_sequence(*sequence))
            }
            _ => self.as_string(),
        }
    }

    /// Returns the escaped single-token task rendering when relevant.
    pub fn escaped_string(&self) -> String {
        match self {
            DocId::TaskSpec {
                exec_slug,
                sequence,
            } => format!(
                "exec-{exec_slug}--task-{}",
                render_local_sequence(*sequence)
            ),
            _ => self.as_string(),
        }
    }

    /// Returns the containing exec slug for Task Specs.
    pub fn exec_slug(&self) -> Option<&str> {
        match self {
            DocId::TaskSpec { exec_slug, .. } => Some(exec_slug.as_str()),
            DocId::ExecPlan(slug) => Some(slug.as_str()),
            _ => None,
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
    /// Optional lifecycle created date.
    pub created: Option<String>,
    /// Optional lifecycle closed date.
    pub closed: Option<String>,
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
    /// Optional legacy singular design-doc reference.
    pub design_doc: Option<String>,
    /// Optional plural design-docs reference.
    pub design_docs: Vec<String>,
    /// Optional exec-plan reference.
    pub exec_plan: Option<String>,
    /// Optional guideline paths retained for backward compatibility during migration.
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

/// Stable kinds of direct-association summaries exposed by the document model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssociationKind {
    /// Design Docs associated with a PRD.
    PrdDesignDocs,
    /// Design Patches associated with a parent Design Doc.
    DesignDocPatches,
    /// Exec Plans associated with a Design Doc or Design Patch.
    DesignDocExecPlans,
    /// Direct Task Specs associated with a Design Doc.
    DesignDocTasks,
    /// Task Specs associated with an Exec Plan.
    ExecPlanTasks,
}

/// Minimal associated-document facts for status views and transition gates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssociatedDocument {
    /// Canonical ID of the associated document.
    pub id: DocId,
    /// Document type.
    pub doc_type: DocType,
    /// Current lifecycle status.
    pub status: Status,
}

/// Aggregated facts about one direct association set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssociationSummary {
    /// Association set kind.
    pub kind: AssociationKind,
    /// Canonical owner document.
    pub owner: DocId,
    /// Related documents in deterministic canonical-id order.
    pub related: Vec<AssociatedDocument>,
}

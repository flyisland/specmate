use std::fmt;

/// All document types recognised by specmate.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DocType {
    Prd,
    DesignDoc,
    DesignPatch,
    ExecPlan,
    TaskSpec,
    ProjectSpec,
    OrgSpec,
    Guideline,
}

impl fmt::Display for DocType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocType::Prd => write!(f, "PRD"),
            DocType::DesignDoc => write!(f, "Design Doc"),
            DocType::DesignPatch => write!(f, "Design Patch"),
            DocType::ExecPlan => write!(f, "Exec Plan"),
            DocType::TaskSpec => write!(f, "Task Spec"),
            DocType::ProjectSpec => write!(f, "project.spec"),
            DocType::OrgSpec => write!(f, "org.spec"),
            DocType::Guideline => write!(f, "Guideline"),
        }
    }
}

/// Document status. Valid values depend on the DocType.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Status {
    // Shared
    Draft,
    Active,
    // PRD
    Approved,
    // Design Doc / Patch
    Candidate,
    Implemented,
    // Terminal states
    Completed,
    Cancelled,
    Abandoned,
    Obsolete,
    ObsoleteMerged,
}

impl Status {
    /// Parse a status string from frontmatter.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(Status::Draft),
            "active" => Some(Status::Active),
            "approved" => Some(Status::Approved),
            "candidate" => Some(Status::Candidate),
            "implemented" => Some(Status::Implemented),
            "completed" => Some(Status::Completed),
            "cancelled" => Some(Status::Cancelled),
            "abandoned" => Some(Status::Abandoned),
            "obsolete" => Some(Status::Obsolete),
            "obsolete:merged" => Some(Status::ObsoleteMerged),
            _ => None,
        }
    }

    /// Return the canonical string representation for frontmatter.
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Draft => "draft",
            Status::Active => "active",
            Status::Approved => "approved",
            Status::Candidate => "candidate",
            Status::Implemented => "implemented",
            Status::Completed => "completed",
            Status::Cancelled => "cancelled",
            Status::Abandoned => "abandoned",
            Status::Obsolete => "obsolete",
            Status::ObsoleteMerged => "obsolete:merged",
        }
    }

    /// Return all valid statuses for a given DocType.
    pub fn valid_for(doc_type: &DocType) -> &'static [&'static str] {
        match doc_type {
            DocType::Prd => &["draft", "approved", "obsolete"],
            DocType::DesignDoc => &["draft", "candidate", "implemented", "obsolete"],
            DocType::DesignPatch => {
                &["draft", "candidate", "implemented", "obsolete:merged"]
            }
            DocType::ExecPlan => &["draft", "active", "completed", "abandoned"],
            DocType::TaskSpec => &["draft", "active", "completed", "cancelled"],
            DocType::ProjectSpec | DocType::OrgSpec | DocType::Guideline => &["active"],
        }
    }

    /// Return valid next statuses for a transition.
    pub fn valid_transitions(doc_type: &DocType, from: &Status) -> Vec<Status> {
        match (doc_type, from) {
            (DocType::Prd, Status::Draft) => vec![Status::Approved, Status::Obsolete],
            (DocType::Prd, Status::Approved) => vec![Status::Obsolete],
            (DocType::DesignDoc, Status::Draft) => vec![Status::Candidate],
            (DocType::DesignDoc, Status::Candidate) => vec![Status::Implemented],
            (DocType::DesignDoc, Status::Implemented) => vec![Status::Obsolete],
            (DocType::DesignPatch, Status::Draft) => vec![Status::Candidate],
            (DocType::DesignPatch, Status::Candidate) => vec![Status::Implemented],
            (DocType::DesignPatch, Status::Implemented) => vec![Status::ObsoleteMerged],
            (DocType::ExecPlan, Status::Draft) => vec![Status::Active],
            (DocType::ExecPlan, Status::Active) => vec![Status::Completed, Status::Abandoned],
            (DocType::TaskSpec, Status::Draft) => vec![Status::Active, Status::Cancelled],
            (DocType::TaskSpec, Status::Active) => vec![Status::Completed, Status::Cancelled],
            _ => vec![],
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

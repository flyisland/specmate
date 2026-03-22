use crate::doc::{
    association_summaries, build_index, expected_directory, is_live_status, is_terminal_status,
    validate_index, AssociatedDocument, AssociationKind, DocType, Document, DocumentIndex, Status,
    ValidationViolation,
};
use anyhow::{bail, Context, Result};
use clap::Args;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Arguments for `specmate status`.
#[derive(Args, Debug, Clone)]
#[command(
    after_help = "Examples:\n  specmate status\n  specmate status --all\n  specmate status design-008\n  specmate status task-0007"
)]
pub struct StatusArgs {
    /// Optional managed document id such as `task-0007` or `design-008`
    pub doc_id: Option<String>,
    /// Expand the dashboard to list all lifecycle-managed documents
    #[arg(long)]
    pub all: bool,
}

#[derive(Debug)]
struct StatusFailure {
    path: Option<PathBuf>,
    message: String,
    fix: String,
}

impl StatusFailure {
    fn new(path: Option<PathBuf>, message: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            path,
            message: message.into(),
            fix: fix.into(),
        }
    }
}

#[derive(Debug, Clone)]
struct IssuePreview {
    path: String,
    message: String,
}

#[derive(Debug, Clone)]
struct ReferenceRow {
    label: &'static str,
    raw_target: String,
    resolved: Option<AssociatedDocument>,
}

/// Run `specmate status`.
pub fn run(args: StatusArgs) -> Result<()> {
    let start_dir = std::env::current_dir().context("reading current working directory")?;
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    run_in_repo(&start_dir, args, &mut stdout, &mut stderr)
}

fn run_in_repo<W: Write, E: Write>(
    start_dir: &Path,
    args: StatusArgs,
    stdout: &mut W,
    stderr: &mut E,
) -> Result<()> {
    let repo_root = match find_repo_root(start_dir) {
        Ok(repo_root) => repo_root,
        Err(failure) => {
            render_failure(stderr, start_dir, &failure)?;
            bail!("specmate status failed");
        }
    };

    let index = match build_index(&repo_root) {
        Ok(index) => index,
        Err(error) => {
            let failure = StatusFailure::new(
                Some(repo_root.clone()),
                format!("failed to build repository status view: {error}"),
                "Repair the repository frontmatter or filesystem state and re-run specmate status.",
            );
            render_failure(stderr, &repo_root, &failure)?;
            bail!("specmate status failed");
        }
    };
    let violations = validate_index(&index);

    let output = match args.doc_id.as_deref() {
        Some(doc_id) => match render_detail(&index, &violations, doc_id) {
            Ok(output) => output,
            Err(failure) => {
                render_failure(stderr, &repo_root, &failure)?;
                bail!("specmate status failed");
            }
        },
        None => render_dashboard(&index, &violations, args.all),
    };

    write!(stdout, "{output}")?;
    Ok(())
}

fn render_dashboard(
    index: &DocumentIndex,
    violations: &[ValidationViolation],
    show_all: bool,
) -> String {
    let mut output = String::new();
    output.push_str("Repository Health\n");
    output.push_str(&format!(
        "  valid managed documents: {}\n",
        index.documents.len()
    ));
    output.push_str(&format!(
        "  invalid managed entries: {}\n",
        index.invalid_entries.len()
    ));
    output.push_str(&format!(
        "  repository validation violations: {}\n",
        violations.len()
    ));

    let previews = issue_previews(index, violations);
    if !previews.is_empty() {
        output.push_str("  issue preview\n");
        for preview in previews.iter().take(4) {
            output.push_str(&format!("    {}\n", preview.path));
            output.push_str(&format!("    {}\n", preview.message));
        }
    }

    output.push('\n');
    output.push_str("Design Overview\n");
    render_design_bucket(&mut output, index, Status::Draft, "draft");
    render_design_bucket(&mut output, index, Status::Implemented, "implemented");
    render_design_bucket(&mut output, index, Status::Candidate, "candidate");

    output.push('\n');
    output.push_str("Execution Overview\n");
    output.push_str("  active exec plans\n");
    let active_execs = documents_by_type_and_status(index, DocType::ExecPlan, Status::Active);
    if active_execs.is_empty() {
        output.push_str("    none\n");
    } else {
        for exec in active_execs {
            let counts = task_status_counts(index, &exec.id.as_string());
            output.push_str(&format!(
                "    {}  {}  design-doc: {}  tasks: draft={} active={} completed={} cancelled={}\n",
                exec.id,
                title_for(exec),
                optional_field(exec.frontmatter.design_doc.as_deref()),
                counts.get(&Status::Draft).copied().unwrap_or(0),
                counts.get(&Status::Active).copied().unwrap_or(0),
                counts.get(&Status::Completed).copied().unwrap_or(0),
                counts.get(&Status::Cancelled).copied().unwrap_or(0),
            ));
        }
    }

    output.push_str("  active task specs\n");
    let active_tasks = documents_by_type_and_status(index, DocType::TaskSpec, Status::Active);
    if active_tasks.is_empty() {
        output.push_str("    none\n");
    } else {
        for task in active_tasks {
            output.push_str(&format!("    {}  {}\n", task.id, title_for(task)));
            output.push_str(&format!(
                "      exec-plan: {}  design-doc: {}\n",
                optional_field(task.frontmatter.exec_plan.as_deref()),
                optional_field(task_design_doc(index, task).as_deref()),
            ));
        }
    }

    let historical_execs = count_by_status(
        index,
        DocType::ExecPlan,
        &[Status::Completed, Status::Abandoned],
    );
    let historical_tasks = count_by_status(
        index,
        DocType::TaskSpec,
        &[Status::Completed, Status::Cancelled],
    );
    output.push_str("  historical totals\n");
    output.push_str(&format!(
        "    exec plans: completed={} abandoned={}\n",
        historical_execs
            .get(&Status::Completed)
            .copied()
            .unwrap_or(0),
        historical_execs
            .get(&Status::Abandoned)
            .copied()
            .unwrap_or(0),
    ));
    output.push_str(&format!(
        "    task specs: completed={} cancelled={}\n",
        historical_tasks
            .get(&Status::Completed)
            .copied()
            .unwrap_or(0),
        historical_tasks
            .get(&Status::Cancelled)
            .copied()
            .unwrap_or(0),
    ));

    output.push('\n');
    output.push_str("Status Totals\n");
    render_status_totals(
        &mut output,
        index,
        DocType::Prd,
        &[Status::Draft, Status::Approved, Status::Obsolete],
    );
    render_status_totals(
        &mut output,
        index,
        DocType::DesignDoc,
        &[
            Status::Draft,
            Status::Candidate,
            Status::Implemented,
            Status::Obsolete,
        ],
    );
    render_status_totals(
        &mut output,
        index,
        DocType::DesignPatch,
        &[
            Status::Draft,
            Status::Candidate,
            Status::Implemented,
            Status::Obsolete,
            Status::ObsoleteMerged,
        ],
    );
    render_status_totals(
        &mut output,
        index,
        DocType::ExecPlan,
        &[
            Status::Draft,
            Status::Active,
            Status::Completed,
            Status::Abandoned,
        ],
    );
    render_status_totals(
        &mut output,
        index,
        DocType::TaskSpec,
        &[
            Status::Draft,
            Status::Active,
            Status::Completed,
            Status::Cancelled,
        ],
    );

    if show_all {
        output.push('\n');
        output.push_str("All Documents\n");
        render_all_documents_bucket(&mut output, index, DocType::Prd);
        render_all_documents_bucket(&mut output, index, DocType::DesignDoc);
        render_all_documents_bucket(&mut output, index, DocType::DesignPatch);
        render_all_documents_bucket(&mut output, index, DocType::ExecPlan);
        render_all_documents_bucket(&mut output, index, DocType::TaskSpec);
    }

    output
}

fn render_detail(
    index: &DocumentIndex,
    violations: &[ValidationViolation],
    doc_id: &str,
) -> std::result::Result<String, StatusFailure> {
    let wanted = doc_id.trim();
    let document = match index
        .documents
        .values()
        .find(|document| document.id.as_string() == wanted)
    {
        Some(document) => document,
        None if looks_like_guideline_slug(wanted) => {
            return Err(StatusFailure::new(
                Some(index.repo_root.clone()),
                format!("guideline lookup target {wanted} is not supported"),
                "Choose a managed document id such as task-0007 or design-008.",
            ))
        }
        None => {
            return Err(StatusFailure::new(
                Some(index.repo_root.clone()),
                format!("managed document {wanted} does not exist"),
                "Use a canonical managed document id such as task-0007 or design-008.",
            ))
        }
    };

    if document.doc_type == DocType::Guideline {
        return Err(StatusFailure::new(
            Some(document.path.clone()),
            format!("guideline lookup target {wanted} is not supported"),
            "Choose a managed document id such as task-0007 or design-008.",
        ));
    }

    let mut output = String::new();
    output.push_str("Overview\n");
    output.push_str(&format!("  id: {}\n", document.id));
    output.push_str(&format!(
        "  title: {}\n",
        document.title.as_deref().unwrap_or("none")
    ));
    output.push_str(&format!("  type: {}\n", document.doc_type));
    output.push_str(&format!("  status: {}\n", document.status));
    output.push_str(&format!(
        "  path: {}\n",
        make_relative(index, &document.path)
    ));
    let expected = expected_directory(document.doc_type, document.status)
        .map(str::to_string)
        .or_else(|| expected_fixed_path(document));
    output.push_str(&format!(
        "  expected directory: {}\n",
        expected.as_deref().unwrap_or("none")
    ));
    output.push_str(&format!(
        "  lifecycle state: {}\n",
        if is_terminal_status(document.doc_type, document.status) {
            "terminal"
        } else if is_live_status(document.doc_type, document.status) {
            "live"
        } else {
            "none"
        }
    ));

    output.push('\n');
    output.push_str("Upstream References\n");
    let references = reference_rows(index, document);
    if references.is_empty() {
        output.push_str("  none\n");
    } else {
        for reference in references {
            match &reference.resolved {
                Some(target) => output.push_str(&format!(
                    "  {}: {} ({})\n",
                    reference.label, reference.raw_target, target.status
                )),
                None => output.push_str(&format!(
                    "  {}: {} (unresolved)\n",
                    reference.label, reference.raw_target
                )),
            }
        }
    }

    output.push('\n');
    output.push_str("Downstream Associations\n");
    let summaries = association_summaries(index, document);
    if summaries.is_empty() {
        output.push_str("  none\n");
    } else {
        for summary in summaries {
            output.push_str(&format!("  {}\n", association_label(summary.kind)));
            if summary.related.is_empty() {
                output.push_str("    none\n");
            } else {
                for related in sorted_related(&summary.related) {
                    output.push_str(&format!("    {} ({})\n", related.id, related.status));
                }
                output.push_str(&format!(
                    "    all terminal: {}\n",
                    if summary.all_terminal() { "yes" } else { "no" }
                ));
            }
        }
    }

    output.push('\n');
    output.push_str("Derived Chain Summary\n");
    render_derived_summary(&mut output, index, document);

    output.push('\n');
    output.push_str("Related Repository Warnings\n");
    let warnings = related_warnings(index, violations, document);
    if warnings.is_empty() {
        output.push_str("  No related warnings.\n");
    } else {
        for warning in warnings {
            output.push_str(&format!("  {}\n", warning.path));
            output.push_str(&format!("  {}\n", warning.message));
            output.push_str("  -> Repair the affected repository reference or managed document.\n");
        }
    }

    Ok(output)
}

fn render_derived_summary(output: &mut String, index: &DocumentIndex, document: &Document) {
    match document.doc_type {
        DocType::Prd => {
            let design_ids = direct_related_ids(index, document, AssociationKind::PrdDesignDocs);
            let exec_count = design_ids
                .iter()
                .map(|design_id| execs_for_design(index, design_id).len())
                .sum::<usize>();
            let task_count = design_ids
                .iter()
                .map(|design_id| tasks_for_design(index, design_id).len())
                .sum::<usize>();
            output.push_str(&format!("  design docs: {}\n", design_ids.len()));
            output.push_str(&format!("  exec plans: {}\n", exec_count));
            output.push_str(&format!("  task specs: {}\n", task_count));
        }
        DocType::DesignDoc => {
            let patches = direct_related_ids(index, document, AssociationKind::DesignDocPatches);
            let execs = direct_related_ids(index, document, AssociationKind::DesignDocExecPlans);
            let task_counts = task_status_counts_for_exec_ids(index, &execs);
            output.push_str(&format!("  patches: {}\n", patches.len()));
            output.push_str(&format!("  exec plans: {}\n", execs.len()));
            output.push_str(&format!(
                "  task specs: draft={} active={} completed={} cancelled={}\n",
                task_counts.get(&Status::Draft).copied().unwrap_or(0),
                task_counts.get(&Status::Active).copied().unwrap_or(0),
                task_counts.get(&Status::Completed).copied().unwrap_or(0),
                task_counts.get(&Status::Cancelled).copied().unwrap_or(0),
            ));
        }
        DocType::ExecPlan => {
            let counts = task_status_counts(index, &document.id.as_string());
            output.push_str(&format!(
                "  task specs: draft={} active={} completed={} cancelled={}\n",
                counts.get(&Status::Draft).copied().unwrap_or(0),
                counts.get(&Status::Active).copied().unwrap_or(0),
                counts.get(&Status::Completed).copied().unwrap_or(0),
                counts.get(&Status::Cancelled).copied().unwrap_or(0),
            ));
        }
        DocType::TaskSpec => {
            output.push_str(&format!(
                "  exec-plan lineage: {}\n",
                task_lineage(index, document)
            ));
            output.push_str(&format!(
                "  completion criteria: {}\n",
                document.frontmatter.completion_criteria.len()
            ));
            output.push_str(&format!(
                "  boundaries.allowed entries: {}\n",
                document
                    .frontmatter
                    .boundaries
                    .as_ref()
                    .map(|boundaries| boundaries.allowed.len())
                    .unwrap_or(0)
            ));
        }
        DocType::DesignPatch => {
            output.push_str(&format!(
                "  parent design: {}\n",
                optional_field(document.frontmatter.parent.as_deref())
            ));
            output.push_str(&format!(
                "  merged-into: {}\n",
                optional_field(document.frontmatter.merged_into.as_deref())
            ));
            output.push_str(&format!(
                "  superseded-by: {}\n",
                optional_field(document.frontmatter.superseded_by.as_deref())
            ));
        }
        DocType::ProjectSpec | DocType::OrgSpec | DocType::Guideline => {
            output.push_str("  none\n");
        }
    }
}

fn render_design_bucket(output: &mut String, index: &DocumentIndex, status: Status, label: &str) {
    output.push_str(&format!("  {label}\n"));
    let designs = documents_by_type_and_status(index, DocType::DesignDoc, status);
    if designs.is_empty() {
        output.push_str("    none\n");
        return;
    }

    for design in designs {
        let design_id = design.id.as_string();
        output.push_str(&format!(
            "    {}  {}  {}  {}\n",
            design.id,
            title_for(design),
            design.status,
            make_relative(index, &design.path)
        ));
        output.push_str(&format!(
            "      prd: {}  exec-plans: {}  task-specs: {}\n",
            optional_field(design.frontmatter.prd.as_deref()),
            execs_for_design(index, &design_id).len(),
            tasks_for_design(index, &design_id).len(),
        ));
    }
}

fn render_status_totals(
    output: &mut String,
    index: &DocumentIndex,
    doc_type: DocType,
    statuses: &[Status],
) {
    let counts = count_by_status(index, doc_type, statuses);
    let parts = statuses
        .iter()
        .map(|status| {
            format!(
                "{}={}",
                status.as_str(),
                counts.get(status).copied().unwrap_or(0)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    output.push_str(&format!("  {:<11} {}\n", doc_type.as_str(), parts));
}

fn render_all_documents_bucket(output: &mut String, index: &DocumentIndex, doc_type: DocType) {
    output.push_str(&format!("  {}\n", doc_type.as_str()));
    let documents = all_documents_by_type(index, doc_type);
    if documents.is_empty() {
        output.push_str("    none\n");
        return;
    }

    for document in documents {
        output.push_str(&format!(
            "    {}  {}  {}  {}\n",
            document.id,
            document.status,
            title_for(document),
            make_relative(index, &document.path)
        ));
    }
}

fn related_warnings(
    index: &DocumentIndex,
    violations: &[ValidationViolation],
    document: &Document,
) -> Vec<IssuePreview> {
    let mut related_ids = BTreeSet::new();
    related_ids.insert(document.id.as_string());
    for reference in reference_rows(index, document) {
        related_ids.insert(reference.raw_target);
    }
    for summary in association_summaries(index, document) {
        for related in summary.related {
            related_ids.insert(related.id.as_string());
        }
    }
    if let Some(exec_plan) = document.frontmatter.exec_plan.as_deref() {
        related_ids.insert(exec_plan.to_string());
    }
    if let Some(design_doc) = task_design_doc(index, document) {
        related_ids.insert(design_doc);
    }

    let mut warnings = Vec::new();
    for entry in &index.invalid_entries {
        let relative = make_relative(index, &entry.path);
        if relative == make_relative(index, &document.path)
            || related_ids
                .iter()
                .any(|id| file_name_matches_id(&relative, id))
        {
            warnings.push(IssuePreview {
                path: relative,
                message: entry.reason.clone(),
            });
        }
    }

    for violation in violations {
        let relative = make_relative(index, &violation.path);
        if relative == make_relative(index, &document.path)
            || related_ids.iter().any(|id| violation.message.contains(id))
            || related_ids
                .iter()
                .any(|id| file_name_matches_id(&relative, id))
        {
            warnings.push(IssuePreview {
                path: relative,
                message: violation.message.clone(),
            });
        }
    }

    warnings.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.message.cmp(&right.message))
    });
    warnings.dedup_by(|left, right| left.path == right.path && left.message == right.message);
    warnings
}

fn reference_rows(index: &DocumentIndex, document: &Document) -> Vec<ReferenceRow> {
    let specs = [
        ("prd", document.frontmatter.prd.as_deref()),
        ("parent", document.frontmatter.parent.as_deref()),
        ("merged-into", document.frontmatter.merged_into.as_deref()),
        (
            "superseded-by",
            document.frontmatter.superseded_by.as_deref(),
        ),
        ("design-doc", document.frontmatter.design_doc.as_deref()),
        ("exec-plan", document.frontmatter.exec_plan.as_deref()),
    ];

    specs
        .into_iter()
        .filter_map(|(label, value)| {
            value.map(|target| ReferenceRow {
                label,
                raw_target: target.to_string(),
                resolved: resolve_associated_document(index, target),
            })
        })
        .collect()
}

fn resolve_associated_document(
    index: &DocumentIndex,
    raw_target: &str,
) -> Option<AssociatedDocument> {
    index
        .documents
        .values()
        .find(|candidate| candidate.id.as_string() == raw_target)
        .map(|candidate| AssociatedDocument {
            id: candidate.id.clone(),
            doc_type: candidate.doc_type,
            status: candidate.status,
        })
}

fn sorted_related(related: &[AssociatedDocument]) -> Vec<AssociatedDocument> {
    let mut sorted = related.to_vec();
    sorted.sort_by(|left, right| left.id.cmp(&right.id));
    sorted
}

fn task_lineage(index: &DocumentIndex, task: &Document) -> String {
    let exec_plan = match task.frontmatter.exec_plan.as_deref() {
        Some(exec_plan) => exec_plan.to_string(),
        None => return "none".to_string(),
    };
    match task_design_doc(index, task) {
        Some(design_doc) => format!("{exec_plan} -> {design_doc}"),
        None => exec_plan,
    }
}

fn task_design_doc(index: &DocumentIndex, task: &Document) -> Option<String> {
    let exec_plan = task.frontmatter.exec_plan.as_deref()?;
    index
        .documents
        .values()
        .find(|candidate| candidate.id.as_string() == exec_plan)
        .and_then(|exec| exec.frontmatter.design_doc.clone())
}

fn execs_for_design<'a>(index: &'a DocumentIndex, design_id: &str) -> Vec<&'a Document> {
    let mut execs = index
        .documents
        .values()
        .filter(|document| document.doc_type == DocType::ExecPlan)
        .filter(|document| document.frontmatter.design_doc.as_deref() == Some(design_id))
        .collect::<Vec<_>>();
    execs.sort_by(|left, right| left.id.cmp(&right.id));
    execs
}

fn tasks_for_design<'a>(index: &'a DocumentIndex, design_id: &str) -> Vec<&'a Document> {
    let exec_ids = execs_for_design(index, design_id)
        .into_iter()
        .map(|exec| exec.id.as_string())
        .collect::<BTreeSet<_>>();
    let mut tasks = index
        .documents
        .values()
        .filter(|document| document.doc_type == DocType::TaskSpec)
        .filter(|document| {
            document
                .frontmatter
                .exec_plan
                .as_ref()
                .is_some_and(|exec_id| exec_ids.contains(exec_id))
        })
        .collect::<Vec<_>>();
    tasks.sort_by(|left, right| left.id.cmp(&right.id));
    tasks
}

fn task_status_counts(index: &DocumentIndex, exec_id: &str) -> BTreeMap<Status, usize> {
    let mut counts = BTreeMap::new();
    for task in index
        .documents
        .values()
        .filter(|document| document.doc_type == DocType::TaskSpec)
        .filter(|document| document.frontmatter.exec_plan.as_deref() == Some(exec_id))
    {
        *counts.entry(task.status).or_insert(0) += 1;
    }
    counts
}

fn task_status_counts_for_exec_ids(
    index: &DocumentIndex,
    exec_ids: &[String],
) -> BTreeMap<Status, usize> {
    let wanted = exec_ids.iter().cloned().collect::<BTreeSet<_>>();
    let mut counts = BTreeMap::new();
    for task in index
        .documents
        .values()
        .filter(|document| document.doc_type == DocType::TaskSpec)
        .filter(|document| {
            document
                .frontmatter
                .exec_plan
                .as_ref()
                .is_some_and(|exec_id| wanted.contains(exec_id))
        })
    {
        *counts.entry(task.status).or_insert(0) += 1;
    }
    counts
}

fn direct_related_ids(
    index: &DocumentIndex,
    document: &Document,
    kind: AssociationKind,
) -> Vec<String> {
    let mut ids = association_summaries(index, document)
        .into_iter()
        .find(|summary| summary.kind == kind)
        .map(|summary| {
            summary
                .related
                .into_iter()
                .map(|document| document.id.as_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    ids.sort();
    ids
}

fn documents_by_type_and_status(
    index: &DocumentIndex,
    doc_type: DocType,
    status: Status,
) -> Vec<&Document> {
    let mut documents = index
        .documents
        .values()
        .filter(|document| document.doc_type == doc_type && document.status == status)
        .collect::<Vec<_>>();
    documents.sort_by(|left, right| left.id.cmp(&right.id));
    documents
}

fn all_documents_by_type(index: &DocumentIndex, doc_type: DocType) -> Vec<&Document> {
    let mut documents = index
        .documents
        .values()
        .filter(|document| document.doc_type == doc_type)
        .collect::<Vec<_>>();
    documents.sort_by(|left, right| left.id.cmp(&right.id));
    documents
}

fn count_by_status(
    index: &DocumentIndex,
    doc_type: DocType,
    statuses: &[Status],
) -> BTreeMap<Status, usize> {
    let mut counts = statuses
        .iter()
        .copied()
        .map(|status| (status, 0usize))
        .collect::<BTreeMap<_, _>>();

    for document in index
        .documents
        .values()
        .filter(|document| document.doc_type == doc_type)
    {
        if let Some(count) = counts.get_mut(&document.status) {
            *count += 1;
        }
    }

    counts
}

fn issue_previews(index: &DocumentIndex, violations: &[ValidationViolation]) -> Vec<IssuePreview> {
    let mut previews = index
        .invalid_entries
        .iter()
        .map(|entry| IssuePreview {
            path: make_relative(index, &entry.path),
            message: entry.reason.clone(),
        })
        .collect::<Vec<_>>();

    previews.extend(violations.iter().map(|violation| IssuePreview {
        path: make_relative(index, &violation.path),
        message: violation.message.clone(),
    }));
    previews.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.message.cmp(&right.message))
    });
    previews
}

fn association_label(kind: AssociationKind) -> &'static str {
    match kind {
        AssociationKind::PrdDesignDocs => "design docs",
        AssociationKind::DesignDocPatches => "design patches",
        AssociationKind::DesignDocExecPlans => "exec plans",
        AssociationKind::ExecPlanTasks => "task specs",
    }
}

fn optional_field(value: Option<&str>) -> &str {
    value.unwrap_or("none")
}

fn title_for(document: &Document) -> &str {
    document.title.as_deref().unwrap_or("Untitled")
}

fn expected_fixed_path(document: &Document) -> Option<String> {
    match document.doc_type {
        DocType::ProjectSpec => Some("specs/project.md".to_string()),
        DocType::OrgSpec => Some("specs/org.md".to_string()),
        _ => None,
    }
}

fn looks_like_guideline_slug(raw: &str) -> bool {
    !raw.is_empty()
        && raw != "project"
        && raw != "org"
        && !raw.starts_with("prd-")
        && !raw.starts_with("design-")
        && !raw.starts_with("exec-")
        && !raw.starts_with("task-")
        && raw
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

fn file_name_matches_id(path: &str, id: &str) -> bool {
    path.rsplit('/')
        .next()
        .map(|file_name| {
            let stem = file_name.trim_end_matches(".md");
            stem == id || stem.starts_with(&format!("{id}-"))
        })
        .unwrap_or(false)
}

fn make_relative(index: &DocumentIndex, path: &Path) -> String {
    path.strip_prefix(&index.repo_root)
        .map(path_to_unix)
        .unwrap_or_else(|_| path_to_unix(path))
}

fn path_to_unix(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn render_failure<E: Write>(
    stderr: &mut E,
    repo_root: &Path,
    failure: &StatusFailure,
) -> Result<()> {
    writeln!(stderr, "[fail] status")?;
    if let Some(path) = &failure.path {
        let display = path
            .strip_prefix(repo_root)
            .map(path_to_unix)
            .unwrap_or_else(|_| path_to_unix(path));
        writeln!(stderr, "       {display}")?;
    }
    writeln!(stderr, "       {}", failure.message)?;
    writeln!(stderr, "       -> {}", failure.fix)?;
    Ok(())
}

fn find_repo_root(start_dir: &Path) -> std::result::Result<PathBuf, StatusFailure> {
    let mut current = fs::canonicalize(start_dir).map_err(|error| {
        StatusFailure::new(
            Some(start_dir.to_path_buf()),
            format!("failed to read {}: {error}", start_dir.display()),
            "Use a readable working directory inside a specmate repository.",
        )
    })?;

    loop {
        if current.join("specs/project.md").is_file() {
            return Ok(current);
        }
        if !current.pop() {
            return Err(StatusFailure::new(
                Some(start_dir.to_path_buf()),
                format!(
                    "could not locate a specmate repository root from {}",
                    start_dir.display()
                ),
                "Run specmate status from a specmate repository or one of its subdirectories.",
            ));
        }
    }
}

#[cfg(test)]
#[allow(dead_code)]
#[path = "../../tests/cmd/check_support.rs"]
mod check_support;

#[cfg(test)]
#[path = "../../tests/cmd/status_test.rs"]
mod status_test;

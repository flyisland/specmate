use crate::doc::{
    association_summaries, build_index, expected_directory, is_live_status, is_terminal_status,
    validate_index, AssociatedDocument, AssociationKind, DocType, Document, DocumentIndex, Status,
    ValidationViolation,
};
use anyhow::{bail, Context, Result};
use clap::{Args, ValueEnum};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{IsTerminal, Write};
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
    /// Control ANSI color output
    #[arg(long, value_enum, default_value_t = ColorWhen::Auto)]
    pub color: ColorWhen,
}

/// ANSI color policy for `specmate status`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum ColorWhen {
    /// Enable color only when writing to a TTY.
    Auto,
    /// Always emit ANSI color sequences.
    Always,
    /// Never emit ANSI color sequences.
    Never,
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

#[derive(Copy, Clone, Debug)]
struct Palette {
    enabled: bool,
}

const ID_COLOR: &str = "1";

/// Run `specmate status`.
pub fn run(args: StatusArgs) -> Result<()> {
    let start_dir = std::env::current_dir().context("reading current working directory")?;
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    let palette = Palette::new(args.color, stdout.is_terminal());
    run_in_repo(&start_dir, args, palette, &mut stdout, &mut stderr)
}

fn run_in_repo<W: Write, E: Write>(
    start_dir: &Path,
    args: StatusArgs,
    palette: Palette,
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
        Some(doc_id) => match render_detail(&index, &violations, doc_id, palette) {
            Ok(output) => output,
            Err(failure) => {
                render_failure(stderr, &repo_root, &failure)?;
                bail!("specmate status failed");
            }
        },
        None => render_dashboard(&index, &violations, args.all, palette),
    };

    write!(stdout, "{output}")?;
    Ok(())
}

fn render_dashboard(
    index: &DocumentIndex,
    violations: &[ValidationViolation],
    show_all: bool,
    palette: Palette,
) -> String {
    let mut output = String::new();
    output.push_str(&palette.header("Repository Health"));
    output.push('\n');
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
            output.push_str(&format!("    {}\n", palette.warning(&preview.message)));
        }
    }

    output.push('\n');
    output.push_str(&palette.header("Design Overview"));
    output.push('\n');
    render_design_bucket(&mut output, index, Status::Draft, "draft", palette);
    render_design_bucket(&mut output, index, Status::Candidate, "candidate", palette);
    render_design_bucket(
        &mut output,
        index,
        Status::Implemented,
        "implemented",
        palette,
    );

    output.push('\n');
    output.push_str(&palette.header("Execution Overview"));
    output.push('\n');
    output.push_str(&format!(
        "  {} exec plans\n",
        palette.status(Status::Active)
    ));
    let active_execs = documents_by_type_and_status(index, DocType::ExecPlan, Status::Active);
    if active_execs.is_empty() {
        output.push_str("    none\n");
    } else {
        for exec in active_execs {
            let counts = task_status_counts(index, &exec.id.as_string());
            output.push_str(&format!(
                "    {}  {}  design-doc: {}  tasks: {}\n",
                palette.doc_id(&exec.id.to_string()),
                title_for(exec),
                palette.optional_doc_id(exec.frontmatter.design_doc.as_deref()),
                render_status_counts(
                    palette,
                    &[
                        (
                            Status::Draft,
                            counts.get(&Status::Draft).copied().unwrap_or(0)
                        ),
                        (
                            Status::Active,
                            counts.get(&Status::Active).copied().unwrap_or(0)
                        ),
                        (
                            Status::Completed,
                            counts.get(&Status::Completed).copied().unwrap_or(0),
                        ),
                        (
                            Status::Cancelled,
                            counts.get(&Status::Cancelled).copied().unwrap_or(0),
                        ),
                    ]
                ),
            ));
        }
    }

    output.push_str(&format!(
        "  {} task specs\n",
        palette.status(Status::Active)
    ));
    let active_tasks = documents_by_type_and_status(index, DocType::TaskSpec, Status::Active);
    if active_tasks.is_empty() {
        output.push_str("    none\n");
    } else {
        for task in active_tasks {
            output.push_str(&format!(
                "    {}  {}\n",
                palette.doc_id(&task.id.to_string()),
                title_for(task)
            ));
            output.push_str(&format!(
                "      exec-plan: {}  design-doc: {}\n",
                palette.optional_doc_id(task.frontmatter.exec_plan.as_deref()),
                palette.optional_doc_id(task_upstream_design_doc(index, task).as_deref()),
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
        "    exec plans: {}\n",
        render_status_counts(
            palette,
            &[
                (
                    Status::Completed,
                    historical_execs
                        .get(&Status::Completed)
                        .copied()
                        .unwrap_or(0),
                ),
                (
                    Status::Abandoned,
                    historical_execs
                        .get(&Status::Abandoned)
                        .copied()
                        .unwrap_or(0),
                ),
            ]
        ),
    ));
    output.push_str(&format!(
        "    task specs: {}\n",
        render_status_counts(
            palette,
            &[
                (
                    Status::Completed,
                    historical_tasks
                        .get(&Status::Completed)
                        .copied()
                        .unwrap_or(0),
                ),
                (
                    Status::Cancelled,
                    historical_tasks
                        .get(&Status::Cancelled)
                        .copied()
                        .unwrap_or(0),
                ),
            ]
        ),
    ));

    output.push('\n');
    output.push_str(&palette.header("Status Totals"));
    output.push('\n');
    render_status_totals(
        &mut output,
        index,
        DocType::Prd,
        &[Status::Draft, Status::Approved, Status::Obsolete],
        palette,
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
        palette,
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
        palette,
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
        palette,
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
        palette,
    );

    if show_all {
        output.push('\n');
        output.push_str(&palette.header("All Documents"));
        output.push('\n');
        render_all_documents_bucket(&mut output, index, DocType::Prd, palette);
        render_all_documents_bucket(&mut output, index, DocType::DesignDoc, palette);
        render_all_documents_bucket(&mut output, index, DocType::DesignPatch, palette);
        render_all_documents_bucket(&mut output, index, DocType::ExecPlan, palette);
        render_all_documents_bucket(&mut output, index, DocType::TaskSpec, palette);
    }

    output
}

fn render_detail(
    index: &DocumentIndex,
    violations: &[ValidationViolation],
    doc_id: &str,
    palette: Palette,
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
    output.push_str(&palette.header("Overview"));
    output.push('\n');
    output.push_str(&format!(
        "  id: {}\n",
        palette.doc_id(&document.id.to_string())
    ));
    output.push_str(&format!(
        "  title: {}\n",
        document.title.as_deref().unwrap_or("none")
    ));
    output.push_str(&format!("  type: {}\n", document.doc_type));
    output.push_str(&format!("  status: {}\n", palette.status(document.status)));
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
    output.push_str(&palette.header("Upstream References"));
    output.push('\n');
    let references = reference_rows(index, document);
    if references.is_empty() {
        output.push_str("  none\n");
    } else {
        for reference in references {
            match &reference.resolved {
                Some(target) => output.push_str(&format!(
                    "  {}: {} ({})\n",
                    reference.label,
                    palette.doc_id(&reference.raw_target),
                    palette.status(target.status)
                )),
                None => output.push_str(&format!(
                    "  {}: {} (unresolved)\n",
                    reference.label,
                    palette.doc_id(&reference.raw_target)
                )),
            }
        }
    }

    output.push('\n');
    output.push_str(&palette.header("Downstream Associations"));
    output.push('\n');
    render_downstream_associations(&mut output, index, document, palette);

    output.push('\n');
    output.push_str(&palette.header("Derived Chain Summary"));
    output.push('\n');
    render_derived_summary(&mut output, index, document, palette);

    output.push('\n');
    output.push_str(&palette.header("Related Repository Warnings"));
    output.push('\n');
    let warnings = related_warnings(index, violations, document);
    if warnings.is_empty() {
        output.push_str("  No related warnings.\n");
    } else {
        for warning in warnings {
            output.push_str(&format!("  {}\n", warning.path));
            output.push_str(&format!("  {}\n", palette.warning(&warning.message)));
            output.push_str(&format!(
                "  -> {}\n",
                palette.warning("Repair the affected repository reference or managed document.")
            ));
        }
    }

    Ok(output)
}

fn render_derived_summary(
    output: &mut String,
    index: &DocumentIndex,
    document: &Document,
    palette: Palette,
) {
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
            let direct_tasks = direct_related_ids(index, document, AssociationKind::DesignDocTasks);
            let task_counts = task_status_counts_for_design(index, &document.id.as_string());
            output.push_str(&format!("  patches: {}\n", patches.len()));
            output.push_str(&format!("  exec plans: {}\n", execs.len()));
            output.push_str(&format!("  direct task specs: {}\n", direct_tasks.len()));
            output.push_str(&format!(
                "  task specs: {}\n",
                render_status_counts(
                    palette,
                    &[
                        (
                            Status::Draft,
                            task_counts.get(&Status::Draft).copied().unwrap_or(0)
                        ),
                        (
                            Status::Active,
                            task_counts.get(&Status::Active).copied().unwrap_or(0),
                        ),
                        (
                            Status::Completed,
                            task_counts.get(&Status::Completed).copied().unwrap_or(0),
                        ),
                        (
                            Status::Cancelled,
                            task_counts.get(&Status::Cancelled).copied().unwrap_or(0),
                        ),
                    ]
                ),
            ));
        }
        DocType::ExecPlan => {
            let counts = task_status_counts(index, &document.id.as_string());
            output.push_str(&format!(
                "  task specs: {}\n",
                render_status_counts(
                    palette,
                    &[
                        (
                            Status::Draft,
                            counts.get(&Status::Draft).copied().unwrap_or(0)
                        ),
                        (
                            Status::Active,
                            counts.get(&Status::Active).copied().unwrap_or(0)
                        ),
                        (
                            Status::Completed,
                            counts.get(&Status::Completed).copied().unwrap_or(0),
                        ),
                        (
                            Status::Cancelled,
                            counts.get(&Status::Cancelled).copied().unwrap_or(0),
                        ),
                    ]
                ),
            ));
        }
        DocType::TaskSpec => {
            match task_lineage(index, document) {
                TaskLineage::ExecPlan(lineage) => {
                    output.push_str(&format!(
                        "  exec-plan lineage: {}\n",
                        palette.render_lineage(&lineage)
                    ));
                }
                TaskLineage::DesignDoc(lineage) => {
                    output.push_str(&format!(
                        "  design-doc lineage: {}\n",
                        palette.render_lineage(&lineage)
                    ));
                }
                TaskLineage::None => output.push_str("  none\n"),
            }
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
                palette.optional_doc_id(document.frontmatter.parent.as_deref())
            ));
            output.push_str(&format!(
                "  merged-into: {}\n",
                palette.optional_doc_id(document.frontmatter.merged_into.as_deref())
            ));
            output.push_str(&format!(
                "  superseded-by: {}\n",
                palette.optional_doc_id(document.frontmatter.superseded_by.as_deref())
            ));
        }
        DocType::ProjectSpec | DocType::OrgSpec | DocType::Guideline => {
            output.push_str("  none\n");
        }
    }
}

fn render_downstream_associations(
    output: &mut String,
    index: &DocumentIndex,
    document: &Document,
    palette: Palette,
) {
    if document.doc_type == DocType::DesignDoc {
        render_design_downstream_associations(output, index, document, palette);
        return;
    }

    let summaries = association_summaries(index, document);
    if summaries.is_empty() {
        output.push_str("  none\n");
        return;
    }

    for summary in summaries {
        render_associated_documents(
            output,
            association_label(summary.kind),
            &sorted_related(&summary.related),
            palette,
        );
    }
}

fn render_design_downstream_associations(
    output: &mut String,
    index: &DocumentIndex,
    document: &Document,
    palette: Palette,
) {
    let design_id = document.id.as_string();
    let patches = direct_related_documents(index, document, AssociationKind::DesignDocPatches);
    let execs = execs_for_design(index, &design_id);
    let exec_tasks = exec_linked_tasks_for_design(index, &design_id);
    let direct_tasks = direct_tasks_for_design(index, &design_id);

    if patches.is_empty() && execs.is_empty() && exec_tasks.is_empty() && direct_tasks.is_empty() {
        output.push_str("  none\n");
        return;
    }

    render_related_documents(output, "design patches", &patches, palette);
    render_related_documents(output, "exec plans", &execs, palette);
    render_related_documents(output, "task specs via exec plans", &exec_tasks, palette);
    render_related_documents(output, "direct task specs", &direct_tasks, palette);
}

fn render_related_documents(
    output: &mut String,
    label: &str,
    related: &[&Document],
    palette: Palette,
) {
    output.push_str(&format!("  {label}\n"));
    if related.is_empty() {
        output.push_str("    none\n");
        return;
    }

    for document in related {
        output.push_str(&format!(
            "    {} ({})\n",
            palette.doc_id(&document.id.to_string()),
            palette.status(document.status)
        ));
    }
}

fn render_associated_documents(
    output: &mut String,
    label: &str,
    related: &[AssociatedDocument],
    palette: Palette,
) {
    output.push_str(&format!("  {label}\n"));
    if related.is_empty() {
        output.push_str("    none\n");
        return;
    }

    for document in related {
        output.push_str(&format!(
            "    {} ({})\n",
            palette.doc_id(&document.id.to_string()),
            palette.status(document.status)
        ));
    }
}

fn render_design_bucket(
    output: &mut String,
    index: &DocumentIndex,
    status: Status,
    label: &str,
    palette: Palette,
) {
    output.push_str(&format!("  {}\n", palette.status_label(label, status)));
    let designs = documents_by_type_and_status(index, DocType::DesignDoc, status);
    if designs.is_empty() {
        output.push_str("    none\n");
        return;
    }

    for design in designs {
        let design_id = design.id.as_string();
        output.push_str(&format!(
            "    {}  {}  {}  {}\n",
            palette.doc_id(&design.id.to_string()),
            title_for(design),
            palette.status(design.status),
            make_relative(index, &design.path)
        ));
        output.push_str(&format!(
            "      prd: {}  exec-plans: {}  direct-task-specs: {}  task-specs: {}\n",
            palette.optional_doc_id(design.frontmatter.prd.as_deref()),
            execs_for_design(index, &design_id).len(),
            direct_tasks_for_design(index, &design_id).len(),
            tasks_for_design(index, &design_id).len(),
        ));
    }
}

fn render_status_totals(
    output: &mut String,
    index: &DocumentIndex,
    doc_type: DocType,
    statuses: &[Status],
    palette: Palette,
) {
    let counts = count_by_status(index, doc_type, statuses);
    let parts = statuses
        .iter()
        .map(|status| {
            format!(
                "{}={}",
                palette.status(*status),
                counts.get(status).copied().unwrap_or(0)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    output.push_str(&format!("  {:<11} {}\n", doc_type.as_str(), parts));
}

fn render_status_counts(palette: Palette, counts: &[(Status, usize)]) -> String {
    counts
        .iter()
        .map(|(status, count)| format!("{}={count}", palette.status(*status)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_all_documents_bucket(
    output: &mut String,
    index: &DocumentIndex,
    doc_type: DocType,
    palette: Palette,
) {
    output.push_str(&format!("  {}\n", doc_type.as_str()));
    let documents = all_documents_by_type(index, doc_type);
    if documents.is_empty() {
        output.push_str("    none\n");
        return;
    }

    for document in documents {
        output.push_str(&format!(
            "    {}  {}  {}  {}\n",
            palette.doc_id(&document.id.to_string()),
            palette.status(document.status),
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
    if document.doc_type == DocType::DesignDoc {
        for exec in execs_for_design(index, &document.id.as_string()) {
            related_ids.insert(exec.id.as_string());
        }
        for task in tasks_for_design(index, &document.id.as_string()) {
            related_ids.insert(task.id.as_string());
        }
    }
    if let Some(exec_plan) = document.frontmatter.exec_plan.as_deref() {
        related_ids.insert(exec_plan.to_string());
    }
    if let Some(design_doc) = task_upstream_design_doc(index, document) {
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

fn direct_related_documents<'a>(
    index: &'a DocumentIndex,
    document: &Document,
    kind: AssociationKind,
) -> Vec<&'a Document> {
    let ids = direct_related_ids(index, document, kind)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut documents = index
        .documents
        .values()
        .filter(|candidate| ids.contains(&candidate.id.as_string()))
        .collect::<Vec<_>>();
    documents.sort_by(|left, right| left.id.cmp(&right.id));
    documents
}

enum TaskLineage {
    None,
    ExecPlan(String),
    DesignDoc(String),
}

fn task_lineage(index: &DocumentIndex, task: &Document) -> TaskLineage {
    if let Some(exec_plan) = task.frontmatter.exec_plan.as_deref() {
        return match task_design_doc_via_exec_plan(index, task) {
            Some(design_doc) => TaskLineage::ExecPlan(format!("{exec_plan} -> {design_doc}")),
            None => TaskLineage::ExecPlan(exec_plan.to_string()),
        };
    }
    match task.frontmatter.design_doc.as_deref() {
        Some(design_doc) => TaskLineage::DesignDoc(design_doc.to_string()),
        None => TaskLineage::None,
    }
}

fn task_upstream_design_doc(index: &DocumentIndex, task: &Document) -> Option<String> {
    task.frontmatter
        .design_doc
        .clone()
        .or_else(|| task_design_doc_via_exec_plan(index, task))
}

fn task_design_doc_via_exec_plan(index: &DocumentIndex, task: &Document) -> Option<String> {
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
            document.frontmatter.design_doc.as_deref() == Some(design_id)
                || document
                    .frontmatter
                    .exec_plan
                    .as_ref()
                    .is_some_and(|exec_id| exec_ids.contains(exec_id))
        })
        .collect::<Vec<_>>();
    tasks.sort_by(|left, right| left.id.cmp(&right.id));
    tasks.dedup_by(|left, right| left.id == right.id);
    tasks
}

fn exec_linked_tasks_for_design<'a>(
    index: &'a DocumentIndex,
    design_id: &str,
) -> Vec<&'a Document> {
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

fn direct_tasks_for_design<'a>(index: &'a DocumentIndex, design_id: &str) -> Vec<&'a Document> {
    let mut tasks = index
        .documents
        .values()
        .filter(|document| document.doc_type == DocType::TaskSpec)
        .filter(|document| document.frontmatter.design_doc.as_deref() == Some(design_id))
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

fn task_status_counts_for_design(
    index: &DocumentIndex,
    design_id: &str,
) -> BTreeMap<Status, usize> {
    let mut counts = BTreeMap::new();
    for task in tasks_for_design(index, design_id) {
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
        AssociationKind::DesignDocTasks => "direct task specs",
        AssociationKind::ExecPlanTasks => "task specs",
    }
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

impl Palette {
    fn new(color_when: ColorWhen, stdout_is_tty: bool) -> Self {
        let enabled = match color_when {
            ColorWhen::Always => true,
            ColorWhen::Never => false,
            ColorWhen::Auto => stdout_is_tty && env::var_os("NO_COLOR").is_none(),
        };
        Self { enabled }
    }

    fn header(self, text: &str) -> String {
        self.wrap("1", text)
    }

    fn warning(self, text: &str) -> String {
        self.wrap("31", text)
    }

    fn status(self, status: Status) -> String {
        self.wrap(self.status_code(status), status.as_str())
    }

    fn status_label(self, label: &str, status: Status) -> String {
        self.wrap(self.status_code(status), label)
    }

    fn doc_id(self, text: &str) -> String {
        self.wrap(ID_COLOR, text)
    }

    fn optional_doc_id(self, value: Option<&str>) -> String {
        value
            .map(|value| self.doc_id(value))
            .unwrap_or_else(|| "none".to_string())
    }

    fn render_lineage(self, raw: &str) -> String {
        raw.split(" -> ")
            .map(|part| self.doc_id(part))
            .collect::<Vec<_>>()
            .join(" -> ")
    }

    fn status_code(self, status: Status) -> &'static str {
        match status {
            Status::Draft => "33",
            Status::Candidate => "34",
            Status::Implemented | Status::Approved | Status::Completed => "32",
            Status::Active => "36",
            Status::Obsolete | Status::ObsoleteMerged | Status::Abandoned | Status::Cancelled => {
                "2;31"
            }
        }
    }

    fn wrap(self, code: &str, text: &str) -> String {
        if self.enabled {
            format!("\u{1b}[{code}m{text}\u{1b}[0m")
        } else {
            text.to_string()
        }
    }
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

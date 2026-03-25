use crate::doc::{
    association_summaries, build_index, expected_directory, is_live_status, is_terminal_status,
    validate_index, AssociationKind, DocType, Document, DocumentIndex, Status, ValidationViolation,
};
use anyhow::{bail, Context, Result};
use clap::{Args, ValueEnum};
use std::collections::BTreeMap;
use std::fs;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

/// Arguments for `specmate status`.
#[derive(Args, Debug, Clone)]
#[command(
    after_help = "Examples:\n  specmate status\n  specmate status --all\n  specmate status design-auth-system\n  specmate status exec-auth-add-oauth/task-01"
)]
pub struct StatusArgs {
    /// Optional managed document id such as `exec-auth-add-oauth/task-01` or `design-auth-system`
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

#[derive(Copy, Clone, Debug)]
pub struct Palette {
    enabled: bool,
}

impl Palette {
    fn new(color: ColorWhen, is_tty: bool) -> Self {
        let enabled = match color {
            ColorWhen::Auto => is_tty,
            ColorWhen::Always => true,
            ColorWhen::Never => false,
        };
        Self { enabled }
    }

    fn paint(&self, code: &str, text: &str) -> String {
        if self.enabled {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    fn header(&self, text: &str) -> String {
        self.paint("1", text)
    }

    fn doc_id(&self, text: &str) -> String {
        self.paint("1", text)
    }

    fn status(&self, status: Status) -> String {
        let code = match status {
            Status::Implemented | Status::Approved | Status::Closed => "32",
            Status::Candidate => "33",
            Status::Draft => "36",
            Status::Obsolete | Status::ObsoleteMerged => "90",
            Status::Active => "35",
        };
        self.paint(code, status.as_str())
    }
}

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
        Some(doc_id) => match render_detail(&index, &violations, doc_id.trim(), palette) {
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

fn find_repo_root(start_dir: &Path) -> std::result::Result<PathBuf, StatusFailure> {
    let mut current = fs::canonicalize(start_dir).map_err(|error| {
        StatusFailure::new(
            Some(start_dir.to_path_buf()),
            format!("failed to read {}: {error}", start_dir.display()),
            "Use a readable working directory inside a specmate repository.",
        )
    })?;

    loop {
        if current.join("docs/specs/project.md").is_file() {
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

    output.push('\n');
    output.push_str(&palette.header("Design Overview"));
    output.push('\n');
    render_design_bucket(&mut output, index, Status::Draft, palette);
    render_design_bucket(&mut output, index, Status::Candidate, palette);
    render_design_bucket(&mut output, index, Status::Implemented, palette);

    output.push('\n');
    output.push_str(&palette.header("Execution Overview"));
    output.push('\n');
    render_exec_overview(&mut output, index, palette);

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
        &[Status::Draft, Status::Candidate, Status::Closed],
        palette,
    );
    render_status_totals(
        &mut output,
        index,
        DocType::TaskSpec,
        &[Status::Draft, Status::Candidate, Status::Closed],
        palette,
    );

    if show_all {
        output.push('\n');
        output.push_str(&palette.header("All Documents"));
        output.push('\n');
        for document in all_documents(index) {
            output.push_str(&format!(
                "  {}  {}  {}\n",
                palette.doc_id(&document.id.as_string()),
                document.doc_type,
                palette.status(document.status)
            ));
        }
    }

    output
}

fn render_exec_overview(output: &mut String, index: &DocumentIndex, palette: Palette) {
    let candidate_execs = documents_by_type_and_status(index, DocType::ExecPlan, Status::Candidate);
    output.push_str(&format!(
        "  {} exec plans\n",
        palette.status(Status::Candidate)
    ));
    if candidate_execs.is_empty() {
        output.push_str("    none\n");
    } else {
        for exec in candidate_execs {
            let design_docs = if exec.frontmatter.design_docs.is_empty() {
                "none".to_string()
            } else {
                exec.frontmatter.design_docs.join(", ")
            };
            let task_counts = task_status_counts(index, &exec.id.as_string());
            output.push_str(&format!(
                "    {}  {}  design-docs: {}  tasks: draft={} candidate={} closed={}\n",
                palette.doc_id(&exec.id.as_string()),
                title_for(exec),
                design_docs,
                task_counts.get(&Status::Draft).copied().unwrap_or(0),
                task_counts.get(&Status::Candidate).copied().unwrap_or(0),
                task_counts.get(&Status::Closed).copied().unwrap_or(0),
            ));
        }
    }

    let candidate_tasks = documents_by_type_and_status(index, DocType::TaskSpec, Status::Candidate);
    output.push_str(&format!(
        "  {} task specs\n",
        palette.status(Status::Candidate)
    ));
    if candidate_tasks.is_empty() {
        output.push_str("    none\n");
    } else {
        for task in candidate_tasks {
            output.push_str(&format!(
                "    {}  {}\n",
                palette.doc_id(&task.id.as_string()),
                title_for(task)
            ));
        }
    }
}

fn render_detail(
    index: &DocumentIndex,
    violations: &[ValidationViolation],
    doc_id: &str,
    palette: Palette,
) -> std::result::Result<String, StatusFailure> {
    let document = index
        .documents
        .values()
        .find(|document| document.id.as_string() == doc_id)
        .ok_or_else(|| {
            StatusFailure::new(
                Some(index.repo_root.clone()),
                format!("managed document {doc_id} does not exist"),
                "Use a canonical managed document id such as design-auth-system or exec-auth-add-oauth/task-01.",
            )
        })?;

    if document.doc_type == DocType::Guideline {
        return Err(StatusFailure::new(
            Some(document.path.clone()),
            format!("guideline lookup target {doc_id} is not supported"),
            "Choose a lifecycle-managed document id.",
        ));
    }

    let mut output = String::new();
    output.push_str(&palette.header("Overview"));
    output.push('\n');
    output.push_str(&format!(
        "  id: {}\n",
        palette.doc_id(&document.id.as_string())
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
    if let Some(expected) = expected_directory(document.doc_type, document.status) {
        output.push_str(&format!("  expected directory: {expected}\n"));
    }

    output.push('\n');
    output.push_str(&palette.header("Upstream References"));
    output.push('\n');
    let references = reference_rows(document);
    if references.is_empty() {
        output.push_str("  none\n");
    } else {
        for (label, values) in references {
            output.push_str(&format!("  {label}: {}\n", values.join(", ")));
        }
    }

    output.push('\n');
    output.push_str(&palette.header("Downstream Associations"));
    output.push('\n');
    let summaries = association_summaries(index, document);
    if summaries.is_empty() || summaries.iter().all(|summary| summary.related.is_empty()) {
        output.push_str("  none\n");
    } else {
        for summary in summaries {
            if summary.related.is_empty() {
                continue;
            }
            output.push_str(&format!(
                "  {}: {}\n",
                association_label(summary.kind),
                summary
                    .related
                    .iter()
                    .map(|related| format!("{} ({})", related.id, related.status))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    output.push('\n');
    output.push_str(&palette.header("Related Repository Warnings"));
    output.push('\n');
    let related = related_warnings(index, violations, document);
    if related.is_empty() {
        output.push_str("  none\n");
    } else {
        for warning in related {
            output.push_str(&format!("  {}: {}\n", warning.path, warning.message));
        }
    }

    Ok(output)
}

fn reference_rows(document: &Document) -> Vec<(&'static str, Vec<String>)> {
    let mut rows = Vec::new();
    if let Some(prd) = document.frontmatter.prd.as_ref() {
        rows.push(("prd", vec![prd.clone()]));
    }
    if let Some(parent) = document.frontmatter.parent.as_ref() {
        rows.push(("parent", vec![parent.clone()]));
    }
    if let Some(merged_into) = document.frontmatter.merged_into.as_ref() {
        rows.push(("merged-into", vec![merged_into.clone()]));
    }
    if let Some(superseded_by) = document.frontmatter.superseded_by.as_ref() {
        rows.push(("superseded-by", vec![superseded_by.clone()]));
    }
    if !document.frontmatter.design_docs.is_empty() {
        rows.push(("design-docs", document.frontmatter.design_docs.clone()));
    }
    if let Some(exec_plan) = document.frontmatter.exec_plan.as_ref() {
        rows.push(("exec-plan", vec![exec_plan.clone()]));
    }
    rows
}

fn related_warnings(
    index: &DocumentIndex,
    violations: &[ValidationViolation],
    document: &Document,
) -> Vec<RelatedWarning> {
    let mut warnings = Vec::new();
    let document_id = document.id.as_string();
    for violation in violations {
        let path = make_relative(index, &violation.path);
        if violation.path == document.path
            || violation.message.contains(&document_id)
            || path == make_relative(index, &document.path)
        {
            warnings.push(RelatedWarning {
                path,
                message: violation.message.clone(),
            });
        }
    }
    warnings
}

fn render_design_bucket(
    output: &mut String,
    index: &DocumentIndex,
    status: Status,
    palette: Palette,
) {
    output.push_str(&format!("  {}\n", palette.status(status)));
    let documents =
        documents_by_types_and_status(index, &[DocType::DesignDoc, DocType::DesignPatch], status);
    if documents.is_empty() {
        output.push_str("    none\n");
        return;
    }
    for document in documents {
        output.push_str(&format!(
            "    {}  {}  {}\n",
            palette.doc_id(&document.id.as_string()),
            title_for(document),
            palette.status(document.status)
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
    let rendered = statuses
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
    output.push_str(&format!("  {}  {}\n", doc_type, rendered));
}

fn documents_by_type_and_status(
    index: &DocumentIndex,
    doc_type: DocType,
    status: Status,
) -> Vec<&Document> {
    documents_by_types_and_status(index, &[doc_type], status)
}

fn documents_by_types_and_status<'a>(
    index: &'a DocumentIndex,
    doc_types: &[DocType],
    status: Status,
) -> Vec<&'a Document> {
    let mut documents = index
        .documents
        .values()
        .filter(|document| doc_types.contains(&document.doc_type) && document.status == status)
        .collect::<Vec<_>>();
    documents.sort_by_key(|document| document.id.as_string());
    documents
}

fn all_documents(index: &DocumentIndex) -> Vec<&Document> {
    let mut documents = index.documents.values().collect::<Vec<_>>();
    documents.sort_by_key(|document| document.id.as_string());
    documents
}

fn count_by_status(
    index: &DocumentIndex,
    doc_type: DocType,
    statuses: &[Status],
) -> BTreeMap<Status, usize> {
    let mut counts = BTreeMap::new();
    for status in statuses {
        counts.insert(
            *status,
            index
                .documents
                .values()
                .filter(|document| document.doc_type == doc_type && document.status == *status)
                .count(),
        );
    }
    counts
}

fn task_status_counts(index: &DocumentIndex, exec_id: &str) -> BTreeMap<Status, usize> {
    let mut counts = BTreeMap::new();
    for status in [Status::Draft, Status::Candidate, Status::Closed] {
        counts.insert(
            status,
            index
                .documents
                .values()
                .filter(|document| document.doc_type == DocType::TaskSpec)
                .filter(|document| document.frontmatter.exec_plan.as_deref() == Some(exec_id))
                .filter(|document| document.status == status)
                .count(),
        );
    }
    counts
}

fn title_for(document: &Document) -> &str {
    document.title.as_deref().unwrap_or("untitled")
}

fn association_label(kind: AssociationKind) -> &'static str {
    match kind {
        AssociationKind::PrdDesignDocs => "design docs",
        AssociationKind::DesignDocPatches => "patches",
        AssociationKind::DesignDocExecPlans => "exec plans",
        AssociationKind::DesignDocTasks => "direct tasks",
        AssociationKind::ExecPlanTasks => "tasks",
    }
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
        writeln!(stderr, "       {}", display_path(repo_root, path))?;
    }
    writeln!(stderr, "       {}", failure.message)?;
    writeln!(stderr, "       -> {}", failure.fix)?;
    Ok(())
}

fn display_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .map(|relative| {
            if relative.as_os_str().is_empty() {
                repo_root.display().to_string()
            } else {
                relative.display().to_string()
            }
        })
        .unwrap_or_else(|_| path.display().to_string())
}

#[derive(Debug, Clone)]
struct RelatedWarning {
    path: String,
    message: String,
}

#[cfg(test)]
#[path = "../../tests/cmd/check_support.rs"]
mod check_support;

#[cfg(test)]
#[path = "../../tests/cmd/status_test.rs"]
mod status_test;

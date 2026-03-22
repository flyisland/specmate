use crate::doc::{
    build_compliant_index, preview_transition, validate_preview, validate_transition, DocType,
    Document, DocumentIndex, Status,
};
use anyhow::{bail, Context, Result};
use clap::Args;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Arguments for `specmate move`.
#[derive(Args, Debug, Clone)]
#[command(
    after_help = "Examples:\n  specmate move exec-001 active\n  specmate move task-0007 completed\n  specmate move design-001 implemented --dry-run"
)]
pub struct MoveArgs {
    /// Managed document id such as `task-0001` or `design-004-patch-01`
    pub doc_id: String,
    /// Target status valid for the resolved document type
    pub to_status: String,
    /// Show the planned file changes without writing files
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug)]
struct MovePlan {
    repo_root: PathBuf,
    source_path: PathBuf,
    destination_path: PathBuf,
    from_status: Status,
    to_status: Status,
    updated_contents: String,
}

#[derive(Debug)]
struct MoveFailure {
    path: Option<PathBuf>,
    message: String,
    fix: String,
}

impl MoveFailure {
    fn new(path: Option<PathBuf>, message: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            path,
            message: message.into(),
            fix: fix.into(),
        }
    }
}

/// Run `specmate move`.
pub fn run(args: MoveArgs) -> Result<()> {
    let start_dir = std::env::current_dir().context("reading current working directory")?;
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    run_in_repo(&start_dir, args, &mut stdout, &mut stderr)
}

fn run_in_repo<W: Write, E: Write>(
    start_dir: &Path,
    args: MoveArgs,
    stdout: &mut W,
    stderr: &mut E,
) -> Result<()> {
    let repo_root = match find_repo_root(start_dir) {
        Ok(repo_root) => repo_root,
        Err(failure) => {
            render_failure(stderr, start_dir, &failure)?;
            bail!("specmate move failed");
        }
    };

    let plan = match plan_move(&repo_root, &args) {
        Ok(plan) => plan,
        Err(failure) => {
            render_failure(stderr, &repo_root, &failure)?;
            bail!("specmate move failed");
        }
    };

    if args.dry_run {
        render_plan(stdout, &plan, true)?;
        return Ok(());
    }

    if let Err(failure) = apply_move(&plan) {
        render_failure(stderr, &repo_root, &failure)?;
        bail!("specmate move failed");
    }

    render_plan(stdout, &plan, false)?;
    Ok(())
}

fn find_repo_root(start_dir: &Path) -> std::result::Result<PathBuf, MoveFailure> {
    let mut current = fs::canonicalize(start_dir).map_err(|error| {
        MoveFailure::new(
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
            return Err(MoveFailure::new(
                Some(start_dir.to_path_buf()),
                format!(
                    "could not locate a specmate repository root from {}",
                    start_dir.display()
                ),
                "Run specmate move from a specmate repository or one of its subdirectories.",
            ));
        }
    }
}

fn plan_move(repo_root: &Path, args: &MoveArgs) -> std::result::Result<MovePlan, MoveFailure> {
    let index = build_compliant_index(repo_root).map_err(|error| {
        MoveFailure::new(
            Some(repo_root.to_path_buf()),
            format!("repository document state is invalid: {error}"),
            "Repair the reported repository violations and re-run specmate move.",
        )
    })?;

    let document = resolve_document(&index, &args.doc_id).ok_or_else(|| {
        MoveFailure::new(
            Some(repo_root.to_path_buf()),
            format!("managed document {} does not exist", args.doc_id.trim()),
            "Use a canonical managed document id such as task-0001 or design-004.",
        )
    })?;

    if matches!(
        document.doc_type,
        DocType::ProjectSpec | DocType::OrgSpec | DocType::Guideline
    ) {
        return Err(MoveFailure::new(
            Some(document.path.clone()),
            format!("{} does not support status transitions", document.id),
            "Choose a PRD, Design Doc, Design Patch, Exec Plan, or Task Spec.",
        ));
    }

    let to_status = parse_target_status(document.doc_type, &args.to_status).ok_or_else(|| {
        MoveFailure::new(
            Some(document.path.clone()),
            format!(
                "status {} is not valid for {}",
                args.to_status.trim(),
                document.doc_type
            ),
            "Choose a target status from the lifecycle for this document type.",
        )
    })?;

    if document.status == to_status {
        return Err(MoveFailure::new(
            Some(document.path.clone()),
            format!("{} is already {}", document.id, document.status),
            "Choose a different target status.",
        ));
    }

    validate_transition(&index, document, to_status).map_err(|error| {
        MoveFailure::new(
            Some(document.path.clone()),
            error.to_string(),
            "Fix the blocking transition rule or choose a different target status.",
        )
    })?;

    let preview = preview_transition(&index, document, to_status).map_err(|error| {
        MoveFailure::new(
            Some(document.path.clone()),
            format!("failed to build post-move preview: {error}"),
            "Repair the document state and re-run specmate move.",
        )
    })?;
    let preview_violations = validate_preview(&preview);
    if let Some(violation) = preview_violations.first() {
        return Err(MoveFailure::new(
            Some(violation.path.clone()),
            format!(
                "post-move repository would be invalid: {}",
                violation.message
            ),
            "Repair the blocking references or choose a different target status.",
        ));
    }

    let destination_path = preview
        .documents
        .get(&document.id)
        .map(|entry| entry.path.clone())
        .ok_or_else(|| {
            MoveFailure::new(
                Some(document.path.clone()),
                format!(
                    "document {} is missing from the post-move preview",
                    document.id
                ),
                "Repair the repository state and re-run specmate move.",
            )
        })?;

    let destination_dir = destination_path.parent().ok_or_else(|| {
        MoveFailure::new(
            Some(destination_path.clone()),
            format!(
                "destination {} has no parent directory",
                destination_path.display()
            ),
            "Repair the repository layout and re-run specmate move.",
        )
    })?;
    if !destination_dir.is_dir() {
        return Err(MoveFailure::new(
            Some(destination_dir.to_path_buf()),
            format!(
                "destination directory {} does not exist",
                display_path(repo_root, destination_dir)
            ),
            "Create the managed destination directory or repair the repository layout.",
        ));
    }

    if destination_path != document.path && destination_path.exists() {
        return Err(MoveFailure::new(
            Some(destination_path.clone()),
            format!(
                "destination path {} already exists",
                display_path(repo_root, &destination_path)
            ),
            "Remove or rename the existing target file and re-run specmate move.",
        ));
    }

    let updated_contents = rewrite_status(&document.raw, to_status).map_err(|message| {
        MoveFailure::new(
            Some(document.path.clone()),
            message,
            "Repair the document frontmatter and re-run specmate move.",
        )
    })?;

    Ok(MovePlan {
        repo_root: index.repo_root.clone(),
        source_path: document.path.clone(),
        destination_path,
        from_status: document.status,
        to_status,
        updated_contents,
    })
}

fn resolve_document<'a>(index: &'a DocumentIndex, raw: &str) -> Option<&'a Document> {
    let wanted = raw.trim();
    index
        .documents
        .values()
        .find(|document| document.id.as_string() == wanted)
}

fn parse_target_status(doc_type: DocType, raw: &str) -> Option<Status> {
    match (doc_type, raw.trim()) {
        (DocType::Prd, "draft") => Some(Status::Draft),
        (DocType::Prd, "approved") => Some(Status::Approved),
        (DocType::Prd, "obsolete") => Some(Status::Obsolete),
        (DocType::DesignDoc, "draft") => Some(Status::Draft),
        (DocType::DesignDoc, "candidate") => Some(Status::Candidate),
        (DocType::DesignDoc, "implemented") => Some(Status::Implemented),
        (DocType::DesignDoc, "obsolete") => Some(Status::Obsolete),
        (DocType::DesignPatch, "draft") => Some(Status::Draft),
        (DocType::DesignPatch, "candidate") => Some(Status::Candidate),
        (DocType::DesignPatch, "implemented") => Some(Status::Implemented),
        (DocType::DesignPatch, "obsolete") => Some(Status::Obsolete),
        (DocType::DesignPatch, "obsolete:merged") => Some(Status::ObsoleteMerged),
        (DocType::ExecPlan, "draft") => Some(Status::Draft),
        (DocType::ExecPlan, "active") => Some(Status::Active),
        (DocType::ExecPlan, "completed") => Some(Status::Completed),
        (DocType::ExecPlan, "abandoned") => Some(Status::Abandoned),
        (DocType::TaskSpec, "draft") => Some(Status::Draft),
        (DocType::TaskSpec, "active") => Some(Status::Active),
        (DocType::TaskSpec, "completed") => Some(Status::Completed),
        (DocType::TaskSpec, "cancelled") => Some(Status::Cancelled),
        _ => None,
    }
}

fn rewrite_status(raw: &str, to_status: Status) -> std::result::Result<String, String> {
    let mut lines: Vec<String> = raw.lines().map(ToOwned::to_owned).collect();
    if !matches!(lines.first().map(|line| line.as_str()), Some("---")) {
        return Err("document is missing a leading frontmatter block".to_string());
    }

    let end = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (line == "---").then_some(index))
        .ok_or_else(|| "document frontmatter is missing its closing delimiter".to_string())?;

    let replacement = format!("status: {}", to_status.as_str());
    let mut replaced = false;
    for line in lines.iter_mut().take(end).skip(1) {
        if line.trim_start().starts_with("status:") {
            *line = replacement.clone();
            replaced = true;
            break;
        }
    }

    if !replaced {
        return Err("document frontmatter does not contain a status field".to_string());
    }

    let mut updated = lines.join("\n");
    if raw.ends_with('\n') {
        updated.push('\n');
    }
    Ok(updated)
}

fn apply_move(plan: &MovePlan) -> std::result::Result<(), MoveFailure> {
    let destination_parent = plan.destination_path.parent().ok_or_else(|| {
        MoveFailure::new(
            Some(plan.destination_path.clone()),
            "destination path has no parent directory",
            "Repair the repository layout and re-run specmate move.",
        )
    })?;

    let temp_path = next_temp_path(destination_parent, &plan.destination_path);
    let mut temp_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(|error| {
            MoveFailure::new(
                Some(temp_path.clone()),
                format!(
                    "failed to create temporary file {}: {error}",
                    display_path(&plan.repo_root, &temp_path)
                ),
                "Check filesystem permissions and available space, then re-run specmate move.",
            )
        })?;
    temp_file
        .write_all(plan.updated_contents.as_bytes())
        .and_then(|_| temp_file.flush())
        .map_err(|error| {
            MoveFailure::new(
                Some(temp_path.clone()),
                format!(
                    "failed to write temporary file {}: {error}",
                    display_path(&plan.repo_root, &temp_path)
                ),
                "Check filesystem permissions and available space, then re-run specmate move.",
            )
        })?;
    drop(temp_file);

    fs::rename(&temp_path, &plan.destination_path).map_err(|error| {
        let _ = fs::remove_file(&temp_path);
        MoveFailure::new(
            Some(plan.destination_path.clone()),
            format!(
                "failed to place updated document at {}: {error}",
                display_path(&plan.repo_root, &plan.destination_path)
            ),
            "Check filesystem permissions and re-run specmate move.",
        )
    })?;

    if plan.source_path != plan.destination_path {
        fs::remove_file(&plan.source_path).map_err(|error| {
            MoveFailure::new(
                Some(plan.source_path.clone()),
                format!(
                    "updated file was written but failed to remove the original {}: {error}",
                    display_path(&plan.repo_root, &plan.source_path)
                ),
                "Delete the stale source file and verify the move result before retrying.",
            )
        })?;
    }

    Ok(())
}

fn next_temp_path(parent: &Path, target: &Path) -> PathBuf {
    let file_name = target
        .file_name()
        .map(OsString::from)
        .unwrap_or_else(|| OsString::from("document.md"));
    for attempt in 0..1000 {
        let mut candidate = OsString::from(".specmate-move-");
        candidate.push(std::process::id().to_string());
        candidate.push("-");
        candidate.push(attempt.to_string());
        candidate.push("-");
        candidate.push(&file_name);
        let path = parent.join(candidate);
        if !path.exists() {
            return path;
        }
    }
    parent.join(".specmate-move-fallback.tmp")
}

fn render_plan<W: Write>(stdout: &mut W, plan: &MovePlan, dry_run: bool) -> Result<()> {
    if dry_run {
        writeln!(stdout, "Planned operations (no files will be written):")?;
    }

    writeln!(
        stdout,
        "  [user] UPDATE    {}  (status: {} -> {})",
        display_path(&plan.repo_root, &plan.source_path),
        plan.from_status,
        plan.to_status
    )?;

    if plan.source_path != plan.destination_path {
        writeln!(
            stdout,
            "  [user] MOVE      {} -> {}",
            display_path(&plan.repo_root, &plan.source_path),
            display_path(&plan.repo_root, &plan.destination_path)
        )?;
    }

    if dry_run {
        writeln!(stdout, "Run without --dry-run to apply.")?;
    }

    Ok(())
}

fn render_failure<E: Write>(stderr: &mut E, repo_root: &Path, failure: &MoveFailure) -> Result<()> {
    writeln!(stderr, "[fail] move")?;
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

#[cfg(test)]
#[allow(dead_code)]
#[path = "../../tests/cmd/check_support.rs"]
mod check_support;

#[cfg(test)]
#[path = "../../tests/cmd/move_test.rs"]
mod move_test;

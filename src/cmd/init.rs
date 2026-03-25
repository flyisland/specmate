use crate::config::{Config, Lang};
use anyhow::{bail, Context, Result};
use clap::Args;
use std::borrow::Cow;
use std::fs;
use std::io::Write;
use std::path::Path;

const STANDARD_PATHS: [&str; 6] = [
    ".specmate/config.yaml",
    "docs/specs",
    "docs/guidelines",
    "docs/prd",
    "docs/design",
    "docs/exec-plans",
];

const REQUIRED_DIRECTORIES: [&str; 15] = [
    ".specmate",
    "docs",
    "docs/specs",
    "docs/guidelines",
    "docs/guidelines/obsolete",
    "docs/prd",
    "docs/prd/draft",
    "docs/prd/approved",
    "docs/prd/obsolete",
    "docs/design",
    "docs/design/draft",
    "docs/design/candidate",
    "docs/design/implemented",
    "docs/design/obsolete",
    "docs/exec-plans",
];

/// Arguments for `specmate init`.
#[derive(Args, Debug, Clone)]
#[command(
    after_help = "Examples:\n  specmate init\n  specmate init --lang zh\n  specmate init --merge --dry-run"
)]
pub struct InitArgs {
    /// Language for generated document content
    #[arg(long, value_enum)]
    pub lang: Option<Lang>,

    /// Print planned operations without writing any files
    #[arg(long)]
    pub dry_run: bool,

    /// Merge into existing repo: overwrite specmate-owned files,
    /// skip user-owned files, create missing structure
    #[arg(long)]
    pub merge: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Ownership {
    Dir,
    Specmate,
    User,
}

impl Ownership {
    fn tag(self) -> &'static str {
        match self {
            Ownership::Dir => "[dir]",
            Ownership::Specmate => "[specmate]",
            Ownership::User => "[user]",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Create,
    Overwrite,
    Skip,
}

impl Action {
    fn label(self) -> &'static str {
        match self {
            Action::Create => "CREATE",
            Action::Overwrite => "OVERWRITE",
            Action::Skip => "SKIP",
        }
    }
}

#[derive(Debug)]
struct Operation {
    ownership: Ownership,
    action: Action,
    path: String,
    content: Option<Cow<'static, str>>,
    note: Option<&'static str>,
}

impl Operation {
    fn directory(path: &str) -> Self {
        Self {
            ownership: Ownership::Dir,
            action: Action::Create,
            path: path.to_string(),
            content: None,
            note: Some("missing directory"),
        }
    }

    fn file(
        ownership: Ownership,
        action: Action,
        path: &str,
        content: Option<Cow<'static, str>>,
        note: Option<&'static str>,
    ) -> Self {
        Self {
            ownership,
            action,
            path: path.to_string(),
            content,
            note,
        }
    }
}

struct TemplateSet {
    agents: &'static str,
    specs_readme: &'static str,
    project: &'static str,
    org: &'static str,
    prd_readme: &'static str,
    design_docs_readme: &'static str,
    exec_plans_readme: &'static str,
}

impl TemplateSet {
    fn for_lang(lang: Lang) -> Self {
        match lang {
            Lang::En => Self {
                agents: include_str!("../template/en/AGENTS.md"),
                specs_readme: include_str!("../template/en/specs-README.md"),
                project: include_str!("../template/en/project.md"),
                org: include_str!("../template/en/org.md"),
                prd_readme: include_str!("../template/en/prd-README.md"),
                design_docs_readme: include_str!("../template/en/design-docs-README.md"),
                exec_plans_readme: include_str!("../template/en/exec-plans-README.md"),
            },
            Lang::Zh => Self {
                agents: include_str!("../template/zh/AGENTS.md"),
                specs_readme: include_str!("../template/zh/specs-README.md"),
                project: include_str!("../template/zh/project.md"),
                org: include_str!("../template/zh/org.md"),
                prd_readme: include_str!("../template/zh/prd-README.md"),
                design_docs_readme: include_str!("../template/zh/design-docs-README.md"),
                exec_plans_readme: include_str!("../template/zh/exec-plans-README.md"),
            },
        }
    }
}

/// Run `specmate init`.
///
/// Deploys the full directory structure and self-documentation into the repo.
/// This is the onboarding command — the first thing a team runs in a new repo.
pub fn run(args: InitArgs) -> Result<()> {
    let repo_root = std::env::current_dir().context("reading current working directory")?;
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    run_in_repo(&repo_root, args, &mut stdout, &mut stderr)
}

fn run_in_repo<W: Write, E: Write>(
    repo_root: &Path,
    args: InitArgs,
    stdout: &mut W,
    stderr: &mut E,
) -> Result<()> {
    let existing_marker = find_existing_marker(repo_root);
    if existing_marker.is_some() && !args.merge {
        write_existing_repo_error(stderr, existing_marker.unwrap_or_default())?;
        bail!("specmate init failed");
    }

    let lang = resolve_lang(repo_root, args.lang, stderr);
    let operations = build_plan(repo_root, lang, args.merge)?;

    if args.dry_run {
        write_dry_run(stdout, &operations)?;
    } else {
        apply_operations(repo_root, &operations)?;
        write_applied(stdout, &operations)?;
    }

    Ok(())
}

fn resolve_lang<E: Write>(repo_root: &Path, explicit_lang: Option<Lang>, stderr: &mut E) -> Lang {
    explicit_lang.unwrap_or_else(|| Config::load_with_warnings(repo_root, stderr).lang)
}

fn build_plan(repo_root: &Path, lang: Lang, merge: bool) -> Result<Vec<Operation>> {
    let templates = TemplateSet::for_lang(lang);
    let config_content = serde_yaml::to_string(&Config { lang }).context("serialising config")?;
    let mut operations = Vec::new();

    for directory in REQUIRED_DIRECTORIES {
        push_directory_if_missing(repo_root, directory, &mut operations);
    }

    let file_specs = [
        (
            Ownership::User,
            "AGENTS.md",
            Cow::Borrowed(templates.agents),
        ),
        (
            Ownership::User,
            ".specmate/config.yaml",
            Cow::Owned(config_content),
        ),
        (
            Ownership::Specmate,
            "docs/specs/README.md",
            Cow::Borrowed(templates.specs_readme),
        ),
        (
            Ownership::User,
            "docs/specs/project.md",
            Cow::Borrowed(templates.project),
        ),
        (
            Ownership::User,
            "docs/specs/org.md",
            Cow::Borrowed(templates.org),
        ),
        (
            Ownership::Specmate,
            "docs/prd/README.md",
            Cow::Borrowed(templates.prd_readme),
        ),
        (
            Ownership::Specmate,
            "docs/design/README.md",
            Cow::Borrowed(templates.design_docs_readme),
        ),
        (
            Ownership::Specmate,
            "docs/exec-plans/README.md",
            Cow::Borrowed(templates.exec_plans_readme),
        ),
    ];

    for (ownership, path, content) in file_specs {
        push_file_operation(repo_root, ownership, path, content, merge, &mut operations);
    }

    Ok(operations)
}

fn push_directory_if_missing(
    repo_root: &Path,
    relative_path: &str,
    operations: &mut Vec<Operation>,
) {
    if !repo_root.join(relative_path).exists() {
        operations.push(Operation::directory(relative_path));
    }
}

fn push_file_operation(
    repo_root: &Path,
    ownership: Ownership,
    relative_path: &str,
    content: Cow<'static, str>,
    merge: bool,
    operations: &mut Vec<Operation>,
) {
    let exists = repo_root.join(relative_path).exists();
    let action = match (ownership, merge, exists) {
        (_, _, false) => Action::Create,
        (Ownership::Specmate, true, true) => Action::Overwrite,
        (Ownership::User, true, true) => Action::Skip,
        (_, false, true) => Action::Skip,
        (Ownership::Dir, _, true) => return,
    };
    let note = match action {
        Action::Skip => Some("already exists"),
        _ => None,
    };
    let content = match action {
        Action::Skip => None,
        _ => Some(content),
    };
    operations.push(Operation::file(
        ownership,
        action,
        relative_path,
        content,
        note,
    ));
}

fn apply_operations(repo_root: &Path, operations: &[Operation]) -> Result<()> {
    for operation in operations {
        if operation.ownership == Ownership::Dir {
            fs::create_dir_all(repo_root.join(&operation.path))
                .with_context(|| format!("creating {}", operation.path))?;
        }
    }

    for operation in operations {
        match operation.action {
            Action::Skip => {}
            Action::Create | Action::Overwrite if operation.ownership != Ownership::Dir => {
                let path = repo_root.join(&operation.path);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("creating {}", parent.display()))?;
                }
                let content = operation
                    .content
                    .as_ref()
                    .context("missing content for file operation")?;
                fs::write(&path, content.as_ref())
                    .with_context(|| format!("writing {}", operation.path))?;
            }
            Action::Create | Action::Overwrite => {}
        }
    }

    Ok(())
}

fn write_dry_run<W: Write>(stdout: &mut W, operations: &[Operation]) -> Result<()> {
    writeln!(stdout, "Planned operations (no files will be written):")?;
    writeln!(stdout)?;
    for operation in operations {
        writeln!(stdout, "{}", format_operation(operation))?;
    }
    writeln!(stdout)?;
    writeln!(stdout, "Run without --dry-run to apply.")?;
    Ok(())
}

fn write_applied<W: Write>(stdout: &mut W, operations: &[Operation]) -> Result<()> {
    for operation in operations {
        writeln!(stdout, "{}", format_operation(operation))?;
    }
    Ok(())
}

fn format_operation(operation: &Operation) -> String {
    let path = if operation.ownership == Ownership::Dir {
        format!("{}/", operation.path)
    } else {
        operation.path.clone()
    };
    match operation.note {
        Some(note) => format!(
            "  {:<10} {:<10} {}  ({note})",
            operation.ownership.tag(),
            operation.action.label(),
            path
        ),
        None => format!(
            "  {:<10} {:<10} {}",
            operation.ownership.tag(),
            operation.action.label(),
            path
        ),
    }
}

fn find_existing_marker(repo_root: &Path) -> Option<&'static str> {
    STANDARD_PATHS
        .iter()
        .find(|path| repo_root.join(path).exists())
        .copied()
}

fn write_existing_repo_error<E: Write>(stderr: &mut E, marker: &str) -> Result<()> {
    writeln!(stderr, "[fail] {marker}")?;
    writeln!(
        stderr,
        "       specmate-managed structure already exists in this repo"
    )?;
    writeln!(stderr, "       -> Re-run with: specmate init --merge")?;
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/cmd/init_test.rs"]
mod init_test;

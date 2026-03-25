use super::check_support::{create_status_repo, temp_repo, write_file};
use super::{run_in_repo, ColorWhen, Palette, StatusArgs};
use clap::{error::ErrorKind, CommandFactory, Parser};

#[derive(Debug, Parser)]
struct RootCli {
    #[command(subcommand)]
    command: crate::cmd::Commands,
}

fn run_status(
    dir: &tempfile::TempDir,
    doc_id: Option<&str>,
    all: bool,
    color: ColorWhen,
) -> (anyhow::Result<()>, String, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let result = run_in_repo(
        dir.path(),
        StatusArgs {
            doc_id: doc_id.map(ToOwned::to_owned),
            all,
            color,
        },
        Palette::new(color, false),
        &mut stdout,
        &mut stderr,
    );
    (
        result,
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        String::from_utf8(stderr).expect("stderr should be utf-8"),
    )
}

#[test]
fn test_status_help_describes_command_surface() {
    let mut command = RootCli::command();
    let status = command
        .find_subcommand_mut("status")
        .expect("status subcommand should exist");
    let mut help = Vec::new();
    status
        .write_long_help(&mut help)
        .expect("help should render");
    let help = String::from_utf8(help).expect("help should be utf-8");

    assert!(help.contains("specmate status"));
    assert!(help.contains("--all"));
    assert!(help.contains("design-auth-system") || help.contains("design-auth"));
    assert!(help.contains("[doc_id]") || help.contains("[DOC_ID]"));
}

#[test]
fn test_status_dashboard_reports_repository_overview() {
    let dir = temp_repo();
    create_status_repo(dir.path());
    write_file(
        dir.path(),
        "docs/design/draft/design-draft-experiment.md",
        "---\nid: design-draft-experiment\ntitle: \"Draft Experiment\"\nstatus: draft\ncreated: 2026-03-25\n---\n\n# Design\n",
    );
    write_file(
        dir.path(),
        "docs/design/candidate/design-status-command-patch-01-tighten-copy.md",
        "---\nid: design-status-command-patch-01-tighten-copy\ntitle: \"Tighten Copy\"\nstatus: candidate\ncreated: 2026-03-25\nparent: design-status-command\n---\n\n# Patch\n",
    );

    let (result, stdout, stderr) = run_status(&dir, None, false, ColorWhen::Never);

    assert!(result.is_ok(), "status failed: {stderr}");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    assert!(stdout.contains("Repository Health"));
    assert!(stdout.contains("Design Overview"));
    assert!(stdout.contains("Execution Overview"));
    assert!(stdout.contains("Status Totals"));
    assert!(stdout.contains("design-draft-experiment  Draft Experiment  draft"));
    assert!(stdout.contains("design-status-command  Status Command  candidate"));
    assert!(stdout.contains("design-status-command-patch-01-tighten-copy  Tighten Copy  candidate"));
    assert!(stdout.contains("design-core-platform  Core Platform  implemented"));
    assert!(stdout.contains("exec-status-rollout  Status Rollout"));
    assert!(stdout.contains("exec-status-follow-up  Status Follow Up"));
    assert!(stdout.contains("exec-status-rollout/task-01  Implement status dashboard"));
    assert!(!stdout.contains("All Documents"));
}

#[test]
fn test_status_dashboard_surfaces_invalid_repository_issues() {
    let dir = temp_repo();
    create_status_repo(dir.path());
    write_file(
        dir.path(),
        "docs/design/draft/not-a-design.md",
        "---\nid: design-bad\ntitle: \"Bad\"\nstatus: draft\ncreated: 2026-03-25\n---\n\n# Broken\n",
    );
    write_file(
        dir.path(),
        "docs/exec-plans/exec-broken/plan.md",
        "---\nid: exec-broken\ntitle: \"Broken\"\nstatus: candidate\ncreated: 2026-03-25\ndesign-docs:\n  - design-missing\n---\n\n# Exec\n",
    );

    let (result, stdout, stderr) = run_status(&dir, None, false, ColorWhen::Never);

    assert!(result.is_ok(), "status failed: {stderr}");
    assert!(stdout.contains("invalid managed entries: 1"));
    assert!(stdout.contains("repository validation violations: 1"));
}

#[test]
fn test_status_detail_for_design_doc_reports_relationships() {
    let dir = temp_repo();
    create_status_repo(dir.path());

    let (result, stdout, stderr) =
        run_status(&dir, Some("design-status-command"), false, ColorWhen::Never);

    assert!(result.is_ok(), "status failed: {stderr}");
    assert!(stdout.contains("Overview"));
    assert!(stdout.contains("id: design-status-command"));
    assert!(stdout.contains("type: DesignDoc"));
    assert!(stdout.contains("status: candidate"));
    assert!(stdout.contains("Upstream References"));
    assert!(stdout.contains("prd: prd-core-platform"));
    assert!(stdout.contains("Downstream Associations"));
    assert!(stdout.contains(
        "exec plans: exec-status-follow-up (candidate), exec-status-rollout (candidate)"
    ));
    assert!(stdout.contains("Related Repository Warnings"));
}

#[test]
fn test_status_detail_for_task_spec_reports_exec_lineage() {
    let dir = temp_repo();
    create_status_repo(dir.path());

    let (result, stdout, stderr) = run_status(
        &dir,
        Some("exec-status-rollout/task-01"),
        false,
        ColorWhen::Never,
    );

    assert!(result.is_ok(), "status failed: {stderr}");
    assert!(stdout.contains("id: exec-status-rollout/task-01"));
    assert!(stdout.contains("type: TaskSpec"));
    assert!(stdout.contains("exec-plan: exec-status-rollout"));
    assert!(stdout.contains("Related Repository Warnings"));
}

#[test]
fn test_status_detail_surfaces_related_reference_warnings() {
    let dir = temp_repo();
    create_status_repo(dir.path());
    write_file(
        dir.path(),
        "docs/design/candidate/design-broken-link.md",
        "---\nid: design-broken-link\ntitle: \"Broken Link\"\nstatus: candidate\ncreated: 2026-03-25\nprd: prd-missing\n---\n\n# Design\n",
    );

    let (result, stdout, stderr) =
        run_status(&dir, Some("design-broken-link"), false, ColorWhen::Never);

    assert!(result.is_ok(), "status failed: {stderr}");
    assert!(stdout.contains("prd: prd-missing"));
    assert!(stdout.contains("Related Repository Warnings"));
    assert!(stdout.contains("prd prd-missing does not exist"));
}

#[test]
fn test_status_rejects_unknown_or_unsupported_lookup_targets() {
    let dir = temp_repo();
    create_status_repo(dir.path());

    let (result, stdout, stderr) =
        run_status(&dir, Some("design-missing"), false, ColorWhen::Never);
    assert!(result.is_err(), "unknown id should fail");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.contains("[fail] status"));
    assert!(stderr.contains("managed document design-missing does not exist"));

    let (result, stdout, stderr) = run_status(&dir, Some("specmate"), false, ColorWhen::Never);
    assert!(result.is_err(), "guideline-like slug should fail");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.contains("guideline lookup target specmate is not supported"));
}

#[test]
fn test_status_all_lists_historical_and_inactive_documents() {
    let dir = temp_repo();
    create_status_repo(dir.path());
    write_file(
        dir.path(),
        "docs/design/draft/design-draft-experiment.md",
        "---\nid: design-draft-experiment\ntitle: \"Draft Experiment\"\nstatus: draft\ncreated: 2026-03-25\n---\n\n# Design\n",
    );
    write_file(
        dir.path(),
        "docs/design/obsolete/design-old-direction.md",
        "---\nid: design-old-direction\ntitle: \"Old Direction\"\nstatus: obsolete\ncreated: 2026-03-25\n---\n\n# Design\n",
    );

    let (result, stdout, stderr) = run_status(&dir, None, true, ColorWhen::Never);

    assert!(result.is_ok(), "status failed: {stderr}");
    assert!(stdout.contains("All Documents"));
    assert!(stdout.contains("design-draft-experiment  DesignDoc  draft"));
    assert!(stdout.contains("design-old-direction  DesignDoc  obsolete"));
    assert!(stdout.contains("exec-core-rollout  ExecPlan  closed"));
    assert!(stdout.contains("exec-core-rollout/task-01  TaskSpec  closed"));
}

#[test]
fn test_status_requires_no_extra_arguments() {
    let error = RootCli::try_parse_from(["specmate", "status", "design-auth", "extra"])
        .expect_err("parse should fail with extra argument");

    assert_eq!(error.kind(), ErrorKind::UnknownArgument);
}

#[test]
fn test_status_color_always_adds_ansi_without_changing_text_content() {
    let dir = temp_repo();
    create_status_repo(dir.path());

    let (dashboard_result, dashboard_stdout, dashboard_stderr) =
        run_status(&dir, None, false, ColorWhen::Always);
    assert!(
        dashboard_result.is_ok(),
        "status failed: {dashboard_stderr}"
    );
    assert!(dashboard_stdout.contains("\u{1b}[36mdraft\u{1b}[0m"));
    assert!(dashboard_stdout.contains("\u{1b}[33mcandidate\u{1b}[0m"));
    assert!(dashboard_stdout.contains("\u{1b}[32mimplemented\u{1b}[0m"));

    let (result, stdout, stderr) = run_status(
        &dir,
        Some("design-status-command"),
        false,
        ColorWhen::Always,
    );

    assert!(result.is_ok(), "status failed: {stderr}");
    assert!(stdout.contains("\u{1b}["));
    assert!(stdout.contains("Overview"));
    assert!(stdout.contains("status: "));
    assert!(stdout.contains("candidate"));
    assert!(stdout.contains("\u{1b}[1mdesign-status-command\u{1b}[0m"));
}

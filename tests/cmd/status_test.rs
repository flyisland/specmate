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
    assert!(help.contains("design-008"));
    assert!(help.contains("[doc_id]") || help.contains("[DOC_ID]"));
}

#[test]
fn test_status_dashboard_reports_repository_overview() {
    let dir = temp_repo();
    create_status_repo(dir.path());
    write_file(
        dir.path(),
        "docs/design-docs/draft/design-011-draft-experiment.md",
        "---\nid: design-011\ntitle: \"Draft Experiment\"\nstatus: draft\n---\n\n# Design\n",
    );

    let (result, stdout, stderr) = run_status(&dir, None, false, ColorWhen::Never);

    assert!(result.is_ok(), "status failed: {stderr}");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    assert!(stdout.contains("Repository Health"));
    assert!(stdout.contains("Design Overview"));
    assert!(stdout.contains("  draft"));
    assert!(stdout.contains("design-011  Draft Experiment  draft"));
    assert!(stdout.contains("Execution Overview"));
    assert!(stdout.contains("Status Totals"));
    assert!(stdout.contains("design-002"));
    assert!(stdout.contains("design-010"));
    assert!(stdout.contains("draft=1"));
    assert!(!stdout.contains("All Documents"));
    assert!(stdout.contains("design-001"));
    assert!(stdout.contains("exec-002"));
    assert!(stdout.contains("task-0002"));
}

#[test]
fn test_status_dashboard_sorts_rows_deterministically() {
    let dir = temp_repo();
    create_status_repo(dir.path());

    let (result, stdout, stderr) = run_status(&dir, None, false, ColorWhen::Never);

    assert!(result.is_ok(), "status failed: {stderr}");
    let design_002 = stdout.find("design-002").expect("design-002 should exist");
    let design_010 = stdout.find("design-010").expect("design-010 should exist");
    let exec_002 = stdout.find("exec-002").expect("exec-002 should exist");
    let exec_010 = stdout.find("exec-010").expect("exec-010 should exist");
    let task_0002 = stdout.find("task-0002").expect("task-0002 should exist");
    let task_0010 = stdout.find("task-0010").expect("task-0010 should exist");

    assert!(
        design_002 < design_010,
        "candidate designs should sort by id: {stdout}"
    );
    assert!(
        exec_002 < exec_010,
        "active exec plans should sort by id: {stdout}"
    );
    assert!(
        task_0002 < task_0010,
        "active task specs should sort by id: {stdout}"
    );
}

#[test]
fn test_status_detail_for_design_doc_reports_relationships() {
    let dir = temp_repo();
    create_status_repo(dir.path());

    let (result, stdout, stderr) = run_status(&dir, Some("design-002"), false, ColorWhen::Never);

    assert!(result.is_ok(), "status failed: {stderr}");
    assert!(stdout.contains("Overview"));
    assert!(stdout.contains("id: design-002"));
    assert!(stdout.contains("type: DesignDoc"));
    assert!(stdout.contains("Upstream References"));
    assert!(stdout.contains("prd: prd-001 (approved)"));
    assert!(stdout.contains("design-doc: design-001 (implemented)"));
    assert!(stdout.contains("Downstream Associations"));
    assert!(stdout.contains("exec plans"));
    assert!(stdout.contains("exec-002 (active)"));
    assert!(stdout.contains("exec-010 (active)"));
    assert!(stdout.contains("Derived Chain Summary"));
    assert!(stdout.contains("exec plans: 2"));
}

#[test]
fn test_status_detail_for_task_spec_reports_lineage() {
    let dir = temp_repo();
    create_status_repo(dir.path());

    let (result, stdout, stderr) = run_status(&dir, Some("task-0002"), false, ColorWhen::Never);

    assert!(result.is_ok(), "status failed: {stderr}");
    assert!(stdout.contains("id: task-0002"));
    assert!(stdout.contains("exec-plan: exec-002 (active)"));
    assert!(stdout.contains("exec-plan lineage: exec-002 -> design-002"));
    assert!(stdout.contains("No related warnings."));
}

#[test]
fn test_status_detail_surfaces_unresolved_references_and_related_warnings() {
    let dir = temp_repo();
    create_status_repo(dir.path());
    write_file(
        dir.path(),
        "docs/design-docs/candidate/design-011-broken-link.md",
        "---\nid: design-011\ntitle: \"Broken Link\"\nstatus: candidate\nprd: prd-999\n---\n\n# Design\n",
    );

    let (result, stdout, stderr) = run_status(&dir, Some("design-011"), false, ColorWhen::Never);

    assert!(result.is_ok(), "status failed: {stderr}");
    assert!(stdout.contains("prd: prd-999 (unresolved)"));
    assert!(stdout.contains("Related Repository Warnings"));
    assert!(stdout.contains("prd prd-999 does not exist"));
}

#[test]
fn test_status_dashboard_surfaces_invalid_repository_issues() {
    let dir = temp_repo();
    create_status_repo(dir.path());
    write_file(
        dir.path(),
        "docs/design-docs/draft/not-a-design.md",
        "---\nid: design-999\ntitle: \"Bad\"\nstatus: draft\n---\n\n# Broken\n",
    );
    write_file(
        dir.path(),
        "docs/exec-plans/active/exec-011-broken-link.md",
        "---\nid: exec-011\ntitle: \"Broken Link\"\nstatus: active\ndesign-doc: design-999\n---\n\n# Exec\n",
    );

    let (result, stdout, stderr) = run_status(&dir, None, false, ColorWhen::Never);

    assert!(result.is_ok(), "status failed: {stderr}");
    assert!(stdout.contains("invalid managed entries: 1"));
    assert!(stdout.contains("repository validation violations: 1"));
    assert!(stdout.contains("issue preview"));
    assert!(stdout.contains("not-a-design.md"));
    assert!(stdout.contains("design-doc design-999 does not exist"));
}

#[test]
fn test_status_rejects_unknown_or_unsupported_lookup_targets() {
    let dir = temp_repo();
    create_status_repo(dir.path());

    let (result, stdout, stderr) = run_status(&dir, Some("design-999"), false, ColorWhen::Never);
    assert!(result.is_err(), "unknown id should fail");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.contains("[fail] status"));
    assert!(stderr.contains("managed document design-999 does not exist"));

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
        "docs/design-docs/draft/design-011-draft-experiment.md",
        "---\nid: design-011\ntitle: \"Draft Experiment\"\nstatus: draft\n---\n\n# Design\n",
    );
    write_file(
        dir.path(),
        "docs/design-docs/obsolete/design-012-old-direction.md",
        "---\nid: design-012\ntitle: \"Old Direction\"\nstatus: obsolete\n---\n\n# Design\n",
    );

    let (result, stdout, stderr) = run_status(&dir, None, true, ColorWhen::Never);

    assert!(result.is_ok(), "status failed: {stderr}");
    assert!(stdout.contains("All Documents"));
    assert!(stdout.contains("  DesignDoc"));
    assert!(stdout.contains("design-011  draft  Draft Experiment"));
    assert!(stdout.contains("design-012  obsolete  Old Direction"));
    assert!(stdout.contains("exec-001  completed  Core Rollout"));
    assert!(stdout.contains("task-0011  cancelled  Cancelled status experiment"));
}

#[test]
fn test_status_requires_no_extra_arguments() {
    let error = RootCli::try_parse_from(["specmate", "status", "design-001", "extra"])
        .expect_err("parse should fail with extra argument");

    assert_eq!(error.kind(), ErrorKind::UnknownArgument);
}

#[test]
fn test_status_color_always_adds_ansi_without_changing_text_content() {
    let dir = temp_repo();
    create_status_repo(dir.path());

    let (result, stdout, stderr) = run_status(&dir, Some("design-002"), false, ColorWhen::Always);

    assert!(result.is_ok(), "status failed: {stderr}");
    assert!(stdout.contains("\u{1b}["));
    assert!(stdout.contains("Overview"));
    assert!(stdout.contains("status: "));
    assert!(stdout.contains("candidate"));
    assert!(stdout.contains("design-doc: design-001"));
}

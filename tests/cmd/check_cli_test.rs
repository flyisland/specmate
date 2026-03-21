use super::check_support::{create_compliant_repo, init_git_repo, temp_repo};
use super::{run_in_repo, BoundariesArgs, CheckArgs, CheckCommand};
use clap::{error::ErrorKind, CommandFactory, Parser};

#[derive(Debug, Parser)]
struct RootCli {
    #[command(subcommand)]
    command: crate::cmd::Commands,
}

fn run_check(
    dir: &tempfile::TempDir,
    command: Option<CheckCommand>,
) -> (anyhow::Result<()>, String, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let result = run_in_repo(dir.path(), CheckArgs { command }, &mut stdout, &mut stderr);
    (
        result,
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        String::from_utf8(stderr).expect("stderr should be utf-8"),
    )
}

#[test]
fn test_check_command_is_listed_in_root_help() {
    let mut command = RootCli::command();
    let mut help = Vec::new();
    command
        .write_long_help(&mut help)
        .expect("help should render");
    let help = String::from_utf8(help).expect("help should be utf-8");

    assert!(
        help.contains("check"),
        "root help should mention check: {help}"
    );
}

#[test]
fn test_check_help_describes_aggregate_and_named_modes() {
    let mut command = RootCli::command();
    let check = command
        .find_subcommand_mut("check")
        .expect("check subcommand should exist");
    let mut help = Vec::new();
    check
        .write_long_help(&mut help)
        .expect("help should render");
    let help = String::from_utf8(help).expect("help should be utf-8");

    assert!(help.contains("specmate check"));
    assert!(help.contains("boundaries"));
    assert!(help.contains("names"));
}

#[test]
fn test_check_boundaries_requires_task_id() {
    let error = RootCli::try_parse_from(["specmate", "check", "boundaries"])
        .expect_err("parse should fail without task id");

    assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument);
}

#[test]
fn test_check_command_dispatches_to_requested_mode() {
    let dir = temp_repo();
    create_compliant_repo(dir.path());

    let (result, stdout, stderr) = run_check(&dir, None);
    assert!(result.is_ok(), "aggregate check failed: {stderr}");
    assert!(stdout.contains("[pass] check names"));
    assert!(stdout.contains("[pass] check refs"));

    let (result, stdout, stderr) = run_check(&dir, Some(CheckCommand::Names));
    assert!(result.is_ok(), "named check failed: {stderr}");
    assert!(stdout.contains("[pass] check names"));

    init_git_repo(dir.path());
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn check_engine() { let _ = 1; }\n",
    )
    .expect("failed to modify allowed file");
    let (result, stdout, stderr) = run_check(
        &dir,
        Some(CheckCommand::Boundaries(BoundariesArgs {
            task_id: "task-0001".to_string(),
        })),
    );
    assert!(result.is_ok(), "boundaries check failed: {stderr}");
    assert!(stdout.contains("check boundaries task-0001"));
}

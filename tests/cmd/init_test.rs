// Tests for `specmate init`.
//
// This file is compiled as a unit-test module from `src/cmd/init.rs` so the
// task can keep its test edits within the declared boundary.
use super::{run_in_repo, InitArgs};
use crate::config::{Config, Lang};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn temp_repo() -> TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

fn args(lang: Option<Lang>, dry_run: bool, merge: bool) -> InitArgs {
    InitArgs {
        lang,
        dry_run,
        merge,
    }
}

fn run_init(dir: &TempDir, init_args: InitArgs) -> (anyhow::Result<()>, String, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let result = run_in_repo(dir.path(), init_args, &mut stdout, &mut stderr);
    (
        result,
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        String::from_utf8(stderr).expect("stderr should be utf-8"),
    )
}

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn assert_exists(path: &Path) {
    assert!(path.exists(), "expected {} to exist", path.display());
}

#[test]
fn test_init_creates_full_directory_structure() {
    let dir = temp_repo();

    let (result, stdout, stderr) = run_init(&dir, args(None, false, false));

    assert!(result.is_ok(), "init failed: {stderr}");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");

    for relative in [
        ".specmate",
        "specs",
        "specs/active",
        "specs/archived",
        "docs",
        "docs/guidelines",
        "docs/prd",
        "docs/prd/draft",
        "docs/prd/approved",
        "docs/prd/obsolete",
        "docs/design-docs",
        "docs/design-docs/draft",
        "docs/design-docs/candidate",
        "docs/design-docs/implemented",
        "docs/design-docs/obsolete",
        "docs/exec-plans",
        "docs/exec-plans/draft",
        "docs/exec-plans/active",
        "docs/exec-plans/archived",
    ] {
        assert_exists(&dir.path().join(relative));
        assert!(
            dir.path().join(relative).is_dir(),
            "{relative} should be a directory"
        );
    }

    for relative in [
        "AGENTS.md",
        ".specmate/config.yaml",
        "specs/README.md",
        "specs/project.md",
        "specs/org.md",
        "specs/active/README.md",
        "specs/archived/README.md",
        "docs/prd/README.md",
        "docs/design-docs/README.md",
        "docs/exec-plans/README.md",
    ] {
        let path = dir.path().join(relative);
        assert_exists(&path);
        assert!(path.is_file(), "{relative} should be a file");
        assert!(
            !read(&path).trim().is_empty(),
            "{relative} should be non-empty"
        );
    }
}

#[test]
fn test_init_lang_zh_generates_chinese_content() {
    let dir = temp_repo();

    let (result, _, stderr) = run_init(&dir, args(Some(Lang::Zh), false, false));

    assert!(result.is_ok(), "init failed: {stderr}");
    assert!(read(&dir.path().join("specs/README.md")).contains("Task Spec 和项目级约束文档。"));
    assert!(read(&dir.path().join("AGENTS.md")).contains("Agent 入职文档"));
}

#[test]
fn test_init_lang_en_generates_english_content() {
    let dir = temp_repo();

    let (result, _, stderr) = run_init(&dir, args(Some(Lang::En), false, false));

    assert!(result.is_ok(), "init failed: {stderr}");
    assert!(read(&dir.path().join("specs/README.md"))
        .contains("Task Specs and project-level constraints."));
    assert!(read(&dir.path().join("AGENTS.md")).contains("Agent Onboarding"));
}

#[test]
fn test_init_default_lang_is_en() {
    let dir = temp_repo();

    let (result, _, stderr) = run_init(&dir, args(None, false, false));

    assert!(result.is_ok(), "init failed: {stderr}");
    assert!(read(&dir.path().join("specs/README.md"))
        .contains("Task Specs and project-level constraints."));
}

#[test]
fn test_init_existing_repo_errors_and_exits() {
    let dir = temp_repo();
    fs::create_dir_all(dir.path().join(".specmate")).unwrap();
    fs::write(dir.path().join(".specmate/config.yaml"), "lang: en\n").unwrap();

    let (result, stdout, stderr) = run_init(&dir, args(None, false, false));

    assert!(result.is_err(), "init should fail in an existing repo");
    assert!(stdout.is_empty(), "unexpected stdout: {stdout}");
    assert!(stderr.contains("[fail]"));
    assert!(stderr.contains("--merge"));
    assert!(
        !dir.path().join("specs").exists(),
        "init should not create new paths"
    );
}

#[test]
fn test_init_dry_run_no_files_written() {
    let dir = temp_repo();

    let (result, stdout, stderr) = run_init(&dir, args(None, true, false));

    assert!(result.is_ok(), "dry-run failed: {stderr}");
    assert!(stdout.contains("Planned operations (no files will be written):"));
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    assert_eq!(
        fs::read_dir(dir.path()).unwrap().count(),
        0,
        "dry-run should not create any files or directories"
    );
}

#[test]
fn test_init_dry_run_groups_output_by_ownership() {
    let dir = temp_repo();

    let (result, stdout, stderr) = run_init(&dir, args(None, true, false));

    assert!(result.is_ok(), "dry-run failed: {stderr}");
    assert!(stdout.contains("[specmate]"));
    assert!(stdout.contains("[user]"));
    assert!(stdout.contains("[dir]"));
    assert!(stdout
        .trim_end()
        .ends_with("Run without --dry-run to apply."));
}

#[test]
fn test_init_merge_overwrites_readmes() {
    let dir = temp_repo();
    let (result, _, stderr) = run_init(&dir, args(Some(Lang::En), false, false));
    assert!(result.is_ok(), "initial init failed: {stderr}");

    fs::write(dir.path().join("specs/README.md"), "custom readme\n").unwrap();
    fs::write(
        dir.path().join("docs/design-docs/README.md"),
        "custom design docs\n",
    )
    .unwrap();

    let (result, _, stderr) = run_init(&dir, args(Some(Lang::Zh), false, true));

    assert!(result.is_ok(), "merge failed: {stderr}");
    assert!(read(&dir.path().join("specs/README.md")).contains("Task Spec 和项目级约束文档。"));
    assert!(read(&dir.path().join("docs/design-docs/README.md"))
        .contains("描述系统如何构建的设计文档。"));
}

#[test]
fn test_init_merge_preserves_user_files() {
    let dir = temp_repo();
    fs::create_dir_all(dir.path().join("specs")).unwrap();
    fs::create_dir_all(dir.path().join(".specmate")).unwrap();
    fs::write(dir.path().join("AGENTS.md"), "custom agents\n").unwrap();
    fs::write(dir.path().join(".specmate/config.yaml"), "lang: en\n").unwrap();
    fs::write(dir.path().join("specs/project.md"), "custom project\n").unwrap();
    fs::write(dir.path().join("specs/org.md"), "custom org\n").unwrap();

    let (result, _, stderr) = run_init(&dir, args(Some(Lang::Zh), false, true));

    assert!(result.is_ok(), "merge failed: {stderr}");
    assert_eq!(read(&dir.path().join("AGENTS.md")), "custom agents\n");
    assert_eq!(
        read(&dir.path().join(".specmate/config.yaml")),
        "lang: en\n"
    );
    assert_eq!(
        read(&dir.path().join("specs/project.md")),
        "custom project\n"
    );
    assert_eq!(read(&dir.path().join("specs/org.md")), "custom org\n");
}

#[test]
fn test_init_merge_creates_missing_structure() {
    let dir = temp_repo();
    fs::create_dir_all(dir.path().join(".specmate")).unwrap();
    fs::create_dir_all(dir.path().join("specs")).unwrap();
    fs::write(dir.path().join(".specmate/config.yaml"), "lang: en\n").unwrap();

    let (result, _, stderr) = run_init(&dir, args(None, false, true));

    assert!(result.is_ok(), "merge failed: {stderr}");
    assert_exists(&dir.path().join("docs/guidelines"));
    assert_exists(&dir.path().join("docs/exec-plans/archived"));
    assert_exists(&dir.path().join("specs/active/README.md"));
    assert_exists(&dir.path().join("docs/prd/README.md"));
}

#[test]
fn test_init_generates_valid_config() {
    let dir = temp_repo();

    let (result, _, stderr) = run_init(&dir, args(Some(Lang::Zh), false, false));

    assert!(result.is_ok(), "init failed: {stderr}");
    let config: Config = serde_yaml::from_str(&read(&dir.path().join(".specmate/config.yaml")))
        .expect("config should parse");
    assert_eq!(config.lang, Lang::Zh);
}

#[test]
fn test_init_generates_agents_md() {
    let dir = temp_repo();

    let (result, _, stderr) = run_init(&dir, args(None, false, false));

    assert!(result.is_ok(), "init failed: {stderr}");
    let agents = read(&dir.path().join("AGENTS.md"));
    assert!(agents.contains("Agent Onboarding"));
    assert!(!agents.trim().is_empty());
}

#[test]
fn test_init_generates_spec_templates() {
    let dir = temp_repo();

    let (result, _, stderr) = run_init(&dir, args(None, false, false));

    assert!(result.is_ok(), "init failed: {stderr}");
    assert!(read(&dir.path().join("specs/project.md")).contains("id: project"));
    assert!(read(&dir.path().join("specs/org.md")).contains("id: org"));
}

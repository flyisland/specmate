/// Integration tests for `specmate init`.
///
/// These tests correspond directly to the completion_criteria in
/// specs/active/task-0001-implement-init-command.md.
///
/// Run all: cargo test --test init_test
/// Run one: cargo test --test init_test test_init_creates_full_directory_structure -- --exact
use std::fs;
use tempfile::TempDir;

/// Helper: run specmate init in a temp directory and return the dir.
fn temp_repo() -> TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

/// cc-001: Init succeeds in an empty directory.
///
/// All directories and README files must be created.
#[test]
fn test_init_creates_full_directory_structure() {
    let dir = temp_repo();
    // TODO: invoke specmate init and assert directory structure
    let _ = dir;
    todo!("implement after specmate init is built")
}

/// cc-002: --lang zh generates Chinese README files.
#[test]
fn test_init_lang_zh_generates_chinese_content() {
    let dir = temp_repo();
    let _ = dir;
    todo!("implement after specmate init is built")
}

/// cc-003: --lang en generates English README files.
#[test]
fn test_init_lang_en_generates_english_content() {
    let dir = temp_repo();
    let _ = dir;
    todo!("implement after specmate init is built")
}

/// cc-004: No --lang defaults to en.
#[test]
fn test_init_default_lang_is_en() {
    let dir = temp_repo();
    let _ = dir;
    todo!("implement after specmate init is built")
}

/// cc-005: Init in an existing repo without --merge exits with a warning.
#[test]
fn test_init_existing_repo_warns_and_exits() {
    let dir = temp_repo();
    // Create a marker file so the repo looks initialised
    fs::create_dir_all(dir.path().join(".specmate")).unwrap();
    fs::write(
        dir.path().join(".specmate/config.yaml"),
        "lang: en\n",
    )
    .unwrap();
    let _ = dir;
    todo!("implement after specmate init is built")
}

/// cc-006: --dry-run prints planned operations without writing files.
#[test]
fn test_init_dry_run_no_files_written() {
    let dir = temp_repo();
    let _ = dir;
    todo!("implement after specmate init is built")
}

/// cc-007: --dry-run output groups [specmate] and [user] owned operations.
#[test]
fn test_init_dry_run_groups_output_by_ownership() {
    let dir = temp_repo();
    let _ = dir;
    todo!("implement after specmate init is built")
}

/// cc-008: --merge silently overwrites specmate-owned README files.
#[test]
fn test_init_merge_overwrites_readmes() {
    let dir = temp_repo();
    let _ = dir;
    todo!("implement after specmate init is built")
}

/// cc-009: --merge never touches user-owned files.
#[test]
fn test_init_merge_preserves_user_files() {
    let dir = temp_repo();
    let _ = dir;
    todo!("implement after specmate init is built")
}

/// cc-010: --merge creates missing directories and files.
#[test]
fn test_init_merge_creates_missing_structure() {
    let dir = temp_repo();
    let _ = dir;
    todo!("implement after specmate init is built")
}

/// cc-011: Init generates a valid .specmate/config.yaml with a lang field.
#[test]
fn test_init_generates_valid_config() {
    let dir = temp_repo();
    let _ = dir;
    todo!("implement after specmate init is built")
}

/// cc-012: Init generates AGENTS.md at the repo root.
#[test]
fn test_init_generates_agents_md() {
    let dir = temp_repo();
    let _ = dir;
    todo!("implement after specmate init is built")
}

/// cc-013: Init generates project.md and org.md templates under specs/.
#[test]
fn test_init_generates_spec_templates() {
    let dir = temp_repo();
    let _ = dir;
    todo!("implement after specmate init is built")
}

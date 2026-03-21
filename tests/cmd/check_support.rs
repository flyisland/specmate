use git2::{IndexAddOption, Repository, Signature};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

pub fn temp_repo() -> TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

pub fn write_file(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("failed to create {}: {error}", parent.display()));
    }
    fs::write(&path, content)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
}

pub fn create_compliant_repo(root: &Path) {
    write_file(
        root,
        "specs/project.md",
        "---\nid: project\nstatus: active\n---\n\n# Project\n",
    );
    write_file(
        root,
        "docs/guidelines/specmate.md",
        "---\ntitle: \"Specmate\"\n---\n\n# Guideline\n",
    );
    write_file(
        root,
        "docs/prd/approved/prd-001-core-checks.md",
        "---\nid: prd-001\ntitle: \"Core Checks\"\nstatus: approved\n---\n\n# PRD\n",
    );
    write_file(
        root,
        "docs/design-docs/implemented/design-001-check-engine.md",
        "---\nid: design-001\ntitle: \"Check Engine\"\nstatus: implemented\nprd: prd-001\n---\n\n# Design\n",
    );
    write_file(
        root,
        "docs/exec-plans/active/exec-001-build-check-engine.md",
        "---\nid: exec-001\ntitle: \"Build Check Engine\"\nstatus: active\ndesign-doc: design-001\n---\n\n# Exec Plan\n",
    );
    write_file(
        root,
        "specs/active/task-0001-implement-check-engine.md",
        "---\nid: task-0001\ntitle: \"Implement check engine\"\nstatus: active\nexec-plan: exec-001\nguidelines:\n  - docs/guidelines/specmate.md\nboundaries:\n  allowed:\n    - \"src/lib.rs\"\n  forbidden_patterns:\n    - \"specs/**\"\ncompletion_criteria:\n  - id: \"cc-001\"\n    scenario: \"task passes\"\n    test: \"test_task\"\n---\n\n# Task\n",
    );
    write_file(root, "src/lib.rs", "pub fn check_engine() {}\n");
    write_file(root, "src/main.rs", "fn main() {}\n");
    write_file(root, "tests/sample.rs", "#[test]\nfn sample() {}\n");
}

pub fn init_git_repo(root: &Path) -> Repository {
    let repo = Repository::init(root)
        .unwrap_or_else(|error| panic!("failed to init repo {}: {error}", root.display()));
    commit_all(&repo, "initial commit");
    repo
}

pub fn commit_all(repo: &Repository, message: &str) {
    let mut index = repo.index().expect("failed to open index");
    index
        .add_all(["*"], IndexAddOption::DEFAULT, None)
        .expect("failed to add all");
    index.write().expect("failed to write index");
    let tree_id = index.write_tree().expect("failed to write tree");
    let tree = repo.find_tree(tree_id).expect("failed to load tree");
    let signature =
        Signature::now("Specmate", "specmate@example.com").expect("failed to create signature");

    let parent = repo
        .head()
        .ok()
        .and_then(|head| head.target())
        .and_then(|oid| repo.find_commit(oid).ok());

    match parent {
        Some(parent) => repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &[&parent],
            )
            .expect("failed to commit"),
        None => repo
            .commit(Some("HEAD"), &signature, &signature, message, &tree, &[])
            .expect("failed to create initial commit"),
    };
}

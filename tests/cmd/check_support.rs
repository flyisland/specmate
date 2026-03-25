#![allow(dead_code)]

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
        "docs/specs/project.md",
        "---\nid: project\nstatus: active\n---\n\n# Project\n",
    );
    write_file(
        root,
        "docs/specs/org.md",
        "---\nid: org\nstatus: active\n---\n\n# Org\n",
    );
    write_file(
        root,
        "docs/guidelines/specmate.md",
        "---\ntitle: \"Specmate\"\n---\n\n# Guideline\n",
    );
    write_file(
        root,
        "docs/prd/approved/prd-core-checks.md",
        "---\nid: prd-core-checks\ntitle: \"Core Checks\"\nstatus: approved\ncreated: 2026-03-25\n---\n\n# PRD\n",
    );
    write_file(
        root,
        "docs/design/implemented/design-check-engine.md",
        "---\nid: design-check-engine\ntitle: \"Check Engine\"\nstatus: implemented\ncreated: 2026-03-25\nprd: prd-core-checks\n---\n\n# Design\n",
    );
    write_file(
        root,
        "docs/exec-plans/exec-build-check-engine/plan.md",
        "---\nid: exec-build-check-engine\ntitle: \"Build Check Engine\"\nstatus: candidate\ncreated: 2026-03-25\ndesign-docs:\n  - design-check-engine\n---\n\n# Exec Plan\n",
    );
    write_file(
        root,
        "docs/exec-plans/exec-build-check-engine/task-01-implement-check-engine.md",
        "---\nid: task-01\ntitle: \"Implement check engine\"\nstatus: candidate\ncreated: 2026-03-25\nexec-plan: exec-build-check-engine\nboundaries:\n  allowed:\n    - \"src/lib.rs\"\n  forbidden_patterns:\n    - \"docs/prd/**\"\n    - \"docs/design/**\"\n    - \"docs/guidelines/**\"\n    - \"docs/specs/**\"\n    - \"docs/exec-plans/**\"\ncompletion_criteria:\n  - id: \"cc-001\"\n    scenario: \"task passes\"\n    test: \"test_task\"\n---\n\n# Task\n",
    );
    write_file(root, "src/lib.rs", "pub fn check_engine() {}\n");
    write_file(root, "src/main.rs", "fn main() {}\n");
    write_file(root, "tests/sample.rs", "#[test]\nfn sample() {}\n");
}

pub fn create_status_repo(root: &Path) {
    write_file(
        root,
        "docs/specs/project.md",
        "---\nid: project\nstatus: active\n---\n\n# Project\n",
    );
    write_file(
        root,
        "docs/specs/org.md",
        "---\nid: org\nstatus: active\n---\n\n# Org\n",
    );
    write_file(
        root,
        "docs/guidelines/specmate.md",
        "---\ntitle: \"Specmate\"\n---\n\n# Guideline\n",
    );
    write_file(
        root,
        "docs/prd/approved/prd-core-platform.md",
        "---\nid: prd-core-platform\ntitle: \"Core Platform\"\nstatus: approved\ncreated: 2026-03-25\n---\n\n# PRD\n",
    );
    write_file(
        root,
        "docs/design/implemented/design-core-platform.md",
        "---\nid: design-core-platform\ntitle: \"Core Platform\"\nstatus: implemented\ncreated: 2026-03-25\nprd: prd-core-platform\n---\n\n# Design\n",
    );
    write_file(
        root,
        "docs/design/candidate/design-status-command.md",
        "---\nid: design-status-command\ntitle: \"Status Command\"\nstatus: candidate\ncreated: 2026-03-25\nprd: prd-core-platform\n---\n\n# Design\n",
    );
    write_file(
        root,
        "docs/design/candidate/design-future-roadmap.md",
        "---\nid: design-future-roadmap\ntitle: \"Future Roadmap\"\nstatus: candidate\ncreated: 2026-03-25\n---\n\n# Design\n",
    );
    write_file(
        root,
        "docs/design/obsolete/design-core-platform-patch-01-fix-links.md",
        "---\nid: design-core-platform-patch-01-fix-links\ntitle: \"Fix Links\"\nstatus: obsolete\ncreated: 2026-03-25\nparent: design-core-platform\n---\n\n# Patch\n",
    );
    write_file(
        root,
        "docs/exec-plans/exec-core-rollout/plan.md",
        "---\nid: exec-core-rollout\ntitle: \"Core Rollout\"\nstatus: closed\ncreated: 2026-03-25\nclosed: 2026-03-25\ndesign-docs:\n  - design-core-platform\n---\n\n# Exec Plan\n",
    );
    write_file(
        root,
        "docs/exec-plans/exec-status-rollout/plan.md",
        "---\nid: exec-status-rollout\ntitle: \"Status Rollout\"\nstatus: candidate\ncreated: 2026-03-25\ndesign-docs:\n  - design-status-command\n---\n\n# Exec Plan\n",
    );
    write_file(
        root,
        "docs/exec-plans/exec-status-follow-up/plan.md",
        "---\nid: exec-status-follow-up\ntitle: \"Status Follow Up\"\nstatus: candidate\ncreated: 2026-03-25\ndesign-docs:\n  - design-status-command\n---\n\n# Exec Plan\n",
    );
    write_file(
        root,
        "docs/exec-plans/exec-core-rollout/task-01-complete-core-rollout.md",
        "---\nid: task-01\ntitle: \"Complete core rollout\"\nstatus: closed\ncreated: 2026-03-25\nclosed: 2026-03-25\nexec-plan: exec-core-rollout\nboundaries:\n  allowed:\n    - \"src/core.rs\"\n  forbidden_patterns:\n    - \"docs/prd/**\"\n    - \"docs/design/**\"\n    - \"docs/guidelines/**\"\n    - \"docs/specs/**\"\n    - \"docs/exec-plans/**\"\ncompletion_criteria:\n  - id: \"cc-001\"\n    scenario: \"done\"\n    test: \"test_done\"\n---\n\n# Task\n",
    );
    write_file(
        root,
        "docs/exec-plans/exec-status-rollout/task-01-implement-status-dashboard.md",
        "---\nid: task-01\ntitle: \"Implement status dashboard\"\nstatus: candidate\ncreated: 2026-03-25\nexec-plan: exec-status-rollout\nboundaries:\n  allowed:\n    - \"src/status_dashboard.rs\"\n  forbidden_patterns:\n    - \"docs/prd/**\"\n    - \"docs/design/**\"\n    - \"docs/guidelines/**\"\n    - \"docs/specs/**\"\n    - \"docs/exec-plans/**\"\ncompletion_criteria:\n  - id: \"cc-001\"\n    scenario: \"dashboard\"\n    test: \"test_dashboard\"\n---\n\n# Task\n",
    );
    write_file(
        root,
        "docs/exec-plans/exec-status-follow-up/task-01-implement-status-follow-up.md",
        "---\nid: task-01\ntitle: \"Implement status follow up\"\nstatus: candidate\ncreated: 2026-03-25\nexec-plan: exec-status-follow-up\nboundaries:\n  allowed:\n    - \"src/status_follow_up.rs\"\n  forbidden_patterns:\n    - \"docs/prd/**\"\n    - \"docs/design/**\"\n    - \"docs/guidelines/**\"\n    - \"docs/specs/**\"\n    - \"docs/exec-plans/**\"\ncompletion_criteria:\n  - id: \"cc-001\"\n    scenario: \"follow up\"\n    test: \"test_follow_up\"\n---\n\n# Task\n",
    );
    write_file(root, "src/lib.rs", "pub fn status() {}\n");
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

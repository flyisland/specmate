---
id: task-03
title: "Implement check boundaries"
status: closed
created: 2026-03-25
closed: 2026-03-25
exec-plan: exec-implement-check-engine
guidelines:
  - docs/guidelines/specmate-principles.md
  - docs/guidelines/cli-conventions.md
boundaries:
  allowed:
    - "src/cmd/check.rs"
    - "src/check/**"
    - "src/error.rs"
    - "tests/cmd/check_boundaries_test.rs"
  forbidden_patterns:
    - "specs/**"
completion_criteria:
  - id: "cc-001"
    scenario: "`specmate check boundaries <task-id>` passes when all changed files are within allowed boundaries."
    test: "test_check_boundaries_passes_for_allowed_changes"
  - id: "cc-002"
    scenario: "Files outside `boundaries.allowed` fail with actionable output listing the allowed patterns."
    test: "test_check_boundaries_reports_files_outside_allowed_patterns"
  - id: "cc-003"
    scenario: "Files matching `forbidden_patterns` fail even when they also match an allowed pattern."
    test: "test_check_boundaries_reports_forbidden_pattern_matches"
  - id: "cc-004"
    scenario: "Missing task ids or non-Task-Spec targets fail with a clear error."
    test: "test_check_boundaries_rejects_missing_or_invalid_task_id"
---

# Intent

Implement `specmate check boundaries <task-id>` so the current working tree and
staged area can be validated against a Task Spec's allowed and forbidden
patterns.

This task isolates git-backed path collection from the repository-index checks
so the changed-file semantics remain explicit and testable.

# Boundaries

- `src/cmd/check.rs`
- `src/check/**`
- `src/error.rs`
- `tests/cmd/check_boundaries_test.rs`

# Completion criteria

- `cc-001` through `cc-004` all pass in the current codebase.

# Design constraints

- Use `git2`, not a system git binary.
- Treat staged and unstaged changes against `HEAD` as in-scope changed paths.
- Keep the command read-only.
- Preserve the same output contract as the other checks.

# Outcome

The git-backed boundary check is implemented and verified.

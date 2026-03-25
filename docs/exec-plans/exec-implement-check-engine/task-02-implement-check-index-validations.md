---
id: task-02
title: "Implement check index validations"
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
    - "src/doc/**"
    - "src/error.rs"
    - "tests/cmd/check_index_test.rs"
    - "tests/doc_model_test.rs"
  forbidden_patterns:
    - "specs/**"
completion_criteria:
  - id: "cc-001"
    scenario: "`specmate check names` reports invalid managed filenames and passes compliant repositories."
    test: "test_check_names_reports_invalid_managed_filenames"
  - id: "cc-002"
    scenario: "`specmate check frontmatter` reports missing or invalid frontmatter fields with actionable output."
    test: "test_check_frontmatter_reports_invalid_frontmatter"
  - id: "cc-003"
    scenario: "`specmate check status` reports directory and status mismatches using move-oriented fix text."
    test: "test_check_status_reports_directory_mismatches"
  - id: "cc-004"
    scenario: "`specmate check refs` reports stale references to obsolete or invalid documents."
    test: "test_check_refs_reports_stale_references"
  - id: "cc-005"
    scenario: "`specmate check conflicts` reports overlapping task boundaries among active or draft task specs."
    test: "test_check_conflicts_reports_overlapping_boundaries"
  - id: "cc-006"
    scenario: "`specmate check` aggregates pass/fail output across all index-backed checks."
    test: "test_check_aggregates_index_check_results"
---

# Intent

Implement the repository-index-backed checks from `design-004`:

- `check names`
- `check frontmatter`
- `check status`
- `check refs`
- `check conflicts`

This task should build on the command surface from `task-0002` and reuse the
document model as the single source of truth.

# Boundaries

- `src/cmd/check.rs`
- `src/check/**`
- `src/doc/**`
- `src/error.rs`
- `tests/cmd/check_index_test.rs`
- `tests/doc_model_test.rs`

# Completion criteria

- `cc-001` through `cc-006` all pass in the current codebase.

# Design constraints

- Reuse `build_index`, `validate_index`, and `expected_directory`.
- Do not duplicate document parsing or repository scanning in the check layer.
- Each violation must include the path, violated rule, and a concrete fix.
- Keep the checks pure reads.

# Outcome

The repository-index-backed checks are implemented and verified.

---
id: task-0007
title: "Implement status command"
status: completed
exec-plan: exec-002
guidelines:
  - docs/guidelines/specmate-principles.md
  - docs/guidelines/cli-conventions.md
boundaries:
  allowed:
    - "docs/design-docs/candidate/design-008-status-command.md"
    - "src/cmd/mod.rs"
    - "src/cmd/status.rs"
    - "tests/cmd/check_support.rs"
    - "tests/cmd/status_test.rs"
  forbidden_patterns:
    - "specs/**"
completion_criteria:
  - id: "cc-001"
    scenario: "`specmate status --help` exposes the subcommand, optional `doc-id`, and at least one usage example."
    test: "test_status_help_describes_command_surface"
  - id: "cc-002"
    scenario: "`specmate status` renders repository health, design overview, execution overview, and status totals for a compliant repository."
    test: "test_status_dashboard_reports_repository_overview"
  - id: "cc-003"
    scenario: "Dashboard rows are rendered in deterministic canonical-id order within each section so repeated runs stay stable."
    test: "test_status_dashboard_sorts_rows_deterministically"
  - id: "cc-004"
    scenario: "`specmate status <doc-id>` for a Design Doc renders overview, upstream references, downstream associations, and derived chain counts."
    test: "test_status_detail_for_design_doc_reports_relationships"
  - id: "cc-005"
    scenario: "`specmate status <doc-id>` for a Task Spec renders exec-plan lineage, derived Design Doc context, and no-warning output when nothing is related."
    test: "test_status_detail_for_task_spec_reports_lineage"
  - id: "cc-006"
    scenario: "If a referenced upstream target is unresolved or stale, the detail view still renders the reference and marks it unresolved while surfacing related warnings."
    test: "test_status_detail_surfaces_unresolved_references_and_related_warnings"
  - id: "cc-007"
    scenario: "Non-compliant repositories still render the dashboard with invalid-entry and validation-issue previews instead of failing closed."
    test: "test_status_dashboard_surfaces_invalid_repository_issues"
  - id: "cc-008"
    scenario: "Unknown document IDs and unsupported guideline-style lookup targets fail with actionable exit-code-1 errors."
    test: "test_status_rejects_unknown_or_unsupported_lookup_targets"
---

# Intent

Implement `specmate status` so the repository exposes a read-only status view
for both the overall docs/spec landscape and a specific managed document.

This task covers the CLI surface, view-model helpers, rendering logic, and
tests required by `design-008`.

# Boundaries

- `docs/design-docs/candidate/design-008-status-command.md`
- `src/cmd/mod.rs`
- `src/cmd/status.rs`
- `tests/cmd/check_support.rs`
- `tests/cmd/status_test.rs`

# Completion criteria

- `cc-001` through `cc-008` all pass in the current codebase.

# Design constraints

- Use the non-strict index build path so repository invalidity becomes visible
  output rather than a hard precondition failure.
- Keep `doc-id` resolution strict and canonical; do not add fuzzy lookup by
  path, slug, or title.
- Reuse the existing public document-model surface for reference inspection,
  association summaries, lifecycle-state facts, and any other repository facts
  the command needs.
- Keep this task out of `src/doc/**` and `tests/doc_model_test.rs` while
  `task-0005` remains active there.
- Preserve deterministic ordering and stable section presence exactly as
  defined by `design-008`.
- Keep warning expansion shallow: current document plus one-hop upstream and
  downstream relationships are sufficient in v1.
- This task does not add `--json`, filtering flags, or any write-path behavior.

# Outcome

`specmate status` is implemented as a read-only observability command and is
verified for dashboard rendering, single-document detail rendering, invalid
repository visibility, and actionable failure behavior.

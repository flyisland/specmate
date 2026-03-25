---
id: task-01
title: "Implement move command"
status: closed
created: 2026-03-25
closed: 2026-03-25
exec-plan: exec-implement-move-command
guidelines:
  - docs/guidelines/specmate-principles.md
  - docs/guidelines/cli-conventions.md
boundaries:
  allowed:
    - "docs/design-docs/candidate/design-007-move-command.md"
    - "src/cmd/mod.rs"
    - "src/cmd/move_.rs"
    - "tests/cmd/check_support.rs"
    - "tests/cmd/move_test.rs"
  forbidden_patterns:
    - "specs/**"
completion_criteria:
  - id: "cc-001"
    scenario: "`specmate move --help` exposes the subcommand, arguments, `--dry-run`, and at least one usage example."
    test: "test_move_help_describes_command_surface"
  - id: "cc-002"
    scenario: "`specmate move <doc-id> <to-status> --dry-run` prints the planned update and move operations without modifying files."
    test: "test_move_dry_run_reports_operations_without_writing_files"
  - id: "cc-003"
    scenario: "A legal cross-directory move updates frontmatter status, preserves the filename, and relocates the file to the directory resolved for the target status."
    test: "test_move_applies_status_update_and_relocates_file"
  - id: "cc-004"
    scenario: "A legal same-directory move updates frontmatter in place and prints only the `UPDATE` line."
    test: "test_move_updates_in_place_when_directory_does_not_change"
  - id: "cc-005"
    scenario: "No-op requests, unsupported document types, and illegal status transitions fail with exit code `1` and actionable errors."
    test: "test_move_rejects_invalid_targets_and_illegal_transitions"
  - id: "cc-006"
    scenario: "The command fails before writing when the current repository is invalid, the predicted post-move repository would be invalid, or required target-status fields such as `merged-into` are missing."
    test: "test_move_fails_before_writing_on_preflight_or_preview_validation_errors"
  - id: "cc-007"
    scenario: "A Design Patch move to `obsolete:merged` accepts the target status string, requires a valid pre-existing `merged-into` value, and relocates the file into `docs/design-docs/obsolete/`."
    test: "test_move_applies_design_patch_merge_transition"
  - id: "cc-008"
    scenario: "Destination path collisions fail before writing and do not overwrite an existing file at the resolved target path."
    test: "test_move_rejects_destination_collisions_without_writing"
---

# Intent

Implement `specmate move` so one managed document can transition to a legal
target status and be relocated atomically to the directory implied by that
status.

This task covers the command surface, dry-run planning, pre-flight validation,
frontmatter rewriting, and filesystem apply path described by `design-007`.

# Boundaries

- `docs/design-docs/candidate/design-007-move-command.md`
- `src/cmd/mod.rs`
- `src/cmd/move_.rs`
- `tests/cmd/check_support.rs`
- `tests/cmd/move_test.rs`

# Completion criteria

- `cc-001` through `cc-008` all pass in the current codebase.

# Design constraints

- Reuse the shared document model for repository loading, transition validation,
  preview validation, directory resolution, and association summaries.
- Do not reimplement cross-document legality rules inside the command layer.
- Preserve CLI output conventions, including ownership-tagged lines and
  mandatory `--dry-run` behaviour.
- Treat destination collisions as hard errors and never overwrite an existing
  target file.
- Keep the write path atomic and prefer leaving the source file unchanged over
  risking a half-applied move.
- This task does not add cascading status changes or workflow automation.

# Outcome

`specmate move` is implemented as a safe command-layer wrapper over the shared
document model and is verified for dry-run, apply, and failure-path behaviour.

---
id: task-0002
title: "Implement check command surface"
status: completed
exec-plan: exec-001
guidelines:
  - docs/guidelines/specmate-principles.md
  - docs/guidelines/cli-conventions.md
boundaries:
  allowed:
    - "src/cmd/mod.rs"
    - "src/cmd/check.rs"
    - "src/lib.rs"
    - "tests/cmd/check_cli_test.rs"
  forbidden_patterns:
    - "specs/**"
completion_criteria:
  - id: "cc-001"
    scenario: "The CLI exposes `specmate check` as a top-level subcommand."
    test: "test_check_command_is_listed_in_root_help"
  - id: "cc-002"
    scenario: "`specmate check --help` documents aggregate mode and the named checks interface."
    test: "test_check_help_describes_aggregate_and_named_modes"
  - id: "cc-003"
    scenario: "`specmate check boundaries` requires a task id and reports bad input via clap."
    test: "test_check_boundaries_requires_task_id"
  - id: "cc-004"
    scenario: "CLI dispatch reaches the check command entrypoint for aggregate and named modes."
    test: "test_check_command_dispatches_to_requested_mode"
---

# Intent

Implement the CLI surface for `design-004` so `specmate check` exists as a
stable command family before repository validation logic is added.

This task delivered the command parsing, help text, and command dispatch layer
for `specmate check`. Concrete check behavior was implemented in the dependent
tasks under the same Exec Plan.

# Boundaries

Allowed changes:

- `src/cmd/mod.rs`
- `src/cmd/check.rs`
- `src/lib.rs`
- `tests/cmd/check_cli_test.rs`

Forbidden:

- `specs/**`
- any `src/doc/**` validation logic
- any git-backed changed-file inspection

# Required behavior

- `specmate check` is a valid command.
- `specmate check <name>` is supported for the check names defined by
  `design-004`.
- `specmate check boundaries <task-id>` parses as a dedicated form.
- Help output follows the command and option conventions in
  `docs/guidelines/cli-conventions.md`.
- The command layer exposes a stable entrypoint that later tasks can extend
  without changing the root CLI contract.

# Notes

- Aggregate execution may use placeholder pass output in this task, but the
  output shape must already match the long-term grouped pass/fail format.
- Any concrete repository validation belongs to later tasks in `exec-001`.

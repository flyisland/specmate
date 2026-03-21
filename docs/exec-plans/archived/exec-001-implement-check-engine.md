---
id: exec-001
title: "Implement Check Engine"
status: completed
design-doc: design-004
---

# Intent

Implement `design-004` so `specmate check` becomes the repository's
mechanical validation entrypoint and the required gate for later
`specmate run` work.

This plan covers the full check-engine slice:

- CLI command surface for `specmate check`
- reusable check-engine internals
- all checks defined in `design-004`
- actionable output and exit codes
- integration tests proving pass/fail behavior

The plan does not include `specmate run`, `specmate rerun`, `specmate move`,
or CI workflow files.

# Execution strategy

Build the feature in three Task Specs so the pure document-index checks land
before git-backed boundary checking.

1. Add the CLI surface and check-engine scaffolding.
2. Implement the repository-index-backed checks and aggregate output.
3. Implement `check boundaries` with `git2`, then finish integration coverage.

This ordering keeps the first two tasks pure-read and independent of working
tree state, while leaving the git-specific logic isolated to the final task.

# Completed Task Specs

## Task 1 — command surface and engine skeleton

Task Spec: `task-0002-implement-check-command-surface`

Goal:

- register `specmate check` in the CLI
- support `specmate check` and named subcommands
- introduce reusable check result types, aggregation, and output formatting
- establish the command/engine module split so later checks plug into the same path

Expected file scope:

- `src/cmd/mod.rs`
- `src/cmd/check.rs`
- `src/lib.rs`
- `tests/cmd/check_cli_test.rs`

Completion target:

- unknown subcommand handling remains clap-owned
- `specmate check` prints grouped results in the format required by
  `docs/guidelines/cli-conventions.md`
- exit code is `0` on all-pass and `1` on any failing check
- named checks dispatch only the requested check

## Task 2 — repository-index-backed checks

Task Spec: `task-0003-implement-check-index-validations`

Goal:

- implement `check names`
- implement `check frontmatter`
- implement `check status`
- implement `check refs`
- implement `check conflicts`

Expected file scope:

- `src/cmd/check.rs`
- `src/check/**`
- `src/doc/**` only if small shared helpers are required by the check engine
- `src/error.rs`
- `tests/cmd/check_index_test.rs`
- `tests/doc_model_test.rs` only if shared validation coverage needs extension

Implementation notes:

- reuse `build_index`, `validate_index`, and `expected_directory` from the
  document model instead of re-scanning or re-parsing in the check layer
- keep each check as an independent pure-read unit returning structured violations
- make actionable fix text part of the check result, not ad hoc CLI formatting

Completion target:

- invalid managed filenames are surfaced by `check names`
- frontmatter and repository-level validation errors are surfaced by the
  correct named checks
- `check conflicts` reports overlapping `boundaries.allowed` entries among
  `draft` and `active` Task Specs
- aggregate `specmate check` output matches the design's grouped pass/fail shape

## Task 3 — boundaries check and final hardening

Task Spec: `task-0004-implement-check-boundaries`

Goal:

- implement `check boundaries <task-id>`
- read changed paths from git through `git2`
- evaluate `allowed` and `forbidden_patterns` with `glob`
- finish end-to-end tests for mixed pass/fail repositories

Expected file scope:

- `src/cmd/check.rs`
- `src/check/**`
- `src/error.rs`
- `tests/cmd/check_boundaries_test.rs`

Implementation notes:

- changed files include working tree and staged changes against `HEAD`
- forbidden matches take precedence over allowed matches
- output must list the violating path and show the allowed patterns for repair
- command remains read-only even when inspecting git state

Completion target:

- clean repos pass `check boundaries`
- files outside `boundaries.allowed` fail with actionable output
- files matching `forbidden_patterns` fail even if also allowed
- missing task IDs or non-Task-Spec targets fail clearly

# Dependencies and order

Execution order:

1. `task-0002-implement-check-command-surface`
2. `task-0003-implement-check-index-validations`
3. `task-0004-implement-check-boundaries`

Dependency rules:

- `task-0003` depends on `task-0002` because it plugs concrete checks into the
  shared command and reporting surface.
- `task-0004` depends on `task-0002`; it may proceed after `task-0003` if the
  check-engine interfaces are stable, but the preferred order is after
  `task-0003` so aggregate behavior is already covered.
- No task in this plan may modify files under `specs/**`.

# Outcome

All three Task Specs are completed and the repository satisfies all of the
following:

- `specmate check`
- `specmate check names`
- `specmate check frontmatter`
- `specmate check status`
- `specmate check refs`
- `specmate check conflicts`
- `specmate check boundaries <task-id>`

All commands must conform to `docs/guidelines/cli-conventions.md`, all tests
must pass, and `cargo clippy -- -D warnings` must remain clean.

`design-004` is now ready to move from `candidate` to `implemented`.

# Risks and controls

- Risk: validation logic gets duplicated between `src/doc` and the check engine.
  Control: treat `src/doc` as the single source of truth for parsing,
  repository indexing, and directory resolution.
- Risk: `check boundaries` semantics drift from later `run` pre-flight needs.
  Control: expose a reusable changed-path collection helper that `run` can call later.
- Risk: output formatting becomes inconsistent across checks.
  Control: centralize pass/fail rendering in one formatter and test exact output.

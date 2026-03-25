---
id: exec-implement-status-command
title: "Implement Status Command"
status: closed
created: 2026-03-25
closed: 2026-03-25
design-docs:
  - design-status-command
---

# Intent

Implement `design-008` so developers and agents can inspect the current
repository document system through a read-only CLI entrypoint.

This plan covers the full v1 `specmate status` slice:

- root CLI registration for `specmate status`
- repository dashboard rendering
- single-document detail rendering
- warning and invalid-entry surfacing for non-compliant repositories
- integration and model-level tests for the supported views

This plan does not add:

- `--json` or filtering flags
- interactive or TUI output
- write-path repairs or workflow automation
- guideline lookup by slug

# Execution strategy

Build the feature in one Task Spec because the command is read-only and the
design is already explicit about view shape, ordering, and failure handling.

Implementation should proceed in this order:

1. Implement the `specmate status` command layer and its two render paths:
   repository dashboard and single-document detail view.
2. Reuse the existing public document-model APIs to derive exact ID lookup,
   direct reference rendering, association summaries, and one-hop lineage
   facts without widening the task into `src/doc/**`.
3. Add integration tests for compliant and non-compliant repositories, then
   harden actionable failure output for unresolved IDs and unsupported targets.

This order keeps the feature isolated to the command layer and avoids overlap
with the currently active document-model task.

# Active Task Spec

## Task 1 — implement status command

Task Spec: `task-0007-implement-status-command`

Goal:

- register `specmate status` in the CLI
- support both repository and single-document modes
- render direct references, direct associations, and required derived summaries
- surface related warnings without failing closed on repository invalidity

Expected file scope:

- `docs/design-docs/candidate/design-008-status-command.md`
- `src/cmd/mod.rs`
- `src/cmd/status.rs`
- `tests/cmd/check_support.rs`
- `tests/cmd/status_test.rs`

Implementation notes:

- use the non-strict repository index path; do not require
  `build_compliant_index()`
- keep `specmate status <doc-id>` on exact canonical ID matching only
- derive required status facts from the existing public document-model surface
  instead of widening into the shared model while `task-0005` is active
- preserve deterministic ordering and section presence exactly as required by
  `design-008`
- keep repository-wide warning expansion shallow; one-hop related warnings are
  sufficient in v1

Completion target:

- `specmate status --help` exposes the command and a usage example
- the dashboard shows health, design overview, execution overview, and status
  totals in a deterministic order
- detail views expose overview, upstream references, downstream associations,
  derived summaries, and related warnings
- non-compliant repositories still render status output instead of failing
- unknown or unsupported lookup targets fail with actionable errors

# Dependencies and order

Execution order:

1. `task-0007-implement-status-command`

Dependency rules:

- this task depends on `design-008` remaining the source of truth for output
  structure and failure handling
- no other active Task Spec currently owns the allowed file set below
- the task must not modify files under `specs/**`

# Outcome

`specmate status` becomes the repository-facing observability command for the
managed document system and supports both dashboard-style inspection and
focused per-document analysis.

`design-008` is ready to move from `candidate` to `implemented` after the task
completes and the implementation passes all completion-criteria tests.

# Risks and controls

- Risk: command code reimplements document-model rules ad hoc.
  Control: use only the existing public document-model surface in this task; if
  a new shared helper becomes unavoidable, stop and create a follow-up spec
  after the current active `src/doc/**` task completes.
- Risk: dashboard output grows too large or unstable for agents to consume.
  Control: keep current-focus rows explicit and historical state aggregated.
- Risk: invalid repositories cause the command to abort before rendering useful
  diagnosis.
  Control: use the non-strict index build path and present invalid entries and
  validation issues as visible warnings.

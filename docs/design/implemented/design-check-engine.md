---
id: design-check-engine
title: "Check Engine"
status: implemented
created: 2026-03-25
guidelines:
  - docs/guidelines/specmate-principles.md
  - docs/guidelines/cli-conventions.md
---

# Check Engine

This document defines the implemented behavior of `specmate check`. The check
engine is read-only: it loads the shared document index, runs named
validations, and renders actionable pass/fail output.

---

## 1. Command family

```bash
specmate check
specmate check names
specmate check frontmatter
specmate check status
specmate check refs
specmate check conflicts
specmate check boundaries <task-id>
```

`<task-id>` uses the canonical Task Spec form, for example:

```bash
specmate check boundaries exec-auth-rollout/task-01
```

---

## 2. Design principles

- Checks are pure reads. They never modify files or git state.
- Checks are composable. Aggregate mode reuses the same named checks.
- Every violation is actionable. Output includes path, problem, and repair.
- The document model is the source of truth. The check layer reuses shared
  parsing, indexing, and repository validation rules.

---

## 3. Implemented checks

### `check names`

Validates managed filenames and managed path shapes.

Rules include:

- PRD: `prd-<slug>.md`
- Design Doc: `design-<slug>.md`
- Design Patch: `design-<parent-slug>-patch-<nn>-<patch-slug>.md`
- Exec Plan: `docs/exec-plans/exec-<slug>/plan.md`
- Task Spec: `docs/exec-plans/exec-<slug>/task-<nn>-<slug>.md`

### `check frontmatter`

Validates per-document frontmatter contracts, including:

- required fields such as `id`, `title`, `status`, and `created`
- `closed` rules for Exec Plans and Task Specs
- patch-specific fields such as `parent` and `merged-into`
- candidate Task Spec runtime fields such as `boundaries` and
  `completion_criteria`

### `check status`

Validates that every managed document is located in the directory implied by
its current status and document type.

Example:

```text
[fail] check status        1 violation
       docs/design/candidate/design-auth-system.md
       expected docs/design/implemented
       -> Run: specmate move design-auth-system implemented
```

### `check refs`

Validates repository-level references and association rules from the shared
document model.

This includes:

- stale or missing PRD references
- stale or missing `exec-plan` references
- invalid `design-docs` entries on Exec Plans
- missing patch parents
- invalid guideline paths

It preserves the model's live-vs-historical distinction instead of applying a
simple "target must be non-terminal" shortcut.

### `check conflicts`

Detects overlapping `boundaries.allowed` patterns among candidate Task Specs.

Only candidate Task Specs participate, because they are the executable work
surface in the current model.

### `check boundaries <task-id>`

Loads the specified candidate Task Spec, reads changed paths from git, and
checks them against:

- `boundaries.forbidden_patterns`
- `boundaries.allowed`

Forbidden matches fail first. Non-forbidden paths must still match an allowed
pattern.

Example:

```text
[fail] check boundaries exec-auth-rollout/task-01  1 violation
       src/cmd/new.rs
       is not in boundaries.allowed for exec-auth-rollout/task-01
       -> Keep changes within the task scope. Allowed: src/cmd/init.rs, tests/cmd/init_test.rs
```

---

## 4. Aggregate rendering

`specmate check` runs:

1. `names`
2. `frontmatter`
3. `status`
4. `refs`
5. `conflicts`

Each result renders as:

- `[pass] <label> <summary>`
- `[fail] <label> <count> violation(s)` plus one or more detail blocks

If any named check fails, aggregate mode exits with code `1` and ends with:

```text
N check(s) failed. Fix violations before running specmate run.
```

If all checks pass, exit code is `0`.

---

## 5. Operational notes

- `check boundaries` fails if the target id is not a Task Spec.
- `check boundaries` also fails if the target Task Spec is not `candidate`.
- Invalid managed entries remain visible to `check names`, `check frontmatter`,
  and `check status`; they are not silently dropped.
- `check refs` and `check frontmatter` both reuse shared repository-level
  validation, but present the results through check-specific fixes.

This is the behavior the current implementation provides today.

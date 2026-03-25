---
id: design-move-command
title: "Move Command"
status: implemented
created: 2026-03-25
guidelines:
  - docs/guidelines/specmate-principles.md
  - docs/guidelines/cli-conventions.md
---

# Move Command

This document defines `specmate move` — the command that performs an allowed
status transition on a managed document and updates its file location to match
the target status.

`design-003` defines the shared document model that this command consumes:
directory resolution, transition validation, post-transition preview
validation, and association-summary queries.

This document owns the command surface and write-path behaviour for applying
those shared rules safely.

---

## 1. Design principles

**One command owns status transitions.** Status-managed documents must change
status through `specmate move`, not by editing frontmatter or moving files by
hand.

**Document-model rules stay centralised.** `specmate move` must call the
document-model loader, directory resolver, transition validator, and
preview / association-summary helpers from `design-003`. It must not
reimplement status legality or path mapping.

**Status update and relocation are one operation.** A successful move updates
frontmatter and file location together. The command must never leave a file
with the new status in the old directory or the old status in the new
directory.

**Fail before writing.** If the repository document state is invalid, the
target transition is illegal, or the destination path is not writable,
`specmate move` stops before changing anything.

**The command is mechanical, not semantic.** `specmate move` enforces
document-model invariants and filesystem consistency. It does not run tests,
invoke agents, or infer whether a human process requirement has been met.
Higher-level commands such as `specmate run` remain responsible for proving
that a document is ready to move to a semantically stronger status.

---

## 2. Command surface

```bash
specmate move <doc-id> <to-status> [--dry-run]
```

Examples:

```bash
specmate move exec-001 active
specmate move task-0007 completed
specmate move design-001 implemented --dry-run
```

Arguments:

- `<doc-id>`: canonical managed document ID such as `prd-001`, `design-004`,
  `design-004-patch-01`, `exec-001`, or `task-0007`
- `<to-status>`: target status string valid for the resolved document type

Options:

- `--dry-run`: show the planned frontmatter update and file relocation without
  writing files

`specmate move` must provide `--help` with at least one example in the output.

---

## 3. Supported document types

`specmate move` supports the managed document types that have explicit
lifecycles:

- PRD
- Design Doc
- Design Patch
- Exec Plan
- Task Spec

`specmate move` rejects these inputs with an error:

- `project`
- `org`
- Guideline IDs such as `specmate-principles`

Reason: those document types are always active and have no legal status
transitions.

---

## 4. Preconditions

Before planning or applying a move, `specmate move` must:

1. Locate the repository root and build the full document index.
2. Validate current repository steady-state compliance through the shared
   document model.
3. Resolve `<doc-id>` to exactly one valid managed document.
4. Parse `<to-status>` in the status vocabulary for that document type.
5. Validate the requested transition through the shared transition validator,
   including any transition-time gates defined by the document model.
6. Build the predicted post-move repository state and validate it through the
   shared document model.
7. Resolve the destination directory using the shared directory resolver.
8. Check whether the destination path would collide with an existing file.

If any of these checks fails, the command exits with code `1` and writes no
files.

Repository validation is mandatory even when the requested document itself is
valid. Commands that mutate managed document state must not operate on top of
an already-invalid document system.

---

## 5. Transition semantics

`specmate move` follows the transition table and association-aware transition
rules defined by `design-003`.

Command-specific rules:

- Moving to the same status is a no-op request and must fail clearly rather
  than silently succeeding.
- If the new status maps to the same directory as the old status, the command
  updates frontmatter in place without changing the filename.
- If the new status maps to a different directory, the command updates
  frontmatter and relocates the file to the destination directory while
  preserving the filename.
- `specmate move` never changes a document ID or filename slug.

`specmate move` does not perform cascading status changes. It applies the
requested transition to exactly one document.

Conditionally required fields remain enforced by the document model after the
status change. For example:

- `design patch -> obsolete:merged` requires `merged-into`
- `design doc -> obsolete` via supersession requires `superseded-by`

If the existing document content would be invalid in the target status,
`specmate move` fails before writing.

Semantic obligations described by document meaning are not re-proven here.
Examples:

- `task active -> completed` does not run completion-criteria tests
- `design candidate -> implemented` does not complete Exec Plans on its own
- a later bug-fix Task Spec against an already `implemented` Design Doc is not
  itself a repository violation

Those obligations must be satisfied by the caller before invoking the command.

When a move succeeds, the command may query the shared document model for
association summaries and render non-blocking informational hints. These hints
must never change any other document automatically and must never affect the
exit code.

Hint rules:

- hints report repository facts only; they do not recommend or execute another
  command
- hints list the exact related document IDs returned by the shared association
  summary
- hint eligibility is determined by the shared document model; this command only
  chooses how to render the returned facts

Examples:

- after `task -> completed`, the model may report that all Task Specs linked to
  the same Exec Plan are now `completed`
- after `exec -> completed`, the model may report that all Exec Plans linked to
  the same Design Doc are now `completed`
- after other supported moves, the model may report other association facts
  defined by the shared document model

Hints are advisory only. The user or a higher-level command remains
responsible for deciding whether any manual follow-up is semantically
appropriate.

---

## 6. Write model

The command produces an updated document body by rewriting only the frontmatter
fields required by the status change. All non-frontmatter content and field
ordering should be preserved whenever practical.

Write strategy:

1. Compute the updated document content in memory.
2. Write it to a temporary sibling path in the destination directory.
3. Atomically rename the temporary file into the final destination path.
4. Remove the original path if the destination differs from the source path.

If the source and destination path are the same, the command writes through a
temporary sibling file and atomically replaces the original file.

If any filesystem step fails, the command exits with code `1` and must not
report success. The implementation should prefer leaving the original file
unchanged over risking a half-applied transition.

`specmate move` never overwrites an existing user-owned file or another managed
document. Destination collisions are hard errors.

---

## 7. Output

### Success output

Applied moves print ownership-tagged lines:

```text
  [user] UPDATE    specs/active/task-0007-add-status-view.md  (status: active -> completed)
  [user] MOVE      specs/active/task-0007-add-status-view.md -> specs/archived/task-0007-add-status-view.md
```

When the directory does not change, only the `UPDATE` line is printed.

Optional advisory hints may follow a successful move:

```text
  [info] exec-001 related tasks are all completed: task-0005, task-0006, task-0007
  [info] design-001 related exec plans are all completed: exec-001, exec-002
```

### Dry-run output

Dry-run output begins with:

```text
Planned operations (no files will be written):
```

And ends with:

```text
Run without --dry-run to apply.
```

Example:

```text
Planned operations (no files will be written):
  [user] UPDATE    docs/exec-plans/draft/exec-001-implement-check-engine.md  (status: draft -> active)
  [user] MOVE      docs/exec-plans/draft/exec-001-implement-check-engine.md -> docs/exec-plans/active/exec-001-implement-check-engine.md

Run without --dry-run to apply.
```

### Error output

Errors must identify:

1. the requested document
2. the violated rule
3. the concrete next action

Example:

```text
[fail] task-0007 cannot move to completed
       requested transition active -> completed is legal, but the destination path already exists:
       specs/archived/task-0007-add-status-view.md
       -> Remove or rename the conflicting file before retrying.
```

---

## 8. Relationship to other commands

- `specmate check status` diagnoses directory/status mismatches but never
  repairs them.
- `specmate run` may call `specmate move <task-id> completed` during finalise
  after its own semantic gates pass.
- `specmate new` creates managed documents but never performs later status
  transitions.

This keeps creation, validation, and transition responsibilities separate.

---

## 9. Verification requirements

An implementation of this command is not complete unless automated tests cover
at least:

- moving each supported document type across a legal cross-directory transition
- moving across a legal same-directory transition
- rejection of an illegal transition
- rejection of same-status requests
- rejection when the repository contains invalid managed documents
- rejection when a transition-time gate from the shared document model blocks
  the move
- rejection for `project`, `org`, and Guideline targets
- rejection when the destination path already exists
- dry-run output with no filesystem changes
- informational output that correctly renders association summaries returned by
  the shared document model
- preservation of body content outside the rewritten frontmatter
- exact CLI output and exit codes for success and failure

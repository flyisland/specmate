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

This document defines the implemented behavior of `specmate move`. The command
performs one legal status transition on one managed document and applies the
required filesystem update, if any.

---

## 1. Command surface

```bash
specmate move <doc-id> <to-status> [--dry-run]
```

Examples:

```bash
specmate move exec-auth-rollout candidate
specmate move exec-auth-rollout/task-01 closed
specmate move design-auth-system implemented --dry-run
```

Supported lifecycle-managed document types:

- PRD
- Design Doc
- Design Patch
- Exec Plan
- Task Spec

Unsupported targets:

- `project`
- `org`
- guideline ids

---

## 2. Design principles

- One command owns status transitions. Manual path edits are not the supported
  write flow.
- The shared document model owns legality. `move` reuses shared transition,
  preview, and path-resolution helpers.
- Fail before writing. If preflight or preview validation fails, nothing is
  modified.
- Update and relocation are one operation. Successful cross-directory moves
  rewrite frontmatter and filesystem state together.

---

## 3. Preconditions

Before planning a move, the command must:

1. locate the repository root
2. build a compliant document index
3. resolve the requested canonical document id
4. parse the requested target status for that document type
5. reject same-status requests
6. validate the requested transition through the shared model
7. build a post-move preview
8. validate the post-move preview
9. verify the destination directory exists
10. reject destination path collisions

If any of these checks fails, the command exits with code `1`.

---

## 4. Implemented transition semantics

`specmate move` follows the shared transition graph exactly.

Examples of legal transitions:

- `design-auth-system`: `candidate -> implemented`
- `design-auth-system-patch-01-fix-links`: `implemented -> obsolete:merged`
- `exec-auth-rollout`: `draft -> candidate`
- `exec-auth-rollout/task-01`: `candidate -> closed`
- `exec-auth-rollout/task-01`: `candidate -> draft`

Examples of blocked transitions:

- any same-status request
- `design -> implemented` while linked Exec Plans are not `closed`
- `exec -> closed` while linked Task Specs are not `closed`
- `patch -> obsolete:merged` without a valid `merged-into`

`move` does not run semantic proofs such as completion-criteria tests. It only
enforces mechanical legality.

---

## 5. Path behavior

For PRDs and Design Docs/Patches, status may change the required directory.

Example:

- `docs/design/candidate/design-auth-system.md`
- `docs/design/implemented/design-auth-system.md`

For Exec Plans and Task Specs, status does not change the path:

- Exec Plan remains at `docs/exec-plans/exec-<slug>/plan.md`
- Task Spec remains at `docs/exec-plans/exec-<slug>/task-<nn>-<slug>.md`

Closing an Exec Plan or Task Spec therefore updates frontmatter in place and
adds `closed: YYYY-MM-DD`, but does not move the file.

---

## 6. Write model

The command rewrites frontmatter in memory first, then writes through a
temporary sibling file and atomically renames it into place.

Behavior:

- `status:` is updated to the target status
- `closed:` is inserted or refreshed when moving to `closed`
- `closed:` is removed when moving away from `closed`
- body content outside frontmatter is preserved

The command never overwrites an existing destination file.

---

## 7. Output

### Applied output

Successful writes print ownership-tagged lines:

```text
  [user] UPDATE    docs/exec-plans/exec-auth-rollout/task-01-implement-login.md  (status: candidate -> closed)
```

Cross-directory transitions print an additional move line:

```text
  [user] UPDATE    docs/design/candidate/design-auth-system.md  (status: candidate -> implemented)
  [user] MOVE      docs/design/candidate/design-auth-system.md -> docs/design/implemented/design-auth-system.md
```

### Dry-run output

Dry-run mode prints the same plan prefixed by:

```text
Planned operations (no files will be written):
```

and ends with:

```text
Run without --dry-run to apply.
```

### Failure output

Failures render in CLI-conventions form:

```text
[fail] move
       docs/design/candidate/design-auth-system.md
       cannot transition to implemented while exec-auth-rollout is candidate
       -> Fix the blocking transition rule or choose a different target status.
```

---

## 8. Relationship to other commands

- `specmate check status` diagnoses directory mismatches but does not repair
  them.
- `specmate check refs` diagnoses stale references but does not transition
  document state.
- Future workflow commands such as `specmate run` may call `specmate move
  <task-id> closed` after their own semantic gates pass.

This is the current implementation contract. The command does not currently
emit advisory relationship hints after success.

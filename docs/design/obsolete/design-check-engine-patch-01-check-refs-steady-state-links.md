---
id: design-check-engine-patch-01-check-refs-steady-state-links
title: "Check Refs Steady-State Links"
status: obsolete:merged
created: 2026-03-25
parent: design-check-engine
merged-into: design-check-engine
---

# Check Refs Steady-State Links

This patch updates `design-004` so `check refs` consumes the steady-state
reference-validity rules defined by `design-003-patch-01`.

`design-004` currently treats some historical references as invalid merely
because the parent document is obsolete or abandoned. That is no longer correct
after the document-model patch introduces a distinction between:

- live references, which are blocked from pointing at forbidden parent states
- historical references, which may be preserved for audit history

This patch keeps `check refs` aligned with the shared document model instead of
redefining reference validity inside the check engine.

---

## 1. Scope

This patch updates only the semantics of `check refs`.

It does not change:

- `check names`
- `check frontmatter`
- `check status`
- `check boundaries`
- `check conflicts`

It also does not define transition-time gates. Those remain outside `check`
and continue to belong to mutating commands such as `specmate move` and
`specmate run`.

---

## 2. Revised rule

`check refs` validates steady-state repository reference validity only.

It must use the current shared document-model rules, including the
live-vs-historical distinction from `design-003-patch-01`.

Therefore:

- a live descendant referencing a parent in a forbidden status is a violation
- a historical descendant may retain a reference to an obsolete or abandoned
  parent if the target exists and the relationship type is still correct
- `check refs` must not fail solely because a transition-time gate would block a
  future status change

Examples:

- an `active` Task Spec referencing an `abandoned` Exec Plan fails
- a `completed` Task Spec referencing an `abandoned` Exec Plan passes
- an `active` Exec Plan referencing an `obsolete` Design Doc fails
- a `completed` Exec Plan referencing an `obsolete` Design Doc passes
- an `implemented` Design Doc with later bug-fix work linked through a new
  draft or active Exec Plan / Task Spec remains valid

---

## 3. Design intent

`check refs` remains a steady-state integrity check.

It must not become a surrogate for:

- transition-time legality checks
- workflow readiness checks
- semantic completion checks

If a repository is valid now but some future transition is blocked, `check refs`
still passes.

---

## 4. Verification expectation

An implementation of this patch is not complete unless `check refs` is proven
to:

- reject invalid live references
- allow valid historical references
- allow ongoing bug-fix work against an already `implemented` Design Doc
- avoid enforcing transition-time gates as steady-state violations

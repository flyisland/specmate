---
id: design-document-system
title: "Document System Design"
status: implemented
created: 2026-03-25
---

# Document System Design

This document defines the current repository document system that specmate
recognises and enforces. It is the source of truth for document kinds,
canonical IDs, naming rules, status lifecycles, and managed paths.

---

## 1. Managed documents

All managed documents are Markdown files under `docs/`.

| Type | Purpose |
|---|---|
| PRD | Product requirements and approval state |
| Design Doc | Long-lived design contract for a module or cross-cutting topic |
| Design Patch | Incremental delta against a Design Doc |
| Exec Plan | Execution container for one rollout |
| Task Spec | Single executable task within an Exec Plan |
| Guideline | Cross-cutting operational guidance |
| `project.md` | Project-level technical constraints |
| `org.md` | Organisation-level constraints |

Repository docs such as `AGENTS.md` and `README.md` are useful, but not part
of the managed document model.

---

## 2. Canonical IDs and filenames

specmate uses slug-based IDs for long-lived knowledge artifacts and
parent-scoped sequence numbers for work artifacts.

### 2.1 Canonical forms

| Type | Canonical ID | Path shape |
|---|---|---|
| PRD | `prd-<slug>` | `docs/prd/<status>/prd-<slug>.md` |
| Design Doc | `design-<slug>` | `docs/design/<status>/design-<slug>.md` |
| Design Patch | `design-<parent-slug>-patch-<nn>-<patch-slug>` | `docs/design/<status>/design-<parent-slug>-patch-<nn>-<patch-slug>.md` |
| Exec Plan | `exec-<slug>` | `docs/exec-plans/exec-<slug>/plan.md` |
| Task Spec | `<exec-id>/task-<nn>` | `docs/exec-plans/exec-<slug>/task-<nn>-<slug>.md` |
| Project Spec | `project` | `docs/specs/project.md` |
| Org Spec | `org` | `docs/specs/org.md` |
| Guideline | relative guideline slug | `docs/guidelines/<slug>.md` |

Task Specs have two renderings:

- Canonical CLI/reference form: `<exec-id>/task-<nn>`
- Escaped single-token form: `<exec-id>--task-<nn>`

The escaped form is for branch names, report stems, and similar single-token
surfaces. Human-facing CLI output should prefer the slash form.

### 2.2 Naming rules

- Slugs are lowercase and hyphen-separated.
- Design Patch and Task Spec sequence numbers are zero-padded to at least
  two digits: `01`, `02`, `10`, `100`.
- `plan.md` is the fixed filename for every Exec Plan.
- Task Spec frontmatter uses the local id `task-<nn>`, while lookup and
  cross-document references use the full `<exec-id>/task-<nn>` form.

Examples:

```text
docs/prd/draft/prd-user-auth.md
docs/design/candidate/design-auth-system.md
docs/design/candidate/design-auth-system-patch-01-remove-username.md
docs/exec-plans/exec-auth-rollout/plan.md
docs/exec-plans/exec-auth-rollout/task-01-implement-login.md
```

---

## 3. Status lifecycles

### PRD

```text
draft -> approved -> obsolete
draft -> obsolete
```

### Design Doc

```text
draft -> candidate -> implemented -> obsolete
candidate -> obsolete
```

### Design Patch

```text
draft -> candidate -> implemented -> obsolete:merged
draft -> obsolete
candidate -> obsolete
```

### Exec Plan

```text
draft <-> candidate
draft -> closed
candidate -> closed
```

### Task Spec

```text
draft <-> candidate
draft -> closed
candidate -> closed
```

### Fixed-path docs and Guidelines

- `docs/specs/project.md` and `docs/specs/org.md` always use `status: active`.
- Guidelines do not declare `status`; they are implicitly active.

`closed`, `obsolete`, and `obsolete:merged` are terminal states. `candidate`
remains editable by design for Exec Plans and Task Specs.

---

## 4. Directory model

For most lifecycle-managed docs, frontmatter status determines the required
directory. Exec Plans and Task Specs are different: they stay inside the
owning Exec Plan directory regardless of status.

```text
docs/
├── specs/
│   ├── project.md
│   └── org.md
├── guidelines/
│   └── <slug>.md
├── prd/
│   ├── draft/
│   ├── approved/
│   └── obsolete/
├── design/
│   ├── draft/
│   ├── candidate/
│   ├── implemented/
│   └── obsolete/
└── exec-plans/
    └── exec-<slug>/
        ├── plan.md
        ├── task-<nn>-<slug>.md
        └── <escaped-task-id>-<slug>-report.md   # ignored by the document index
```

Consequences:

- `ls docs/design/implemented/` is the current set of implemented design
  contracts.
- `specmate move` may relocate PRDs and Design Docs/Patches across status
  directories.
- `specmate move` updates Exec Plans and Task Specs in place because their
  directory does not encode lifecycle state.

---

## 5. Required metadata

### Lifecycle-managed docs

PRD, Design Doc, Design Patch, Exec Plan, and Task Spec must declare:

- `id`
- `title`
- `status`
- `created`

Additional rules:

- Exec Plans and Task Specs may declare `closed`, but only when
  `status: closed`.
- Design Patches require `parent`.
- Design Patches in `obsolete:merged` require `merged-into`.
- Exec Plans use `design-docs` as a non-empty list.
- Task Specs require `exec-plan`.
- Task Specs must not use the legacy `design-doc` field in the current model.

### Candidate Task Specs

Candidate Task Specs are executable, so they must also declare:

- `boundaries.allowed`
- `boundaries.forbidden_patterns`
- `completion_criteria`

Required forbidden patterns:

```yaml
boundaries:
  forbidden_patterns:
    - "docs/prd/**"
    - "docs/design/**"
    - "docs/guidelines/**"
    - "docs/specs/**"
    - "docs/exec-plans/**"
```

### Fixed-path docs and Guidelines

- `docs/specs/project.md` and `docs/specs/org.md` declare `id` plus
  `status: active`.
- Guidelines declare `title` only. They must not declare `id` or `status`.

---

## 6. Relationship model

The current reference model is:

- Design Docs may reference one PRD via `prd`.
- Design Patches reference their parent Design Doc via `parent`.
- Exec Plans reference one or more Design Docs / Design Patches via
  `design-docs`.
- Task Specs reference their owning Exec Plan via `exec-plan`.
- Task Specs may optionally list guideline file paths in `guidelines`.

Task Specs are not standalone in the current model. Every Task Spec belongs to
an Exec Plan.

If an Exec Plan references a Design Patch, it must also include the patch's
parent Design Doc in `design-docs`.

---

## 7. Operational rules

- Frontmatter is authoritative for status; path mismatch is a repository error.
- Managed docs are changed through specmate commands, not by manual ad hoc
  moves.
- Closed Exec Plans and Task Specs remain in place as historical records.
- Candidate Design Docs and Design Patches are approved but not yet terminal.
- Implemented Design Docs are the current long-lived design contracts.

This is the model the current codebase implements. New command work such as
`specmate run` must build on these rules rather than the older numeric-ID
layout.

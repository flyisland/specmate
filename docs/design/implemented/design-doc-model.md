---
id: design-doc-model
title: "Document Model"
status: implemented
created: 2026-03-25
---

# Document Model

This document defines the internal model specmate uses to parse, validate,
index, and transition managed documents. Command modules consume this shared
model instead of reimplementing document rules locally.

---

## 1. Core types

### DocType

```text
Prd
DesignDoc
DesignPatch
ExecPlan
TaskSpec
ProjectSpec
OrgSpec
Guideline
```

### Status

```text
Prd:          Draft | Approved | Obsolete
DesignDoc:    Draft | Candidate | Implemented | Obsolete
DesignPatch:  Draft | Candidate | Implemented | Obsolete | ObsoleteMerged
ExecPlan:     Draft | Candidate | Closed
TaskSpec:     Draft | Candidate | Closed
ProjectSpec:  Active
OrgSpec:      Active
Guideline:    Active   # implicit, not read from frontmatter
```

### DocId

```text
Prd(String)                          -> prd-<slug>
DesignDoc(String)                    -> design-<slug>
DesignPatch(parent, sequence, slug)  -> design-<parent>-patch-<nn>-<slug>
ExecPlan(String)                     -> exec-<slug>
TaskSpec(exec_slug, sequence)        -> exec-<slug>/task-<nn>
ProjectSpec                          -> project
OrgSpec                              -> org
Guideline(String)                    -> relative guideline slug
```

Important Task Spec ID rules:

- canonical id: `exec-<slug>/task-<nn>`
- frontmatter `id`: `task-<nn>`
- escaped single-token form: `exec-<slug>--task-<nn>`

---

## 2. Frontmatter contract

### 2.1 Required fields

PRD, Design Doc, Design Patch, Exec Plan, and Task Spec require:

- `id`
- `title`
- `status`
- `created`

Exec Plans and Task Specs may also require:

- `closed` when `status: closed`

Fixed-path specs:

- `docs/specs/project.md` must declare `id: project` and `status: active`
- `docs/specs/org.md` must declare `id: org` and `status: active`

Guidelines:

- require `title`
- must not declare `id`
- must not declare `status`

### 2.2 Type-specific fields

Design Patch:

- `parent` is required
- `merged-into` is required when `status: obsolete:merged`

Exec Plan:

- `design-docs` is required and must be non-empty
- legacy singular `design-doc` must not be used together with `design-docs`

Task Spec:

- `exec-plan` is required
- legacy `design-doc` is rejected in the current model
- `guidelines` is optional
- `boundaries` and `completion_criteria` are required when status is
  `candidate`

Candidate Task Spec requirements:

- `boundaries.allowed` must contain at least one pattern
- `boundaries.forbidden_patterns` must include:
  `docs/prd/**`, `docs/design/**`, `docs/guidelines/**`,
  `docs/specs/**`, and `docs/exec-plans/**`
- `completion_criteria` must be non-empty
- each completion criterion must include non-empty `id`, `scenario`, and
  `test`
- completion criterion ids use `cc-NNN`

Date rules:

- `created` and `closed` use `YYYY-MM-DD`
- `closed` is only allowed on Exec Plans and Task Specs
- `closed` must be absent unless status is `closed`

---

## 3. Path classification and indexing

The document model scans the repository and classifies paths into:

- valid managed entries
- invalid managed entries
- ignored files

Managed path rules:

```text
docs/specs/project.md
docs/specs/org.md
docs/guidelines/<slug>.md
docs/prd/<status>/prd-<slug>.md
docs/design/<status>/design-<slug>.md
docs/design/<status>/design-<parent>-patch-<nn>-<slug>.md
docs/exec-plans/exec-<slug>/plan.md
docs/exec-plans/exec-<slug>/task-<nn>-<slug>.md
```

Files ending in `-report.md` under an Exec Plan directory are ignored by the
document index. They are specmate-managed workflow artifacts, not managed
documents.

`README.md` files inside managed directories are also ignored by the model.

### Expected directories

```text
Prd + Draft        -> docs/prd/draft
Prd + Approved     -> docs/prd/approved
Prd + Obsolete     -> docs/prd/obsolete

DesignDoc/Patch + Draft         -> docs/design/draft
DesignDoc/Patch + Candidate     -> docs/design/candidate
DesignDoc/Patch + Implemented   -> docs/design/implemented
DesignDoc/Patch + Obsolete*     -> docs/design/obsolete

ProjectSpec + Active -> docs/specs
OrgSpec + Active     -> docs/specs
Guideline + Active   -> docs/guidelines
```

Exec Plans and Task Specs do not use status-based directories:

- Exec Plan expected directory: `docs/exec-plans/<exec-id>`
- Task Spec expected directory: `docs/exec-plans/exec-<exec-slug>`

---

## 4. Repository-level validation

The shared validator enforces cross-document rules that depend on the loaded
index.

### 4.1 Reference validity

- A live Design Doc must not reference an obsolete PRD.
- A live Task Spec must not reference a closed Exec Plan.
- Historical descendants may retain references to terminal parents when the
  current implementation treats that relationship as valid history.
- Guideline paths listed in `guidelines` must resolve to actual Guidelines.

### 4.2 Exec Plan rules

- `design-docs` must contain at least one reference.
- `design-docs` entries must be unique.
- Every `design-docs` entry must resolve to an existing Design Doc or
  Design Patch.
- Referenced Design Docs and Design Patches must be in `candidate` or
  `implemented`.
- If a Design Patch appears in `design-docs`, its parent Design Doc must also
  appear in `design-docs`.

### 4.3 Task Spec rules

- Every Task Spec must belong to an Exec Plan.
- Candidate Task Specs must satisfy the runtime contract described above.

---

## 5. Transition rules

`validate_transition` implements the legal status graph.

### PRD

- `draft -> approved`
- `draft -> obsolete`
- `approved -> obsolete`

### Design Doc

- `draft -> candidate`
- `candidate -> implemented`
- `candidate -> obsolete`
- `implemented -> obsolete`

### Design Patch

- `draft -> candidate`
- `draft -> obsolete`
- `candidate -> implemented`
- `candidate -> obsolete`
- `implemented -> obsolete:merged`

### Exec Plan

- `draft -> candidate`
- `draft -> closed`
- `candidate -> draft`
- `candidate -> closed`

### Task Spec

- `draft -> candidate`
- `draft -> closed`
- `candidate -> draft`
- `candidate -> closed`

Project Specs, Org Specs, and Guidelines have no legal transitions.

---

## 6. Transition-time gates

Legal status edges may still be blocked by repository facts.

- `Prd -> Obsolete` is blocked while any live Design Doc still references it.
- `DesignDoc -> Implemented` is blocked while any linked Exec Plan is not
  `closed`.
- `DesignDoc -> Obsolete` is blocked while any live linked Exec Plan still
  references it.
- `DesignPatch -> ObsoleteMerged` requires `merged-into` to resolve to an
  existing Design Doc.
- `ExecPlan -> Closed` is blocked while any linked Task Spec is not `closed`.

The document model treats `draft` and `candidate` as live for Exec Plans and
Task Specs. `closed` is terminal.

---

## 7. Preview and mutation support

The shared model exposes preview helpers used by write commands:

- `preview_transition` predicts the post-move repository state
- `validate_preview` verifies that the predicted state would still be valid
- `expected_directory` resolves status-based destination directories

This keeps command modules small:

- `specmate move` uses the shared transition, preview, and path rules
- `specmate check` uses the shared index and validation logic
- `specmate status` uses the shared index and association summaries

---

## 8. Association summaries

The model provides direct downstream association summaries for:

- PRD -> Design Docs
- Design Doc -> Design Patches
- Design Doc -> Exec Plans
- Design Doc -> direct Task Specs
- Exec Plan -> Task Specs

These summaries are intentionally direct. Command-layer views may add derived
presentation, but they should not redefine the underlying relationship rules.

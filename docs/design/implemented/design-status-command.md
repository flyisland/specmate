---
id: design-status-command
title: "Status Command"
status: implemented
created: 2026-03-25
guidelines:
  - docs/guidelines/specmate-principles.md
  - docs/guidelines/cli-conventions.md
---

# Status Command

This document defines the implemented behavior of `specmate status`. The
command is read-only and exposes the current repository state through either a
dashboard view or a single-document detail view.

---

## 1. Command surface

```bash
specmate status [doc-id] [--all] [--color <when>]
```

Examples:

```bash
specmate status
specmate status --all
specmate status design-status-command
specmate status exec-status-rollout/task-01
```

`doc-id` must be a canonical managed document id. Guideline lookup targets are
rejected in v1.

---

## 2. Design principles

- Status is diagnostic, not mutating.
- One command serves both dashboard and focused lookup flows.
- Invalid repositories should still produce useful read-only output whenever a
  non-strict view can be built.
- The shared document model remains the source of truth for parsing, IDs,
  statuses, and direct associations.

---

## 3. Data loading model

`specmate status` uses the shared document index in non-strict mode:

- valid managed documents are loaded
- invalid managed entries are retained for visibility
- repository validation violations are collected separately

The command does not require a compliant repository.

Lookup rules:

- exact canonical id match only
- no fuzzy lookup by title or slug fragments
- guideline ids are rejected explicitly

Exit codes:

- `0` for a rendered dashboard or detail view
- `1` for unresolved or unsupported lookup targets, or repository discovery
  failures
- `2` for CLI parse failures

---

## 4. Dashboard view

`specmate status` without `doc-id` renders four sections in this order:

1. `Repository Health`
2. `Design Overview`
3. `Execution Overview`
4. `Status Totals`

If `--all` is passed, `All Documents` is appended afterward.

### 4.1 Repository Health

Shows:

- valid managed document count
- invalid managed entry count
- repository validation violation count

### 4.2 Design Overview

Lists Design Docs and Design Patches in these status buckets:

- `draft`
- `candidate`
- `implemented`

Rows include canonical id, title, and status.

### 4.3 Execution Overview

Lists:

- candidate Exec Plans
- candidate Task Specs

Exec Plan rows include linked `design-docs` plus task counts by
`draft` / `candidate` / `closed`.

### 4.4 Status Totals

Shows counts by document type and lifecycle order for:

- PRD
- DesignDoc
- DesignPatch
- ExecPlan
- TaskSpec

### 4.5 All Documents

When requested with `--all`, every valid managed document is listed in
canonical-id order with its type and status.

---

## 5. Detail view

`specmate status <doc-id>` renders four sections:

1. `Overview`
2. `Upstream References`
3. `Downstream Associations`
4. `Related Repository Warnings`

### 5.1 Overview

Shows:

- canonical id
- title
- document type
- status
- repository-relative path
- lifecycle state (`live` or `terminal`)
- expected directory when one exists

### 5.2 Upstream References

The current implementation renders these direct frontmatter fields when
present:

- `prd`
- `parent`
- `merged-into`
- `superseded-by`
- `design-docs`
- `exec-plan`

If none exist, the section prints `none`.

### 5.3 Downstream Associations

Uses shared direct association summaries. Implemented association families are:

- PRD -> design docs
- Design Doc -> patches
- Design Doc -> exec plans
- Design Doc -> direct tasks
- Exec Plan -> tasks

Each row renders canonical ids plus current statuses.

### 5.4 Related Repository Warnings

Shows repository validation warnings related to the current document, including
warnings on the document path itself and violations whose message mentions the
document id.

If none apply, the section prints `none`.

---

## 6. Rendering rules

- Output is plain text with stable section headers.
- Paths are repository-relative.
- Document rows are sorted by canonical id.
- Color is optional enhancement only.
- The no-color form remains the canonical semantic rendering.

### Implemented color mapping

The current implementation uses:

- `draft`: cyan
- `candidate`: yellow
- `implemented`, `approved`, `closed`: green
- `obsolete`, `obsolete:merged`: dim gray
- `active`: magenta

`--color auto` enables color only on TTY output.

---

## 7. Example shapes

Dashboard example:

```text
Repository Health
  valid managed documents: 18
  invalid managed entries: 1
  repository validation violations: 2

Design Overview
  draft
    design-draft-experiment  Draft Experiment  draft
  candidate
    design-status-command  Status Command  candidate
  implemented
    design-core-platform  Core Platform  implemented

Execution Overview
  candidate exec plans
    exec-status-rollout  Status Rollout  design-docs: design-status-command  tasks: draft=0 candidate=1 closed=0
  candidate task specs
    exec-status-rollout/task-01  Implement status dashboard

Status Totals
  ExecPlan  draft=0 candidate=2 closed=1
  TaskSpec  draft=0 candidate=2 closed=3
```

Detail example:

```text
Overview
  id: design-status-command
  type: DesignDoc
  status: candidate

Upstream References
  prd: prd-core-platform

Downstream Associations
  exec plans: exec-status-follow-up (candidate), exec-status-rollout (candidate)

Related Repository Warnings
  none
```

---

## 8. Failure handling

Repository invalidity alone does not fail the command.

`specmate status` fails only when:

- no specmate repository root can be found
- the requested managed document id does not resolve
- the requested target is a guideline lookup
- the repository cannot even be loaded in non-strict mode

This document matches the current implementation rather than the earlier,
broader design ambitions.

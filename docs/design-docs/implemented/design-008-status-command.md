---
id: design-008
title: "Status Command"
status: implemented
design-doc: design-001
guidelines:
  - docs/guidelines/specmate-principles.md
  - docs/guidelines/cli-conventions.md
---

# Status Command

This document defines `specmate status` — the read-only command that answers
two operational questions:

1. "What is the current dashboard view of docs and specs in this repository?"
2. "What is the current state and relationship graph of a specific document?"

The command gives developers and agents a fast repository-facing view without
requiring them to manually inspect directories or cross-reference frontmatter.

`design-003` remains the source of truth for document parsing, status meaning,
and direct-association discovery. This document defines only the CLI surface
and the view-model behaviour for presenting that information.

---

## 1. Design principles

**Status is diagnostic, not mutating.** `specmate status` never edits files,
moves documents, creates branches, or repairs invalid state. It is always safe
to run.

**One command, two scopes.** The same command serves both repository dashboard
queries and single-document inspection so users do not need to learn multiple
entry points for related status questions.

**Show useful state even when the repository is imperfect.** Unlike mutating
commands such as `specmate move`, `specmate status` should still render a view
when the repository contains invalid managed entries or repository-level
violations. Read-only observability is more useful than failing closed.

**Relationships are first-class output.** The command must not stop at path and
status. It must expose upstream references and downstream associations so a
user can understand where a document sits in the managed system.

**Output stays CLI-native and English-only.** As with all CLI output, text is
English regardless of repository content language. The output is intended to be
readable in terminals and by agents without requiring TUI widgets.

---

## 2. Command surface

```bash
specmate status [doc-id] [--all] [--color <when>]
```

Examples:

```bash
specmate status
specmate status --all
specmate status --color always
specmate status design-008
specmate status task-0005
```

Semantics:

- `specmate status` renders the repository dashboard
- `specmate status --all` renders the repository dashboard plus an exhaustive
  all-documents listing across statuses
- `specmate status <doc-id>` renders a focused detail view for one managed
  document

Options:

- `--all`: append the exhaustive all-documents view to the dashboard
- `--color auto|always|never`: control ANSI color output; default is `auto`

Argument rules:

- `<doc-id>` must use the canonical managed ID spelling such as `prd-001`,
  `design-003`, `design-003-patch-01`, `exec-001`, `task-0005`, `project`, or
  `org`
- guideline slugs are not accepted in v1 as direct lookup targets because the
  command is focused on lifecycle-managed docs and specs
- lookup resolves against valid managed documents only; an id that appears only
  inside an invalid managed entry is treated as unresolved for the detail view

Exit codes:

- `0` when a dashboard or document view is rendered successfully
- `1` when the requested `doc-id` does not exist or cannot be resolved
- `2` for CLI argument parse failures

---

## 3. Data source and loading model

`specmate status` builds the repository document index using the shared
document-model loader from `design-003`.

It must use the non-strict index build path:

- load all valid managed documents
- collect invalid managed entries
- collect repository-level validation violations separately

It must not require `build_compliant_index()` because that would hide useful
state whenever the repository already needs diagnosis.

The command may reuse these shared document-model facilities:

- document parsing and ID resolution
- status typing and directory expectations
- direct-association summaries
- repository-level validation for warnings

The command must not duplicate filename parsing, status parsing, or
relationship rules in its own module.

Color handling is a presentation concern in the command layer. It must not
change the underlying text content, section order, or repository facts.

When a specific `doc-id` is requested, the command resolves it against the
loaded valid document index by canonical ID equality. It must not attempt
fuzzy matching by slug, title, path fragment, or status bucket.

---

## 4. Repository dashboard view

`specmate status` with no `doc-id` renders a repository dashboard with four
sections in this order.

### 4.1 Repository health

This section reports:

- number of valid managed documents
- number of invalid managed entries
- number of repository-level validation violations

If invalid entries or validation violations exist, the section must say so
explicitly and keep rendering the rest of the dashboard.

If either count is non-zero, the section should also include a short
non-exhaustive preview of the first few issues so the user can immediately see
what kind of repository damage exists without re-running a different command.
This preview is informational only and must not replace `specmate check`.

### 4.2 Design overview

This section highlights Design Docs because they are the current design
contracts and roadmap anchors.

It must list at least:

- `draft` Design Docs
- `candidate` Design Docs
- `implemented` Design Docs

Each listed design row must include:

- doc id
- title
- status
- repository-relative path
- linked PRD id when present
- count of linked Exec Plans
- count of directly linked Task Specs
- count of total linked Task Specs across both direct and Exec Plan-linked
  paths

Rows must be sorted by canonical design id ascending within each status group.
Status groups themselves must follow lifecycle order.

This allows a user to answer:

- which designs are still being authored
- which designs are current contracts
- which designs are ready for implementation
- whether a design already has associated execution work

Design Docs in inactive end states such as `obsolete` are not expanded by
default in this section. They remain visible through totals and the optional
`--all` view.

### 4.3 Execution overview

This section reports active and historical execution state.

It must include:

- active Exec Plans
- active Task Specs
- completed or abandoned Exec Plan counts
- completed or cancelled Task Spec counts

Each active Exec Plan row must include:

- doc id
- title
- linked Design Doc id when present
- counts of linked Task Specs by status

Each active Task Spec row must include:

- doc id
- title
- linked Exec Plan id when present
- otherwise linked Design Doc id when present

Rows must be sorted by canonical id ascending.

### 4.4 Status totals

This section provides compact counts by document type and status so users can
quickly scan the full repository state without relying on directory listings.

At minimum it must cover:

- PRD
- Design Doc
- Design Patch
- Exec Plan
- Task Spec

Fixed-path docs (`project`, `org`) and guidelines may be omitted from the
totals table in v1 because they do not represent lifecycle progress.

Within each document type row, statuses must appear in lifecycle order rather
than alphabetical order so the output mirrors the document model.

### 4.5 Optional all-documents view

When `--all` is passed without a `doc-id`, the command must append an
`All Documents` section after `Status Totals`.

This section lists every valid lifecycle-managed document, regardless of
status, grouped by document type and sorted by canonical id ascending.

Each row must include:

- doc id
- document type
- status
- title
- repository-relative path

This view exists so users can explicitly request the exhaustive inventory
without making the default dashboard noisy.

---

## 5. Single-document detail view

`specmate status <doc-id>` renders a focused detail view with five sections.

### 5.1 Overview

This section includes:

- doc id
- title when present
- document type
- status
- repository-relative path
- expected directory for the current status when applicable
- whether the status is live or terminal

For `project` and `org`, the expected directory line should still be shown
using the fixed managed path semantics from the document model.

### 5.2 Upstream references

This section lists direct frontmatter references owned by the document itself.

Supported fields:

- `prd`
- `parent`
- `merged-into`
- `superseded-by`
- `design-doc`
- `exec-plan`

Each reference row must include the target id and, when the target resolves,
its current status.

If a field is present but the target does not resolve, the row must still be
shown and marked as unresolved so the user can see the broken edge directly in
the detail view.

If no upstream references exist, the command should print `none`.

### 5.3 Downstream associations

This section lists the most relevant downstream dependents for the requested
document.

Supported association families:

- PRD → Design Docs
- Design Doc → Design Patches
- Design Doc → Exec Plans
- Design Doc → Task Specs via linked Exec Plans
- Design Doc → Task Specs (direct)
- Exec Plan → Task Specs

Each family must show:

- association kind
- related document ids
- current statuses of those related documents

For `DesignDoc` detail views, the command must show both task paths
separately:

- Task Specs linked through the design's Exec Plans
- Task Specs linked directly to the design through `design-doc`

This split is required even though only the direct task path comes from the
shared direct-association summary model. The detail view is allowed to enrich
the presentation with one additional derived downstream family so users can see
all implementation work related to a design in one place.

### 5.4 Derived chain summary

For lifecycle-managed docs, the command should also show the most useful
derived chain facts without requiring the user to traverse multiple levels
manually.

Required derived summaries:

- PRD: total linked Design Docs, Exec Plans, and Task Specs
- Design Doc: total linked patches, Exec Plans, direct Task Specs, and total
  Task Specs across both direct and Exec Plan-linked paths
- Exec Plan: linked Task Spec totals by status
- Task Spec: linked Exec Plan lineage when present, otherwise direct linked
  Design Doc lineage when present
- Design Patch: parent design lineage and merged / superseded target facts when
  present
- ProjectSpec / OrgSpec: no derived chain summary beyond `none`

This section is informational only; it does not define new model-level
relationships.

### 5.5 Related repository warnings

If the current document is affected by any invalid managed entry or repository
validation violation, those warnings must be shown here.

Examples:

- an active Task Spec points to an abandoned Exec Plan
- an Exec Plan points to an obsolete Design Doc
- a linked document exists only as an invalid managed entry

If no warnings affect the requested document, the command should print
`No related warnings.`

A warning counts as related when at least one of these is true:

- the warning path is the current document path
- the warning references the current document id directly
- the warning references a directly linked upstream or downstream document
- an invalid managed entry has a filename-derived canonical id equal to the
  requested id

The command does not need full graph-wide transitive warning expansion in v1.
One hop from the current document is sufficient.

---

## 6. Rendering rules

The output should use plain text sections and line-based rows, not ANSI tables
or cursor-driven dashboards.

Formatting rules:

- section headers are stable and human-readable
- repository paths are shown relative to repo root
- status words use canonical lowercase spellings from frontmatter
- rendered doc ids should be visually distinguishable from surrounding prose
  when color is enabled
- empty sections print `none` rather than disappearing silently
- list ordering is deterministic across runs

Deterministic ordering rules:

- document rows sort by canonical id ascending
- warnings sort by repository-relative path, then message
- association family order follows the order defined in this design document
- upstream reference field order is: `prd`, `parent`, `merged-into`,
  `superseded-by`, `design-doc`, `exec-plan`

The command may use aligned columns where helpful, but alignment is a
presentation detail rather than a contract.

### 6.1 Color rules

Color is optional enhancement only. It must never be the sole carrier of
meaning.

Requirements:

- every colored token must still include the full underlying text such as
  `draft`, `candidate`, or `implemented`
- every rendered canonical status token should use the same color mapping,
  whether it appears as a standalone field value, a bucket label, or a
  `status=count` summary entry
- `--color auto` enables color only when stdout is a TTY
- `--color never` disables ANSI color unconditionally
- `--color always` emits ANSI color even when stdout is not a TTY
- `NO_COLOR` disables color when `--color auto` is in effect

Recommended status palette:

- `draft`: yellow
- `candidate`: blue
- `implemented`: green
- `approved`: green
- `active`: cyan
- `completed`: green
- `obsolete`, `obsolete:merged`, `abandoned`, `cancelled`: dim red or dim gray

Recommended structural palette:

- section headers: bold
- warning lines: red or yellow

The exact palette may evolve, but status-to-color mapping must remain stable
enough that human users can build recognition over time.

The output is intentionally human-readable, not a stable machine API. A future
`--json` mode may be added later, but it is out of scope for v1.

The dashboard must remain reasonably compact. Large repositories should still
fit in a normal terminal scrollback without expanding every historical
document. For that reason, dashboard sections list current-focus rows plus
aggregated totals, while the single-document view is allowed to be more
verbose.

`--all` is the explicit opt-in escape hatch for users who want the exhaustive
inventory instead of the compact default dashboard.

Agent-readability constraint:

- the no-color rendering remains the canonical semantic form
- any agent or script consuming output must still receive complete meaning when
  color is disabled or stripped

---

## 7. Example output

The following examples are illustrative. Exact spacing may vary, but the
section order, field presence, and overall information shape are part of the
design contract.

### 7.1 Repository dashboard example

```text
specmate status

Repository Health
  valid managed documents: 18
  invalid managed entries: 1
  repository validation violations: 2
  issue preview
    docs/design-docs/draft/design-999-bad.md
    invalid filename for DesignDoc/DesignPatch at docs/design-docs/draft/design-999-bad.md
    specs/active/task-0008-broken.md
    exec-plan exec-999 does not exist

Design Overview
  draft
    design-011  Draft Experiment  draft      docs/design-docs/draft/design-011-draft-experiment.md
      prd: none  exec-plans: 0  direct-task-specs: 0  task-specs: 0
  candidate
    design-005  Agent Loop       candidate  docs/design-docs/candidate/design-005-agent-loop.md
      prd: none  exec-plans: 1  direct-task-specs: 0  task-specs: 2
    design-008  Status Command   candidate  docs/design-docs/candidate/design-008-status-command.md
      prd: none  exec-plans: 0  direct-task-specs: 0  task-specs: 0
  implemented
    design-001  Check Engine     implemented  docs/design-docs/implemented/design-001-check-engine.md
      prd: prd-001  exec-plans: 2  direct-task-specs: 0  task-specs: 5
    design-007  Move Command     implemented  docs/design-docs/implemented/design-007-move-command.md
      prd: none  exec-plans: 0  direct-task-specs: 0  task-specs: 0

Execution Overview
  active exec plans
    exec-003  Build Agent Loop     design-doc: design-005  tasks: active=1 completed=1 cancelled=0
  active task specs
    task-0005  Implement association-aware transitions
      exec-plan: exec-003  design-doc: design-005
  historical totals
    exec plans: completed=2 abandoned=1
    task specs: completed=4 cancelled=1

Status Totals
  PRD          draft=0 approved=1 obsolete=0
  DesignDoc    draft=0 candidate=2 implemented=4 obsolete=0
  DesignPatch  draft=0 candidate=0 implemented=0 obsolete=2 obsolete:merged=1
  ExecPlan     draft=0 active=1 completed=2 abandoned=1
  TaskSpec     draft=0 active=1 completed=4 cancelled=1
```

### 7.2 Single-document detail example

```text
specmate status design-005

Overview
  id: design-005
  title: Agent Loop
  type: DesignDoc
  status: candidate
  path: docs/design-docs/candidate/design-005-agent-loop.md
  expected directory: docs/design-docs/candidate
  lifecycle state: live

Upstream References
  design-doc: design-001 (implemented)
  prd: none
  superseded-by: none

Downstream Associations
  design patches
    none
  exec plans
    exec-003 (active)
    exec-004 (completed)
  task specs via exec plans
    task-0005 (active)
    task-0006 (completed)
    task-0007 (completed)
  direct task specs
    none

Derived Chain Summary
  patches: 0
  exec plans: 2
  direct task specs: 0
  task specs: draft=0 active=1 completed=2 cancelled=0

Related Repository Warnings
  docs/exec-plans/active/exec-003-build-agent-loop.md
  design-doc design-999 does not exist
  -> Repair the reference in the linked Exec Plan.
```

### 7.3 Single-task detail example

```text
specmate status task-0005

Overview
  id: task-0005
  title: Implement association-aware transitions
  type: TaskSpec
  status: active
  path: specs/active/task-0005-implement-association-aware-transitions.md
  expected directory: specs/active
  lifecycle state: live

Upstream References
  exec-plan: exec-003 (active)

Downstream Associations
  none

Derived Chain Summary
  exec-plan lineage: exec-003 -> design-005
  completion criteria: 8
  boundaries.allowed entries: 7

Related Repository Warnings
  No related warnings.
```

---

## 8. Failure handling

Repository invalidity does not by itself fail the command.

`specmate status` fails only when:

- the working directory is not inside a specmate repository
- a requested `doc-id` does not resolve to a valid managed document
- a requested `doc-id` refers to a guideline slug in v1
- an unexpected I/O or parse failure prevents building even the non-strict view

It must not fail solely because:

- the repository has invalid managed entries
- repository-level validation violations exist
- related linked documents are unresolved or stale

Failure messages must remain actionable and explain what the user should check
next, following `docs/guidelines/cli-conventions.md`.

---

## 9. Out of scope

This design does not add:

- a TUI or interactive dashboard
- filtering flags such as `--type`, `--status`, or `--json`
- automatic repairs or suggestions that mutate repository state
- support for querying arbitrary unmanaged markdown files
- status history or git-aware timeline views

Those can be added later without changing the core command split defined here.

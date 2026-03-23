---
id: design-003
title: "Document Model"
status: implemented
design-doc: design-001
---

# Document Model

This document defines the internal model that specmate uses to represent,
validate, and transition documents. It is the foundation all other specmate
modules build on — commands read and write documents through this model,
never by manipulating files directly.

---

## 1. Core types

### DocType

Enumerates all document types recognised by specmate.

```
DocType:
  Prd
  DesignDoc
  DesignPatch
  ExecPlan
  TaskSpec
  ProjectSpec       # specs/project.md
  OrgSpec           # specs/org.md
  Guideline         # docs/guidelines/*.md
```

### Status

Each `DocType` has its own set of valid statuses. The status enum is
parameterised by `DocType` — a `Status::Candidate` is only valid for
`DesignDoc` and `DesignPatch`, not for `TaskSpec`.

```
Prd:          Draft | Approved | Obsolete
DesignDoc:    Draft | Candidate | Implemented | Obsolete
DesignPatch:  Draft | Candidate | Implemented | Obsolete | ObsoleteMerged
ExecPlan:     Draft | Active | Completed | Abandoned
TaskSpec:     Draft | Active | Completed | Cancelled
ProjectSpec:  Active          (single status, no transitions)
OrgSpec:      Active          (single status, no transitions)
Guideline:    Active          (single status, no transitions)
```

### DocId

The canonical identifier for a document. For filename-addressed documents,
the canonical ID is derived from the filename and must exactly match the
frontmatter `id` field. For fixed-path documents, the canonical ID is derived
from the path (`project`, `org`). Guidelines have no explicit ID field.

```
DocId:
  Prd(u32)                          # prd-001
  DesignDoc(u32)                    # design-001
  DesignPatch(u32, u8)              # design-001-patch-01 → (1, 1)
  ExecPlan(u32)                     # exec-001
  TaskSpec(u32)                     # task-0001
  ProjectSpec
  OrgSpec
  Guideline(String)                 # docs/guidelines/<slug>
```

ID allocation is per `DocType`. Task Specs use four-digit IDs (max 9999),
all others use three-digit IDs (max 999). IDs are globally incremented and
never reused.

### Document

A parsed, fully-validated document.

```
Document:
  id:        DocId
  doc_type:  DocType
  status:    Status           # Guideline status is implicit Active
  title:     Option<String>
  path:      PathBuf          # absolute path to the file
  frontmatter: Frontmatter    # all parsed frontmatter fields
  raw:       String           # original file content
```

---

## 2. Frontmatter

Frontmatter is the YAML block between the opening `---` and the first closing
`---` in a `.md` file. It is the source of truth for human-authored metadata.
For filename-addressed managed documents, the filename and frontmatter `id`
must agree exactly.

### Required fields by DocType

**PRD / DesignDoc / DesignPatch / ExecPlan / TaskSpec**

| Field | Type | Constraint |
|---|---|---|
| `id` | string | must exactly match the filename-derived ID |
| `title` | string | non-empty |
| `status` | string | must be a valid status for this DocType |

**ProjectSpec / OrgSpec**

| Field | Type | Constraint |
|---|---|---|
| `id` | string | must be `project` or `org` respectively |
| `status` | string | must be `active` |

**Guideline**

| Field | Type | Constraint |
|---|---|---|
| `title` | string | non-empty |

Guidelines do not carry `id` or `status` in frontmatter. They are always
treated as `active` when loaded.

### Optional fields by DocType

**DesignDoc / DesignPatch**

| Field | Type | Meaning |
|---|---|---|
| `module` | string | the codebase module this design covers |
| `prd` | string | linked PRD id (e.g. `prd-001`) |
| `parent` | string | patch only — required, parent design doc id |
| `merged-into` | string | patch only — required when status is `obsolete:merged` |
| `superseded-by` | string | required when status is `obsolete` via Flow B |

**ExecPlan**

| Field | Type | Meaning |
|---|---|---|
| `design-doc` | string | optional, linked Design Doc id |

**TaskSpec**

| Field | Type | Meaning |
|---|---|---|
| `exec-plan` | string | optional, linked Exec Plan id |
| `design-doc` | string | optional, linked Design Doc id for a standalone task |
| `guidelines` | string[] | guideline files injected at run time |
| `boundaries.allowed` | string[] | glob patterns — files agent may modify |
| `boundaries.forbidden_patterns` | string[] | glob patterns — files agent must never touch |
| `completion_criteria` | object[] | see below |

**completion_criteria item**

| Field | Type | Constraint |
|---|---|---|
| `id` | string | format `cc-NNN`, unique within this spec |
| `scenario` | string | human-readable description, non-empty |
| `test` | string | exact test function name, non-empty |

---

## 3. Task Spec runtime contract

Task Specs are verification documents, but a subset of their frontmatter is
also executed by specmate at runtime. These fields are part of the document
model and must be parsed consistently by `check`, `run`, and any future
automation commands.

### `exec-plan`

Optional. Links the Task Spec to its parent Exec Plan.

- Value must be a valid Exec Plan id such as `exec-001`
- If present, it must point to an existing Exec Plan document
- `specmate run` uses this link to resolve task dependencies before execution

Tasks may omit `exec-plan` when they are intentionally standalone and are not
part of a broader execution plan.

### `design-doc`

Optional. Links the Task Spec directly to a Design Doc when the task is
standalone and does not belong to an Exec Plan.

- Value must be a valid Design Doc id such as `design-001`
- If present, it must point to an existing Design Doc document
- It must not be used together with `exec-plan` on the same Task Spec

Tasks may omit `design-doc` when they are intentionally standalone and do not
need a recorded design upstream.

### `guidelines`

Optional. A list of guideline file paths relative to the repository root.

- Every listed file must exist
- Every listed file must resolve to a Guideline document
- `specmate run` injects the referenced guideline files into coding and review
  agent context verbatim

### `boundaries`

Required for Task Specs with status `active`.

`allowed` is a list of repository-relative glob patterns describing the files
the agent may modify.

`forbidden_patterns` is an optional list of repository-relative glob patterns
describing files the agent must never modify, even if they also match an
`allowed` pattern.

Rules:

- `boundaries.allowed` must contain at least one pattern for an `active` Task Spec
- if a file matches both `allowed` and `forbidden_patterns`, it is forbidden
- `specs/**` must appear in `forbidden_patterns` for every `active` Task Spec

### `completion_criteria`

Required for Task Specs with status `active`. Must contain at least one item.

Each item binds a human-readable scenario to an exact test function name.
`specmate run` executes each `test` by exact name using the project's test
runner contract defined in `specs/project.md`.

Rules:

- every item must include `id`, `scenario`, and `test`
- `id` values must be unique within the spec and follow `cc-NNN`
- `scenario` must be non-empty
- `test` must be non-empty
- a missing or undiscoverable test is a failure, not a skip

---

## 4. Filename parsing

For filename-addressed documents, document type and canonical ID are derived
from the filename first, then validated against frontmatter `id`. This gives
the file a stable self-declared identity even if it is copied elsewhere, while
still making the repository filename authoritative for placement and indexing.

**Parsing rules**

```
prd-{NNN}-{slug}.md           → DocType::Prd, id=NNN
design-{NNN}-{slug}.md        → DocType::DesignDoc, id=NNN
design-{NNN}-patch-{NN}-{slug}.md  → DocType::DesignPatch, id=(NNN,NN)
exec-{NNN}-{slug}.md          → DocType::ExecPlan, id=NNN
task-{NNNN}-{slug}.md         → DocType::TaskSpec, id=NNNN
project.md                    → DocType::ProjectSpec
org.md                        → DocType::OrgSpec
docs/guidelines/{slug}.md     → DocType::Guideline, id=slug
```

Files outside managed directories that do not match any pattern are ignored by
specmate.

Files inside managed directories are not ignored. If a file appears in a
managed directory but does not match the required naming pattern for that
location, specmate must surface it as an invalid managed document so `check`
can report a concrete violation.

---

## 5. Directory resolver

Given a `DocType` and `Status`, the directory resolver returns the expected
path for the file. This is used by `specmate move` to determine where to
place a file after a status transition.

```
Prd + Draft         → docs/prd/draft/
Prd + Approved      → docs/prd/approved/
Prd + Obsolete      → docs/prd/obsolete/

DesignDoc + Draft        → docs/design-docs/draft/
DesignDoc + Candidate    → docs/design-docs/candidate/
DesignDoc + Implemented  → docs/design-docs/implemented/
DesignDoc + Obsolete     → docs/design-docs/obsolete/

DesignPatch + Draft           → docs/design-docs/draft/
DesignPatch + Candidate       → docs/design-docs/candidate/
DesignPatch + Implemented     → docs/design-docs/implemented/
DesignPatch + Obsolete        → docs/design-docs/obsolete/
DesignPatch + ObsoleteMerged → docs/design-docs/obsolete/

ExecPlan + Draft       → docs/exec-plans/draft/
ExecPlan + Active      → docs/exec-plans/active/
ExecPlan + Completed   → docs/exec-plans/archived/
ExecPlan + Abandoned   → docs/exec-plans/archived/

TaskSpec + Draft    → specs/active/
TaskSpec + Active   → specs/active/
TaskSpec + Completed  → specs/archived/
TaskSpec + Cancelled  → specs/archived/

Guideline           → docs/guidelines/   (no subdirectory)
ProjectSpec         → specs/
OrgSpec             → specs/
```

---

## 6. Status transition rules

The transition table defines which status changes are legal. Illegal
transitions are rejected with an error.

**PRD**

| From | To | Notes |
|---|---|---|
| `draft` | `approved` | |
| `approved` | `obsolete` | |
| `draft` | `obsolete` | feature cancelled before approval |

**Design Doc**

| From | To | Notes |
|---|---|---|
| `draft` | `candidate` | |
| `candidate` | `implemented` | all Exec Plans referencing this doc must be `completed` |
| `candidate` | `obsolete` | design abandoned or split before implementation; keep the document for ID continuity |
| `implemented` | `obsolete` | module removed (Flow C) or superseded (Flow B) |

**Design Patch**

| From | To | Notes |
|---|---|---|
| `draft` | `candidate` | |
| `draft` | `obsolete` | patch abandoned before review or implementation; keep the document for ID continuity |
| `candidate` | `implemented` | |
| `candidate` | `obsolete` | patch abandoned after review; keep the document for ID continuity |
| `implemented` | `obsolete:merged` | requires `merged-into` in frontmatter |

**Exec Plan**

| From | To | Notes |
|---|---|---|
| `draft` | `active` | |
| `active` | `completed` | |
| `active` | `abandoned` | must record reason |

**Task Spec**

| From | To | Notes |
|---|---|---|
| `draft` | `active` | human approval gate |
| `active` | `completed` | all completion criteria must pass |
| `active` | `cancelled` | must record reason |
| `draft` | `cancelled` | |

---

## 7. Transition evaluation model

Commands that change managed-document state must evaluate a requested move in
three distinct steps:

```text
validate_transition(index, document, to_status)
    checks whether the requested status edge is legal for this DocType
    checks any transition-time gates tied to that edge

preview_transition(index, document, to_status)
    returns a predicted repository index with:
      - the moved document status updated
      - the moved document path resolved through the directory resolver
      - all other documents unchanged

validate_preview(preview_index)
    runs repository-level validation on the predicted post-transition state
```

This split keeps two rule classes separate:

- **steady-state validity** answers whether the repository is valid now
- **transition-time gates** answer whether one specific status move may happen now

At minimum, the shared document model must enforce these transition-time gates:

- `Prd -> Obsolete` is blocked while any live Design Doc still references that PRD
- `DesignDoc -> Implemented` is blocked while any referencing Exec Plan is not `completed`
- `DesignDoc -> Obsolete` is blocked while any live Exec Plan still references that Design Doc
- `DesignPatch -> ObsoleteMerged` requires a valid `merged-into` Design Doc reference
- `ExecPlan -> Completed` is blocked while any referencing Task Spec is not `completed`
- `ExecPlan -> Abandoned` is blocked while any live Task Spec still references that Exec Plan

Commands such as `specmate move` and `specmate run` must fail before writing if:

- the current repository is invalid, or
- the predicted post-transition repository would be invalid

The document model never performs implicit cascading transitions on related
documents. It only reports whether the requested move is legal.

---

## 8. Validation rules

The document model enforces these rules on every document it loads.
Violations produce structured errors that include the file path, the
field that failed, and the expected value.

| Rule | Applies to | Check |
|---|---|---|
| Title non-empty | docs that declare title | `title` field exists and is not blank where that DocType requires one |
| ID present | non-Guideline docs | `id` field exists in frontmatter |
| ID matches path | non-Guideline docs | frontmatter `id` matches the canonical ID derived from filename or fixed path |
| Valid status | docs with explicit status | `status` value is in the allowed set for this DocType |
| Guideline implicit active | Guideline | no `status` field is required; loaded status is `active` |
| Directory matches status | all managed docs | file location matches directory resolver output |
| merged-into present | DesignPatch with `obsolete:merged` | `merged-into` field exists and points to an existing doc |
| parent present | DesignPatch | `parent` field exists and points to an existing Design Doc |
| design-doc valid when present | ExecPlan, TaskSpec | if `design-doc` exists, it points to an existing Design Doc |
| superseded-by present | DesignDoc with `obsolete` via Flow B | `superseded-by` field exists and points to an existing doc |
| No stale live refs | live Design Docs, Exec Plans, Task Specs | live references (`prd`, `design-doc`, `exec-plan`) must not point to obsolete or abandoned parents; historical descendants may retain those links if the target still exists and has the correct type |
| One task upstream path | TaskSpec | a Task Spec must not declare both `exec-plan` and `design-doc` |
| Unique IDs | per DocType | no two documents of the same type share an ID |
| CC ids unique | TaskSpec | no two completion criteria share an `id` within a spec |
| Guideline files exist | TaskSpec | each `guidelines` path resolves to an existing Guideline |
| allowed non-empty | TaskSpec with `active` status | `boundaries.allowed` has at least one entry |
| specs locked | TaskSpec with `active` status | `boundaries.forbidden_patterns` includes `specs/**` |
| criteria non-empty | TaskSpec with `active` status | `completion_criteria` has at least one entry |

---

## 9. Association summaries

The shared document model must also expose read-only association summaries for
higher-level commands that want to report repository facts without redefining
relationship logic.

Supported direct associations:

- PRD ↔ Design Doc via `DesignDoc.prd`
- Design Doc ↔ Design Patch via `DesignPatch.parent`
- Design Doc ↔ Exec Plan via `ExecPlan.design-doc`
- Design Doc ↔ Task Spec via `TaskSpec.design-doc`
- Exec Plan ↔ Task Spec via `TaskSpec.exec-plan`

For each association set, the model must support aggregate predicates that are
useful to commands:

- all related documents are in a caller-specified status
- all related documents are terminal for their own document type
- no related documents exist

These summaries are facts only. They do not imply or trigger automatic status
changes.

---

## 10. ID allocation

When `specmate new` creates a document, it allocates the next available ID
for that DocType by scanning all existing documents across all subdirectories.

```
next_id(DocType) → u32:
  scan all files matching the DocType pattern in all known subdirectories
  parse the ID from each filename
  return max(found_ids) + 1, or 1 if no documents exist
```

IDs are never reused. A cancelled `task-0003` means `task-0004` is the next
ID, not a new `task-0003`.

For `DesignPatch`, the patch sequence number is scoped to the parent:

```
next_patch_number(parent_id) → u8:
  scan all patch files for this parent
  return max(found_patch_numbers) + 1, or 1 if no patches exist
```

---

## 11. Implementation responsibilities

The document model is a shared subsystem. `specmate check`, `specmate move`,
`specmate new`, and `specmate run` must all consume the same document-model
logic rather than reimplementing parsing or validation independently.

At minimum, the implementation must provide these capabilities:

- identify whether a path is a managed document path, and if so which `DocType`
  it belongs to
- derive the canonical document ID from filename or fixed path, then validate
  it against frontmatter where required
- parse frontmatter into a typed in-memory representation
- load a document into a validated `Document` value
- resolve the expected directory for a document from its `(DocType, Status)`
- validate whether a requested status transition is legal
- build a predicted repository index for a requested status transition
- validate the predicted repository index after a requested status transition
- allocate the next available document ID for a given `DocType`
- allocate the next patch sequence number for a given parent Design Doc
- expose direct-association summaries and aggregate predicates
- validate Task Spec runtime fields used by execution-time commands

The implementation must also expose a repository-level document index that can
represent:

- valid managed documents
- invalid entries found inside managed directories
- ignored files outside the managed document system

Invalid entries inside managed directories must remain visible to validation so
`specmate check` can report actionable errors. They must not be silently
dropped during indexing.

Operations that depend on document-model correctness must not proceed on top of
an already-invalid repository state. Commands that allocate IDs, create managed
documents, move managed documents, or transition managed document status must
first build and validate the repository document index. If the repository
contains invalid managed entries or repository-level validation violations,
those operations must fail and report the violations instead of inferring new
document state from inconsistent input.

---

## 12. Verification requirements

An implementation of this design is not considered complete unless the
document-model behaviour is verified through automated tests.

At minimum, tests must cover:

- filename parsing for every managed document type
- canonical ID matching between filename or fixed path and frontmatter `id`
- Guideline loading without `id` or `status`, with implicit `active` status
- rejection of malformed managed filenames inside managed directories
- ignoring of unrelated markdown files outside managed directories
- frontmatter validation for required and conditionally required fields
- directory resolution for every valid `(DocType, Status)` combination
- status transition validation for both legal and illegal transitions
- predicted-state validation for post-transition repository checks
- Task Spec runtime-field validation, including `guidelines`, `boundaries`,
  and `completion_criteria`
- association-summary queries for each supported direct-association type
- ID allocation across mixed active, archived, and obsolete documents
- patch sequence allocation scoped to a parent Design Doc

Command-level tests for `check`, `move`, `new`, and `run` should verify that
those commands reuse the document model rather than implementing divergent
parsing or validation rules.

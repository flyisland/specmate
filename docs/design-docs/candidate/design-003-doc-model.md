---
id: design-003
title: "Document Model"
status: candidate
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
DesignPatch:  Draft | Candidate | Implemented | ObsoleteMerged
ExecPlan:     Draft | Active | Completed | Abandoned
TaskSpec:     Draft | Active | Completed | Cancelled
ProjectSpec:  Active          (single status, no transitions)
OrgSpec:      Active          (single status, no transitions)
Guideline:    Active          (single status, no transitions)
```

### DocId

The identifier for a document, parsed from its filename.

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
  status:    Status
  title:     String
  path:      PathBuf          # absolute path to the file
  frontmatter: Frontmatter    # all parsed frontmatter fields
  raw:       String           # original file content
```

---

## 2. Frontmatter

Frontmatter is the YAML block between the opening `---` and the first closing
`---` in a `.md` file. It is the source of truth for all document metadata.

### Required fields (all document types)

| Field | Type | Constraint |
|---|---|---|
| `title` | string | non-empty |
| `status` | string | must be a valid status for this DocType |

`id` is derived from the filename, not stored in frontmatter.

### Optional fields by DocType

**DesignDoc / DesignPatch**

| Field | Type | Meaning |
|---|---|---|
| `module` | string | the codebase module this design covers |
| `prd` | string | linked PRD id (e.g. `prd-001`) |
| `parent` | string | patch only — parent design doc id |
| `merged-into` | string | patch only — required when status is `obsolete:merged` |
| `superseded-by` | string | required when status is `obsolete` via Flow B |

**ExecPlan**

| Field | Type | Meaning |
|---|---|---|
| `design-doc` | string | linked Design Doc id |

**TaskSpec**

| Field | Type | Meaning |
|---|---|---|
| `exec-plan` | string | linked Exec Plan id |
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

Document type and ID are derived from the filename, not from frontmatter.
The filename is the canonical identifier — frontmatter `id` is not stored
(it would be redundant and a source of inconsistency).

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

Files that do not match any pattern are ignored by specmate.

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
| `candidate` | `implemented` | |
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

## 7. Validation rules

The document model enforces these rules on every document it loads.
Violations produce structured errors that include the file path, the
field that failed, and the expected value.

| Rule | Applies to | Check |
|---|---|---|
| Title non-empty | all | `title` field exists and is not blank |
| Valid status | all | `status` value is in the allowed set for this DocType |
| Directory matches status | all | file location matches directory resolver output |
| merged-into present | DesignPatch with `obsolete:merged` | `merged-into` field exists and points to an existing doc |
| superseded-by present | DesignDoc with `obsolete` via Flow B | `superseded-by` field exists and points to an existing doc |
| No stale refs | candidate, implemented | referenced docs (`prd`, `design-doc`, `exec-plan`) are not `obsolete` or `obsolete:merged` |
| Unique IDs | per DocType | no two documents of the same type share an ID |
| CC ids unique | TaskSpec | no two completion criteria share an `id` within a spec |
| Guideline files exist | TaskSpec | each `guidelines` path resolves to an existing Guideline |
| allowed non-empty | TaskSpec with `active` status | `boundaries.allowed` has at least one entry |
| specs locked | TaskSpec with `active` status | `boundaries.forbidden_patterns` includes `specs/**` |
| criteria non-empty | TaskSpec with `active` status | `completion_criteria` has at least one entry |

---

## 8. ID allocation

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

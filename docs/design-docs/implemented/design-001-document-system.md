---
id: design-001
title: "Document System Design"
status: implemented
---

# Document System Design

This document defines the document types, naming conventions, status lifecycles,
and directory structure that form the foundation of the document-driven AI coding
system. It is tool-agnostic — any tooling built on top of this system must
conform to these definitions, not the other way around.

---

## 1. Document types

All documents use `.md` extension. Each document's type and status are defined
in its YAML frontmatter — frontmatter is the source of truth. The directory a
file lives in must reflect its frontmatter status. When they diverge, the
frontmatter is authoritative and the file location is the error to fix.

### Two categories

**Comprehension docs** — read by humans and agents to understand intent,
context, and decisions.

| Type | Purpose |
|---|---|
| PRD | Why to build it, for whom, business acceptance criteria |
| Design Doc | How the system is designed, technical decisions, module boundaries |
| Design Patch | An incremental change to an existing Design Doc |
| Exec Plan | Execution order, task dependencies, overall progress |
| Guideline | Cross-cutting concerns — principles and standards that apply across all modules (security, reliability, quality, etc.) |

**Verification docs** — machine-parseable, enforced by tooling.
Located under `specs/`.

| Type | Purpose |
|---|---|
| Task Spec | Single task authorisation: intent, boundaries, completion criteria |
| project.md | Project-wide technical constraints |
| org.md | Organisation-wide constraints (security, compliance) |

---

## 2. File naming

```
prd-001-<slug>.md
design-001-<slug>.md
design-001-patch-01-<slug>.md
exec-001-<slug>.md
task-0001-<slug>.md
docs/guidelines/<slug>.md        # no ID, no status transitions
```

**ID rules**

- Task Specs use **four digits** (`task-0001`). All other types use **three digits** (`prd-001`).
- IDs are globally incremented per document type, never reused.
- IDs are assigned in creation order and are permanent. `prd-001` is always
  `prd-001` regardless of whether the feature it describes still exists.

**Slug rules**

- Lowercase, hyphen-separated
- Verb + noun structure describing what it does, not what it is
- Maximum 5 words
- Good: `implement-auth-register`, `add-email-verification`, `fix-duplicate-email-check`
- Bad: `auth`, `AuthRegisterDesign`, `the-implementation-of-user-registration-feature`

**Patch doc naming**

A patch doc is bound to its parent by name and carries a two-digit sequence number:

```
design-001-billing.md                          ← parent
design-001-patch-01-remove-username.md         ← first patch
design-001-patch-02-add-multi-currency.md      ← second patch
```

Multiple patches can coexist. Each patch represents one independent change intent.
The two-digit sequence number ensures uniqueness independent of slug content.

---

## 3. Status lifecycles

Each document type has its own status vocabulary chosen to match its semantics.
Statuses are stored in the YAML frontmatter `status` field.

### PRD

```
draft → approved → obsolete
```

| Status | Meaning |
|---|---|
| `draft` | Being written. Not ready for design or implementation decisions. |
| `approved` | Signed off. Design Docs and Exec Plans can be created against this PRD. |
| `obsolete` | Feature killed or direction cancelled. No active doc should reference an obsolete PRD. |

### Design Doc

```
draft → candidate → implemented → obsolete
```

Patch doc terminal status: `obsolete:merged`

| Status | Meaning |
|---|---|
| `draft` | Design being written. Agents must not execute against a draft design. |
| `candidate` | Design finalised. Codebase not yet implemented. Agent's job: *implement this design*. Exec Plans and Task Specs are created at this stage. |
| `implemented` | Codebase fully consistent with this document. Any divergence is a bug, not a doc issue. Only one `implemented` Design Doc per module at any time. |
| `obsolete` | Module removed from codebase entirely. No replacement doc exists. |
| `obsolete:merged` | Patch doc only. Content has been merged back into the parent Design Doc. Requires `merged-into: <doc-id>` in frontmatter. Parent doc remains `implemented` and is the sole source of truth going forward. |

**Key rule**: `implemented` means the document IS the source of truth.
`ls docs/design-docs/implemented/` returns all current design contracts without
parsing any files. Any divergence between code and an `implemented` doc is a bug.

### Exec Plan

```
draft → active → completed
                ↘ abandoned
```

| Status | Meaning |
|---|---|
| `draft` | Tasks and dependencies being planned. Not yet executable. |
| `active` | Execution in progress. Tasks running in dependency order. |
| `completed` | All phases done. Corresponding Design Doc can now move to `implemented`. |
| `abandoned` | Stopped mid-execution. Must record which phases completed and why stopped. Partial work preserved in codebase. |

### Task Spec

```
draft → active → completed
                ↘ cancelled
```

| Status | Meaning |
|---|---|
| `draft` | Intent and criteria being written. Must not be executed against. |
| `active` | Human-approved. Execution can start. Spec is locked — must not be modified during execution. |
| `completed` | All completion criteria passed and changes committed. |
| `cancelled` | Decided not to implement. Must record reason and downstream tasks affected. |

### project.md / org.md

Always `active`. Created and immediately in effect.
Changes tracked via version control history. No status transitions needed.

### Guideline

Always `active`. No status transitions.

Guidelines describe cross-cutting concerns — security, reliability, quality
standards, and similar principles that span multiple modules. They are
continuously evolving active documents, not something that gets "implemented"
or "deprecated". Changes are made in place and tracked via version control.

Unlike Design Docs, Guidelines have no `implemented` state because they are
not tied to a specific codebase implementation. They define *how the system
should behave* across all modules, not *what a specific module does*.

Agents consult Guidelines when they are referenced in a Task Spec's
`guidelines` field or when the AGENTS.md index indicates they are relevant
to the task at hand.

---

## 4. Design Doc change flows

### Flow A — patch: incremental change to an existing design

Use when the change is bounded — adding a field, adjusting behaviour,
updating a decision. The parent doc remains the primary document throughout.

```
design-001                     implemented  (unchanged throughout)
design-001-patch-01-<slug>     draft → candidate → implemented → obsolete:merged
```

During implementation, agent reads both:
- `design-001` — current state of the system
- `design-001-patch-01` — the delta to apply

When the patch is complete:
1. Merge patch content back into `design-001`
2. Mark patch as `obsolete:merged` with `merged-into: design-001`
3. `design-001` stays `implemented` — updated content, same status

**Result**: one module, one document, always.

### Flow B — supersede: full redesign of a module

Use when the change is large enough that a fresh document is clearer than
patching the old one. Breaking interface changes, architectural rewrites.

```
design-001     implemented → obsolete  (frozen, never modified again)
               requires: superseded-by: design-015
design-015     draft → candidate → implemented
```

### Flow C — deprecate: module removed entirely

```
design-001     implemented → obsolete
```

No replacement doc. Module deleted from codebase.

### Flow D — codebase diverged from an implemented doc

| Situation | Action |
|---|---|
| Code is right, doc is stale | Treat as Flow A. Patch the doc first, then implement. Doc is always updated before code. |
| Doc is right, code diverged | Create a Task Spec to fix the code. Doc stays `implemented` and is the source of truth. The divergence is a bug. |

---

## 5. Directory structure

**Frontmatter is the source of truth. Directory is its physical reflection.**

The subdirectory a file lives in must match its frontmatter `status` field.
This makes the file system directly readable without parsing any files.
Status transitions must update both the frontmatter and the file location
atomically — never move files manually without updating frontmatter, and
never update frontmatter without moving the file.

```
repo/
├── AGENTS.md
├── specs/
│   ├── project.md                   # always active
│   ├── org.md                       # always active
│   ├── active/                      # draft + active task specs
│   └── archived/                    # completed + cancelled
└── docs/
    ├── guidelines/                  # cross-cutting concerns, always active
    │   ├── security.md
    │   ├── reliability.md
    │   └── ...
    ├── prd/
    │   ├── draft/
    │   ├── approved/
    │   └── obsolete/
    ├── design-docs/
    │   ├── draft/
    │   ├── candidate/
    │   ├── implemented/             # ← ls here = all current design contracts
    │   └── obsolete/                # obsolete + obsolete:merged
    └── exec-plans/
        ├── draft/
        ├── active/
        └── archived/                # completed + abandoned
```

**Status to directory mapping**

| Document type | Status | Directory |
|---|---|---|
| PRD | `draft` | `docs/prd/draft/` |
| PRD | `approved` | `docs/prd/approved/` |
| PRD | `obsolete` | `docs/prd/obsolete/` |
| Design Doc | `draft` | `docs/design-docs/draft/` |
| Design Doc | `candidate` | `docs/design-docs/candidate/` |
| Design Doc | `implemented` | `docs/design-docs/implemented/` |
| Design Doc | `obsolete` / `obsolete:merged` | `docs/design-docs/obsolete/` |
| Exec Plan | `draft` | `docs/exec-plans/draft/` |
| Exec Plan | `active` | `docs/exec-plans/active/` |
| Exec Plan | `completed` / `abandoned` | `docs/exec-plans/archived/` |
| Task Spec | `draft` / `active` | `specs/active/` |
| Task Spec | `completed` / `cancelled` | `specs/archived/` |
| Guideline | `active` (always) | `docs/guidelines/` |

---

## 6. Frontmatter reference

### PRD

```yaml
---
id: prd-001
title: "User Registration"
status: draft        # draft | approved | obsolete
created: 2026-03-01
author: "@username"
design-doc: design-001   # set when approved
---
```

### Design Doc

```yaml
---
id: design-001
title: "Auth System Design"
status: candidate    # draft | candidate | implemented | obsolete
module: auth
prd: prd-001
---
```

### Design Patch

```yaml
---
id: design-001-patch-01-remove-username
title: "Remove username field from auth"
status: candidate    # draft | candidate | implemented | obsolete:merged
parent: design-001
merged-into: design-001   # required when status is obsolete:merged
---
```

### Exec Plan

```yaml
---
id: exec-001
title: "Auth System Implementation"
status: active       # draft | active | completed | abandoned
design-doc: design-001
---
```

### Task Spec

```yaml
---
id: task-0001
title: "Implement init command"
status: draft        # draft | active | completed | cancelled
exec-plan: exec-001
guidelines:          # optional — guideline files relevant to this task
  - docs/guidelines/security.md
  - docs/guidelines/reliability.md
boundaries:
  allowed:
    - "src/cmd/init.rs"
    - "tests/cmd/init_test.rs"
  forbidden_patterns:
    - "specs/**"
completion_criteria:
  - id: "cc-001"
    scenario: "Init succeeds in an empty directory"
    test: "test_init_creates_full_directory_structure"
---
```

The `guidelines` field is optional. When present, the listed files are
injected into the coding agent and review agent context at execution time.
The review agent verifies that the implementation conforms to the referenced
guidelines as part of its review pass.

### Guideline

```yaml
---
title: "Security"
---
```

No `id`, no `status`. Guidelines are always active and live directly in
`docs/guidelines/`. Changes are made in place; version control tracks history.

### project.md / org.md

```yaml
---
id: project
status: active
---
```

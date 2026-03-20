# specmate

> Your docs don't enforce themselves. **specmate does.**

Structure, lifecycle, and agent loops — all in one CLI tool for document-driven AI coding.

---

## What it does

### 01 · Document structure

Create PRDs, Design Docs, Exec Plans and Task Specs with auto-assigned IDs. Validate naming, frontmatter, cross-references, and status-to-directory consistency.

### 02 · Agent dev loop

Drive the code → verify → review → repeat cycle via ACP. Orchestrates coding and review agents against Task Specs until all completion criteria pass.

### 03 · Mechanical checks

Turn every enforceable rule from your doc system into a CLI command. Boundary violations, forbidden patterns, stale references — caught before they reach CI.

---

## Commands

### `new` — create docs with auto ID

```
specmate new prd <slug>          # New PRD under docs/prd/draft/
specmate new design <slug>       # New Design Doc under docs/design-docs/draft/
specmate new plan <slug>         # New Exec Plan under docs/exec-plans/draft/
specmate new task <slug>         # New Task Spec under specs/active/
specmate new patch <design-id>   # New patch doc for an existing Design Doc
```

### `move` — kanban-style status transition

Atomically updates frontmatter status and moves the file to the correct subdirectory.

```
specmate move design-001 candidate    # Design finalised, ready for implementation
specmate move task-008 active         # Human sign-off — unlocks agent loop
specmate move design-001 implemented  # Codebase now consistent with design
specmate move exec-001 archived       # Prompts for reason, then archives
```

### `check` — mechanical validation

Safe to run anytime. CI-friendly, zero side effects.

```
specmate check                        # Full sweep — all rules across all docs
specmate check names                  # Filename pattern validation
specmate check status                 # Status ↔ directory consistency
specmate check refs                   # Dead or obsolete cross-references
specmate check boundaries <task-id>   # Changed files vs Task Spec boundaries
specmate check conflicts              # Overlapping boundaries across active specs
specmate check patterns               # Forbidden patterns in source code
```

### `run` — agent dev loop via ACP

```
specmate run <task-id>                # Full loop: code → verify → review → repeat
specmate run <task-id> --code-only    # Skip review pass
specmate run <task-id> --review-only  # Review current state without re-coding
specmate run plan <plan-id>           # Execute all tasks respecting dependencies
```

### `status` — visibility across the doc system

```
specmate status                       # Overview of all docs by type and status
specmate status plan <id>             # Exec Plan progress and dependency graph
specmate status design <id>           # Design Doc + all active patches
specmate status stale                 # Overdue spikes, orphaned docs, plans with all tasks done
```

---

## Status lifecycles

Each document type has its own status vocabulary, chosen to match its semantics.

### PRD

```
draft → approved → obsolete
```

| Status | Meaning |
|---|---|
| `draft` | Requirements being written. Not ready for design or implementation. |
| `approved` | Requirements signed off. Design Docs and Exec Plans can be created. |
| `obsolete` | Feature killed or direction cancelled. No active doc should reference this. |

### Design Doc

```
draft → candidate → implemented → obsolete
                                ↘ (patch docs only) obsolete:merged
```

| Status | Meaning |
|---|---|
| `draft` | Design being written. Agents must not execute against a draft design. |
| `candidate` | Design finalised. Codebase not yet implemented. Agent's job: *implement this design*. |
| `implemented` | Codebase fully consistent with this document. Any divergence is a bug, not a doc issue. Only one `implemented` doc per module at any time. |
| `obsolete` | Module removed from codebase entirely. No replacement doc exists. |
| `obsolete:merged` | Patch doc whose content has been merged back into the parent Design Doc. Requires `merged-into: <doc-id>` in frontmatter. |

### Exec Plan

```
draft → active → completed
                ↘ abandoned
```

| Status | Meaning |
|---|---|
| `draft` | Tasks and dependencies being planned. Not yet executable. |
| `active` | Execution in progress. |
| `completed` | All phases done. Design Doc can now move to `implemented`. |
| `abandoned` | Stopped mid-execution. Must record which phases completed and why stopped. |

### Task Spec

```
draft → active → completed
                ↘ cancelled
```

| Status | Meaning |
|---|---|
| `draft` | Intent and criteria being written. `specmate run` will refuse to start. |
| `active` | Human-approved. Agent loop can start. Spec is locked — agents must not modify it. |
| `completed` | All completion criteria passed and PR merged. |
| `cancelled` | Decided not to implement. Must record reason and downstream tasks affected. |

### project.spec / org.spec

Always `active`. Created and immediately in effect. Changes tracked via git history, no status transitions needed.

---

## Directory structure

Directory is status. `ls` is the answer. No frontmatter parsing needed.

```
repo/
├── specs/
│   ├── project.md          # always active, no subdirs
│   ├── org.md              # always active, no subdirs
│   ├── active/             # draft + active task specs
│   └── archived/           # completed + cancelled
└── docs/
    ├── prd/
    │   ├── draft/
    │   ├── approved/
    │   └── obsolete/
    ├── design-docs/
    │   ├── draft/
    │   ├── candidate/
    │   ├── implemented/    # ← ls here = all current design contracts
    │   └── obsolete/       # obsolete + obsolete:merged
    └── exec-plans/
        ├── draft/
        ├── active/
        └── archived/       # completed + abandoned
```

---

## Design Doc change flows

### Flow A — patch: incremental change to an existing design

```
design-001                 implemented  (stays implemented — still accurate)
design-001-patch-<slug>    draft → candidate → implemented → obsolete:merged
```

During implementation, agent reads `design-001` (current state) and `design-001-patch` (delta) together. When the patch is complete, its content is merged back into `design-001`, and the patch moves to `obsolete:merged`. `design-001` remains `implemented` — updated content, same status.

### Flow B — supersede: full redesign of a module

When the change is large enough to warrant a fresh document:

```
design-001     implemented → obsolete  (frozen, never modified again)
design-015     draft → candidate → implemented
```

### Flow C — module removed entirely

```
design-001     implemented → obsolete
```

No replacement doc. Module deleted from codebase.

### Flow D — codebase diverged from an implemented doc

| Situation | Action |
|---|---|
| Code is right, doc is stale | Treat as Flow A. Patch the doc first, then implement. |
| Doc is right, code diverged | Create a Task Spec to fix the code. Doc stays `implemented` and is the source of truth. |

---

## File naming

```
prd-001-<slug>.md
design-001-<slug>.md
design-001-patch-<slug>.md
exec-001-<slug>.md
task-0001-<slug>.md          # four digits — tasks are high-frequency
```

IDs are globally incremented per document type, never reused. `specmate new`
assigns the next available ID automatically.

**Slug rules:** lowercase, hyphen-separated, verb + noun, max 5 words.
Good: `implement-auth-register`, `add-email-verification`
Bad: `auth`, `AuthRegisterDesign`, `the-implementation-of-user-registration`



| Command | Rule |
|---|---|
| `check status` | File must live in the subdirectory matching its status. Mismatch is a blocking CI error. |
| `check refs` | A `candidate` or `implemented` doc must not reference any `obsolete` doc. |
| `run` | Agent loop refuses to start if Task Spec status is not `active`. Explicit `specmate move task-0001 active` required after human review. |
| `done:merged` | Requires `merged-into: <doc-id>` in frontmatter. `check frontmatter` fails if missing. |
| `implemented` | Only one `implemented` Design Doc per module allowed. `check` detects duplicates. |

---

## Main scenario walkthrough

### New feature: design → implement → codebase in sync

```
specmate new design "billing"
# design-001-billing.md created in docs/design-docs/draft/

specmate move design-001 candidate
# Design finalised, ready for implementation

specmate new plan "billing-impl"
specmate move exec-001 active
# Exec Plan created and started

specmate new task "create-billing-table"
specmate new task "implement-billing-service"
specmate new task "add-billing-api"
specmate move task-0001 active
specmate move task-0002 active
specmate move task-0003 active

specmate run plan exec-001
# Runs tasks in dependency order via ACP

specmate move exec-001 completed
specmate move design-001 implemented
# Codebase now consistent with design-001. Any divergence from here is a bug.
```

### Modify existing design

```
specmate new patch design-001 "remove-username"
# design-001-patch-remove-username.md created in docs/design-docs/draft/

specmate move design-001-patch-remove-username candidate
# Patch finalised

specmate new task "remove-username-from-billing"
specmate move task-0008 active

specmate run task-0008
# Agent reads design-001 (current state) + patch (delta) together

# After task completes:
# 1. Merge patch content back into design-001
specmate move design-001-patch-remove-username obsolete:merged
# design-001 stays implemented, content updated
```

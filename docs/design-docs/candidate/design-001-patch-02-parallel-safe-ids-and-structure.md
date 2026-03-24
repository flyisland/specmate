---
id: design-001-patch-02
title: "Branch-friendly IDs, exec-as-directory, and unified status vocabulary"
status: candidate
parent: design-001
---

# Patch: Branch-friendly IDs, exec-as-directory, and unified status vocabulary

This patch addresses three structural problems discovered in practice:

1. Sequential numeric IDs assume a serialised creation model that breaks under
   parallel git branches.
2. Exec Plans and Task Specs placed in status-based subdirectories cause
   cascading path changes when status transitions occur, and destroy git history
   for files nested inside a moved directory.
3. Exec Plan and Task Spec status vocabularies are inconsistent with each other
   and with Design Doc vocabulary, forcing tooling to handle multiple state
   machine shapes.

---

## Changes to Section 2 — File naming

### 2.1  ID strategy per document type

Replace the single "globally incremented numeric ID" rule with a per-type
strategy based on the nature of each document type.

**Knowledge artefacts** (PRD, Design Doc, Design Patch, Exec Plan) use a
**slug as their primary identifier**. Slug is the value used in all
cross-document references.

When a proposed slug collides with an existing document of the same type, the
new document must be renamed before merge. The tool must not silently reuse or
alias an occupied slug. The conflict-resolution rule is explicit rename, not
sequence allocation.

```
# Before
prd-001-<slug>.md
design-001-<slug>.md
exec-001-<slug>.md

# After
prd-<slug>.md
design-<slug>.md
exec-<slug>/          ← directory, see Section 5
```

**Work artefacts** (Task Spec) use a **locally scoped decimal sequence
number**, unique only within the containing Exec Plan directory. The displayed
form is zero-padded to **at least two digits** (`01`, `02`, ..., `99`, `100`,
...). A Task Spec's full identifier is `<exec-slug>/task-<nn>`.

All CLI-facing and user-facing references to a Task Spec use that full
identifier, not bare `task-<nn>`. Standalone Task Specs are not supported in
this model.

When a git-facing or other single-token surface cannot safely use the slash
form directly, tooling must use the stable escaped rendering
`<exec-id>--task-<nn>`. This applies to branch names, report stems, and similar
single-token outputs. Human-readable CLI output should still prefer the
canonical `<exec-id>/task-<nn>` form.

If multiple branches independently allocate the same next Task number inside
the same Exec Plan, the branch that merges later must rename its Task Spec to
the next available sequence before merge and update any references in that
branch. The conflict-resolution rule is explicit re-sequencing at merge time.

This design removes the need for a repository-wide global ID counter, but it
does not eliminate all merge-time renaming. Parent-scoped design patches and
exec-scoped Task Specs may still need local re-sequencing when concurrent
branches touch the same parent design or Exec Plan.

```
# Before
task-0001-<slug>.md     ← globally unique four-digit number

# After (inside docs/exec-plans/exec-<slug>/)
task-01-<slug>.md       ← zero-padded local sequence, minimum width 2
```

**Rationale for the split**: Knowledge artefacts are referenced across many
documents over long periods; their identifiers must be stable and semantic.
Work artefacts are referenced only within their parent Exec Plan; global
uniqueness is unnecessary overhead, and scoped numbering is sufficient for the
local reference model even though concurrent branches may still require
merge-time re-sequencing.

### 2.2  Design Patch naming — parent-scoped sequence retained

Design Patch naming keeps a parent-scoped decimal patch sequence number with a
minimum display width of two digits:

```
design-<slug>.md                            ← parent
design-<slug>-patch-01-<patch-slug>.md      ← first patch
design-<slug>-patch-02-<patch-slug>.md      ← second patch
design-<slug>-patch-100-<patch-slug>.md     ← after 99, width expands
```

If multiple branches independently allocate the same next patch number for the
same parent design, the branch that merges later must rename its patch to the
next available sequence before merge and update any references in that branch.
The conflict-resolution rule is explicit re-sequencing at merge time.

### 2.3  Cross-document reference fields

All managed-document reference fields now use the target document's canonical ID
in the new model, not the legacy numeric-ID form.

This includes at least:

- `prd`
- `design-doc`
- `design-docs`
- `exec-plan`
- `parent`
- `merged-into`
- `superseded-by`

For PRD, Design Doc, and Exec Plan this means slug-based IDs. For Design Patch
references this means the full patch canonical ID
`design-<parent-slug>-patch-<nn>-<patch-slug>`. If a future field references a
Task Spec, it must use the repo-wide Task canonical form `<exec-id>/task-<nn>`.

---

## Changes to Section 3 — Status lifecycles

### 3.0  Global rule: mutability by status

Add the following as a preamble to Section 3, applying to all document types.

**`draft`** — freely editable. The document is still being written.

**`candidate`** — approved and executable. A `candidate` document may still be
edited in place while work proceeds. This is deliberate: implementation often
discovers information that should refine the design or work plan before the
system reaches a terminal state.

Tooling may also update `closed` as part of the defined close workflow.

**Terminal statuses** (`implemented`, `closed`, `obsolete`, `obsolete:merged`,
`approved`) — must not be directly modified. The correct action depends on
document type:

| Terminal document | How to make changes |
|---|---|
| `implemented` Design Doc | Create a patch doc (Flow A) or supersede (Flow B) |
| `closed` Task Spec | Do not modify. It is a historical record. If the implementation needs adjustment, create a new Task Spec. |
| `closed` Exec Plan | Do not modify. If work needs to continue, create a new Exec Plan. |
| `obsolete` any doc | Do not modify. It is frozen. |
| `approved` PRD | Do not modify. If requirements change, create a new PRD or a superseding PRD and mark this one `obsolete`. |

This rule is a hard constraint for agents. An agent that identifies an error
in a terminal document must not fix it in place — it must surface the issue
to a human and await instruction before any change flow begins.

**One-time migration exception**: when adopting this patch from the older
numeric-ID repository model, the repository may perform a single atomic
migration that rewrites existing managed docs in place solely to adopt the new
IDs, paths, and required metadata. This exception applies only during that
one-time cutover. After the cutover completes, the normal mutability rules
apply.

### 3.1  Exec Plan status

Replace the four-status model with three statuses aligned with Design Doc
vocabulary.

```
# Before
draft → active → completed
                ↘ abandoned

# After
draft ↔ candidate
  ↘         ↘
    closed
```

| Status | Meaning |
|---|---|
| `draft` | Plan being written. Not yet executable. |
| `candidate` | Human-approved. Execution can begin. Still editable while implementation proceeds. |
| `closed` | Terminal. No further action required. |

**Direct close path**: `draft -> closed` is legal when a plan is intentionally
dropped before approval but should remain in history as a real abandoned work
artefact rather than lingering forever as an open draft.

**Rollback path**: `candidate -> draft` is legal when an approved plan should
stop being acted on and return to the writing stage. It is not required for
ordinary in-progress refinement.

**`abandoned` is removed.** It attempted to encode narrative information
(partial completion, reasons for stopping) into an enum value. That information
belongs in the document body, not in the status field. A tool receiving
`abandoned` could only treat it as a terminal state — identical to `completed`.
The distinction carries no actionable signal for any reader, human or agent.

**Why `candidate` instead of `active`**: Unifies vocabulary with Design Doc.
`candidate` consistently means "human-approved, ready for agents to act on"
across all document types.

**Why `closed` instead of `completed`/`abandoned`**: `closed` is a historical
claim — "this work happened". It makes no assertion about the current state of
the codebase. This is the correct semantic for a work artefact. `completed`
implies the work's effects are still in force, which cannot be guaranteed as
later tasks may reverse them.

On any transition into `closed`, tooling must set `closed` as part of the same
terminalization workflow.

### 3.2  Task Spec status

Replace the four-status model with three statuses using the same vocabulary.

```
# Before
draft → active → completed
                ↘ cancelled

# After
draft ↔ candidate
  ↘         ↘
    closed
```

| Status | Meaning |
|---|---|
| `draft` | Spec being written. Must not be executed against. |
| `candidate` | Human-approved. Agent may begin execution. Still editable while implementation proceeds. |
| `closed` | Terminal. All criteria passed, or task decided against. Reason in doc body. |

**Direct close path**: `draft -> closed` is legal when a task idea is
intentionally dropped before approval but should remain in history as a real
discarded work item.

**Rollback path**: `candidate -> draft` is legal when an approved task spec
should stop being acted on and return to the writing stage. It is not required
for ordinary in-progress refinement.

For Task Specs, "still editable" does not mean "silently rewrite the contract
mid-run". Clarifying edits are fine, but any change that materially alters the
execution contract — especially `boundaries`, dependencies, or
`completion_criteria` — requires renewed human confirmation before execution
continues under the revised spec. This patch deliberately keeps that as a
process rule, not a separate lifecycle status.

Operationally, `specmate run` and `specmate rerun` always execute against the
current committed text of the Task Spec. If a human materially revises a
candidate Task Spec after an earlier approval decision, the next explicit human
`run`/`rerun` invocation is the renewed confirmation event for that committed
revision. Tooling does not attempt to reconstruct approval history beyond that
committed execution boundary.

**`completed` and `cancelled` are merged into `closed`** for the same reason as
above: the distinction between "finished successfully" and "deliberately
stopped" is narrative, not structural. It belongs in the document body or git
history, not in a status field that tooling must branch on. As a consequence,
tooling must not use `closed` alone to prove downstream semantic claims such as
"this Design Doc is now implemented".

For dependency gating, this means predecessor relationships are only
serialization rules: downstream work may require predecessor Task Specs to be
terminal (`closed`), but tooling must not infer successful delivery from that
fact alone. A closed predecessor satisfies only the mechanical ordering gate,
not semantic readiness. Continuing with downstream work remains an explicit
human decision at `run`/`rerun` time. If humans decide that a closed
predecessor invalidates downstream work, they must revise or close the
downstream Task Specs explicitly.

On any transition into `closed`, tooling must set `closed` as part of the same
terminalization workflow.

### 3.3  Design Doc and PRD status — unchanged, with clarification on `draft`

Design Doc retains `draft → candidate → implemented → obsolete`.

`implemented` is a **stateful claim**: the document is the current source of
truth, maintained by humans, and any divergence between code and document is a
bug. This is fundamentally different from Task Spec's `closed`, which is a
**historical claim** with no ongoing maintenance obligation.

PRD retains `draft → approved → obsolete`. `approved` represents a real human
decision gate that must not be implicitly collapsed.

**`draft` as inbox**: For PRD and Design Doc, `draft` deliberately serves a
dual role. It is both "document being actively written" and **"inbox for
unformed ideas"** — a half-formed thought, a question worth capturing, a
direction not yet ready to discuss. A `draft` document carries no commitment:
it does not need to be coherent, complete, or even correct. The only obligation
of a `draft` is that it not be acted on.

This means it is always correct to create a `draft` document. The friction of
capture should be near zero. Promotion to `candidate` is where the thinking
gets done, not before.

### 3.4  Unified status signal for tooling

After this patch, all document types share a common tooling view of whether a
document is actionable, live source-of-truth, or historical:

| Status | Signal to agent | Signal to tooling |
|---|---|---|
| `draft` | Do not act | Skip |
| `candidate` | Act on this | Validate/enforce |
| `implemented` / `approved` | Do not implement from scratch; treat as current source of truth | Validate as live contract |
| `closed` / `obsolete` / `obsolete:merged` | Terminal | Archive/history |

---

## Changes to Section 4 — Design Doc change flows

### 4.1  Exec Plan to Design Doc relationship

Remove the implicit assumption that one Design Doc maps to one Exec Plan.

A single Design Doc will accumulate multiple Exec Plans over its lifetime:
initial implementation, incremental patches, and eventual rewrites. Each Exec
Plan represents a distinct delivery intent — what is being changed, not which
document it relates to. The Exec Plan slug should describe the delivery goal:

```
# Poor: describes the design document
exec-auth-system

# Good: describes the delivery intent
exec-auth-initial-implementation
exec-auth-add-oauth
exec-auth-migrate-to-argon2
```

### 4.2  `design-docs` field (plural)

The `design-doc` field on Exec Plan becomes `design-docs` (a list). A single
Exec Plan often implements a base Design Doc plus one or more patches
simultaneously; the agent needs to read all of them.

Discovery rules:

- module-scoped design docs and design patches are listed explicitly in
  `design-docs`
- if a design patch is listed in `design-docs`, its parent Design Doc must also
  be listed explicitly so execution context always includes the base contract as
  well as the delta
- implemented cross-cutting `design-principles-*.md` documents are always
  included in execution-time design context automatically
- any candidate `design-principles-*.md` document must be listed explicitly in
  `design-docs` before agents may act on it
- draft `design-principles-*.md` documents remain non-actionable and must not
  be included in execution context

```yaml
# Before
design-doc: design-001

# After
design-docs:
  - design-auth-system
  - design-auth-system-patch-01-add-oauth
```

### 4.3  Patch merge discipline

An implemented Design Doc may be changed through a Design Patch. This remains
the standard incremental-change flow.

However, a Design Patch is not intended to become a long-lived parallel source
of truth. Once a patch has been implemented and its effect is accepted, its
content should be merged back into the parent Design Doc, and the patch should
move to `obsolete:merged`.

The maintenance goal is simple: for a given module or domain topic, keep one
long-lived Design Doc as the primary source of truth whenever practical.
Patches are temporary change vehicles, not permanent replacements for
consolidated design ownership.

---

## Changes to Section 5 — Directory structure

### 5.1  Exec Plan as directory

Each Exec Plan is a directory. The plan itself is `plan.md` inside that
directory. Task Specs live directly alongside it.

```
docs/exec-plans/
├── exec-auth-initial-implementation/
│   ├── plan.md
│   ├── task-01-create-users-migration.md
│   └── task-02-implement-user-repo.md
├── exec-auth-add-oauth/
│   ├── plan.md
│   └── task-01-add-oauth-provider.md
└── exec-auth-migrate-argon2/
    ├── plan.md
    └── task-01-update-hash-algorithm.md
```

### 5.2  No status-based subdirectories for Exec Plans or Task Specs

Exec Plans and Task Specs do not use status-based subdirectories. Status lives
in frontmatter only. The directory structure reflects ownership and hierarchy,
not lifecycle state.

**Rationale**: Moving an Exec Plan directory on status transition cascades path
changes to every Task Spec inside it, breaking git history for all nested files.
The "ls to see active items" benefit does not justify this cost. Active items
are found via a script that reads frontmatter — a one-time investment that does
not require ongoing directory-move discipline.

```bash
# Find all candidate exec plans
grep -rl "status: candidate" docs/exec-plans/ --include="plan.md"

# Find all open tasks in a specific exec plan
grep -rl "status: candidate" docs/exec-plans/exec-auth-add-oauth/
```

### 5.3  `specs/` moved into `docs/`

With Task Specs moved into Exec Plan directories, the remaining contents of
`specs/` are only `project.md` and `org.md`. There is no longer a reason for
these to live at the repo root rather than alongside all other documentation.
Moving them into `docs/` makes the root cleaner and the docs directory the
single location for all document artefacts.

```
# Before
specs/
├── project.md
└── org.md

# After
docs/
└── specs/
    ├── project.md
    └── org.md
```

Any tooling or AGENTS.md references to `specs/project.md` or `specs/org.md`
must be updated to `docs/specs/project.md` and `docs/specs/org.md`.

The `forbidden_patterns` entry in Task Spec frontmatter should be updated
accordingly:

```yaml
# Before
forbidden_patterns:
  - "specs/**"

# After
forbidden_patterns:
  - "docs/specs/**"
```

Because Task Specs and `plan.md` now live under `docs/exec-plans/**`,
executable Task Specs must also treat managed control docs under
`docs/exec-plans/**` as protected. In practice, a running task must not modify
its owning `plan.md`, sibling task files, or unrelated Exec Plan directories
unless a later design explicitly introduces a safe exception.

More generally, executable Task Specs must not modify managed docs under
`docs/prd/**`, `docs/design/**`, `docs/exec-plans/**`, `docs/specs/**`, or
`docs/guidelines/**` unless a later design explicitly introduces a safe
document-authoring exception flow.

This boundary rule applies to agent-authored task changes. It does not forbid
specmate's own mechanical writes when finalising workflow state, such as
updating the owning Task Spec's frontmatter during `move`/`run` or writing an
execution report to the designated specmate-managed report path.

### 5.4  `docs/guidelines/` — operational standards, always injected

Add a `docs/guidelines/` directory for documents that tell agents **how to
do things**: coding standards, security rules, testing conventions. These are
operational references consulted while writing code.

```
docs/guidelines/
├── coding-standards.md
├── error-handling.md
├── security.md
└── obsolete/
```

**Injection rule**: all non-obsolete documents directly under
`docs/guidelines/` are automatically included in every agent's context at task
start. Files under `docs/guidelines/obsolete/` are explicitly excluded. The
directory is the declaration — no per-document frontmatter field is needed.

Guidelines documents do not use the `draft → candidate → implemented`
lifecycle. They are continuously maintained reference material, analogous to
`docs/specs/project.md`. Changes are made in place and tracked via git history.
Retired guidelines are moved to `docs/guidelines/obsolete/`.

### 5.5  `docs/design/` — cross-cutting design principles

Some design decisions are not scoped to a single module but inform how the
entire system is shaped. These belong in `docs/design/` alongside module-level
Design Docs, using the same lifecycle and the same naming convention.

```
docs/design/
├── implemented/
│   ├── design-auth-system.md              # module-scoped
│   ├── design-principles-domain.md        # cross-cutting
│   └── design-principles-errors.md        # cross-cutting
├── candidate/
└── draft/
```

These documents follow the full `draft → candidate → implemented` lifecycle.
A cross-cutting principle carries the same weight as a module design: it must
be human-approved (`candidate`) before agents act on it, and once `implemented`
it is the source of truth that code must conform to.

**Boundary with `docs/guidelines/`**

The distinction is what the document affects:

| Question | Answer | Location |
|---|---|---|
| Why is the system shaped this way? | Design reasoning, tradeoffs, intent | `docs/design/design-principles-*.md` |
| How should code be written? | Operational rules, patterns to follow or avoid | `docs/guidelines/` |

The same team consensus often has both a design-level and a guidelines-level
expression. They do not duplicate each other — they address different readers
at different moments.

**Example — error handling**

`docs/design/design-principles-errors.md` (`implemented`):
> Errors are domain concepts, not exceptions. Recoverable failures are part of
> the domain model and must be expressed explicitly in return types. This shapes
> how modules define their interfaces and how callers reason about failure.

`docs/guidelines/error-handling.md`:
> Use `Result<T, AppError>` for all fallible functions. Never use `.unwrap()`
> or `.expect()` in production code. Log errors at the boundary where they are
> handled, not where they originate.

The design principle explains *why* the system treats errors this way. The
guideline tells the agent *what* to write. An agent needs both: the principle
to make good architectural decisions, the guideline to write conformant code.

### 5.6  Updated full directory structure

```
repo/
├── AGENTS.md
└── docs/
    ├── specs/
    │   ├── project.md                 # always active
    │   └── org.md                     # always active
    ├── guidelines/                    # always injected into agent context
    │   ├── coding-standards.md
    │   ├── error-handling.md
    │   ├── security.md
    │   └── obsolete/
    ├── prd/
    │   ├── draft/
    │   ├── approved/
    │   └── obsolete/
    ├── design/
    │   ├── draft/
    │   ├── candidate/
    │   ├── implemented/               # ← ls here = all current design contracts
    │   └── obsolete/
    └── exec-plans/
        ├── exec-<slug>/
        │   ├── plan.md
        │   └── task-<nn>-<slug>.md
        └── ...
```

Replace the previous 5.4 directory structure and status-to-directory mapping
table with this updated version.

| Document type | Status | Directory |
|---|---|---|
| PRD | `draft` | `docs/prd/draft/` |
| PRD | `approved` | `docs/prd/approved/` |
| PRD | `obsolete` | `docs/prd/obsolete/` |
| Design Doc / Design Principles | `draft` | `docs/design/draft/` |
| Design Doc / Design Principles | `candidate` | `docs/design/candidate/` |
| Design Doc / Design Principles | `implemented` | `docs/design/implemented/` |
| Design Doc / Design Principles | `obsolete` | `docs/design/obsolete/` |
| Design Patch | `draft` | `docs/design/draft/` |
| Design Patch | `candidate` | `docs/design/candidate/` |
| Design Patch | `implemented` | `docs/design/implemented/` |
| Design Patch | `obsolete` / `obsolete:merged` | `docs/design/obsolete/` |
| Guidelines | n/a (continuously maintained) | `docs/guidelines/` |
| Exec Plan | any status | `docs/exec-plans/<exec-slug>/plan.md` |
| Task Spec | any status | `docs/exec-plans/<exec-slug>/task-<nn>-<slug>.md` |
| project.md / org.md | always active | `docs/specs/` |

---

## Changes to Section 6 — Frontmatter reference

### 6.0  Date fields

All lifecycle-managed documents include a `created` field.

Exec Plan and Task Spec additionally use `closed` when status is `closed`.

PRD, Design Doc, and Design Patch do not gain terminal-date fields in this
patch.

### PRD

```yaml
---
id: prd-user-registration        # slug, no numeric prefix
title: "User Registration"
status: draft                    # draft | approved | obsolete
created: 2026-03-01
---
```

### Design Doc

```yaml
---
id: design-auth-system           # slug, no numeric prefix
title: "Auth System Design"
status: candidate                # draft | candidate | implemented | obsolete
created: 2026-03-01
module: auth
prd: prd-user-registration       # slug reference
---
```

### Design Patch

```yaml
---
id: design-auth-system-patch-01-add-oauth
title: "Add OAuth to auth system"
status: candidate                # draft | candidate | implemented | obsolete:merged
created: 2026-03-01
parent: design-auth-system
---
```

### Exec Plan (plan.md)

```yaml
---
id: exec-auth-add-oauth          # slug describing delivery intent
title: "Add OAuth to Auth System"
status: candidate                # draft | candidate | closed
created: 2026-03-01
design-docs:                     # list, was singular design-doc
  - design-auth-system
  - design-auth-system-patch-01-add-oauth
---
```

### Task Spec

```yaml
---
id: task-01                      # local sequence, zero-padded to minimum width 2
title: "Add OAuth provider configuration"
status: candidate                # draft | candidate | closed
created: 2026-03-01
exec-plan: exec-auth-add-oauth   # slug reference
boundaries:
  allowed:
    - "src/auth/oauth.rs"
    - "tests/auth/oauth_test.rs"
  forbidden_patterns:
    - "docs/prd/**"
    - "docs/design/**"
    - "docs/exec-plans/**"
    - "docs/specs/**"
    - "docs/guidelines/**"
completion_criteria:
  - id: "cc-001"
    scenario: "OAuth provider initialises with valid config"
    test: "test_oauth_provider_init_with_valid_config"
---
```

### Closed Exec Plan / Task Spec

```yaml
---
id: exec-auth-add-oauth
title: "Add OAuth to Auth System"
status: closed
created: 2026-03-01
closed: 2026-03-10
design-docs:
  - design-auth-system
---
```

```yaml
---
id: task-01
title: "Add OAuth provider configuration"
status: closed
created: 2026-03-01
closed: 2026-03-03
exec-plan: exec-auth-add-oauth
---
```

### project.md / org.md — unchanged

```yaml
---
id: project
status: active
---
```

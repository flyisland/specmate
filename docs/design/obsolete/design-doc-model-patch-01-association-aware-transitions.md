---
id: design-doc-model-patch-01-association-aware-transitions
title: "Association-Aware Transitions"
status: obsolete:merged
created: 2026-03-25
parent: design-doc-model
merged-into: design-doc-model
---

# Association-Aware Transitions

This patch extends `design-003` with explicit rules for association-aware
status transitions.

`design-003` already defines document parsing, repository indexing, directory
resolution, and the base transition table. This patch adds the missing rule:
transition legality is determined against the full post-transition repository
state, not only against the moved document in isolation.

This patch exists so command-level docs such as `specmate move` can consume one
shared source of truth for cross-document transition legality instead of
defining their own association rules.

This patch also makes an explicit distinction that was previously only implied:

- **steady-state validity**: whether the repository as it currently exists is
  structurally and semantically valid
- **transition-time gate**: whether a specific requested status transition is
  allowed to happen now

Those two concerns are related, but they are not the same rule set.

---

## 1. Scope

This patch adds five capabilities to the shared document model:

1. explicit direct-association definitions between managed document types
2. steady-state association validity rules for the current repository
3. post-transition repository validation for write commands
4. association-aware blocking rules for status transitions
5. association-summary helpers that higher-level commands may use for
   informational output

This patch does not define CLI syntax, output formatting, or dry-run behaviour.
Those remain command-level concerns.

---

## 2. Design principles

**Transition legality belongs to the document model.** If a status change is
blocked because it would leave the repository in an invalid cross-document
state, that rule must live in the shared document model, not in a single
command design.

**Validate the predicted repository, not only the current one.** A write
command must be able to ask the document model whether a proposed transition
would leave the repository valid after the change is applied.

**Steady-state validity and transition gates must stay separate.** A repository
state can be valid even when some future transition is not yet allowed. For
example, an `implemented` Design Doc may legitimately have new `draft` or
`active` Task Specs during a later bug-fix or conformance effort.

**Associations are facts; automation is optional.** The document model defines
which documents are associated and when an aggregate state is true. It does not
decide whether a command must print a hint, perform a follow-up action, or
drive a workflow.

**Direct links are authoritative.** Association-aware rules operate on explicit
frontmatter references (`prd`, `parent`, `design-doc`, `exec-plan`) rather than
filename heuristics or directory proximity.

---

## 3. Direct associations

The shared document model must expose direct associations between managed
documents.

Supported direct associations:

- PRD ↔ Design Doc via `DesignDoc.prd`
- Design Doc ↔ Design Patch via `DesignPatch.parent`
- Design Doc ↔ Exec Plan via `ExecPlan.design-doc`
- Exec Plan ↔ Task Spec via `TaskSpec.exec-plan`

Association direction matters for validation, but the model must support both:

- outgoing references from a document to the document IDs it explicitly names
- incoming references from other documents that explicitly name it

Commands may build higher-level workflows from these associations, but they
must not infer new association types beyond this set in v1.

---

## 4. Post-transition validation model

In addition to validating the current repository index, the document model must
support validating a proposed transition against the predicted repository state
after the move is applied.

Conceptually:

```text
validate_transition(index, document, to_status)
    checks local transition legality
    checks any transition-specific gates

preview_transition(index, document, to_status)
    returns a predicted repository state with:
      - moved document status updated
      - moved document path resolved to the target directory
      - all other documents unchanged

validate_preview(preview_index)
    runs repository-level validation on the predicted state
```

Responsibility split:

- `validate_transition(...)` owns transition-specific gates tied to the
  requested status edge itself
- `validate_preview(...)` owns repository-wide integrity checks on the
  predicted post-transition state

Examples:

- `DesignDoc: candidate -> implemented` waiting for linked Exec Plans to become
  `completed` is a transition-time gate
- rejecting a predicted repository because a reference would point to a status
  that steady-state validity forbids is preview validation

A write command such as `specmate move` must fail before writing if either:

- the current repository is invalid, or
- the predicted repository after the requested move would be invalid

This keeps transition legality consistent with repository-wide reference
integrity.

---

## 5. Steady-state validity vs transition-time gates

The shared document model must distinguish two classes of rule.

### Steady-state validity

Steady-state validity answers:

```text
Is the repository valid right now?
```

These rules are consumed by repository-wide validators such as `specmate check`
and apply to the current document set without assuming any proposed move.

Examples of steady-state rules:

- references must point to existing documents of the correct type
- references must not point to statuses forbidden by the steady-state
  relationship rules after this patch is applied
- required association fields such as `parent`, `merged-into`, and
  `superseded-by` must exist when their status requires them
- live references must not point to parent documents whose status is forbidden
  for that relationship

This patch explicitly refines the base steady-state reference rules for
association-bearing relationships. Where the pre-patch model rejected a parent
status unconditionally, the rules in this patch take precedence and may allow
historical descendants to retain that reference.

### Historical-link rule

Steady-state validity distinguishes live workflow references from historical
record links.

Definitions:

- **live reference**: a reference from a document that is still participating in
  an active workflow for its type
- **historical reference**: a reference from a document that is already in a
  terminal archival state for its type

Live vs historical status mapping:

| Document type | Live statuses | Historical statuses |
|---|---|---|
| PRD | `draft`, `approved` | `obsolete` |
| Design Doc | `draft`, `candidate`, `implemented` | `obsolete` |
| Design Patch | `draft`, `candidate`, `implemented` | `obsolete`, `obsolete:merged` |
| Exec Plan | `draft`, `active` | `completed`, `abandoned` |
| Task Spec | `draft`, `active` | `completed`, `cancelled` |

The model must preserve historical links where they remain useful for audit
history. Therefore:

- live descendants must not point to parent documents in statuses forbidden by
  the relationship
- historical descendants may continue to point to obsolete or abandoned
  parents, as long as the reference target exists and the relationship type is
  still correct

Examples:

- a `completed` Task Spec may continue to reference an `abandoned` Exec Plan
- a `completed` or `cancelled` Task Spec may continue to reference a
  `completed` or `abandoned` Exec Plan
- an `active` Task Spec must not reference an `abandoned` Exec Plan
- a `completed` Exec Plan may continue to reference an `obsolete` Design Doc
- an `abandoned` Exec Plan may continue to reference an `obsolete` Design Doc
- an `active` Exec Plan must not reference an `obsolete` Design Doc

Steady-state validity must not treat every unfinished workflow as a violation.
In particular, the following state is valid:

- a Design Doc is already `implemented`
- a later bug-fix or conformance Task Spec linked through an Exec Plan is
  still `draft` or `active`

That state represents ongoing work against an already-correct design, not a
broken repository.

### Transition-time gate

Transition-time gating answers:

```text
May this specific document transition from status A to status B now?
```

These rules are consumed by mutating commands such as `specmate move` and
`specmate run` when they are about to perform a status change.

Transition-time gates may be stricter than steady-state validity. A transition
can be blocked even when both the current repository and the predicted
post-transition repository are otherwise valid.

Example:

- `DesignDoc: candidate -> implemented` is blocked until every referencing
  Exec Plan is `completed`
- once the Design Doc is already `implemented`, later `draft` or `active`
  Exec Plans / Task Specs for bug-fix work do not make the repository invalid

### Command responsibilities

- `specmate check` consumes steady-state validity rules
- `specmate move` consumes both steady-state validity and transition-time gates
- `specmate run` consumes both steady-state validity and transition-time gates

---

## 6. Association-aware blocking rules

The following rules extend the base transition table in `design-003`.

### General rule

A legal status edge in the transition table is still rejected if applying it
would cause repository-level validation to fail in the predicted post-transition
state.

This includes both:

- invalid outgoing references from the moved document
- invalid incoming references from other documents that would become stale or
  illegal because of the move

### Required blocking cases

At minimum, the shared model must reject these transitions:

- `Prd -> Obsolete` when any live Design Doc, including `draft`,
  `candidate`, or `implemented`, still references that PRD
- `DesignDoc -> Implemented` when any referencing Exec Plan is not `completed`
- `DesignDoc -> Obsolete` when any live Exec Plan still references that Design
  Doc
- `DesignPatch -> ObsoleteMerged` when `merged-into` is missing or invalid
- `ExecPlan -> Completed` when any referencing Task Spec is not `completed`
- `ExecPlan -> Abandoned` when any live Task Spec still references that Exec
  Plan

These are minimum required cases, not an exclusive list. Any other transition
that would make the predicted repository violate the document model must also
be rejected.

### No implicit cascading transitions

The document model never changes the status of related documents as part of
validating or applying one requested transition.

If `task-0007 -> completed` makes `exec-001` eligible for `completed`, that
eligibility is an observable fact. It is not an automatic state transition.

---

## 7. Association summary queries

The shared document model must expose association-summary queries for higher
level commands.

These summaries are read-only facts derived from the current repository state.
They are intended for commands such as `specmate move` or `specmate status`
that want to report association progress without owning the underlying model
rules.

At minimum, the model must support queries of the form:

- all Design Docs associated with a PRD
- all Design Patches associated with a parent Design Doc
- all Exec Plans associated with a Design Doc
- all Task Specs associated with an Exec Plan

For each association set, the model should also support aggregate predicates
such as:

- all associated documents are in a caller-specified target status
- all associated documents are in a terminal status for their type
- no associated documents exist

This patch deliberately does not prescribe CLI wording. It only defines the
facts that commands may surface.

### Terminal-status definition

Because lifecycle vocabularies differ by document type, "terminal" must be
defined per document type:

- PRD: `obsolete`
- Design Doc: `obsolete`
- Design Patch: `obsolete` or `obsolete:merged`
- Exec Plan: `completed` or `abandoned`
- Task Spec: `completed` or `cancelled`

The document model must not use a generic "non-active" shortcut because some
types do not have an `active` status and some non-active states are not
terminal.

---

## 8. Command-facing responsibilities

After this patch, command designs should rely on the shared document model as
follows:

- `specmate check` owns repository reporting, but not the definition of
  repository validity
- `specmate move` owns command syntax, dry-run planning, file updates, and CLI
  output
- `specmate move` does not redefine cross-document transition legality
- `specmate run` may use the same association-aware transition validation when
  deciding whether a final status move is allowed
- `specmate status` may use association summaries for reporting, without owning
  the summary semantics

If a future command needs a new association-aware legality rule, that rule must
be added to the document model design first.

---

## 9. Verification requirements

An implementation of this patch is not complete unless automated tests cover at
least:

- predicted-state rejection for `Prd -> Obsolete` with linked live Design Docs
- predicted-state rejection for `DesignDoc -> Implemented` with incomplete Exec
  Plans
- predicted-state rejection for `DesignDoc -> Obsolete` with referencing Exec
  Plans
- predicted-state rejection for `ExecPlan -> Completed` with incomplete Task
  Specs
- predicted-state rejection for `ExecPlan -> Abandoned` with any referencing
  Task Specs
- steady-state acceptance of an `implemented` Design Doc with later
  `draft`/`active` bug-fix work linked through a new Exec Plan / Task Spec
- successful preview validation when all linked documents satisfy the required
  state
- association-summary queries for each supported direct-association type
- terminal-state aggregation using the per-doc-type terminal definitions

---

## 10. Intended follow-up

Once this patch is accepted:

1. update `design-007` so it references these shared document-model rules
   rather than defining its own legality matrix
2. keep `design-007` focused on `specmate move` command behaviour
3. reuse the same model rules in any future `run` or `status` designs

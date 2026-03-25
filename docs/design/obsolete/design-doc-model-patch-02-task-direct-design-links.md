---
id: design-doc-model-patch-02-task-direct-design-links
title: "Task Direct Design Links"
status: obsolete:merged
created: 2026-03-25
parent: design-doc-model
merged-into: design-doc-model
---

# Task Direct Design Links

This patch extends `design-003` so a `TaskSpec` may link directly to a
`DesignDoc` when the task is intentionally standalone and does not belong to an
`ExecPlan`.

`design-003` already allows `TaskSpec.exec-plan` to be omitted for standalone
tasks. The missing piece is a standard way to preserve the design upstream for
those tasks. Repositories have started to use ad hoc fields such as `design`,
but those fields are outside the specmate model and are therefore ignored by
shared validation and status/reporting commands.

This patch standardises the direct design upstream so the document model,
checks, and read-only reporting all agree on one contract.

---

## 1. Scope

This patch adds four capabilities to the shared document model:

1. a standard optional `design-doc` field on `TaskSpec`
2. a direct `DesignDoc ↔ TaskSpec` association for standalone tasks
3. validation rules that prevent ambiguous or stale task upstream links
4. command-facing guidance for `check`, `status`, and other consumers of the
   shared model

This patch does not change lifecycle states, directory mappings, or transition
tables.

---

## 2. Design principles

**Standalone tasks are first-class.** A task that does not belong to an
`ExecPlan` is a valid repository object, not a special-case error.

**One upstream path per task.** A `TaskSpec` must declare at most one workflow
upstream: either `exec-plan` or direct `design-doc`. The model must reject
documents that try to use both because that creates redundant and potentially
divergent lineage.

**Use the existing field name family.** The direct design link on `TaskSpec`
must use `design-doc`, not a new field spelling such as `design`. This keeps
field names aligned with `ExecPlan.design-doc`.

**Historical links remain useful.** A completed or cancelled task may retain a
direct link to an obsolete design for audit history, following the same
historical-link policy already used elsewhere in the document model.

---

## 3. Frontmatter contract changes

`TaskSpec` gains a second optional upstream field:

| Field | Type | Meaning |
|---|---|---|
| `exec-plan` | string | optional, linked Exec Plan id |
| `design-doc` | string | optional, linked Design Doc id for a standalone task |

Rules:

- a `TaskSpec` may omit both fields when it is intentionally standalone and has
  no recorded upstream
- a `TaskSpec` may include `exec-plan`
- a `TaskSpec` may include `design-doc`
- a `TaskSpec` must not include both `exec-plan` and `design-doc`

`design` is not standardised by this patch. Repositories may still contain
arbitrary extra frontmatter keys, but specmate commands must continue to ignore
`design` as a non-model field.

---

## 4. Direct association model

The shared document model must add one direct association family:

- Design Doc ↔ Task Spec via `TaskSpec.design-doc`

After this patch, supported direct associations are:

- PRD ↔ Design Doc via `DesignDoc.prd`
- Design Doc ↔ Design Patch via `DesignPatch.parent`
- Design Doc ↔ Exec Plan via `ExecPlan.design-doc`
- Exec Plan ↔ Task Spec via `TaskSpec.exec-plan`
- Design Doc ↔ Task Spec via `TaskSpec.design-doc`

Association summaries must treat direct task links as a separate family from
tasks reached through Exec Plans. This keeps the graph explicit and prevents
the shared model from inventing synthetic intermediate nodes.

For design-level aggregate counts, commands may need both views:

- direct task links from `TaskSpec.design-doc`
- indirect task links reached through Exec Plans associated with the same
  design

When a command reports a total task count for a Design Doc, it should count the
union of those two sets by canonical task id.

---

## 5. Validation rules

### Steady-state validity

When `TaskSpec.design-doc` is present:

- the value must parse as a Design Doc id
- the target must exist
- the target must be a `DesignDoc`
- a live `TaskSpec` (`draft` or `active`) must not point to an `obsolete`
  Design Doc
- a historical `TaskSpec` (`completed` or `cancelled`) may continue to point to
  an `obsolete` Design Doc

When both `exec-plan` and `design-doc` are present on the same `TaskSpec`, the
document model must reject the document as invalid rather than choosing one.

### Transition and preview validation

No new lifecycle edges are added by this patch, but predicted-state validation
must continue to reject any move that would leave a live `TaskSpec` directly
pointing to an obsolete Design Doc.

This means commands such as `specmate move` must fail before writing if:

- a requested `DesignDoc -> Obsolete` transition would strand a live directly
  linked `TaskSpec`
- a requested `TaskSpec` move would create an invalid live direct design link

---

## 6. Command-facing consequences

### `specmate check`

`check refs` and any other repository-level validation that consumes the shared
document model must treat `TaskSpec.design-doc` as a first-class reference.

Actionable failures should mirror the existing wording style for other
reference fields:

- `design-doc design-999 does not exist`
- `design-doc design-003 is obsolete`
- `task-0007 must not declare both exec-plan and design-doc`

### `specmate status`

`specmate status` should expose the new link without widening the meaning of
its sections:

- **Upstream References** shows only direct frontmatter references owned by the
  current document
- **Derived Chain Summary** shows lineage derived through those references

For a `TaskSpec` with direct `design-doc` and no `exec-plan`:

- Upstream References should render `design-doc: design-013 (<status>)`
- Derived Chain Summary should render `design-doc lineage: design-013`

For a `TaskSpec` with `exec-plan`, current behaviour stays unchanged:

- Upstream References shows `exec-plan`
- Derived Chain Summary may show `exec-plan lineage: exec-001 -> design-003`

For a `DesignDoc`, downstream reporting should include directly linked Task
Specs as their own association family, separate from Exec Plans.

### Other command consumers

- `specmate move` must consume the new steady-state and preview rules through
  the shared model only
- future commands such as `specmate status` dashboard or `specmate run` must
  not infer direct task-design links from ad hoc fields like `design`

---

## 7. Migration guidance

Repositories that currently use ad hoc task metadata such as:

```yaml
design: design-013
```

should migrate to:

```yaml
design-doc: design-013
```

if they want specmate to recognise that upstream relationship.

Repositories should not add both:

```yaml
exec-plan: exec-007
design-doc: design-013
```

because the design link is already derivable through the Exec Plan path.

---

## 8. Verification requirements

An implementation of this patch is not complete unless automated tests cover at
least:

- loading a standalone `TaskSpec` with direct `design-doc`
- rejection of a `TaskSpec` that declares both `exec-plan` and `design-doc`
- rejection of a live `TaskSpec` whose direct `design-doc` is obsolete
- acceptance of a completed or cancelled `TaskSpec` whose direct `design-doc`
  is obsolete
- direct association summaries for `DesignDoc ↔ TaskSpec`
- `specmate status <task-id>` rendering for a task with direct `design-doc`
- `specmate status <design-id>` rendering that distinguishes direct task links
  from Exec Plan associations

---

## 9. Intended follow-up

Once this patch is accepted:

1. update `design-003` to include `TaskSpec.design-doc` in its frontmatter and
   validation tables
2. update `design-008` so the status command renders the new direct upstream
   and keeps direct references separate from derived lineage
3. update command implementations and tests to consume the shared model instead
   of repository-specific ad hoc task fields

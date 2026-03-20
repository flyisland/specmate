---
id: design-001-patch-01-clarify-document-levels
title: "Clarify document levels"
status: obsolete:merged
parent: design-001
merged-into: design-001
---

# Clarify document levels

This patch clarifies the boundary of the specmate document system without
expanding the set of managed document types or requiring immediate code
changes.

The goal is to distinguish documents that specmate actively manages from
documents that may exist in the repository, but are outside the formal status
and transition model.

---

## 1. Decision

The repository's markdown files are divided into three levels:

1. **Managed documents**
2. **Repository documents outside management**
3. **Supporting material**

This classification is about **how specmate treats a document**, not whether
the document is useful or important to humans or agents.

### Managed documents

Managed documents are part of the formal specmate document system.

Properties:

- specmate recognises their meaning
- specmate enforces their placement and naming rules
- some have frontmatter, status, and lifecycle transitions
- they may be created, checked, or moved by specmate commands

Managed documents are:

- PRD
- Design Doc
- Design Patch
- Exec Plan
- Guideline
- Task Spec
- `specs/project.md`
- `specs/org.md`

### Repository documents outside management

Some repository-level documents may exist at conventional paths, but are
**not** part of the formal specmate document model.

Properties:

- specmate does not assign IDs for them
- specmate does not track status or lifecycle for them
- they do not participate in `move` or status transitions
- they should be treated as `user-owned`

Examples:

- `AGENTS.md`
- `ARCHITECTURE.md` when a project chooses to include it

### Supporting material

Supporting material is allowed in the repository, but does not belong to
the formal specmate document model.

Properties:

- specmate does not assign IDs or statuses
- specmate does not manage lifecycle or transitions
- these files may be read and referenced by humans or agents
- these files must not replace managed documents as the source of truth

Examples:

- files under `docs/references/`
- files under `docs/generated/`

---

## 2. Guideline scope

`Guideline` remains a managed document type.

Its scope includes cross-cutting principles, standards, and review criteria
that apply across multiple modules or tasks, not only security or reliability
concerns in the narrow sense.

Typical guideline topics include:

- security
- reliability
- frontend
- product-sense
- design-principles

Boundary rule:

- if a document describes **how a specific module is currently designed**, it
  is a Design Doc
- if a document describes **principles or standards that apply across multiple
  modules or tasks**, it is a Guideline

---

## 3. Supporting directories

### `docs/references/`

`docs/references/` stores reference material such as external framework notes,
tooling summaries, protocol excerpts, or design-system guidance prepared for
human or agent consumption.

These files are informative inputs only. They are not managed documents and
do not participate in status, movement, or lifecycle rules.

### `docs/generated/`

`docs/generated/` stores derived artifacts such as generated schema snapshots,
API summaries, or other machine-produced repository documentation.

These files are not managed documents and are not treated as the source of
truth unless a managed document explicitly says otherwise.

---

## 4. Repository documents outside management

### `AGENTS.md`

`AGENTS.md` is a repository-root onboarding document for agents. It is not a
managed document type.

### `ARCHITECTURE.md`

`ARCHITECTURE.md` may exist as a repository-level architecture overview.

It is not a managed document type.

specmate does not assign it an ID, status, or lifecycle, and does not move
or validate it as part of the formal document system.

It should be treated as `user-owned` and must not be silently overwritten by
later commands.

---

## 5. Non-goals

This patch does not:

- add any new managed document type
- add status management for `ARCHITECTURE.md`
- make `docs/references/` or `docs/generated/` specmate-managed
- require `specmate init` to generate any additional document
- define rules for `index.md`

---

## 6. Impact on design-001

When this patch is merged, `design-001-document-system` should be updated to:

- introduce the three document levels
- clarify that `Guideline` covers broader cross-cutting topics
- explicitly acknowledge `docs/references/` and `docs/generated/` as
  supporting material outside the managed model
- explicitly acknowledge `AGENTS.md` and optional `ARCHITECTURE.md` as
  repository documents outside the managed model

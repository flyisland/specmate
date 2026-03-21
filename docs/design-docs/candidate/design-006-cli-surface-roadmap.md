---
id: design-006
title: "CLI Surface Roadmap"
status: candidate
design-doc: design-001
guidelines:
  - docs/guidelines/specmate-principles.md
  - docs/guidelines/cli-conventions.md
---

# CLI Surface Roadmap

This document is the planning container for specmate commands that are
intended but not yet implemented. It exists to keep roadmap-level CLI
information out of repository-facing documents such as `README.md` and
`AGENTS.md`.

This document is intentionally long-lived. As command families mature, their
detailed behavior should move into dedicated Design Docs. This roadmap remains
as the summary of what is still planned. When no planned CLI surface remains,
this document should move to `obsolete`.

---

## 1. Role of this document

`design-001` defines the document system and document lifecycles. It does not
define the full specmate CLI surface.

This document fills that gap at the planning level:

- records which command families are planned
- distinguishes implemented commands from planned commands
- points to the Design Doc that owns detailed behavior when one exists
- defines when planning content should be split out or removed

This document is a roadmap, not an execution contract. If a command family has
its own Design Doc, that dedicated document is the source of truth for design
details.

---

## 2. Current implementation boundary

As of this document revision, the implemented top-level commands are:

- `specmate init`
- `specmate check`

All other command families mentioned below are planning-only until code lands
and the command is wired into the CLI.

Repository-facing docs must reflect this boundary clearly:

- `README.md` may describe implemented commands and may link to this roadmap
- `AGENTS.md` may reference this roadmap for planned CLI surface
- planning-only commands must not be presented as already available behavior

---

## 3. Planned command families

| Command | Purpose | Design home | Status |
|---|---|---|---|
| `specmate new` | Create managed documents with allocated IDs and initial frontmatter | Split into a dedicated Design Doc; currently depends on doc model rules in `design-003` | planned |
| `specmate move` | Perform status transitions and relocate managed documents atomically | `design-007` | planned |
| `specmate check` | Run mechanical validation across the document system | `design-004` | implemented |
| `specmate run` | Execute the coding loop for a Task Spec via ACP | `design-005` | planned |
| `specmate rerun` | Re-enter the agent loop for a previously run task | `design-005` | planned |
| `specmate status` | Show system status and doc progress views | Split into a dedicated Design Doc | planned |
| `specmate update-guides` | Refresh specmate-owned guide files after template or guidance changes | Split into a dedicated Design Doc | planned |

Notes:

- `specmate run` and `specmate rerun` are grouped because they share one loop design.
- `specmate new` is not yet owned by a dedicated command Design Doc even though `design-003` already defines some shared document-model behavior it depends on.
- `specmate status` and `specmate update-guides` remain roadmap items only until their own design work starts.

---

## 4. Decomposition rule

When a planned command family is ready for detailed design or implementation:

1. Create or update a dedicated candidate Design Doc for that command family.
2. Move command syntax, workflow, invariants, and examples into that document.
3. Reduce this roadmap entry to a short summary plus the owning Design Doc.
4. Update repository-facing docs only after the command is implemented.

This rule keeps the roadmap small while preserving a single place to answer
"what commands are still planned?".

---

## 5. Exit condition

This document should remain `candidate` while at least one planned command
family still lacks implementation.

Once every roadmap item has either:

- been implemented and documented as current behavior, or
- been dropped from the product direction

this document no longer serves a planning purpose and should move to
`obsolete`.

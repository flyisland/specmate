---
id: design-status-command-patch-01-design-overview-includes-patches
title: "Design overview includes active design patches"
status: obsolete:merged
created: 2026-03-25
parent: design-status-command
merged-into: design-status-command
---

# Design overview includes active design patches

This patch aligns `design-008` with the implemented dashboard behavior for
`specmate status`.

The dashboard's `Status Totals` section already counts `DesignPatch`
separately. If `Design Overview` shows only `DesignDoc`, the default view can
claim there is candidate design work without showing the actual candidate patch
that caused the count.

That makes the overview internally inconsistent and forces the user to infer
which design patch is active from totals alone.

## Decision

`Design Overview` shows all active design artefacts, not only parent Design
Docs.

For the default dashboard, that means:

- `draft` bucket includes `DesignDoc` and `DesignPatch`
- `candidate` bucket includes `DesignDoc` and `DesignPatch`
- `implemented` bucket includes `DesignDoc` and `DesignPatch`

Terminal design artefacts such as `obsolete` and `obsolete:merged` remain out
of the default design overview. They continue to appear through `Status Totals`
and the optional `--all` section.

## Rationale

Active design patches are part of the live design surface:

- a `candidate` patch is approved design work that may drive implementation
- an `implemented` patch is a live delta awaiting merge-back into the parent
  design
- a `draft` patch is active design authorship in progress

Excluding them from `Design Overview` hides relevant current design work and
breaks the dashboard's own totals story.

## Scope

This patch changes only repository-dashboard presentation semantics for
`Design Overview`.

It does not change:

- detail-view association rules
- status totals semantics
- document-model status definitions
- patch merge flow


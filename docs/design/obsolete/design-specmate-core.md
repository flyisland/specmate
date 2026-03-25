---
id: design-specmate-core
title: "specmate Core"
status: obsolete
created: 2026-03-25
---

# specmate Core

This design doc has been retired.

It originally attempted to define several unrelated concerns in one place:

- template deployment
- project configuration
- Task Spec runtime fields

That structure created duplication and drift across the actual subsystem
designs, so the content was split into the documents that own those concerns:

- `design-003` — document model and Task Spec runtime contract
- `design-004` — check engine behaviour
- `design-005` — agent loop behaviour
- `docs/guidelines/specmate-principles.md` — file ownership, language, and
  general system principles

This document remains in `obsolete/` so `design-002` continues to exist in the
document history and its ID is never reused.

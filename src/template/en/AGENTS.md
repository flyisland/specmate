# <project-name> — Agent Onboarding

## Quick start

```bash
# Add your build / test / lint commands here
```

## Core documents

- `docs/specs/project.md` — technical constraints for this project
- `docs/design/implemented/` — current design contracts (ls = source of truth)
- The Task Spec under the current Exec Plan directory — defines intent, boundaries, completion criteria

## Guidelines — read when relevant

<!-- Add guideline files and when to read them, e.g.:
| File | Read when |
|---|---|
| `docs/guidelines/security.md` | any task touching auth, credentials, user data |
-->

## Before starting any task

1. Read `docs/specs/project.md` — confirm technical constraints
2. Read relevant docs in `docs/design/implemented/`
3. Read guidelines listed in the Task Spec's `guidelines` field (if any)
4. Read the Task Spec — note `boundaries.allowed` and `completion_criteria`
5. Code strictly within `boundaries.allowed`
6. All `completion_criteria` tests must pass before the task is done

Do not modify managed documents under `docs/specs/`, `docs/design/`, or `docs/exec-plans/` during task execution unless the task explicitly exists to update them.

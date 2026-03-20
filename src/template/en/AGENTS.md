# <project-name> — Agent Onboarding

## Quick start

```bash
# Add your build / test / lint commands here
```

## Core documents

- `specs/project.md` — technical constraints for this project
- `docs/design-docs/implemented/` — current design contracts (ls = source of truth)
- Active Task Spec — defines intent, boundaries, completion criteria

## Guidelines — read when relevant

<!-- Add guideline files and when to read them, e.g.:
| File | Read when |
|---|---|
| `docs/guidelines/security.md` | any task touching auth, credentials, user data |
-->

## Before starting any task

1. Read `specs/project.md` — confirm technical constraints
2. Read relevant docs in `docs/design-docs/implemented/`
3. Read guidelines listed in the Task Spec's `guidelines` field (if any)
4. Read the Task Spec — note `boundaries.allowed` and `completion_criteria`
5. Code strictly within `boundaries.allowed`
6. All `completion_criteria` tests must pass before the task is done

Do not modify any file under `specs/` during task execution.

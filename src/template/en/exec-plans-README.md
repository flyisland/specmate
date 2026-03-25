# docs/exec-plans/

Execution Plans coordinating one or more Task Specs toward a shared goal.

## Exec Plan lifecycle

```
draft <-> candidate
   \         \
    \         -> closed
     -> closed
```

| Status | Meaning |
|---|---|
| `draft` | Planning stage. Not executable yet. |
| `candidate` | Approved for execution. |
| `closed` | Historical terminal state. Work is no longer active. |

## Layout

- `docs/exec-plans/exec-<slug>/plan.md`
- `docs/exec-plans/exec-<slug>/task-01-<slug>.md`

## Naming

- Exec Plan canonical ID: `exec-<slug>`
- Task Spec frontmatter ID: `task-01`
- Repo-wide Task reference: `exec-<slug>/task-01`

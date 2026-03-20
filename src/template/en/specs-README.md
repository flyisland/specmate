# specs/

Task Specs and project-level constraints.

## Files

- `project.md` — project-wide technical constraints, enforced by `specmate check`
- `org.md` — organisation-wide constraints (security, compliance)

## Subdirectories

- `active/` — task specs in `draft` or `active` status
- `archived/` — task specs that are `completed` or `cancelled`

## Task Spec lifecycle

```
draft -> active -> completed
                 \ cancelled
```

| Status | Meaning |
|---|---|
| `draft` | Being written. `specmate run` will refuse to start. |
| `active` | Human-approved. Agent loop can begin. Spec is locked. |
| `completed` | All criteria passed, PR merged. |
| `cancelled` | Decided not to implement. Reason recorded. |

## Naming

`task-0001-<slug>.md` — four-digit ID, globally incremented, never reused.

# docs/exec-plans/

Execution Plans coordinating multiple Task Specs toward a shared goal.

## Exec Plan lifecycle

```
draft -> active -> completed
                \ abandoned
```

| Status | Meaning |
|---|---|
| `draft` | Tasks and dependencies being planned. |
| `active` | Execution in progress. |
| `completed` | All phases done. Design Doc can now move to `implemented`. |
| `abandoned` | Stopped mid-execution. Reason and completed phases recorded. |

## Naming

`exec-001-<slug>.md` — three-digit ID, globally incremented.

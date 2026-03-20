# docs/design-docs/

Design documents describing how the system is built.

## Design Doc lifecycle

```
draft -> candidate -> implemented -> obsolete
```

| Status | Meaning |
|---|---|
| `draft` | Design being written. Agents must not execute against this. |
| `candidate` | Design finalised. Codebase not yet implemented. Agent's job: implement this. |
| `implemented` | Codebase fully consistent with this doc. Divergence = bug. |
| `obsolete` | Module removed or superseded. |

## Patch docs

To modify an implemented design, create a patch doc:
`design-001-patch-01-<slug>.md`

The patch follows the same lifecycle. When complete, its content is merged back
into the parent doc and the patch moves to `obsolete:merged`.

## Rule

Only one `implemented` Design Doc per module at any time.
`ls implemented/` = all current design contracts.

## Naming

`design-001-<slug>.md` — three-digit ID, globally incremented.

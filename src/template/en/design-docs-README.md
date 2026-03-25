# docs/design/

Design documents describing how the system is built.

## Design Doc lifecycle

```
draft -> candidate -> implemented -> obsolete
```

| Status | Meaning |
|---|---|
| `draft` | Design being written. Agents must not execute against this. |
| `candidate` | Design is approved for execution. It may still evolve while implementation proceeds. |
| `implemented` | Codebase fully consistent with this doc. Divergence = bug. |
| `obsolete` | Module removed or superseded. |

## Patch docs

To modify an implemented design, create a patch doc:
`design-<parent-slug>-patch-01-<slug>.md`

The patch follows the same lifecycle. When complete, its content is merged back
into the parent doc and the patch moves to `obsolete:merged`.

## Rule

Keep one long-lived `implemented` Design Doc per module or domain where practical.
Use patches for change, then merge the patch content back into the parent Design Doc.

## Naming

`design-<slug>.md` — slug-based canonical ID.

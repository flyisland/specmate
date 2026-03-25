# docs/exec-plans/

协调一个或多个 Task Spec 实现共同目标的执行计划。

## Exec Plan 状态流转

```
draft <-> candidate
   \         \
    \         -> closed
     -> closed
```

| 状态 | 含义 |
|---|---|
| `draft` | 规划中，尚不可执行 |
| `candidate` | 已批准，可执行 |
| `closed` | 历史终态，不再继续执行 |

## 布局

- `docs/exec-plans/exec-<slug>/plan.md`
- `docs/exec-plans/exec-<slug>/task-01-<slug>.md`

## 命名规则

- Exec Plan canonical ID: `exec-<slug>`
- Task Spec frontmatter ID: `task-01`
- repo 级 Task 引用: `exec-<slug>/task-01`

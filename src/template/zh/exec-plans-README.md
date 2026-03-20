# docs/exec-plans/

协调多个 Task Spec 实现共同目标的执行计划。

## Exec Plan 状态流转

```
draft -> active -> completed
                \ abandoned
```

| 状态 | 含义 |
|---|---|
| `draft` | 任务和依赖关系规划中 |
| `active` | 执行中 |
| `completed` | 所有 phase 完成，Design Doc 可以移至 `implemented` |
| `abandoned` | 中途停止，需记录原因和已完成的 phase |

## 命名规则

`exec-001-<slug>.md` — 三位数 ID，全局递增。

# specs/

Task Spec 和项目级约束文档。

## 文件

- `project.md` — 项目级技术约束，由 `specmate check` 强制执行
- `org.md` — 组织级约束（安全、合规）

## 子目录

- `active/` — 状态为 `draft` 或 `active` 的 Task Spec
- `archived/` — 状态为 `completed` 或 `cancelled` 的 Task Spec

## Task Spec 状态流转

```
draft -> active -> completed
                \ cancelled
```

| 状态 | 含义 |
|---|---|
| `draft` | 编写中，`specmate run` 会拒绝启动 |
| `active` | 人工审核通过，可以启动 agent loop |
| `completed` | 所有验收条件通过，PR 已合并 |
| `cancelled` | 决定不实现，需记录原因 |

## 命名规则

`task-0001-<slug>.md` — 四位数 ID，全局递增，不复用。

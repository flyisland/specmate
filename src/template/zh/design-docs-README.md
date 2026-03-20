# docs/design-docs/

描述系统如何构建的设计文档。

## 设计文档状态流转

```
draft -> candidate -> implemented -> obsolete
```

| 状态 | 含义 |
|---|---|
| `draft` | 设计编写中，agent 不得据此执行 |
| `candidate` | 设计定稿，codebase 尚未实现，agent 的任务是实现它 |
| `implemented` | codebase 与文档完全一致，任何偏离都是 bug |
| `obsolete` | 模块已删除或被取代 |

## Patch 文档

要修改一个 implemented 的设计，创建 patch 文档：
`design-001-patch-01-<slug>.md`

Patch 走相同的状态流转。完成后内容合并回父文档，patch 状态改为 `obsolete:merged`。

## 规则

每个模块同一时刻只能有一个 `implemented` 状态的设计文档。
`ls implemented/` = 所有当前有效的设计合约。

## 命名规则

`design-001-<slug>.md` — 三位数 ID，全局递增。

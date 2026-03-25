# docs/design/

描述系统如何构建的设计文档。

## 设计文档状态流转

```
draft -> candidate -> implemented -> obsolete
```

| 状态 | 含义 |
|---|---|
| `draft` | 设计编写中，agent 不得据此执行 |
| `candidate` | 已批准可实施；实施过程中内容仍可继续演进 |
| `implemented` | codebase 与文档完全一致，任何偏离都是 bug |
| `obsolete` | 模块已删除或被取代 |

## Patch 文档

要修改一个 implemented 的设计，创建 patch 文档：
`design-<parent-slug>-patch-01-<slug>.md`

Patch 走相同的状态流转。完成后内容合并回父文档，patch 状态改为 `obsolete:merged`。

## 规则

尽量为每个模块或领域维护一份长期存在的 `implemented` 设计文档。
通过 patch 演进，并在 patch 完成后把内容合并回父设计文档。

## 命名规则

`design-<slug>.md` — 基于 slug 的 canonical ID。

# <项目名> — Agent 入职文档

## 快速启动

```bash
# 在此填写构建 / 测试 / lint 命令
```

## 核心文档

- `specs/project.md` — 项目技术约束
- `docs/design-docs/implemented/` — 当前设计合约（ls = source of truth）
- 当前 Task Spec — 定义意图、边界、验收条件

## Guideline — 按需查阅

<!-- 添加 guideline 文件和查阅时机，例如：
| 文件 | 何时查阅 |
|---|---|
| `docs/guidelines/security.md` | 任何涉及认证、凭证、用户数据的任务 |
-->

## 开始任何任务前

1. 读 `specs/project.md` — 确认技术约束
2. 读 `docs/design-docs/implemented/` 中的相关文档
3. 阅读 Task Spec `guidelines` 字段列出的文件（如有）
4. 读 Task Spec — 注意 `boundaries.allowed` 和 `completion_criteria`
5. 严格在 `boundaries.allowed` 范围内编码
6. 所有 `completion_criteria` 测试通过后任务才算完成

执行任务期间不得修改 `specs/` 目录下的任何文件。

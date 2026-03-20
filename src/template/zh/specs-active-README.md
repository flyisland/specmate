# specs/active/

状态为 `draft` 或 `active` 的 Task Spec。

- `draft` — 编写中，尚未审核
- `active` — 人工审核通过，可通过 `specmate run <task-id>` 启动 agent loop

完成后移出：`specmate move task-0001 completed`

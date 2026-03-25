# specmate

> Your docs don't enforce themselves. **specmate does.**

CLI companion for document-driven AI coding.

---

## Current status

The currently implemented CLI surface is:

```bash
specmate init [--lang <en|zh>] [--merge] [--dry-run]
specmate check [names|frontmatter|status|refs|conflicts|boundaries <task-id>]
specmate move <doc-id> <to-status> [--dry-run]
specmate status [doc-id] [--all] [--color <when>]
```

These commands cover repository bootstrap, mechanical validation, status
transitions, and repository/document inspection.

Planned but not yet implemented commands are tracked in
`docs/design/draft/design-cli-surface-roadmap.md`.
Repository-facing docs should not present those commands as already available.

---

## Source documents

- `docs/design/implemented/`
  Current implemented design contracts.
- `docs/design/candidate/design-agent-loop.md`
  Current candidate design for the planned `run` / `rerun` workflow.
- `docs/design/draft/design-cli-surface-roadmap.md`
  Planning container for remaining CLI surface.
- `docs/specs/project.md`
  Project constraints and coding conventions for this repository.
- `AGENTS.md`
  Contributor and agent onboarding for the current codebase.

---

## Development

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
cargo run -- --help
```

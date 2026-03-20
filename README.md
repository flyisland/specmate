# specmate

> Your docs don't enforce themselves. **specmate does.**

CLI companion for document-driven AI coding.

---

## Current status

The currently implemented CLI surface is intentionally small:

```bash
specmate init [--lang <en|zh>] [--merge] [--dry-run]
```

`specmate init` bootstraps a repository with the specmate document structure,
templates, and config files.

Planned but not yet implemented commands are tracked in
`docs/design-docs/candidate/design-006-cli-surface-roadmap.md`.
Repository-facing docs should not present those commands as already available.

---

## Source documents

- `docs/design-docs/implemented/design-001-document-system.md`
  Current document-system contract and status model.
- `docs/design-docs/candidate/design-006-cli-surface-roadmap.md`
  Planning container for future CLI surface.
- `specs/project.md`
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
cargo run -- init --help
```

# specmate — Agent Onboarding

## Quick start

```bash
cargo build                          # build the project
cargo test                           # run all tests
cargo test <test_name>               # run a specific test
cargo clippy -- -D warnings          # lint (must pass, no warnings)
cargo fmt --check                    # format check
cargo run -- <command> [options]     # run locally, e.g. cargo run -- init --lang zh
```

## Project structure

```
specmate/
├── src/
│   ├── main.rs              # CLI entry point, command routing
│   ├── cmd/
│   │   ├── mod.rs           # subcommand registration
│   │   └── init.rs          # specmate init
│   ├── config.rs            # .specmate/config.yaml read/write
│   └── error.rs             # error types
├── tests/
│   └── cmd/
│       └── init_test.rs
├── Cargo.toml
└── docs/
    ├── specs/               # project/org constraints
    ├── design/              # design contracts
    ├── exec-plans/          # dogfooded exec/task history
    └── guidelines/          # always-injected guidance
```

## Core documents

Read these before working on any task:

- `docs/specs/project.md` — technical constraints and coding conventions
- `docs/design/implemented/` — all current design contracts; codebase must
  be consistent with every doc in this directory
- `docs/design/draft/design-cli-surface-roadmap.md` — roadmap for
  planned CLI commands that are not implemented yet
- The relevant Task Spec under `docs/exec-plans/<exec-id>/` — defines intent,
  boundaries, and completion criteria

## Guidelines — read when relevant

| File | Read when |
|---|---|
| `docs/guidelines/specmate-principles.md` | any task touching file I/O, git operations, CLI output, or language support |

When a Task Spec includes a `guidelines` field, those files are injected
automatically. For tasks without explicit guidelines, use this table to
decide which files to read before starting.

## Key dependencies

```toml
clap       # CLI argument parsing (derive feature)
serde      # serialization
serde_yaml # YAML frontmatter parsing
anyhow     # error handling
walkdir    # directory traversal for check commands
```

Templates live in `src/template/en/` and `src/template/zh/` as standalone
`.md` files and are embedded at compile time using `include_str!`. Never
hardcode template content in Rust source files — always add a template file
first, then embed it.

## Coding conventions

- All errors surface via `anyhow::Result`. Use `?` for propagation.
- Use `thiserror` for defining domain error types in `error.rs`.
- No `unwrap()` or `expect()` in production code paths. Tests may use `unwrap()`.
- Every public function must have a doc comment.
- Clippy must pass with `-D warnings` — no warnings allowed.
- Format with `rustfmt` before committing. Run `cargo fmt`.

## Document type ID format

| Type | Format | Example |
|---|---|---|
| PRD | `prd-<slug>` | `prd-user-registration.md` |
| Design Doc | `design-<slug>` | `design-auth-system.md` |
| Design Patch | `design-<parent-slug>-patch-01-<slug>` | `design-auth-system-patch-01-remove-username.md` |
| Exec Plan | `exec-<slug>` | `exec-auth-impl` |
| Task Spec | `<exec-id>/task-01` | `exec-auth-impl/task-01` |

Task Specs and Design Patches use two-digit local sequence numbers.

## Status lifecycles

```
PRD:          draft → approved → obsolete
Design Doc:   draft → candidate → implemented → obsolete
              patch only: ... → obsolete:merged
Exec Plan:    draft ↔ candidate → closed
              draft → closed
Task Spec:    draft ↔ candidate → closed
              draft → closed
Guideline:    active (always, no transitions)
```

## Directory = status

The subdirectory a file lives in reflects its status. `specmate move` handles
file relocation atomically — never move files manually.

```
docs/design/implemented/        ← ls here = all current design contracts
docs/guidelines/                ← all active guidelines
docs/exec-plans/exec-*/         ← exec plan plus sibling task specs
```

## File ownership

- **specmate-owned**: any `README.md` inside a specmate-managed subdirectory.
  Overwritten silently by `specmate init --merge` and `specmate update-guides`.
- **user-owned**: everything else. Created by `init` if absent, never overwritten.

## Running a Task Spec

1. Read `docs/specs/project.md` — confirm technical constraints
2. Read relevant docs in `docs/design/implemented/`
3. Read guidelines listed in the Task Spec's `guidelines` field (if any),
   and any additional guidelines from the table above that apply to this task
4. Read the Task Spec — note `boundaries.allowed` and `completion_criteria`
5. Code strictly within `boundaries.allowed`
6. Run `cargo clippy -- -D warnings` and `cargo test` — both must pass
7. All `completion_criteria` tests must pass before the task is considered done

Do not modify managed documents under `docs/specs/`, `docs/design/`, or
`docs/exec-plans/` during task execution unless the task explicitly exists to
update them.

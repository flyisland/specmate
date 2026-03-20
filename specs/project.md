---
id: project
status: active
---

# Project Constraints

## Tech stack

- **Language**: Rust (stable, latest)
- **CLI framework**: `clap` v4 with derive feature
- **YAML parsing**: `serde_yaml` + `serde`
- **Git operations**: `git2` (libgit2 bindings, no system git dependency)
- **Directory traversal**: `walkdir`
- **Error handling**: `anyhow` (propagation) + `thiserror` (domain error types)
- **Glob matching**: `glob`

## Test runner

specmate run executes completion_criteria tests using:

```
cargo test <test_function_name> -- --exact
```

All tests:

```bash
cargo test                              # run all tests
cargo test <test_name> -- --exact       # run a specific test by name
cargo test --test <file>                # run a specific integration test file
```

## Build and lint

```bash
cargo build --release                   # production build
cargo clippy -- -D warnings             # must pass with zero warnings
cargo fmt --check                       # must pass before commit
```

## Coding conventions

- All errors surface via `anyhow::Result`. Use `?` for propagation.
- Domain error types defined with `thiserror` in `src/error.rs`.
- No `unwrap()` or `expect()` in production code under `src/`. Tests may use them.
- Every `pub` function and type must have a doc comment (`///`).
- Clippy must pass with `-D warnings`. No warnings allowed in production code.
- Format with `rustfmt` before every commit. Run `cargo fmt`.
- Modules are declared in `src/lib.rs` or `src/main.rs` — no `mod.rs` files
  except where the module has submodules.

## Forbidden patterns

- No `unwrap()` or `expect()` in `src/` (outside of tests)
- No hardcoded template content in Rust source files — all generated content
  must come from `src/template/en/` or `src/template/zh/` via `include_str!`
- No runtime file loading for templates
- No dependency on a system `git` binary — use `git2` crate only
- No `std::process::Command` for git operations

## Directory layout

```
specmate/
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── error.rs
│   ├── config.rs
│   ├── cmd/
│   │   ├── mod.rs
│   │   ├── init.rs
│   │   ├── new.rs
│   │   ├── check.rs
│   │   ├── run.rs
│   │   ├── move_.rs
│   │   └── status.rs
│   ├── doc/
│   │   ├── mod.rs
│   │   ├── types.rs
│   │   ├── id.rs
│   │   └── frontmatter.rs
│   └── template/
│       ├── en/
│       └── zh/
├── tests/
│   └── cmd/
│       └── init_test.rs
├── Cargo.toml
├── specs/
└── docs/
```

## Coverage requirements

- All `completion_criteria` tests in active Task Specs must pass before
  a task is considered complete.
- New code in `src/` should have corresponding tests in `tests/`.

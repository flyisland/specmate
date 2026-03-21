---
title: "specmate Design Principles"
---

# specmate Design Principles

Cross-cutting principles that apply across all specmate commands and features.
When in doubt about a design decision, consult this document first.

---

## File ownership

specmate manages files in two distinct classes. This distinction governs
behaviour under `specmate init --merge` and `specmate update-guides`, and
must be respected by all commands that write to the filesystem.

**specmate-owned**: any `README.md` inside a specmate-managed subdirectory.
These files carry the system's self-documentation. They are generated from
embedded templates and should not be edited by users. specmate may overwrite
them silently at any time.

**user-owned**: everything else — `AGENTS.md`, `.specmate/config.yaml`,
`project.md`, `org.md`, and all docs and specs created by the team.
specmate creates these files if absent, but never overwrites them afterward.

**The rule**: if the file is a `README.md` inside a specmate-managed
subdirectory, specmate owns it. Everything else belongs to the user.

Any new command that writes files must classify each file it touches as
specmate-owned or user-owned and apply the corresponding behaviour.

---

## Language strategy

specmate supports two content languages: English (`en`) and Chinese (`zh`).
The active language is read from `.specmate/config.yaml`.

**Generated content** (README files, `AGENTS.md` template) is rendered in
the configured language.

**CLI output** (error messages, status output, dry-run output, execution
reports) is always in English, regardless of the configured language.
Rationale: CLI output is consumed by scripts, CI systems, and agents that
expect consistent, parseable English text.

When adding new generated content, always provide both `en` and `zh`
versions. When adding new CLI output, always write in English only.

---

## Git integration

specmate uses git for task isolation and traceability. All git operations
follow these principles:

**Branch naming**: `specmate/task-{id}-{slug}`

```
specmate/task-0001-implement-init-command
```

Branches are created from the current HEAD of the default branch.
specmate never deletes branches — cleanup is left to the team's workflow.

**Commit message format**:

```
task-0001: <title>

Completes task-0001-implement-init-command.
All completion criteria passed.
```

**Platform independence**: specmate integrates only with git, not with any
specific hosting platform (GitHub, GitLab, Gitea, etc.). Platform-specific
features such as PR or MR creation are out of scope for the core tool and
may be handled via optional plugins or external scripts.

---

## Single binary, zero runtime dependencies

specmate ships as a single self-contained binary. This principle applies to
all features:

- Templates are maintained as standalone files in `src/template/en/` and
  `src/template/zh/` within the repository. They are embedded at compile
  time using `include_str!` and require no external files at runtime.
  Editing a template requires recompiling specmate, but users need no
  additional files beyond the binary itself.
- Git operations use `git2` (libgit2 bindings). No dependency on a system
  git binary.
- All configuration is read from `.specmate/config.yaml`. No environment
  variables required for normal operation.

When adding new generated content, add a template file to both `src/template/en/`
and `src/template/zh/`, then embed it with `include_str!`. Never hardcode
template content directly in Rust source files.

---

## Fail loudly, never silently

specmate prefers explicit errors over silent degradation. When a constraint
is violated, specmate must:

1. Exit with a non-zero status code
2. Print a clear error message naming the specific file, rule, and action needed
3. Never proceed past a failed check by ignoring the failure

This principle applies especially to `specmate check` — every violation must
produce an actionable error message that an agent can act on without consulting
additional documentation.

---

## Document integrity

specmate-managed documents are a shared operational contract. The repository's
document model must remain verifiably compliant and self-consistent at all
times; specmate must not treat a known-invalid or half-consistent document
state as a normal input to write operations.

When a command depends on the document model to make decisions, it must first
validate the repository-level document state. This includes operations such as
allocating IDs, creating managed documents, moving managed documents, or
changing managed document status.

If the repository contains known document-model violations, specmate must stop
and report them instead of continuing on top of that damaged state. In
particular:

1. Invalid managed entries must remain visible and actionable.
2. Write operations must not infer new state from a repository that is already
   known to be invalid.
3. Users must repair document-model violations before proceeding with
   operations that mutate the managed document system.

This rule exists to prevent document rot: once specmate knows the document
system is inconsistent, it must restore compliance before making further
document-model decisions.

---

## Idempotency

All specmate commands that write files or create git branches must be
idempotent. Running the same command twice must produce the same result
as running it once, with no duplicate side effects.

This is essential for agent reliability — agents may retry failed operations,
and retries must not corrupt state.

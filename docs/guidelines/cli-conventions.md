---
title: "CLI Conventions"
---

# CLI Conventions

Standards and conventions that apply to all specmate commands. Any new command
must conform to these rules. The review agent checks compliance with this
guideline on every Task Spec that touches CLI code.

---

## Command structure

specmate uses a subcommand model. All functionality is exposed through named
subcommands, never through flags on the root command.

```
specmate <subcommand> [arguments] [options]
```

Subcommands are implemented using `clap` with the derive macro. Every subcommand
must provide a `--help` flag that describes its purpose, arguments, options, and
at least one usage example.

---

## Output language

All CLI output — error messages, status messages, dry-run output, execution
reports, progress indicators — is always in English, regardless of the
`lang` setting in `.specmate/config.yaml`.

The `lang` setting only affects generated document content (README files,
`AGENTS.md` template). It never affects CLI output.

---

## Output format

### Ownership-tagged lines

Any operation that creates, overwrites, or skips a file must prefix the line
with an ownership tag:

```
  [specmate] CREATE    specs/README.md
  [specmate] OVERWRITE docs/design-docs/README.md
  [user]     CREATE    AGENTS.md
  [user]     SKIP      specs/project.md  (already exists)
  [dir]      CREATE    docs/prd/draft/
```

Tags are left-aligned in a fixed-width column. Actions are left-aligned in a
fixed-width column. Paths follow without truncation.

Write commands must use the same ownership-tagged format for real applied
operations, not only for `--dry-run`. A successful write command must not
complete silently if it changed repository state.

Examples:

- `specmate init --merge` prints the directories and files it created,
  overwrote, or skipped
- `specmate move` prints the update and move operations it applied
- future write commands should keep their applied-output format as close as
  practical to their `--dry-run` output

### Check output

Each check result is prefixed with a status indicator:

```
[pass] check names         all 23 documents pass
[fail] check status        1 violation
       specs/active/task-0001.md: status is 'completed' but file is in active/
       -> Move to specs/archived/ or update status field
[warn] check refs          1 warning
```

Every `[fail]` line must be followed by:
1. The exact file path
2. The specific rule that was violated
3. A concrete action the user (or agent) can take to fix it

### Dry-run output

Dry-run output must begin with:

```
Planned operations (no files will be written):
```

And end with:

```
Run without --dry-run to apply.
```

---

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | Error — a rule was violated, a check failed, or an operation could not complete |
| `2` | Bad input — invalid arguments, unknown subcommand, missing required option |

specmate never exits with code `0` if any check failed or any error was
encountered. Partial success is not success.

---

## --dry-run

Every command that writes files, moves files, or creates git branches must
support `--dry-run`. When `--dry-run` is set:

- No files are created, modified, or deleted
- No git operations are performed
- Output shows exactly what would happen, using the ownership-tagged format
- Exit code is `0` if the plan is valid, `1` if pre-flight checks fail

`--dry-run` can be combined with any other flag. The combination must always
be valid and must never produce side effects.

---

## Error messages

Error messages must be actionable. Every error must answer three questions:

1. **What** went wrong — the specific file, field, or constraint
2. **Why** it is wrong — the rule that was violated
3. **How** to fix it — a concrete next step

```
# Good
[fail] specs/active/task-0001-implement-init.md
       status is 'completed' but file is in specs/active/
       -> Run: specmate move task-0001 completed

# Bad
[fail] Status mismatch detected
```

Errors that reference files must include the full path from the repo root,
never a relative or abbreviated path.

---

## Progress and feedback

Long-running commands (`specmate run`, `specmate check` on large repos) must
emit progress feedback rather than running silently.

- Use a spinner or step counter for operations whose duration is unknown
- Print each step before executing it, not after
- On completion, print a summary: how many items were checked, created, or passed

```
Running task-0001...
  [1/4] Pre-flight checks          pass
  [2/4] Creating branch            pass  specmate/task-0001-implement-init-command
  [3/4] Coding agent (round 1)     pass
  [4/4] Review agent               pass

Done. Branch ready for review: specmate/task-0001-implement-init-command
Execution report: specs/active/task-0001-implement-init-command-report.md
```

---

## Warnings vs errors

**Errors** (exit code `1`): the operation cannot or should not proceed.
specmate must stop. Prefixed with `[fail]` in output.

**Warnings** (exit code `0`): something is noteworthy but the operation
can proceed. Prefixed with `[warn]` in output. Warnings must be visible
but must not block the user.

Never downgrade an error to a warning to avoid blocking the user. If a rule
must be enforced, it is an error.

---

## Interactive prompts

specmate avoids interactive prompts in normal operation. All required information
must be provided via arguments or options.

Exception: when a destructive or irreversible action is about to be taken
(e.g. `specmate rerun --reset` deleting an existing branch), specmate may
prompt for confirmation unless `--yes` is passed.

```
specmate rerun task-0001 --reset
Branch specmate/task-0001-implement-init-command already exists.
This will delete it and start fresh. Continue? [y/N]
```

The default must always be the safe option (N / no).

---
id: task-01
title: "Implement specmate init command"
status: closed
created: 2026-03-25
closed: 2026-03-25
exec-plan: exec-implement-init-command
guidelines:
  - docs/guidelines/specmate-principles.md
  - docs/guidelines/cli-conventions.md
boundaries:
  allowed:
    - "src/cmd/init.rs"
    - "src/cmd/mod.rs"
    - "src/template/**"
    - "src/config.rs"
    - "src/error.rs"
    - "tests/cmd/init_test.rs"
  forbidden_patterns:
    - "specs/**"
completion_criteria:
  - id: "cc-001"
    scenario: "Init succeeds in an empty directory"
    test: "test_init_creates_full_directory_structure"
  - id: "cc-002"
    scenario: "Init with --lang zh generates Chinese README files"
    test: "test_init_lang_zh_generates_chinese_content"
  - id: "cc-003"
    scenario: "Init with --lang en generates English README files"
    test: "test_init_lang_en_generates_english_content"
  - id: "cc-004"
    scenario: "Init defaults to en when no --lang provided"
    test: "test_init_default_lang_is_en"
  - id: "cc-005"
    scenario: "Init in existing repo without --merge exits with an actionable error"
    test: "test_init_existing_repo_errors_and_exits"
  - id: "cc-006"
    scenario: "Init --dry-run prints planned operations without writing files"
    test: "test_init_dry_run_no_files_written"
  - id: "cc-007"
    scenario: "Init --dry-run output groups specmate-owned vs user-owned files"
    test: "test_init_dry_run_groups_output_by_ownership"
  - id: "cc-008"
    scenario: "Init --merge silently overwrites specmate-owned README files"
    test: "test_init_merge_overwrites_readmes"
  - id: "cc-009"
    scenario: "Init --merge never touches user-owned files"
    test: "test_init_merge_preserves_user_files"
  - id: "cc-010"
    scenario: "Init --merge creates missing directories and files"
    test: "test_init_merge_creates_missing_structure"
  - id: "cc-011"
    scenario: "Init generates valid .specmate/config.yaml with lang field"
    test: "test_init_generates_valid_config"
  - id: "cc-012"
    scenario: "Init generates AGENTS.md at repo root"
    test: "test_init_generates_agents_md"
  - id: "cc-013"
    scenario: "Init generates project.md and org.md templates under specs/"
    test: "test_init_generates_spec_templates"
---

## Intent

`specmate init` is the onboarding command — the first thing a developer runs in a
new repo. Its job is to deploy the entire document system structure and knowledge
base into the repo, so that both humans and agents can understand how to use
specmate without reading any external documentation.

This is the first command to implement because everything else depends on the
directory structure it creates.

## Decisions

- **Language**: Rust. Single binary, zero runtime dependencies.
- **Templates**: Embedded at compile time via `include_str!`. Templates live in
  `src/template/en/` and `src/template/zh/`. No external template files at runtime.
- **Ownership model**: Two classes of files with different merge behavior:
  - *specmate-owned*: README files inside specmate-managed subdirectories only
    (e.g. `specs/README.md`, `specs/active/README.md`, `docs/prd/README.md`, etc.).
    Silently overwritten on `--merge`.
  - *user-owned*: everything else — `AGENTS.md`, `project.md`, `org.md`, and any
    doc/spec the user creates. Created by `init` if not present, never touched again.
  - **Rule**: if a file lives inside a specmate-managed subdirectory AND is named
    `README.md`, it is specmate-owned. All other files are user-owned. No exceptions.
- **config.yaml**: Only contains `lang`. No version field. Located at `.specmate/config.yaml`.
- **Default lang**: `en` when `--lang` is not provided and no config exists.
  If config exists, read lang from config.

## Boundaries

### Allowed changes
- `src/cmd/init.rs` — command implementation
- `src/cmd/mod.rs` — register init subcommand
- `src/template/**` — embedded template files (en + zh)
- `src/config.rs` — config read/write logic
- `src/error.rs` — error types
- `tests/cmd/init_test.rs` — integration tests

### Forbidden
- Must not modify any existing user files under `docs/` or `specs/`
- Must not modify `specs/**` (this spec)
- Must not introduce any runtime file-loading — templates must be compiled in

## Directory structure created by init

```
repo/
├── AGENTS.md
├── .specmate/
│   └── config.yaml
├── specs/
│   ├── README.md
│   ├── project.md          # user-owned template
│   ├── org.md              # user-owned template
│   ├── active/
│   │   └── README.md
│   └── archived/
│       └── README.md
└── docs/
    ├── guidelines/
    ├── prd/
    │   ├── README.md
    │   ├── draft/
    │   ├── approved/
    │   └── obsolete/
    ├── design-docs/
    │   ├── README.md
    │   ├── draft/
    │   ├── candidate/
    │   ├── implemented/
    │   └── obsolete/
    └── exec-plans/
        ├── README.md
        ├── draft/
        ├── active/
        └── archived/
```

## CLI interface

```
specmate init [OPTIONS]

OPTIONS:
  --lang <LANG>    Language for generated docs [default: en] [possible values: en, zh]
  --merge          Merge into existing repo: overwrite specmate-owned files,
                   skip user-owned files, create missing structure
  --dry-run        Print planned operations without writing any files
  -h, --help       Print help
```

## --dry-run output format

```
specmate init --dry-run --lang zh

Planned operations (no files will be written):

  [user]     CREATE  AGENTS.md
  [user]     CREATE  .specmate/config.yaml
  [specmate] CREATE  specs/README.md
  [specmate] CREATE  specs/active/README.md
  [specmate] CREATE  specs/archived/README.md
  [user]     CREATE  specs/project.md
  [user]     CREATE  specs/org.md
  [dir]      CREATE  docs/guidelines/
  [specmate] CREATE  docs/prd/README.md
  ... (all files listed)

Run without --dry-run to apply.
```

When used with `--merge`:

```
specmate init --dry-run --merge

Planned operations (no files will be written):

  [specmate] OVERWRITE  specs/README.md
  [specmate] OVERWRITE  docs/design-docs/README.md
  [specmate] CREATE     docs/exec-plans/archived/   (missing directory)
  [user]     SKIP       AGENTS.md                   (user-owned, never overwritten)
  [user]     SKIP       specs/project.md            (user-owned, never overwritten)
  [user]     SKIP       specs/active/task-0001-implement-init-command.md

Run without --dry-run to apply.
```

## Error handling

| Situation | Behavior |
|---|---|
| Existing repo, no `--merge` | Print an actionable `[fail]` message explaining `--merge`, exit code 1 |
| `--merge` + `--dry-run` together | Valid combination, show merge dry-run output |
| Unknown `--lang` value | Bad input handled by clap, exit code 2 |
| File write permission denied | Error with path, exit code 1 |
| `.specmate/config.yaml` malformed | Print a visible `[warn]` message, fall back to default lang `en` |

## Completion criteria detail

### cc-001: Init creates full directory structure

Given an empty directory, `specmate init` must create all directories and files
listed in the directory structure above. Every directory must exist, every README
must be non-empty.

### cc-005: Init in existing repo fails and exits

"Existing repo" is defined as: `.specmate/config.yaml` exists OR any of the
standard directories (`specs/`, `docs/guidelines/`, `docs/prd/`,
`docs/design-docs/`, `docs/exec-plans/`) exist. The error message must mention
`--merge` and be actionable.

### cc-006 + cc-007: --dry-run writes nothing, groups output

No files or directories may be created. Output must clearly separate
`[specmate]` owned operations from `[user]` owned operations.

### cc-008 + cc-009: --merge ownership boundary

specmate-owned files: any `README.md` that lives inside a specmate-managed
subdirectory (`specs/`, `specs/active/`, `specs/archived/`, `docs/prd/`,
`docs/design-docs/`, `docs/design-docs/draft/`, etc.).

user-owned files: `AGENTS.md`, `.specmate/config.yaml`, `specs/project.md`,
`specs/org.md`, and any file not matching the specmate-owned rule above.

The rule in one sentence: **if the file is a `README.md` inside a
specmate-managed subdirectory, specmate owns it. Everything else belongs to
the user.**

User-owned files must be completely untouched on `--merge` — not read, not
stat-checked for content, not overwritten. If a user-owned file doesn't exist
yet, create it.

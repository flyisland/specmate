---
id: design-002
title: "specmate Core"
status: candidate
design-doc: design-001
guidelines:
  - docs/guidelines/specmate-principles.md
---

# specmate Core

This document covers the foundational elements shared across all specmate
commands: project configuration and the machine-readable fields that specmate
parses from Task Specs at runtime.

The document model (types, statuses, transitions) is in design-002-doc-model.
The check engine is in design-003-check-engine.
The agent loop is in design-004-agent-loop.
Individual command designs are in their own Design Docs.

---

## 1. Template system

specmate generates files from templates when running `specmate init` and
`specmate update-guides`. Templates are maintained as standalone `.md` files
in the repository under `src/template/` and embedded at compile time using
`include_str!`. No external template files are required at runtime.

**Template directory layout:**

```
src/template/
├── en/
│   ├── AGENTS.md
│   ├── specs-README.md
│   ├── specs-active-README.md
│   ├── specs-archived-README.md
│   ├── prd-README.md
│   ├── design-docs-README.md
│   ├── exec-plans-README.md
│   ├── project.md
│   └── org.md
└── zh/
    ├── AGENTS.md
    ├── specs-README.md
    ├── specs-active-README.md
    ├── specs-archived-README.md
    ├── prd-README.md
    ├── design-docs-README.md
    ├── exec-plans-README.md
    ├── project.md
    └── org.md
```

Every template must exist in both `en/` and `zh/`. A missing template in
either language is a compile error, not a runtime error.

Template content must never be hardcoded in Rust source files. When adding
a new generated file, create the template in both language directories first,
then embed with `include_str!`.

The `lang` setting in `.specmate/config.yaml` selects which set of templates
to use. `specmate update-guides` re-deploys all specmate-owned files from
the embedded templates, overwriting any previous content.

---

## 2. Project configuration

specmate stores project-level configuration in `.specmate/config.yaml`.

```yaml
lang: en   # en | zh
```

**This file is user-owned.** Created by `specmate init` if absent, never
overwritten afterward.

**Fallback behaviour**: if the file is missing or the YAML is malformed,
specmate falls back to `lang: en` and prints a warning. specmate never
refuses to run because of a missing or malformed config.

**Changes take effect immediately** on the next specmate command. There is
no need to restart or reload.

---

## 2. Task Spec machine-readable fields

The following frontmatter fields are parsed and executed by specmate at
runtime. All other fields are treated as human-readable metadata and are
ignored by the tool (though they may be read by agents as context).

### `guidelines`

```yaml
guidelines:
  - docs/guidelines/security.md
  - docs/guidelines/reliability.md
```

Optional. A list of guideline file paths relative to the repo root.

When `specmate run` starts, each listed file is read from disk and injected
verbatim into the coding agent and review agent context. The review agent
is expected to verify that the implementation conforms to the referenced
guidelines as part of its review pass.

Files listed here must exist. A missing guideline file is a pre-flight
check failure.

### `boundaries`

```yaml
boundaries:
  allowed:
    - "src/cmd/init.rs"
    - "src/**/*.rs"
  forbidden_patterns:
    - "specs/**"
```

Required for Task Specs with status `active`.

`allowed` is a list of glob patterns relative to the repo root. Files
changed in git must match at least one allowed pattern to pass
`specmate check boundaries`.

`forbidden_patterns` is a list of glob patterns that the agent must never
touch, regardless of whether they would otherwise match `allowed`. A file
that matches both `allowed` and `forbidden_patterns` is treated as forbidden.

`specs/**` must always appear in `forbidden_patterns` to prevent agents
from modifying their own spec.

### `completion_criteria`

```yaml
completion_criteria:
  - id: "cc-001"
    scenario: "Init succeeds in an empty directory"
    test: "test_init_creates_full_directory_structure"
  - id: "cc-002"
    scenario: "Chinese README is generated when --lang zh is passed"
    test: "test_init_lang_zh_generates_chinese_content"
```

Required for Task Specs with status `active`. Must contain at least one item.

Each item binds a human-readable scenario description to an exact test
function name. `specmate run` executes each test by name using the project's
test runner (configured in `specs/project.md`). All criteria must pass for
the loop to proceed to the review agent.

`id` must be unique within the spec and follow the format `cc-NNN`.
`scenario` must be non-empty. `test` must be non-empty and must match an
existing test function at the time of execution.

`skip != pass`. A test that does not exist or is not discovered by the test
runner is treated as a failure, not a skip.

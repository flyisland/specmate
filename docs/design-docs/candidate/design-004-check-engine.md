---
id: design-004
title: "Check Engine"
status: candidate
design-doc: design-001
guidelines:
  - docs/guidelines/specmate-principles.md
  - docs/guidelines/cli-conventions.md
---

# Check Engine

This document defines the check engine — the subsystem that validates the
document system's structural and semantic integrity. All `specmate check`
subcommands are implemented through this engine.

---

## 1. Design principles

**Checks are pure reads.** The check engine never modifies files, moves
documents, or creates git branches. It is always safe to run.

**Checks are composable.** Each check is an independent unit that takes the
document index as input and returns a list of violations. `specmate check`
runs all checks and aggregates results. `specmate check <name>` runs a
single check.

**Every violation is actionable.** Each violation must include the file path,
the rule violated, and a concrete fix. See `docs/guidelines/cli-conventions.md`
for the required error format.

---

## 2. Document index

Before running any check, the check engine builds a document index — a
complete in-memory map of all documents in the repo.

```
DocumentIndex:
  documents: Map<DocId, Document>
  by_type:   Map<DocType, Vec<Document>>
  by_status: Map<Status, Vec<Document>>
  by_path:   Map<PathBuf, Document>
```

The index is built by scanning all known directories, parsing each `.md`
file's frontmatter, and validating the filename pattern. Files that do not
match any known pattern are ignored.

---

## 3. Checks

### check names

Validates that every file's name conforms to the naming pattern for its
inferred document type.

**Rules:**
- PRD: `prd-{NNN}-{slug}.md` where NNN is exactly 3 digits
- Design Doc: `design-{NNN}-{slug}.md` where NNN is exactly 3 digits
- Design Patch: `design-{NNN}-patch-{NN}-{slug}.md` where NNN is 3 digits, NN is 2 digits
- Exec Plan: `exec-{NNN}-{slug}.md` where NNN is exactly 3 digits
- Task Spec: `task-{NNNN}-{slug}.md` where NNNN is exactly 4 digits
- Slug: lowercase, hyphen-separated, 1–5 words, matches `[a-z][a-z0-9-]*`

**Infers DocType from directory**, not from frontmatter, since a malformed
filename cannot reliably yield a DocType.

### check frontmatter

Validates frontmatter fields for every document.

**Rules:**
- `title` is present and non-empty
- `status` is present and is a valid value for this DocType
- `obsolete:merged` DesignPatch has `merged-into` field pointing to an existing doc
- `obsolete` DesignDoc (Flow B) has `superseded-by` field pointing to an existing doc
- TaskSpec with status `active` has non-empty `boundaries.allowed`
- TaskSpec with status `active` has non-empty `completion_criteria`
- Each `completion_criteria` item has `id`, `scenario`, and `test` fields
- `cc-` IDs are unique within a single TaskSpec

### check status

Validates that every file lives in the directory that matches its status.

Uses the directory resolver from design-003 to compute the expected directory
for each document's `(DocType, Status)` pair, then compares against the
actual file location.

**Violation example:**
```
[fail] check status
       specs/active/task-0003-add-payment.md
       status is 'completed' but file is in specs/active/
       -> Run: specmate move task-0003 completed
```

### check refs

Validates that no active document references an obsolete or archived document.

**Rules:**
- A document with status `candidate` or `implemented` (DesignDoc) must not
  reference a `prd` that is `obsolete`
- A document with status `active` or `completed` (ExecPlan, TaskSpec) must
  not reference a `design-doc` or `exec-plan` that is `obsolete`, `abandoned`,
  or `obsolete:merged`

### check boundaries `<task-id>`

Validates that the files changed in the current git working tree or staged
area fall within the `boundaries.allowed` patterns of the specified Task Spec.

**Steps:**
1. Load the Task Spec and parse `boundaries.allowed` and `boundaries.forbidden_patterns`
2. Get the list of changed files from git (working tree diff against HEAD)
3. For each changed file:
   - If it matches any `forbidden_patterns` → violation
   - If it does not match any `allowed` pattern → violation

**Violation example:**
```
[fail] check boundaries task-0001
       src/cmd/new.rs is not in boundaries.allowed
       -> This file is outside the scope of task-0001
       -> Allowed: src/cmd/init.rs, tests/cmd/init_test.rs
```

### check conflicts

Validates that no two active Task Specs have overlapping `boundaries.allowed`
entries.

**Algorithm:**
1. Load all Task Specs with status `draft` or `active`
2. For each pair of specs, check if any `allowed` pattern from one spec
   matches any `allowed` pattern from the other (using glob matching)
3. If overlap is found → violation listing both specs and the overlapping pattern

**Violation example:**
```
[fail] check conflicts
       task-0003 <-> task-0005: 'src/config.rs' overlaps 'src/**/*.rs'
       -> Resolve by serialising the tasks or splitting boundaries
```

---

## 4. Output aggregation

When running `specmate check` (all checks), results are grouped by check name:

```
specmate check

[pass] check names         all 23 documents pass
[pass] check frontmatter   all 23 documents pass
[fail] check status        1 violation
       specs/active/task-0003-add-payment.md
       status is 'completed' but file is in specs/active/
       -> Run: specmate move task-0003 completed
[pass] check refs          all references valid
[pass] check conflicts     no boundary conflicts

1 check failed. Fix violations before running specmate run.
```

Exit code is `1` if any check fails, `0` if all pass.

---

## 5. CI integration

`specmate check` is designed to run as a CI gate. Recommended configuration:

```yaml
# .github/workflows/specmate.yml (example — platform-agnostic logic)
- name: specmate check
  run: specmate check
```

`check boundaries` must be run with the Task Spec ID of the current PR's
task, and is typically run as a pre-push hook or PR gate:

```bash
specmate check boundaries task-0001
```

`check conflicts` should run on every push to ensure no two active specs
have developed overlapping boundaries.

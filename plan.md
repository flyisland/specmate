# Implementation Plan: Branch-friendly IDs and Unified Exec/Task Structure

## 1. Goal

Primary design input for this implementation plan:

- `docs/design-docs/candidate/design-001-patch-02-parallel-safe-ids-and-structure.md`

Implement the structural change described in the design patch, with the
clarifications already agreed during discussion:

- replace repository-wide numeric IDs with slug-led identities:
  slug-based IDs for PRD, Design Doc, and Exec Plan, plus parent-scoped /
  exec-scoped local numbering for Design Patch and Task Spec
- make Exec Plan a directory with `plan.md` as the plan document
- make Task Spec IDs locally scoped within an Exec Plan
- move `project.md` and `org.md` from `specs/` to `docs/specs/`
- unify Exec Plan and Task Spec statuses to `draft -> candidate -> closed`
- add `created` to all lifecycle-managed docs
- add `closed` to Exec Plan and Task Spec only

This is a repository-wide model migration, not a small feature change.

---

## 2. Agreed Decisions

### 2.1 Directory and naming

- use `docs/design/` instead of `docs/design-docs/`
- use `docs/specs/project.md` and `docs/specs/org.md`
- keep Task Spec files as `.md`
- Exec Plan lives at:

```text
docs/exec-plans/exec-<slug>/plan.md
```

- Task Spec lives alongside the plan:

```text
docs/exec-plans/exec-<slug>/task-<nn>-<slug>.md
```

### 2.2 Canonical IDs

- PRD: `prd-<slug>`
- Design Doc: `design-<slug>`
- Design Patch: `design-<parent-slug>-patch-<nn>-<patch-slug>`
- Exec Plan: `exec-<slug>`
- Task Spec frontmatter id: `task-<nn>`
- Task Spec global canonical identity: `<exec-id>/task-<nn>`
- Task Spec escaped single-token rendering for git/report/file-stem surfaces:
  `<exec-id>--task-<nn>`

### 2.3 References and fields

- Exec Plan uses `design-docs` as a list field
- Task Spec must belong to an Exec Plan
- standalone Task Spec is not supported in the new model
- implemented Design Docs remain the primary long-lived source of truth for a
  module or domain topic; Design Patches are temporary change vehicles whose
  content should be merged back into the parent Design Doc after implementation
- no `seq` field
- `docs/guidelines/` is directory-declared always-inject context, not
  per-Task frontmatter-declared injection
- cross-cutting design principles live under `docs/design/` with
  `design-principles-` prefixes and the same lifecycle as module-scoped design
  docs

### 2.4 Date fields

- all lifecycle-managed docs have `created`
- Exec Plan and Task Spec also have `closed`
- PRD / Design Doc / Design Patch do not gain terminal-date fields in this
  change

### 2.5 Compatibility strategy

- do not preserve compatibility with the old numeric-ID model or old directory
  layout during implementation
- execute the migration as one atomic cutover for the current repository, code,
  tests, templates, and docs
- avoid dual-read, dual-write, fallback parsing, alias IDs, or transitional
  compatibility layers unless a later design explicitly introduces them
- implementation phases describe work decomposition inside that one cutover;
  they are not intended to be merged or released as independent intermediate
  states

---

## 3. Desired End State

### 3.1 Lifecycle status model

- PRD: `draft -> approved -> obsolete`
- Design Doc: `draft -> candidate -> implemented -> obsolete`
- Design Patch: `draft -> candidate -> implemented -> obsolete:merged | obsolete`
- Exec Plan: `draft -> candidate -> closed`
- Task Spec: `draft -> candidate -> closed`

### 3.2 Directory model

```text
repo/
├── AGENTS.md
└── docs/
    ├── specs/
    │   ├── project.md
    │   └── org.md
    ├── guidelines/
    │   ├── coding-standards.md
    │   ├── error-handling.md
    │   └── obsolete/
    ├── prd/
    │   ├── draft/
    │   ├── approved/
    │   └── obsolete/
    ├── design/
    │   ├── draft/
    │   ├── candidate/
    │   ├── implemented/
    │   └── obsolete/
    ├── exec-plans/
    │   └── exec-<slug>/
    │       ├── plan.md
    │       └── task-<nn>-<slug>.md
```

`docs/design/` contains both module-scoped design docs and cross-cutting
design-principles docs such as `design-principles-errors.md`. They share the
same lifecycle and become agent-operable only after `candidate` approval.

`docs/guidelines/` is always-injected operational reference material. The
directory itself declares injection; Task Specs do not list guideline files in
frontmatter to opt in.

### 3.3 Example frontmatter

PRD:

```yaml
---
id: prd-user-registration
title: "User Registration"
status: draft
created: 2026-03-24
---
```

Design Doc:

```yaml
---
id: design-auth-system
title: "Auth System"
status: candidate
created: 2026-03-24
prd: prd-user-registration
---
```

Exec Plan:

```yaml
---
id: exec-auth-add-oauth
title: "Add OAuth to Auth System"
status: candidate
created: 2026-03-24
design-docs:
  - design-auth-system
  - design-auth-system-patch-01-add-oauth
---
```

Task Spec:

```yaml
---
id: task-01
title: "Add OAuth provider configuration"
status: candidate
created: 2026-03-24
exec-plan: exec-auth-add-oauth
boundaries:
  allowed:
    - "src/auth/oauth.rs"
  forbidden_patterns:
    - "docs/prd/**"
    - "docs/design/**"
    - "docs/guidelines/**"
    - "docs/specs/**"
    - "docs/exec-plans/**"
completion_criteria:
  - id: "cc-001"
    scenario: "OAuth provider initialises with valid config"
    test: "test_oauth_provider_init_with_valid_config"
---
```

Closed Task Spec:

```yaml
---
id: task-01
title: "Add OAuth provider configuration"
status: closed
created: 2026-03-24
closed: 2026-03-25
exec-plan: exec-auth-add-oauth
---
```

---

## 4. Current-State Gap Summary

The current repository and implementation still assume the old model:

- numeric canonical IDs
- `docs/design-docs/`
- `specs/project.md`
- Exec Plan as a single markdown file
- Task Spec stored under `specs/active/` and `specs/archived/`
- Exec Plan statuses `draft/active/completed/abandoned`
- Task Spec statuses `draft/active/completed/cancelled`
- single `design-doc` reference on Exec Plan
- completion criterion IDs in `cc-NNN` format

The current test baseline is green, so this work should be treated as a
deliberate full-model migration rather than an opportunistic fix.

---

## 5. Impact Areas

### 5.1 Design contracts

These design docs must be updated to reflect the new model:

- current path `docs/design-docs/implemented/design-001-document-system.md`,
  target shape under the new model:
  `docs/design/implemented/design-<slug>.md`
- current path `docs/design-docs/implemented/design-003-doc-model.md`, target
  shape under the new model: `docs/design/implemented/design-<slug>.md`
- current path `docs/design-docs/implemented/design-004-check-engine.md`,
  target shape under the new model: `docs/design/implemented/design-<slug>.md`
- current path `docs/design-docs/implemented/design-007-move-command.md`,
  target shape under the new model: `docs/design/implemented/design-<slug>.md`
- current path `docs/design-docs/implemented/design-008-status-command.md`,
  target shape under the new model: `docs/design/implemented/design-<slug>.md`
- current path `docs/design-docs/candidate/design-006-cli-surface-roadmap.md`,
  target shape under the new model: `docs/design/draft/design-<slug>.md`
- `design-001` and `design-003` must also absorb the new boundary between
  cross-cutting design principles in `docs/design/design-principles-*.md` and
  always-injected operational guidance in `docs/guidelines/`

### 5.2 Core document model

Likely affected modules:

- `src/doc/types.rs`
- `src/doc/frontmatter.rs`
- `src/doc/id.rs`
- `src/doc/mod.rs`
- `src/error.rs`

Primary changes:

- replace numeric `DocId` variants with the new canonical identity model
- represent scoped Task identity cleanly
- parse and validate `created` and `closed`
- support `design-docs` list
- keep completion criterion ID format as `cc-NNN`
- rework filename parsing and managed-path classification
- replace global next-ID allocation with creation helpers that validate slug
  uniqueness and allocate parent-scoped / exec-scoped local sequence numbers
- remove status-directory mapping for Exec Plan and Task Spec
- reclassify guideline handling from Task-linked references to
  directory-declared always-inject context
- support cross-cutting design-principles documents within the design-doc
  lifecycle and indexing model

### 5.3 Commands

Likely affected command modules:

- `src/cmd/init.rs`
- `src/cmd/check.rs`
- `src/cmd/move_.rs`
- `src/cmd/status.rs`
- `src/check/mod.rs`

Primary changes:

- repo root discovery must use `docs/specs/project.md`
- command help and examples must use new IDs
- status parsing and transition validation must use `candidate/closed`
- `check conflicts` must continue to reject overlapping `candidate` Task Specs;
  `draft` Task Specs are not executable and must not block execution
- Task lookup must accept `<exec-id>/task-<nn>`
- human-facing and CLI-facing Task identifiers must use the canonical rendering
  `<exec-id>/task-<nn>`, not bare `task-<nn>`
- git-facing or other single-token surfaces that cannot safely carry `/` must
  use the escaped rendering `<exec-id>--task-<nn>`
- outputs and diagnostics must use new paths and field names
- any command or runtime path that currently depends on TaskSpec guideline
  frontmatter for injection must move to directory-based guideline loading

### 5.4 Templates and generated repo layout

Likely affected templates:

- `src/template/en/*`
- `src/template/zh/*`

Primary changes:

- generated layout from `init`
- README text for `docs/design/`, `docs/exec-plans/`, and `docs/specs/`
- AGENTS template references to project/design paths and status vocabulary

### 5.5 Repository onboarding and guidance docs

Likely affected docs:

- `AGENTS.md`
- `docs/guidelines/specmate-principles.md`
- `docs/guidelines/cli-conventions.md`

Primary changes:

- update path references from `specs/project.md` to `docs/specs/project.md`
- update path references from `docs/design-docs/` to `docs/design/`
- update command examples and output examples that mention old Exec/Task paths
- ensure guidance examples use `candidate/closed` where they currently use
  `active/completed/cancelled/abandoned`
- clarify the boundary: design docs explain why the system is shaped a certain
  way; guidelines tell agents what code patterns and operational rules to apply
- update git-facing examples such as branch naming, commit subjects, and task
  references to use the canonical or escaped Task rendering consistently,
  depending on whether the surface can safely carry `/`

### 5.6 Tests

Likely affected test files:

- `tests/doc_model_test.rs`
- `tests/cmd/init_test.rs`
- `tests/cmd/move_test.rs`
- `tests/cmd/status_test.rs`
- `tests/cmd/check_*`

Primary changes:

- fixture paths
- canonical IDs
- expected CLI text
- transition rules
- new date-field validation
- guideline loading and obsolete-guideline exclusion
- design-principles document classification

---

## 6. Implementation Phases

### Phase 1: Update the design contracts

Goal:

- make the written contracts internally consistent before or alongside code
  refactoring

Work:

- update `design-001` with the new naming, structure, status, and frontmatter
  rules
- update `design-003` with the new internal model contract
- update `design-004` for the new Task ID shape, new Exec/Task status
  vocabulary, new Task/Exec paths, and the updated `check conflicts` contract
- update `design-007` for new IDs, new status vocabulary, and new Exec/Task
  path rules
- update `design-008` for new lookup IDs and dashboard/detail behavior
- update `design-005` for the new Exec/Task lifecycle, task identity rendering,
  guideline loading model, dependency vocabulary, branch/report paths,
  committed-spec execution semantics, and completion flow assumptions
- update `design-006` to reflect that `move` and `status` already exist, and to
  use the new terminology
- define the explicit contract boundary between `docs/design/` cross-cutting
  principles and `docs/guidelines/` operational standards
- define the one-time migration exception for existing managed documents:
  - the pre-migration repository may rewrite existing managed docs in place,
    including terminal docs, solely to adopt the new IDs, paths, and required
    metadata
  - this exception applies only inside the single atomic cutover from the old
    model to the new model
  - after the cutover completes, the normal terminal-document mutability rules
    apply again

Deliverable:

- repository design docs describe one coherent target model, including the
  one-time migration exception that enables atomic adoption of that model

### Phase 2: Refactor the document model

Goal:

- make parsing, indexing, validation, and transition logic speak the new model

Work:

- redesign `DocId`
- redesign status parsing
- add date fields to typed frontmatter
- replace single `design-doc` on Exec Plan with `design-docs`
- migrate all managed-document reference fields to the new canonical ID model,
  including `prd`, `design-doc`, `design-docs`, `exec-plan`, `parent`,
  `merged-into`, and `superseded-by`
- make Task identity scoped to Exec Plan
- preserve completion criterion ID format as `cc-NNN`
- change path classification to:
  - `docs/design/...`
  - `docs/specs/...`
  - `docs/exec-plans/<exec-id>/plan.md`
  - `docs/exec-plans/<exec-id>/task-<nn>-<slug>.md`
- update transition gates and validation rules
- remove TaskSpec-driven guideline injection semantics from the runtime model
- classify `design-principles-*.md` as design artefacts, not guidelines

Deliverable:

- document-model tests pass against the new filesystem and ID model

### Phase 3: Migrate command behavior

Goal:

- make CLI commands operate correctly on the new model

Work:

- update `init` to generate the new layout
- update `check` to validate the new layout and references
- update `move` to work without status directories for Exec/Task and to set the
  required `closed` timestamp when closing Exec Plans or Task Specs
- update `status` to resolve scoped Task IDs and render the new views
- update any document-creation path or shared allocator helpers to use slug
  validation plus parent/exec-local numbering instead of repository-wide next-ID
  allocation
- update executable-task assumptions from `active` to `candidate`
- ensure agent/runtime context assembly uses `docs/guidelines/` as the
  always-injected set, excluding `docs/guidelines/obsolete/`
- ensure runtime design-context assembly always includes implemented
  `design-principles-*.md`, plus any draft/candidate principles explicitly
  listed in `Exec Plan.design-docs`

Deliverable:

- command integration tests pass on the new model

### Phase 4: Migrate repository contents and templates

Goal:

- move this repository's own managed docs and templates to the new structure

Work:

- define a deterministic migration map from current numeric IDs to new slug IDs
- define a deterministic per-Exec numbering map for existing Task Specs
- move `docs/design-docs/` to `docs/design/`
- move `specs/project.md` and `specs/org.md` to `docs/specs/`
- create or migrate `docs/guidelines/obsolete/` as needed
- move existing Exec Plans into `docs/exec-plans/<exec-id>/plan.md`
- move existing Task Specs into the matching Exec directories
- rewrite frontmatter IDs, paths, and references for existing managed docs as
  part of the one-time atomic migration exception defined in Phase 1
- update root `AGENTS.md` and guideline docs that hardcode old paths or status
  vocabulary
- update template content in both languages

Deliverable:

- repository contents conform to the same model the code enforces

Migration rules:

- slug mapping must be decided once before file moves, then applied
  consistently across filenames, frontmatter, references, tests, and examples
- because this migration is a hard cutover, the mapping is used to replace the
  old model, not to maintain backward-compatible aliases
- historical `created` and `closed` values must be backfilled deterministically:
  - `created` = the document's existing frontmatter date if present; otherwise
    the date of the earliest git commit that introduced the file at its
    pre-migration path
  - `closed` for historical Exec Plans and Task Specs = the existing
    frontmatter date if present; otherwise the date of the git commit that last
    moved the document into its old terminal state, or if unavailable, the last
    commit that modified the document before migration
- if git history is unavailable or ambiguous for a required backfill value, the
  migration must fail with an actionable error and require an explicit manual
  override value during the cutover
- existing Task Specs linked to the same Exec Plan must receive stable local
  numbering such as `task-01`, `task-02`, ... in a deterministic order
- deterministic order should be based on current task ID ascending unless a
  document-specific reason requires a different mapping
- Task filenames and frontmatter IDs must remain aligned after renumbering
- historical standalone Task Specs with no owning Exec Plan must not be
  assigned heuristically from free-form task content
- each historical standalone Task Spec must instead receive an explicit
  migration mapping during the cutover:
  - attach it to an existing migrated Exec Plan, or
  - create a synthetic closed Exec Plan dedicated to that task
- when a synthetic closed Exec Plan is created for a historical standalone
  Task Spec, its slug, `design-docs`, `created`, and `closed` values must all be
  provided explicitly in the migration manifest rather than inferred
- if any historical standalone Task Spec lacks an explicit mapping, the
  migration must fail with an actionable error instead of guessing ownership

### Phase 5: Stabilise and verify

Goal:

- verify the migration end-to-end

Work:

- run `cargo fmt`
- run `cargo clippy -- -D warnings`
- run `cargo test`
- manually spot-check `specmate status`, `specmate move`, and `specmate check`
  flows using the migrated repo

Deliverable:

- green validation baseline on the new model

---

## 7. Key Validation Rules To Add or Change

### 7.1 Dates

- `created` is required for all lifecycle-managed docs
- `created` must be a valid date
- `closed` is allowed only on Exec Plan and Task Spec
- `closed` is required when Exec Plan or Task Spec status is `closed`
- `closed` must be absent when status is not `closed`
- when both exist, `closed` must be on or after `created`

### 7.2 Exec Plan

- `plan.md` must live directly under `docs/exec-plans/<exec-id>/`
- frontmatter `id` must match the containing directory name
- `design-docs` must exist and contain at least one value
- every referenced design document must exist and have the correct type
- every referenced design document must already be in an agent-actionable or
  source-of-truth state before the Exec Plan becomes `candidate`; in practice,
  each referenced design must be `candidate` or `implemented`
- an implemented Design Patch must not become the long-term steady-state source
  of truth; after its change is absorbed, its content should be merged into the
  parent Design Doc and the patch should transition to `obsolete:merged`
- a Design Doc may transition to `implemented` only when its implementation is
  proven separately; `Exec Plan.closed` alone is not sufficient evidence
- if one Exec Plan references multiple design docs or design patches, that same
  Exec Plan is considered associated with each referenced design for `status`,
  association summaries, and transition gates
- design patches referenced in `design-docs` participate in Exec Plan
  association summaries and gating the same way as other referenced designs for
  the duration of that Exec Plan
- if an Exec Plan references a design patch in `design-docs`, the parent Design
  Doc must also be listed explicitly so execution context always includes the
  base contract together with the patch delta
- because `closed` is only a terminal historical state, downstream semantic
  claims such as `Design Doc -> implemented` must not be inferred mechanically
  from Exec Plan closure alone

### 7.3 Guidelines and design principles

- documents under `docs/guidelines/` are always injected into agent context by
  directory membership, not by Task Spec frontmatter declaration
- documents under `docs/guidelines/obsolete/` are not injected
- guideline documents do not participate in lifecycle status transitions
- documents named `design-principles-*.md` under `docs/design/` participate in
  the same lifecycle and validation rules as other design docs
- implemented `design-principles-*.md` docs are always included in execution
  design context
- draft/candidate `design-principles-*.md` docs are included only when
- candidate `design-principles-*.md` docs are included only when explicitly
  listed in `Exec Plan.design-docs`
- draft `design-principles-*.md` docs must not be included in execution context
- design-principles docs answer "why is the system shaped this way?"
- guideline docs answer "what operational rules and coding patterns should be
  applied?"

### 7.4 Task Spec

- file must live directly under the owning Exec Plan directory
- frontmatter `id` must be `task-<nn>`
- canonical lookup identity is `<exec-id>/task-<nn>`
- all git-facing, CLI-facing, and user-facing references to a Task Spec must
  use the canonical rendering `<exec-id>/task-<nn>`, except git/report/file-stem
  surfaces that must use the escaped rendering `<exec-id>--task-<nn>`
- `exec-plan` must exist and must match the containing Exec Plan directory
- `task-<nn>` must be unique within the containing Exec Plan directory
- standalone task specs are invalid
- `docs/specs/**` must replace `specs/**` in forbidden-pattern validation
- executable Task Specs must not allow modifications to managed control docs
  under `docs/prd/**`, `docs/design/**`, `docs/guidelines/**`,
  `docs/exec-plans/**`, or `docs/specs/**`
- the protected-doc boundary applies to agent-authored task changes, not to
  specmate's own mechanical workflow writes such as closing the owning Task Spec
  or writing the designated execution report
- runtime boundary enforcement must treat the owning `plan.md`, sibling task
  files, and unrelated Exec Plan directories as protected unless a later design
  explicitly relaxes that rule
- completion criterion IDs continue to use `cc-NNN`
- candidate Task Specs may be edited in place, but any change that materially
  alters the execution contract — especially boundaries, dependencies, or
  completion criteria — requires renewed human confirmation before execution
  continues under the revised spec
- operationally, `run`/`rerun` execute against the current committed candidate
  Task Spec text; a new explicit human `run`/`rerun` invocation is the renewed
  confirmation event for any materially revised committed candidate spec
- predecessor dependencies are serialization rules only: downstream work may
  require predecessor Task Specs to be terminal (`closed`), but `closed` alone
  must not be interpreted as proof of successful delivery; a closed predecessor
  satisfies only the mechanical ordering gate, while downstream continuation
  remains an explicit human `run`/`rerun` decision

### 7.5 Status transitions

- Exec Plan:
  - `draft -> closed` when the plan is intentionally dropped before approval but
    should remain as historical record
  - `draft -> candidate`
  - `candidate -> draft` when the approved plan should stop being acted on and
    return to the writing stage
  - `candidate -> closed`
- Task Spec:
  - `draft -> closed` when the task is intentionally dropped before approval but
    should remain as historical record
  - `draft -> candidate`
  - `candidate -> draft` when the approved task spec should stop being acted on
    and return to the writing stage
  - `candidate -> closed`
- Design promotion must remain a separate semantic judgment about code/doc
  consistency, not a mechanical consequence of Exec Plan closure

---

## 8. Major Risks

### 8.1 Scoped Task IDs affect many layers

Task identity is no longer a single local token. Lookup, sorting, rendering,
reference validation, and CLI parsing will all need careful changes.

### 8.2 Path classification becomes more structural

The loader can no longer infer document type from filename alone for Exec Plan
and Task Spec. Directory context becomes part of identity.

### 8.3 Repository migration and code migration must stay aligned

If the repository contents are migrated before the loader and commands are
ready, the tool breaks. If the code changes first but tests still use the old
fixtures, the suite breaks. This needs staged but tightly coordinated changes.

### 8.4 Existing design docs are part of the product

Because specmate dogfoods its own document system, repository docs are not just
examples. They are part of the managed state and must be migrated with the code.

---

## 9. Recommended Execution Strategy

Recommended order:

1. update design docs to the clarified target model
2. refactor doc-model types and parsing
3. refactor validation and transition logic
4. refactor command behavior
5. migrate templates
6. migrate repository-managed docs and tests
7. run full validation

Reason:

- the document model is the shared foundation
- command-level changes are difficult to stabilise before identity and path
  rules are final
- template and repository migration should happen after the code understands the
  new world

---

## 10. Open Items For Follow-up Discussion

- whether `status` should sort active/candidate Exec Plans by `created`
  additionally, or keep pure canonical-id order
- whether a future cold-archive command should move long-closed Exec directories
  elsewhere without changing canonical identity

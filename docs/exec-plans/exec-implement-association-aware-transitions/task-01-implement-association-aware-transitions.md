---
id: task-01
title: "Implement association-aware transitions"
status: closed
created: 2026-03-25
closed: 2026-03-25
exec-plan: exec-implement-association-aware-transitions
guidelines:
  - docs/guidelines/specmate-principles.md
  - docs/guidelines/cli-conventions.md
boundaries:
  allowed:
    - "docs/design-docs/draft/design-003-patch-01-association-aware-transitions.md"
    - "docs/design-docs/draft/design-004-patch-01-check-refs-steady-state-links.md"
    - "src/doc/**"
    - "src/check/**"
    - "src/error.rs"
    - "tests/doc_model_test.rs"
    - "tests/cmd/check_index_test.rs"
  forbidden_patterns:
    - "specs/**"
completion_criteria:
  - id: "cc-001"
    scenario: "Transition-time gates reject blocked association-aware transitions, including `prd -> obsolete` when any linked Design Doc is still live (`draft`, `candidate`, or `implemented`), plus `design -> implemented`, `design -> obsolete`, `design patch -> obsolete:merged`, `exec -> completed`, and `exec -> abandoned` when linked work or references still block the move."
    test: "test_validate_transition_rejects_blocked_association_aware_moves"
  - id: "cc-002"
    scenario: "Steady-state validation allows an already `implemented` Design Doc to have later draft or active bug-fix work linked through a new Exec Plan and Task Spec."
    test: "test_validate_index_allows_later_bugfix_work_for_implemented_design"
  - id: "cc-003"
    scenario: "Steady-state validation preserves historical links, allowing completed or cancelled descendants to keep references to obsolete or abandoned parents when the target still exists and the relationship type is correct."
    test: "test_validate_index_preserves_historical_association_links"
  - id: "cc-004"
    scenario: "Steady-state validation still rejects stale or forbidden references created by obsolete or abandoned linked documents."
    test: "test_validate_index_rejects_stale_associated_references"
  - id: "cc-005"
    scenario: "Predicted post-transition validation rejects a locally legal status edge when the resulting repository would violate steady-state association validity."
    test: "test_validate_preview_rejects_post_transition_repository_violation"
  - id: "cc-006"
    scenario: "Predicted post-transition validation succeeds when the linked documents satisfy the required association-aware gate for the requested move."
    test: "test_validate_preview_accepts_satisfied_association_aware_move"
  - id: "cc-007"
    scenario: "Association-summary queries return linked document ids plus caller-selected target-status and terminal-state aggregates for PRD, Design Doc, Design Patch, and Exec Plan relationships using per-doc-type terminal definitions."
    test: "test_association_summaries_report_related_documents_target_statuses_and_terminal_states"
  - id: "cc-008"
    scenario: "`specmate check refs` reflects the steady-state rules: it permits ongoing bug-fix work against an implemented design but fails on truly invalid linked references."
    test: "test_check_refs_distinguishes_steady_state_validity_from_transition_gates"
---

# Intent

Implement `design-003-patch-01-association-aware-transitions` in the shared
document model and align `specmate check` with the resulting steady-state
reference semantics through a paired `design-004` patch.

This task should keep the model-level split explicit:

- `check` validates steady-state repository validity only
- transition-time gates remain available to mutating commands such as
  `specmate move` and future `specmate run`

The task does not implement the `specmate move` command itself.

# Boundaries

- `docs/design-docs/draft/design-003-patch-01-association-aware-transitions.md`
- `docs/design-docs/draft/design-004-patch-01-check-refs-steady-state-links.md`
- `src/doc/**`
- `src/check/**`
- `src/error.rs`
- `tests/doc_model_test.rs`
- `tests/cmd/check_index_test.rs`

# Completion criteria

- `cc-001` through `cc-008` all pass in the current codebase.

# Design constraints

- Keep transition legality in the shared document model rather than the check
  layer.
- Reuse repository-index validation for steady-state rules; do not make
  `specmate check` enforce transition-time gates.
- Post-transition validation must evaluate the predicted repository state
  without mutating the working tree.
- Association-summary helpers must expose facts only; they must not trigger
  cascading status changes.
- Preserve existing valid bug-fix workflows where an already `implemented`
  design can gain later draft or active follow-up work.

# Outcome

The shared document model exposes association-aware transition validation and
association summaries, and `specmate check refs` reports only steady-state
reference violations in a way that is consistent with the `design-004` patch.

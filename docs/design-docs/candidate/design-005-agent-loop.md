---
id: design-005
title: "Agent Loop"
status: candidate
design-doc: design-001
guidelines:
  - docs/guidelines/specmate-principles.md
  - docs/guidelines/cli-conventions.md
---

# Agent Loop

This document defines the agent loop — the subsystem that drives automated
task execution via ACP. It covers the full lifecycle of `specmate run` and
`specmate rerun`, from pre-flight checks through to the execution report.

---

## 1. Design principles

**The loop is orchestration, not intelligence.** The agent loop does not make
coding decisions. It sequences operations, enforces boundaries, and delegates
to the coding agent and review agent via ACP. All intelligence lives in the
agents.

**Mechanical checks gate agent invocation.** The coding agent is only invoked
after pre-flight checks pass. The review agent is only invoked after all
mechanical checks pass. Agents are never invoked to compensate for failed
checks.

**Fail loudly at the earliest gate.** If a pre-flight check fails, the loop
stops immediately with a clear error. It does not attempt partial execution.

---

## 2. Pre-flight checks

All checks must pass before the loop starts. Failure at any check exits with
code `1` and a clear error message.

| Check | Rule |
|---|---|
| Spec status | Task Spec status must be `active`. `draft` specs are not executable. |
| Spec committed | The Task Spec file must be committed in git. A dirty or untracked spec is not a locked contract. |
| Clean working tree | No uncommitted changes in the working tree. Dirty state creates ambiguity in boundary checking. |
| Dependencies complete | All predecessor Task Specs listed in the Exec Plan must be `completed`. |

---

## 3. Branch management

After pre-flight checks pass, specmate creates a git branch:

```
specmate/task-{id}-{slug}
```

Branch is created from the current HEAD of the default branch. If the branch
already exists (e.g. on `specmate rerun`), specmate uses it as-is unless
`--reset` is passed, in which case the branch is deleted and recreated.

Branch creation uses `git2` (libgit2). No dependency on a system git binary.

---

## 4. Loop structure

```
pre-flight checks
    ↓ pass
create / reuse branch
    ↓
[coding round]
  inject context → invoke coding agent via ACP
      ↓
  run mechanical checks:
    specmate check boundaries <task-id>
    run each completion_criteria test by name
      ↓ all pass
  invoke review agent via ACP
      ↓
  review result:
    pass      → proceed to finalise
    fail      → add review output to context, increment round, retry coding
    uncertain → write execution report, pause, exit with code 0
      ↓ (fail path loops back to coding round)
[finalise]
  commit all changes
  write execution report
  specmate move <task-id> completed
  print handoff message
```

---

## 5. Context injection

At the start of each coding round, the following context is assembled and
passed to the coding agent:

1. The full Task Spec content
2. All files listed in `guidelines` (verbatim)
3. All files in `docs/design-docs/implemented/` referenced by the Task Spec
4. If this is round 2+: the mechanical check output or review agent output
   from the previous round

Context is injected as structured text. The exact format is defined by the
ACP protocol in use.

The review agent receives:
1. The full Task Spec content
2. All files listed in `guidelines` (verbatim)
3. A diff of all changes made in the current branch against the default branch

---

## 6. Iteration limits

| Agent | Max rounds |
|---|---|
| Coding agent (mechanical check failure) | 5 |
| Coding agent (review agent failure) | 3 |

If the coding agent fails to pass mechanical checks after 5 rounds, the loop
stops, writes an execution report with `result: failed`, and exits with
code `1`.

If the coding agent fails to pass review after 3 rounds following a mechanical
pass, the loop pauses with `result: uncertain` and exits with code `0`.

These limits prevent runaway loops. They are not configurable in v1.

---

## 7. Review agent output

The review agent must return a structured response with one of three verdicts:

```
verdict: pass
reason: <optional explanation>
```

```
verdict: fail
reason: <required — specific issue found>
suggestions: <optional — concrete changes to make>
```

```
verdict: uncertain
reason: <required — what the review agent could not determine>
```

`uncertain` is reserved for cases where the review agent cannot make a
confident determination — e.g. the change is outside its context window,
or the spec's intent is ambiguous. It is not a soft `fail`.

---

## 8. Commit format

After a successful loop, specmate commits all changes with:

```
task-{id}: {title}

Completes {task-id}-{slug}.
All completion criteria passed.
Coding rounds: N
Review rounds: N
```

The commit is made on the specmate branch, not on the default branch.
Merging the branch is the human's decision.

---

## 9. Execution report

Written to `specs/active/task-{id}-{slug}-report.md` immediately after
the loop completes (pass, uncertain, or failed).

```markdown
---
task: task-0001
result: pass            # pass | uncertain | failed
branch: specmate/task-0001-implement-init-command
timestamp: 2026-04-01T14:32:00Z
iterations:
  coding: 2
  review: 1
---

## Files changed
- src/cmd/init.rs  (+312 / -0)
- tests/cmd/init_test.rs  (+89 / -0)

## Completion criteria
- [x] cc-001: test_init_creates_full_directory_structure
- [x] cc-002: test_init_lang_zh_generates_chinese_content

## Review agent conclusion
pass — implementation consistent with intent, boundaries respected,
all edge cases covered.

## Notes
<!-- populated when result is uncertain or failed -->
```

The execution report is moved to `specs/archived/` alongside the Task Spec
when the spec is marked `completed`.

---

## 10. Handoff message

On successful completion, specmate prints:

```
task-0001 complete

Branch ready for review:
  specmate/task-0001-implement-init-command

Execution report:
  specs/active/task-0001-implement-init-command-report.md

Next step: review the branch and merge when satisfied.
```

On `uncertain`:

```
task-0001 paused -- review agent returned uncertain

Branch:
  specmate/task-0001-implement-init-command

Execution report:
  specs/active/task-0001-implement-init-command-report.md

Next step: read the execution report, then run:
  specmate rerun task-0001 --context "your guidance here"
```

---

## 11. specmate rerun

`specmate rerun <task-id>` re-enters the loop for a task that was previously run.

**Without flags**: resumes on the existing branch from the current state.
Used when the human has reviewed the branch and wants to give the coding
agent another chance without changing the spec.

**With `--reset`**: deletes the existing branch and starts from scratch.
Required when the Task Spec has been modified since the last run. specmate
detects spec modifications by comparing the current spec content against the
committed version at the time of the last run, and warns if `--reset` is
needed but not provided.

**With `--context <text>`**: injects the provided text as the first item in
the coding agent's context on the first round. Intended for human review
comments that should guide the next attempt.

`--context` and `--reset` can be combined.

---
name: pre-pr-review-swarm
description: Run a concurrent multi-angle review only when the user explicitly invokes `pre-pr-review-swarm` by name. Spawn parallel reviewers for documentation/comment correctness, simplification opportunities, language idiomaticity, correctness risks, security vulnerabilities, test quality gaps, AI slop detection, README or equivalent documentation drift, and SPEC.md compliance (when a SPEC.md exists at the project root).
---

# Pre-PR Review Swarm

Run this skill only when the user explicitly invokes `pre-pr-review-swarm` by name. Do not infer it from ordinary PR
creation, submission, or readiness work.

## Arguments

The skill accepts optional keyword arguments (case-insensitive, any order):

- `nofix` — report findings only; do not make any changes to the code.
- `commit` — review the current commit (`git show`) instead of uncommitted changes.

These can be combined: `nofix commit`

Defaults (no arguments): review uncommitted changes in the working copy, then fix actionable findings. If there are
clearly no uncommitted changes, fall back to reviewing the current commit.

## Workflow

1. Parse arguments (see above).
2. Materialize the review scope into a named diff file before spawning reviewers:
   - If `commit`: write the current commit diff and touched-file summary to the scope file.
   - Otherwise: write the uncommitted working-copy diff and touched-file summary to the scope file. If that scope is
     empty, replace it with the current commit diff.
   - Abort instead of spawning reviewers if the selected scope file is still empty.
   - Keep the checkout aligned with the selected scope's after-state while reviewers run. If that is not true, use an
     isolated checkout/worktree or abort instead of asking reviewers to infer context from stale files.
   - Record a short human-readable scope label, such as `current commit <id>` or
     `uncommitted working-copy diff
     (<n> files)`.
3. Report the selected scope before spawning reviewers.
4. Check whether a `SPEC.md` exists at the project root.
5. Run the reviewer charters concurrently when the environment supports it (eight always, plus a ninth if `SPEC.md`
   exists). Keep each reviewer focused on its own charter so the review instructions stay separate. If the environment
   cannot spawn reviewer agents and wait for their results, stop and report that the swarm could not be run. Do not
   replace the swarm with a coordinator-only read-through and do not report PR readiness from a review that did not
   actually spawn reviewers.
6. For each reviewer, pass the exact same scope file path and the selected scope label. Instruct the reviewer to read
   its charter, use the scope file as the review boundary, and use the checkout only as after-state context. Do not
   describe the review scope only in prose, and do not let reviewers infer which changes to review from the working
   tree. Charter files live in the `reviewers/` directory next to this skill file.
7. Require each reviewer to return only actionable findings, each tagged as **definite** or **possible**, with file
   references and a short rationale. If a reviewer has zero findings, it returns an empty list—do not invent low-value
   observations. Every expected reviewer must return a result before the coordinator can merge findings. A missing
   reviewer result is a failed swarm run, not an empty finding list.
8. Merge and deduplicate findings using these rules:
   - Priority order: correctness, security, spec compliance, test quality, AI slop, docs drift, non-idiomatic patterns,
     simplification opportunities.
   - If two reviewers flag the same code region, keep the finding from the higher-priority reviewer and note the
     overlap.
   - Findings at different code locations are never duplicates, even if they describe similar patterns.
9. Assign feedback identifiers after merge/deduplication. See "Feedback identifiers" below.
10. Present all findings to the user. Every user-visible finding must include its feedback identifier, and the report
    must include the selected scope label.
11. If `nofix` was specified, stop here — do not make any changes.
12. Otherwise, fix the findings. Follow the rules in "Fixing findings" below.
13. If no actionable findings remain, state that explicitly before asking for PR creation.

## Feedback identifiers

Every reported finding gets a compound identifier:

`Fn / REVIEWER_TYPE-MNEMONIC`

Reviewers do not assign identifiers. The `Fn` portion is strictly monotonically increasing across all findings in final
report order: `F1`, `F2`, ..., `Fn`. It is global across the whole report, not local to a section or reviewer.

The `REVIEWER_TYPE` portion comes from the retained finding's reviewer/category:

| Reviewer/category       | Code    |
| ----------------------- | ------- |
| correctness             | `COR`   |
| security                | `SEC`   |
| spec compliance         | `SPEC`  |
| test quality            | `TEST`  |
| AI slop                 | `SLOP`  |
| docs/comments or README | `DOC`   |
| idiomaticity            | `IDIOM` |
| simplification          | `SIMP`  |

The `MNEMONIC` portion should be short, uppercase, and tied to the issue itself, for example `PATH`, `PRIVSEC`, or
`EMPTY-ASSERT`. Prefer something the user can remember while scanning the report. Type-mnemonic identifiers must be
unique within a report. If the natural mnemonic collides, add a short differentiator rather than reusing the same
identifier.

Users may identify a finding using either side of the compound identifier. `F3` and `SEC-PATH` are functionally
equivalent ways to refer to the same finding.

## Fixing findings

Unless `nofix` was specified, the default is to fix every finding. Do not silently cherry-pick. Do not skip a finding
because it feels low-value, cosmetic, or "nice to have" — if a reviewer surfaced it and the fix is clearly an
improvement without major trade-offs, apply it.

For each finding, place it into exactly one of these buckets:

1. **Fix.** The finding is clearly correct and the change is clearly an improvement without major trade-offs. Apply the
   fix. This is the default — most findings land here. Do not defer a fix because it is small, or because it touches
   something outside the immediate diff but is clearly related to the change under review. "Trivial" is a reason to fix,
   not a reason to skip.
2. **Surface for user decision.** The finding is ambiguous: it is not clear whether the proposed change would actually
   be correct, whether it reflects a real improvement, or whether it involves a trade-off the user should weigh (e.g.
   behavior change, API change, performance vs. readability, scope creep into unrelated code). In this case, do not fix
   silently and do not drop the finding. Surface it to the user with a concrete question and your current reading, then
   wait.
3. **Reject with reason.** The finding is wrong, based on a misreading of the code, or already addressed elsewhere.
   State briefly why you are rejecting it.

Do not invent a fourth bucket of "valid but not worth fixing". If you find yourself reaching for that framing, the
finding belongs in bucket 1. If you genuinely believe a valid finding should not be fixed in this PR, that is a
trade-off call — put it in bucket 2 and let the user decide.

After fixing, report per-finding what you did: fixed, surfaced (with the question), or rejected (with the reason). Refer
to each finding by its feedback identifier.

## Reviewers

| Name                                                    | Charter file                   |
| ------------------------------------------------------- | ------------------------------ |
| docs-comments-reviewer                                  | `reviewers/docs-comments.md`   |
| simplification-reviewer                                 | `reviewers/simplification.md`  |
| idiomaticity-reviewer                                   | `reviewers/idiomaticity.md`    |
| correctness-reviewer                                    | `reviewers/correctness.md`     |
| security-reviewer                                       | `reviewers/security.md`        |
| test-quality-reviewer                                   | `reviewers/test-quality.md`    |
| ai-slop-reviewer                                        | `reviewers/ai-slop.md`         |
| readme-drift-reviewer                                   | `reviewers/readme-drift.md`    |
| spec-compliance-reviewer _(only when `SPEC.md` exists)_ | `reviewers/spec-compliance.md` |

## Output Contract

Report results in this structure. Each finding in every section must begin with its feedback identifier and be tagged
**definite** or **possible**.

Always include `Reviewed scope: <selected scope label>` before the findings sections. This is not cosmetic: it is the
user-visible guard against accidentally reviewing an empty working-copy diff or giving different reviewers different
scope.

Always include `Reviewer execution: <n>/<expected> reviewers completed` before the findings sections. If that number is
not complete, the report must say `PR Readiness: not ready` and explain that the swarm did not run to completion. Do not
present an empty finding set as a successful swarm unless every expected reviewer actually returned a result.

Example finding:

- `F1 / SEC-PRIVSEC` — **definite** — `src/auth.rs:42` leaks private session material into logs.

- `Correctness`: findings from the correctness reviewer.
- `Security`: findings from the security reviewer.
- `Spec Compliance` _(only when `SPEC.md` exists)_: list of divergences, each stating whether the implementation or the
  spec appears to need updating.
- `Test Quality`: findings from the test-quality reviewer.
- `AI Slop`: findings from the ai-slop-reviewer.
- `Docs/README Drift`: findings from the docs-comments and readme-drift reviewers.
- `Idiomaticity`: non-idiomatic patterns found.
- `Simplification`: safe simplification opportunities.
- `PR Readiness`: `ready` or `not ready`, with blockers listed if not ready.

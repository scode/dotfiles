---
name: pre-pr-review-swarm
description: Run a concurrent multi-angle review immediately before proposing PR creation. Use when implementation and tests are complete and you are about to ask for PR creation or submission. Spawn parallel reviewers for documentation/comment correctness, simplification opportunities, language idiomaticity, correctness risks, security vulnerabilities, test quality gaps, AI slop detection, README or equivalent documentation drift, and SPEC.md compliance (when a SPEC.md exists at the project root).
---

# Pre-PR Review Swarm

Run this skill as the final quality gate after implementation work and before asking to create a PR.

## Arguments

The skill accepts optional keyword arguments (case-insensitive, any order):

- `nofix` — report findings only; do not make any changes to the code.
- `commit` — review the current commit (`git show`) instead of uncommitted changes.

These can be combined: `nofix commit`

Defaults (no arguments): review uncommitted changes in the working copy, then fix actionable findings. If there are
clearly no uncommitted changes, fall back to reviewing the current commit.

## Workflow

1. Parse arguments (see above).
2. Determine review scope:
   - If `commit`: use `git show` for the diff and touched files.
   - Otherwise: use uncommitted changes. If none exist, fall back to `git show`.
3. Check whether a `SPEC.md` exists at the project root.
4. Run the reviewer charters concurrently when the environment supports it (eight always, plus a ninth if `SPEC.md`
   exists). Keep each reviewer focused on its own charter so the review instructions stay separate.
5. For each reviewer, include the review scope (diff and touched files) and instruct the reviewer to read its charter
   file before reviewing. Charter files live in the `reviewers/` directory next to this skill file.
6. Require each reviewer to return only actionable findings, each tagged as **definite** or **possible**, with file
   references and a short rationale. If a reviewer has zero findings, it returns an empty list—do not invent low-value
   observations.
7. Merge and deduplicate findings using these rules:
   - Priority order: correctness, security, spec compliance, test quality, AI slop, docs drift, non-idiomatic patterns,
     simplification opportunities.
   - If two reviewers flag the same code region, keep the finding from the higher-priority reviewer and note the
     overlap.
   - Findings at different code locations are never duplicates, even if they describe similar patterns.
8. Present all findings to the user.
9. If `nofix` was specified, stop here — do not make any changes.
10. Otherwise, fix the findings. Follow the rules in "Fixing findings" below.
11. If no actionable findings remain, state that explicitly before asking for PR creation.

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

After fixing, report per-finding what you did: fixed, surfaced (with the question), or rejected (with the reason).

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

Report results in this structure. Each finding in every section should be tagged **definite** or **possible**.

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

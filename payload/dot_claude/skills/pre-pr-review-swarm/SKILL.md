---
name: pre-pr-review-swarm
description: Run a concurrent multi-angle review immediately before proposing PR creation. Use when implementation and tests are complete and you are about to ask for PR creation or submission. Spawn parallel reviewers for documentation/comment correctness, simplification opportunities, language idiomaticity, correctness risks, security vulnerabilities, test quality gaps, AI slop detection, README or equivalent documentation drift, and SPEC.md compliance (when a SPEC.md exists at the project root).
---

# Pre-PR Review Swarm

Run this skill as the final quality gate after implementation work and before asking to create a PR.

## Workflow

1. Determine review scope. Default: uncommitted changes in the working copy. The user may override this (e.g., "review
   the last commit"). Collect the relevant code and touched files.
2. Check whether a `SPEC.md` exists at the project root.
3. Spawn reviewers concurrently using the Agent tool (eight always, plus a ninth if `SPEC.md` exists). Each reviewer
   runs as its own sub-agent to keep review instructions out of the main context.
4. For each reviewer, spawn an Agent with a prompt that includes the review scope (diff and touched files) and instructs
   the sub-agent to read its charter file before reviewing. Charter files are in the `reviewers/` directory next to this
   skill file. Tell each sub-agent to read its charter from `<base_directory>/reviewers/<name>.md` where
   `<base_directory>` is the base directory shown in the skill loading header.
5. Require each reviewer to return only actionable findings, each tagged as **definite** or **possible**, with file
   references and a short rationale. If a reviewer has zero findings, it returns an empty list—do not invent low-value
   observations.
6. Merge and deduplicate findings using these rules:
   - Priority order: correctness, security, spec compliance, test quality, AI slop, docs drift, non-idiomatic patterns,
     simplification opportunities.
   - If two reviewers flag the same code region, keep the finding from the higher-priority reviewer and note the
     overlap.
   - Findings at different code locations are never duplicates, even if they describe similar patterns.
7. Present all findings to the user. The parent agent decides whether to fix issues or proceed.
8. If no actionable findings remain, state that explicitly before asking for PR creation.

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

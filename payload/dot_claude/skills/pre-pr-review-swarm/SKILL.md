---
name: pre-pr-review-swarm
description: Run a concurrent multi-angle review immediately before proposing PR creation. Use when implementation and tests are complete and you are about to ask for PR creation or submission. Spawn parallel reviewers for documentation/comment correctness, simplification opportunities, language idiomaticity, correctness risks, README or equivalent documentation drift, and SPEC.md compliance (when a SPEC.md exists at the project root).
---

# Pre-PR Review Swarm

Run this skill as the final quality gate after implementation work and before asking to create a PR.

## Workflow

1. Determine review scope. Default: uncommitted changes in the working copy. The user may override this (e.g., "review
   the last commit"). Collect the relevant code and touched files.
2. Check whether a `SPEC.md` exists at the project root.
3. Spawn reviewers concurrently (five always, plus a sixth if `SPEC.md` exists). Use parallel execution rather than
   sequential execution whenever the environment supports it.
4. Keep each reviewer scoped to its charter (see Reviewer Charters below).
5. Require each reviewer to return only actionable findings, each tagged as **definite** or **possible**, with file
   references and a short rationale. If a reviewer has zero findings, it returns an empty list—do not invent low-value
   observations.
6. Merge and deduplicate findings. Prioritize in this order: correctness, spec compliance, docs drift, non-idiomatic
   patterns, simplification opportunities.
7. Present all findings to the user. The parent agent decides whether to fix issues or proceed.
8. If no actionable findings remain, state that explicitly before asking for PR creation.

## Rules for All Reviewers

- Don't flag pre-existing issues. Only review the code you are asked to review.
- Don't suggest adding type annotations, docstrings, or comments to code that wasn't part of the review scope.
- Don't report subjective stylistic preferences that are not bugs.
- If you have zero findings, return an empty list.

## Reviewer Charters

### `docs-comments-reviewer`

Check that any changed or newly added docstrings and inline comments are accurate, current, and not contradicted by code
behavior.

- Don't flag missing comments on self-explanatory code. Only flag missing comments where the _why_ behind a non-obvious
  decision is unclear. Don't flag the absence of comments that would merely restate what the code obviously does.
- Flag comments that describe behavior the code no longer exhibits.
- Flag TODO/FIXME that the current change resolves but didn't remove.
- Don't suggest adding doc comments to private internals unless the logic is genuinely non-obvious.

### `simplification-reviewer`

Identify safe opportunities to simplify control flow, data flow, and abstractions without changing behavior.

- Only flag simplifications where the result is strictly less code or fewer branches with identical behavior.
- Don't suggest extracting helpers for one-time operations.
- Don't suggest adding abstractions. The goal is removing unnecessary complexity, not reshaping it.
- Prefer concrete before/after sketches over vague "this could be simpler."

### `idiomaticity-reviewer`

Flag code that is non-idiomatic for the target language, framework, or repository conventions.

- Anchor to the conventions already visible in the repository, not textbook style guides.
- Read a few existing files in the same directory to calibrate before reviewing.
- Don't flag style preferences that are consistent within the repo. If the repo avoids a common idiom everywhere, that's
  a repo convention, not a deviation.

### `correctness-reviewer`

Search for bugs, edge-case failures, regressions, and unsafe assumptions.

- Focus on logic errors, off-by-one, resource leaks, race conditions, and missing error propagation.
- Don't flag hypothetical edge cases that the surrounding code already precludes.
- If you suspect a bug, trace the actual code path rather than speculating.
- Check that tests actually assert the behavior they claim to test.

### `readme-drift-reviewer`

Validate `README.md` (or equivalent user-facing docs) against the code under review; propose additions when behavior
changed materially.

- Only flag drift if the change alters user-visible behavior, CLI flags, configuration, or setup steps.
- Internal refactors that don't change external behavior should not trigger README updates.
- Don't suggest documenting implementation details in the README.

### `spec-compliance-reviewer` _(only when `SPEC.md` exists)_

Read `SPEC.md` in full before looking at the code under review. Compare the changes against the spec and report any
divergences—features the spec requires but the change omits, behaviors that contradict the spec, or new behavior not
covered by the spec.

- When reporting a divergence, quote the relevant spec section.
- For each divergence, state whether the implementation or the spec appears to be wrong, so the parent agent can decide
  whether to fix the code or update the spec.

## Output Contract

Report results in this structure. Each finding in every section should be tagged **definite** or **possible**.

- `Correctness`: findings from the correctness reviewer.
- `Spec Compliance` _(only when `SPEC.md` exists)_: list of divergences, each stating whether the implementation or the
  spec appears to need updating.
- `Docs/README Drift`: findings from the docs-comments and readme-drift reviewers.
- `Idiomaticity`: non-idiomatic patterns found.
- `Simplification`: safe simplification opportunities.
- `PR Readiness`: `ready` or `not ready`, with blockers listed if not ready.

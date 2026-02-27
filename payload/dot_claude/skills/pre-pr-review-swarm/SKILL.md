---
name: pre-pr-review-swarm
description: Run a concurrent multi-angle review immediately before proposing PR creation. Use when implementation and tests are complete and you are about to ask for PR creation or submission. Spawn parallel reviewers for documentation/comment correctness, simplification opportunities, language idiomaticity, correctness risks, README or equivalent documentation drift, and SPEC.md compliance (when a SPEC.md exists at the project root).
---

# Pre-PR Review Swarm

Run this skill as the final quality gate after implementation work and before asking to create a PR.

## Workflow

1. Collect review scope from the current change set (diff, touched files, tests, and user requirements).
2. Check whether a `SPEC.md` exists at the project root.
3. Spawn reviewers concurrently (five always, plus a sixth if `SPEC.md` exists). Use parallel execution rather than
   sequential execution whenever the environment supports it.
4. Keep each reviewer scoped to its charter:
   - `docs-comments-reviewer`: Check that any changed or newly added docstrings and inline comments are accurate,
     current, and not contradicted by code behavior.
   - `simplification-reviewer`: Identify safe opportunities to simplify control flow, data flow, and abstractions
     without changing behavior.
   - `idiomaticity-reviewer`: Flag code that is non-idiomatic for the target language, framework, or repository
     conventions.
   - `correctness-reviewer`: Search for bugs, edge-case failures, regressions, and unsafe assumptions.
   - `readme-drift-reviewer`: Validate `README.md` (or equivalent user-facing docs) against the current changes; propose
     additions when behavior changed materially.
   - `spec-compliance-reviewer` _(only when `SPEC.md` exists)_: Read `SPEC.md` in full. Compare the current changes
     against the spec and report any divergences—features the spec requires but the change omits, behaviors that
     contradict the spec, or new behavior not covered by the spec. For each divergence, state whether the implementation
     or the spec appears to be wrong, so the parent agent can decide whether to fix the code or update the spec.
5. Require each reviewer to return only actionable findings with file references and a short rationale.
6. Merge and deduplicate findings. Prioritize in this order: correctness, spec compliance, docs drift, non-idiomatic
   patterns, simplification opportunities.
7. Resolve high-confidence/high-severity issues before asking for PR creation when feasible in the current turn.
8. If no actionable findings remain, state that explicitly before asking for PR creation.

## Output Contract

Report results in this structure:

- `Findings`: ordered by severity, each with file path and concise rationale.
- `Spec Compliance` _(only when `SPEC.md` exists)_: list of divergences, each stating whether the implementation or the
  spec appears to need updating.
- `README Impact`: whether docs are still accurate; include specific proposed additions when needed.
- `PR Readiness`: `ready` or `not ready`, with blockers listed if not ready.

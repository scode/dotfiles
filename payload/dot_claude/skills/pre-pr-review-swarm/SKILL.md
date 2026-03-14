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
3. Spawn reviewers concurrently (eight always, plus a ninth if `SPEC.md` exists). Use parallel execution rather than
   sequential execution whenever the environment supports it.
4. Keep each reviewer scoped to its charter (see Reviewer Charters below).
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

### `security-reviewer`

Search for security vulnerabilities introduced or exposed by the changed code.

- Focus on: injection vulnerabilities (command, SQL, XSS, path traversal), credential or secret exposure in code or
  config, unsafe deserialization, TOCTOU races, and missing input validation at trust boundaries (user input, external
  APIs, file system input).
- Trace concrete code paths to demonstrate exploitability. Don't flag theoretical attacks that require preconditions the
  code already prevents.
- Don't flag internal function calls between trusted components that never handle external input.
- Don't flag use of low-level APIs that are used safely in context.

### `test-quality-reviewer`

Evaluate whether the changed code has adequate, meaningful test coverage.

- Flag changed or new behavior that lacks any corresponding test.
- Flag assertions that don't actually verify the claimed behavior (e.g., only checking that a function returns without
  error, not that it produced the correct result).
- Flag edge cases visible in the diff (error paths, boundary values, empty inputs) that have no test coverage.
- Flag test names or descriptions that don't match what the test actually verifies.
- Don't flag missing tests for unchanged code, trivial getters/setters, or simple delegations.
- Don't suggest specific test implementations—identify the gaps and let the parent agent decide how to fill them.

### `ai-slop-reviewer`

Detect patterns characteristic of AI-generated code that was produced without genuine understanding of the codebase or
problem domain.

**General patterns (all languages):**

- **Hallucinated APIs**: calls to functions, methods, constants, or modules that don't exist in the dependency or
  standard library being used. Verify the API actually exists before flagging—don't guess.
- **Cargo cult code**: structures copied without understanding—unused parameters, no-op branches, config options that
  are never read, defensive checks against conditions that provably can't occur in context.
- **Over-engineering**: wrapper types, factory patterns, abstraction layers, or indirection that serves no purpose for
  the current use case. Especially suspicious when surrounding code solves similar problems more directly.
- **Reinvented wheels**: reimplementing functionality that already exists in the codebase or its direct dependencies.
  Check the same module and imported crates/packages before flagging.
- **Vacuous comments**: comments that restate the next line of code in prose (`// increment counter` above
  `counter += 1`), or docstrings that just rephrase the function signature. Distinct from docs-comments-reviewer which
  checks accuracy—this checks for zero-information commentary.
- **Silent failure paths**: error handling that swallows errors, returns plausible-looking defaults, or
  logs-and-continues where the caller needs to know about the failure.
- **Unnecessary dependencies**: importing a crate or package for trivial functionality that's a few lines to implement,
  or that's already available through an existing dependency.
- **Proportionality violations**: solutions dramatically larger than the problem warrants—50 lines for a 5-line problem,
  entire modules for single-use functionality, test infrastructure more complex than the code under test.

**Rust-specific patterns:**

- **Gratuitous `.clone()`**: cloning to silence the borrow checker when a reference or borrow would work, especially in
  loops or on large types.
- **`Arc<Mutex<T>>` by default**: reaching for shared-ownership with locking when the data has a single owner, or when
  channels or simpler patterns would be clearer.
- **`.unwrap()` outside tests**: using `unwrap()` or `expect()` in library or application code where the error is not
  provably impossible. Especially on I/O, parsing, or external input.
- **Fighting the type system**: liberal `as` casts, long `.into()` chains, or unnecessary turbofish annotations that
  paper over design problems rather than fixing them.
- **Collecting when streaming would do**: `.collect::<Vec<_>>()` followed by iteration over the collected vec, where the
  intermediate collection serves no purpose.

**What NOT to flag:**

- Patterns consistent with the surrounding codebase—if the whole repo clones liberally, individual clones aren't slop.
- Code that is merely verbose but correct and clear—the simplification reviewer handles that.
- Style preferences—the idiomaticity reviewer handles that.
- Pre-existing patterns in unchanged code.

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
- `Security`: findings from the security reviewer.
- `Spec Compliance` _(only when `SPEC.md` exists)_: list of divergences, each stating whether the implementation or the
  spec appears to need updating.
- `Test Quality`: findings from the test-quality reviewer.
- `AI Slop`: findings from the ai-slop-reviewer.
- `Docs/README Drift`: findings from the docs-comments and readme-drift reviewers.
- `Idiomaticity`: non-idiomatic patterns found.
- `Simplification`: safe simplification opportunities.
- `PR Readiness`: `ready` or `not ready`, with blockers listed if not ready.

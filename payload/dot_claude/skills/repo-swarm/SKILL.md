---
name: repo-swarm
description: >
  Run a repository-wide review swarm, ask the user how to handle each finding,
  then create a linear stacked PR series with one PR per accepted fix. Use when
  the user asks for "repo-swarm", for a whole-repo swarm, or for constrained
  scopes such as "repo-swarm all the code under foo/".
---

# repo-swarm

Use this skill when the user wants a repository-wide quality pass that turns review findings into a clean stack of small
PRs. This is intentionally heavier than a normal review. The point is to find issues across a chosen scope, decide which
ones are real work, and land each accepted fix as its own reviewable PR.

Use the `pre-pr-review-swarm` skill for review passes. Use the `jjstack` skill for creating and updating the linear PR
stack. Adhere to the repo's own instructions for commit titles, PR titles, validation commands, formatting, comments,
and tests.

## Scope

Default scope is the whole repository. If the user gives constraints, keep both the initial review and all follow-up
fixes inside that scope unless fixing an issue requires touching a nearby support file. Examples:

- `repo-swarm`
- `repo-swarm the entire codebase`
- `repo-swarm all the code under foo/`
- `repo-swarm only the CLI commands`

State the interpreted scope before launching the initial review. If the scope is ambiguous but a reasonable reading
exists, proceed with that reading and say what you assumed.

## Initial Review

Run the `pre-pr-review-swarm` skill in `nofix` mode against the chosen scope. This initial pass is not a PR review. It
is a finding-generation pass over the selected repository area.

If the existing `pre-pr-review-swarm` workflow is diff-oriented, adapt the reviewer prompts so each reviewer reviews the
chosen scope directly. Keep the same reviewer categories, deduplication rules, and output discipline: actionable
findings only, tagged as definite or possible, with file references and a short rationale.

After the initial review, merge and deduplicate findings. Do not start fixing yet.

## User Triage

Immediately after the initial review only, ask the user what to do with each finding. Do not ask this again for findings
discovered during individual PR review swarms; those are handled by the normal fix/reject rules below.

For each initial finding, present these three choices:

- **Fix it**: implement the fix and create a dedicated PR for it.
- **Ignore it**: do nothing for this finding. Record it in the final summary as ignored.
- **Document as intentional in `SPEC.md`**: update `SPEC.md` to make the behavior explicit. If `SPEC.md` does not exist,
  create it. If `AGENTS.md` exists and does not already require agents to conform to `SPEC.md`, update it to say agents
  must always conform to `SPEC.md`. If `AGENTS.md` is a symlink, edit the target file.

Interactive triage can be a compact checklist, but the user must make an explicit choice for every finding before
implementation starts. If several findings are duplicates or should clearly be handled by one change, ask whether to
combine them into one PR. Otherwise, default to one PR per accepted finding.

## Stack Shape

Create a linear stack. Each accepted finding becomes exactly one reviewable commit, one stable bookmark, and one PR,
unless the user explicitly approves combining findings.

Use the `jjstack` skill for the mechanics:

- Start from the current trunk/default branch or from the top of any existing stack the user asked to extend.
- Keep one stable bookmark per PR.
- Create PRs in bottom-up order.
- Each PR base must be the previous accepted PR's bookmark, or the default branch for the bottom PR.
- Push only the affected bookmarks.
- If rewriting a lower PR moves descendants, push every affected descendant bookmark and update GitHub PR bases when
  needed.

Use conventional commits and conventional PR titles if the repo instructions require them. Otherwise still keep titles
short and factual.

## Per-Finding Workflow

For each accepted finding, in stack order:

1. Move to the current top of the stack.
2. Implement only that finding's fix. Keep the diff narrow.
3. Add or update tests for behavior changes. If tests cannot be added, stop and ask for approval before continuing
   without them.
4. Run the repo's relevant local validation for that fix.
5. Run the `pre-pr-review-swarm` skill on the individual fix before creating the PR. This review should be scoped to the
   pending diff for that one fix.
6. Handle individual-fix review findings:
   - **Fix** findings that are clearly correct and improve the PR without major trade-offs.
   - **Reject** findings that are wrong, based on a misread, already addressed, or outside the PR's scope. Keep the
     reason for the final summary.
   - **Surface for user decision** only when the finding involves a real trade-off, API/behavior change, or scope
     expansion.
7. After applying any individual-fix review changes, rerun the relevant validation and rerun the `pre-pr-review-swarm`
   skill on that same PR until it is clean or only rejected/surfaced findings remain.
8. Commit the final diff, set or move the bookmark, push it, and create or update the PR.
9. Move to the next accepted finding.

Do not create a PR before the individual-fix `pre-pr-review-swarm` is clean enough to explain. The PR should be
review-ready at creation time.

## Documenting Intentional Behavior

When the user chooses "document as intentional in `SPEC.md`", treat that as its own accepted change. Create a dedicated
PR that updates `SPEC.md` and, if needed, the agent instruction file.

The `SPEC.md` entry should state the required behavior, the rationale, and any important constraints future code must
preserve. Do not write a vague note like "this is intentional"; make the intended behavior testable enough that future
reviews can use it.

If `AGENTS.md` needs updating, add a short instruction such as:

```markdown
Agents must conform to `SPEC.md`. If implementation and `SPEC.md` disagree, treat that as a bug or explicitly update
`SPEC.md` in the same change.
```

If there is already an equivalent instruction, do not duplicate it.

## Validation

Follow the repo's own validation instructions. If no repo-specific validation exists, run the relevant available checks
for the files touched, then run the broad test/lint/format commands that are practical for the project.

Before the final response, verify:

- every accepted fix has a PR,
- each PR has had its own `pre-pr-review-swarm` pass before creation or update,
- the GitHub PR bases match the local linear stack,
- the working copy is clean,
- final validation has passed, or any failures are clearly reported.

## Final Summary

Keep the final summary compact. Include:

- initial findings and the user's triage decisions,
- PR links in stack order,
- fixes made during individual PR review swarms,
- rejected individual-PR findings with reasons,
- ignored initial findings,
- validation commands and results,
- current working-copy status.

Do not imply that ignored findings were fixed. Do not merge the stack unless the user explicitly asks.

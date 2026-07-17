---
name: pre-pr-review-swarm
description: Run a concurrent multi-angle review only when the user explicitly invokes `pre-pr-review-swarm` by name. Spawn parallel reviewers for documentation/comment correctness and README drift, simplification opportunities, language idiomaticity, correctness risks, security vulnerabilities, test quality gaps, AI slop detection, and SPEC.md compliance (when a SPEC.md exists at the project root).
---

# Pre-PR Review Swarm

Run this skill only when the user explicitly invokes `pre-pr-review-swarm` by name. Do not infer it from ordinary PR
creation, submission, or readiness work.

## Arguments

The skill accepts optional keyword arguments (case-insensitive, any order):

- `nofix` — report findings only; do not make any changes to the code.
- `commit` — review only the current commit (in git: `git show`), ignoring uncommitted changes.
- `uncommitted` — review only the uncommitted working-copy diff, even when the current commit is itself unpublished
  work.

These can be combined: `nofix commit`. `commit` and `uncommitted` contradict each other; if both are passed, stop and
ask the user which scope they meant.

Defaults (no arguments): review the change a PR reviewer would actually see, then fix actionable findings. What that
means depends on where the current commit sits. The rules below are stated in VCS-neutral terms; the git commands are
illustrations only, and other version control systems have their own equivalents for the same questions. Throughout,
"the current commit" means the reviewable commit backing the change, and "uncommitted changes" means edits not yet part
of it. In systems where the working copy is itself a commit (jj, for example), the working-copy commit's content plays
the uncommitted-changes role and the current commit is its parent — do not let the WIP commit shift the combined range
up by one and collapse the scope to just the fixes.

- If the current commit is already on the mainline branch (in git: `git merge-base --is-ancestor HEAD origin/main`
  succeeds), the working copy holds the whole change: review the uncommitted working-copy diff. If there are clearly no
  uncommitted changes, fall back to reviewing the current commit.
- If the current commit is _not_ on the mainline — an unpushed commit, or a branch commit backing an open PR — and there
  are uncommitted changes on top of it, review the combined diff from the parent of the current commit through the
  working copy. Uncommitted edits in this state are almost always follow-up fixes destined to land in that same commit
  or PR, and the eventual reviewer will see both together. Reviewing only the uncommitted slice produces a misleadingly
  small — sometimes near-empty — review of a large in-flight change. If the working copy is clean in this state, review
  the current commit by itself.
- The mainline test alone cannot distinguish follow-up fixes from the start of the next change: a finished commit whose
  PR is already up to date, with fresh edits on top meant for a new PR, looks mechanically identical to the fix-up case.
  Use session context to decide — the work that produced the edits usually makes their destination obvious. When the
  edits are clearly the beginning of a new review unit, review the uncommitted slice alone and say so in the scope
  report. When the destination is genuinely unclear, present both candidate scopes and ask the user which they meant
  instead of silently taking the combined default; one question is cheaper than a swarm run over the wrong scope.
  Unclear is not an edge case to argue away: a session with no knowledge of where the uncommitted edits came from, or
  edits touching files unrelated to what the commit changes, is the unclear case.
- The combined range is deliberately the current commit plus its pending fixes, not the whole branch: this assumes the
  stacked workflow where each commit is its own reviewable unit, and sweeping in earlier commits would re-review work
  that is not part of this change. On a multi-commit branch destined for a single PR, this scope is narrower than what
  the PR reviewer will see; say so in the scope report so the user can widen it if that is not what they want.
- If the mainline cannot be determined (no remote, detached state with no obvious default branch), use the uncommitted
  working-copy diff and note the ambiguity in the scope label.

Review the uncommitted slice by itself despite an unpublished current commit only when `uncommitted` is passed.

## Workflow

1. Parse arguments (see above).
2. Materialize the review scope into a named diff file before spawning reviewers:
   - If `commit`: write the current commit diff and touched-file summary to the scope file.
   - If `uncommitted`: write the uncommitted working-copy diff and touched-file summary to the scope file.
   - Otherwise, follow the defaults above. For the combined default (unpublished commit plus uncommitted fixes), write
     the diff from the parent of the current commit through the working copy (in git: `git diff HEAD^`) and a
     touched-file summary over that same range — the scope must contain the already-committed content, not just the
     follow-up edits. For the mainline default, write the uncommitted working-copy diff; if that scope is empty, replace
     it with the current commit diff.
   - Exclude dependency lock files (`Cargo.lock`, `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `go.sum`,
     `poetry.lock`, `uv.lock`, and equivalents) and clearly generated or vendored files from the diff written to the
     scope file. Every reviewer pays to read every token of the scope file, and mechanical churn in generated files is
     almost never what a reviewer needs to see.
   - Never omit silently. End the scope file with an `Omitted from scope: <paths>` trailer listing every excluded file,
     so reviewers know the change touched them and can read them from the checkout when their charter calls for it
     (dependency changes during a security review, for example). Keep omitted files in the touched-file summary, marked
     as omitted.
   - Abort instead of spawning reviewers if the selected scope file is still empty. If the diff became empty only
     because every touched file was excluded, say so: the change touched only generated or lock files.
   - Keep the checkout aligned with the selected scope's after-state while reviewers run. If that is not true, use an
     isolated checkout/worktree or abort instead of asking reviewers to infer context from stale files.
   - Record a short human-readable scope label that names the selection and includes the touched-file count and diff
     line count, such as `current commit <id> (3 files, 120 diff lines)` or
     `commit <id> + uncommitted changes (7 files, 480 diff lines)`. The counts are a guard, not decoration: a label
     showing a handful of diff lines when the in-flight change is known to be large means the wrong scope was selected,
     and the user should be able to see that before reviewers spend anything on it.
3. Report the selected scope before spawning reviewers.
4. Check whether a `SPEC.md` exists at the project root.
5. Decide the reviewer panel. By default every reviewer runs (seven always, plus an eighth if `SPEC.md` exists). If
   every file touched by the change — including files listed under `Omitted from scope:` — has a prose extension (`.md`,
   `.markdown`, `.txt`), apply the prose-only fast path instead: spawn only the docs-comments, ai-slop, simplification,
   and (when `SPEC.md` exists) spec-compliance reviewers. The correctness, security, test-quality, and idiomaticity
   charters have nothing to bite on in a prose-only change, so skipping them saves their agent contexts rather than
   paying each to return an empty list. The trigger is purely mechanical: a file with any other extension anywhere in
   the change means the full panel runs. When in doubt, run the full panel.
6. Run the selected reviewer charters concurrently when the environment supports it. Keep each reviewer focused on its
   own charter so the review instructions stay separate. If the session has an active model-routing skill — one whose
   job is to decide which model and reasoning effort each delegated task or subagent runs on — then which model and
   effort each reviewer runs at is that skill's decision: route each spawn through it and pass explicit model/effort
   overrides where the spawn mechanism supports them, rather than letting reviewers silently inherit the coordinator's
   model. This applies equally to the reduced prose-only panel. If the environment cannot spawn reviewer agents and wait
   for their results, stop and report that the swarm could not be run. Do not replace the swarm with a coordinator-only
   read-through and do not report PR readiness from a review that did not actually spawn reviewers.
7. For each reviewer, pass the exact same scope file path and the selected scope label. Instruct the reviewer to read
   its charter, use the scope file as the review boundary, and use the checkout only as after-state context. Also tell
   reviewers that files listed under `Omitted from scope:` were part of the change but were excluded as generated or
   lock files; they exist in the checkout and may be consulted when a charter needs them. Do not describe the review
   scope only in prose, and do not let reviewers infer which changes to review from the working tree. Charter files live
   in the `reviewers/` directory next to this skill file.
8. Require each reviewer to return only findings with a concrete recommended action, each tagged as **definite** or
   **possible**, with file references and a short rationale. Actionability is a quality bar, not an invitation to
   summarize: a reviewer that spots the same pattern in several independently editable places returns one finding per
   place, not one aggregate finding for the pattern. If a reviewer has zero findings, it returns an empty list—do not
   invent low-value observations. Every expected reviewer must return a result before the coordinator can merge
   findings. A missing reviewer result is a failed swarm run, not an empty finding list.
9. Merge and deduplicate findings using these rules. Granularity is an output invariant of this step, not a style
   preference: every independently editable location a reviewer surfaced must still be visible as its own finding
   afterwards. Deduplication exists to remove same-location overlap between reviewers, and for nothing else.
   - Priority order: correctness, security, spec compliance, test quality, AI slop, docs drift, non-idiomatic patterns,
     simplification opportunities.
   - If two reviewers flag the same code region, keep the finding from the higher-priority reviewer and note the
     overlap.
   - Findings at different code locations are never duplicates, even when they share a category, rationale, or
     recommended fix. Sharing a reason to change is the wrong equivalence relation: each location needs its own edit and
     may get its own accept/reject decision. Merging findings is allowed only when they describe one contiguous code
     region that a single edit would resolve. Adjacent assertions in one test pinning the same behavior can be one
     finding; the same anti-pattern in two files is two findings, always.
   - Theme summaries are additive, never a substitute. When many findings share a category, a one-line theme
     introduction above them is welcome, but each location keeps its own finding and identifier beneath it. Concision
     must not erase the inventory.
   - Account for every reviewer finding. After this step each one is either retained as its own finding, merged into a
     retained finding at the same location, or rejected as incorrect or non-actionable with a stated reason. A finding
     that silently vanishes into a broader summary is a coordinator bug: category-level compression is exactly the
     failure mode the location rule guards against, and the accounting is what makes a violation visible.
10. Assign feedback identifiers after merge/deduplication. See "Feedback identifiers" below.
11. Present all findings to the user. Every user-visible finding must include its feedback identifier, and the report
    must include the selected scope label.
12. If `nofix` was specified, stop here — do not make any changes.
13. Otherwise, fix the findings. Follow the rules in "Fixing findings" below.
14. If no actionable findings remain, state that explicitly before asking for PR creation.

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
| spec-compliance-reviewer _(only when `SPEC.md` exists)_ | `reviewers/spec-compliance.md` |

## Output Contract

Report results in this structure. Each finding in every section must begin with its feedback identifier and be tagged
**definite** or **possible**.

Always include `Reviewed scope: <selected scope label>` before the findings sections. This is not cosmetic: it is the
user-visible guard against reviewing an empty or under-sized scope (such as a sliver of uncommitted fixes when the real
change is a whole in-flight commit) or giving different reviewers different scope.

Always include `Reviewer execution: <n>/<expected> reviewers completed` before the findings sections. If that number is
not complete, the report must say `PR Readiness: not ready` and explain that the swarm did not run to completion. Do not
present an empty finding set as a successful swarm unless every expected reviewer actually returned a result.

`<expected>` is the size of the selected panel. When the prose-only fast path applied, say so on the same line and name
the reviewers it skipped. A skipped reviewer's findings section must say it was skipped by the fast path — an unspawned
reviewer did not return an empty finding list, and the report must not read as if it did.

Always include `Finding accounting: <r> reviewer findings → <n> reported (<m> same-location merges, <k> rejected)`
before the findings sections, where the numbers satisfy `r = n + m + k`. This is the count-based guard against lossy
aggregation: reviewer findings that are neither reported, merged at the same location, nor rejected with a reason were
silently collapsed, and the arithmetic makes that loss visible to the user instead of leaving the swarm looking less
thorough than it was.

Example finding:

- `F1 / SEC-PRIVSEC` — **definite** — `src/auth.rs:42` leaks private session material into logs.

- `Correctness`: findings from the correctness reviewer.
- `Security`: findings from the security reviewer.
- `Spec Compliance` _(only when `SPEC.md` exists)_: list of divergences, each stating whether the implementation or the
  spec appears to need updating.
- `Test Quality`: findings from the test-quality reviewer.
- `AI Slop`: findings from the ai-slop-reviewer.
- `Docs/README Drift`: findings from the docs-comments reviewer, which also owns README drift.
- `Idiomaticity`: non-idiomatic patterns found.
- `Simplification`: safe simplification opportunities.
- `PR Readiness`: `ready` or `not ready`, with blockers listed if not ready.

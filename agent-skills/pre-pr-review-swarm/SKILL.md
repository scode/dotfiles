---
name: pre-pr-review-swarm
description: Run a concurrent multi-angle review only when the user explicitly invokes `pre-pr-review-swarm` by name. Spawn parallel reviewers for documentation/comment correctness and README drift, simplification opportunities, language idiomaticity, correctness risks including systems behavior and resource bounds, security vulnerabilities, test quality gaps, AI slop detection, and SPEC.md compliance (when a SPEC.md exists at the project root).
---

# Pre-PR Review Swarm

## Arguments

The skill accepts optional keyword arguments (case-insensitive, any order):

- `nofix` — report findings only; do not make any changes to the code. Findings are still sorted into the fix buckets
  (without applying anything) so the run log records what the default mode would have done.
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
   - Name the run: `<YYYYMMDD>-<HHMM>-<short commit id>-<4 hex of scope hash>`, for example
     `20260829-0412-62d866d-9c2e`, built from `date -u +%Y%m%d-%H%M`, the abbreviated id of the current commit (the
     parent, when the working copy is itself a commit), and the first four characters of `sha256sum` over the scope
     file. The name is how a run is referred to afterwards — by the user who has lost the console output, and by
     `swarm-triage` — so it has to be unique in practice and cheap to read in a directory listing. Every part carries
     meaning: the same commit re-swarmed later shares a commit id and differs in time; the same uncommitted content
     re-swarmed shares the content hash as well, so "same input, re-run" is visible at a glance, while a changed hash
     says the input moved. Compute every component with shell commands; never invent characters to look random.
3. Report the selected scope and the run name before spawning reviewers.
4. Check whether a `SPEC.md` exists at the project root.
5. Decide the reviewer panel. By default every reviewer runs (thirteen always, plus a fourteenth if `SPEC.md` exists).
   If every file touched by the change — including files listed under `Omitted from scope:` — has a prose extension
   (`.md`, `.markdown`, `.txt`), apply the prose-only fast path instead: spawn only the docs-comments, ai-slop,
   simplification, and (when `SPEC.md` exists) spec-compliance reviewers. The correctness and security reviewers and the
   test-quality and idiomaticity charters have nothing to bite on in a prose-only change, so skipping them saves their
   agent contexts rather than paying each to return an empty list. The trigger is purely mechanical: a file with any
   other extension anywhere in the change means the full panel runs. When in doubt, run the full panel.
6. Run the selected reviewer charters concurrently when the environment supports it. Keep each reviewer focused on its
   own charter so the review instructions stay separate. If the session has an active model-routing skill — one whose
   job is to decide which model and reasoning effort each delegated task or subagent runs on — then which model and
   effort each reviewer runs at is that skill's decision: route each spawn through it and pass explicit model/effort
   overrides where the spawn mechanism supports them, rather than letting reviewers silently inherit the coordinator's
   model. This applies equally to the reduced prose-only panel. Retain each reviewer's stable agent or thread handle
   until finding collection is complete so productive reviewers can continue in the context they already paid to build.
   Track the selected charters before spawning and create exactly one initial agent for each. A spawn that fails without
   returning a thread handle may be retried; once a charter has a live handle, never spawn it again. Continuations go to
   that handle instead of creating replacements or duplicate reviewers. If the environment cannot spawn the complete
   reviewer panel and wait for its results, stop and report that the swarm could not be run. Do not treat a host
   capacity limit as an unavailable reviewer, replace the swarm with a coordinator-only read-through, or report PR
   readiness from a partial review.
7. For each reviewer, pass the exact same scope file path and the selected scope label. Instruct the reviewer to read
   its charter, use the scope file as the review boundary, and use the checkout only as after-state context. Also tell
   reviewers that files listed under `Omitted from scope:` were part of the change but were excluded as generated or
   lock files; they exist in the checkout and may be consulted when a charter needs them. Do not describe the review
   scope only in prose, and do not let reviewers infer which changes to review from the working tree. Charter files live
   in the `reviewers/` directory next to this skill file. Make the initial prompt explicit that this is a complete
   charter review, not a top-N search: finding an issue is not a stopping condition, and the reviewer must finish its
   main pass and make a deliberate second sweep for unrelated issues before returning. Pass the plain-language finding
   requirements from step 8 in the prompt as well. They are part of every reviewer's output contract, not cleanup left
   for the coordinator.
8. Require each reviewer to return only findings with a concrete recommended action, each tagged as **definite** or
   **possible**, with file references. **Definite** means the code, the applicable contract, and the claimed consequence
   establish a real defect with no unresolved assumption; **possible** means at least one material premise is still open
   (an unverified caller, an assumed contract, an uncertain regression value). The tag drives what gets fixed without
   asking (see "Fixing findings"), so reviewers must not inflate it. Each finding must make sense to a reader who has no
   detailed knowledge of the codebase. Explain in plain terms what the relevant code currently does, the concrete
   failure or needless complexity, why that matters, and the recommended action. Introduce project-specific terms before
   relying on them; labels such as "cache contract," "assembly path," or "trust boundary" are not explanations on their
   own. Every finding must carry at least one specific source file reference; that reference is evidence and an anchor,
   not a substitute for context. Length follows the explanation: a quarter to half a page is fine when the claim needs
   it, and neither forced brevity nor padding is acceptable. The reader should not need to open the code merely to
   understand the claim and decide whether it is worth addressing. Return each finding with the literal fields
   `What happens:`, `Why it matters:`, and `Suggested change:` from the Output Contract. Do not collapse them into one
   rationale paragraph. Actionability is a quality bar, not an invitation to summarize: a reviewer that spots the same
   pattern in several independently editable places returns one finding per place, not one aggregate finding for the
   pattern. If a reviewer has zero findings, it returns an empty list together with a one-line statement that it
   reviewed the scope and found nothing—do not invent low-value observations. That statement is the only thing that
   makes an empty result checkable, so require it in the spawn prompt: a bare empty list is indistinguishable from a
   reviewer that never got to review. Every expected reviewer must return a result before the coordinator can merge
   findings. A missing reviewer result is a failed swarm run, not an empty finding list. So is a result that arrived
   without the review behind it. Treat an empty result as clean only when it carries that statement; a reply reporting a
   blocked tool, an unreadable scope file, a refusal, or any other reason the reviewer could not do the work is a failed
   reviewer wearing the same shape as a clean one, and neither a zero exit status nor a returned result distinguishes
   them. Watch hardest when the whole panel comes back empty at once, because a shared cause — a scope path nobody can
   read, a sandbox that will not start — fails every reviewer identically and leaves a broken swarm looking unanimously
   clean. A failed reviewer does not count toward the completed-reviewer total, is not eligible for continuation, and
   makes the swarm incomplete: stop before merging and report the failure and its cause instead of a finding set. Before
   merging, continue productive reviewers through a bounded search. Run eligible continuations concurrently when the
   host supports it; one reviewer's extra pass must not serialize the rest of the panel.
   - A pass is one agent turn: the initial spawn is pass 1, and each later message to that same stable handle starts one
     additional pass. The deliberate second sweep required by step 7 happens inside pass 1; it does not become pass 2
     unless the coordinator resumes the reviewer after receiving its first result.
   - A confirmed-empty first pass completes that reviewer. After a first pass that returned findings, send one follow-up
     to the same reviewer using the host's stable-handle continuation mechanism (`followup_task` on Codex, or
     `SendMessage` to the original agent ID on Claude Code). Tell it that the earlier findings are recorded, to reuse
     its existing context, and to return only new independently actionable findings or an empty list. Repeating or
     rephrasing an earlier finding does not count as new.
   - A third pass is allowed only when the coordinator judges that at least one new second-pass finding is both
     significant and credible. A credible finding is grounded in the reviewed code, supported by concrete evidence,
     independently actionable, and neither a duplicate nor speculation presented as fact. A significant finding would
     materially affect correctness, security, data integrity, resource behavior, externally visible behavior, spec
     compliance, or test adequacy. Style-only observations, marginal simplifications, optional polish, and fixes whose
     complexity outweighs their likely benefit do not qualify.
   - When that gate passes, send one final follow-up to the same reviewer with the same no-repeat instruction. Three
     total passes is a hard cap. Never start a fourth pass; if the third pass still returns new findings, record that
     the reviewer reached the cap before saturation.
   - Every finding from every completed pass still enters the normal merge and accounting flow. The third-pass gate
     controls search depth, not whether second-pass feedback is retained.
   - Keep the checkout at the same after-state until all scheduled continuations return. A failed scheduled continuation
     makes the swarm incomplete just like a failed first pass.
   - If the host can spawn reviewers but cannot resume a completed reviewer, do not spawn a fresh replacement and repay
     the exploration cost. Keep the first-pass result, disclose that continuation was unavailable, and rely on the
     mandatory internal sweep from step 7. This salvages a pass that actually reviewed something. It never applies to a
     failed reviewer, which did not run the sweep it is being credited with.
9. Merge and deduplicate findings using these rules. Granularity is an output invariant of this step, not a style
   preference: every independently editable location a reviewer surfaced must still be visible as its own finding
   afterwards. Deduplication exists to remove same-location overlap between reviewers, and for nothing else.
   - Record provenance on every retained finding as it arrives: the originating reviewer and its pass number (1–3, as
     defined in step 8). Provenance survives merge and restatement; for a merged finding, keep the earliest pass among
     its inputs. The run log needs this, and it is not recoverable later from the finding text.
   - Priority order: correctness, security, spec compliance, test quality, AI slop, docs drift, non-idiomatic patterns,
     simplification opportunities.
   - If two reviewers flag the same code region, keep the finding from the higher-priority reviewer and note the
     overlap.
   - Lens siblings — the correctness lens reviewers among themselves, and likewise the security lens reviewers — share
     one category and priority. When two of them flag the same code region, keep one finding and note that multiple
     lenses agreed: agreement is a confidence signal worth surfacing, not a duplicate to discard silently. When the
     merged inputs carry different confidence tags, the merged finding keeps the lower one; the confirm step in "Fixing
     findings" is where a tag gets raised, not the merge.
   - Findings at different code locations are never duplicates, even when they share a category, rationale, or
     recommended fix. Sharing a reason to change is the wrong equivalence relation: each location needs its own edit and
     may get its own accept/reject decision. Merging findings is allowed only when they describe one contiguous code
     region that a single edit would resolve. Adjacent assertions in one test pinning the same behavior can be one
     finding; the same anti-pattern in two files is two findings, always.
   - Theme summaries are additive, never a substitute. When many findings share a category, a one-line theme
     introduction above them is welcome, but each location keeps its own finding and identifier beneath it. Concision
     must not erase the inventory.
   - Preserve the plain-language explanation while merging. The coordinator may rewrite a reviewer finding for clarity,
     but must not compress away what the code does, the concrete consequence, or the recommended action. Before
     reporting, read each retained finding on its own and fix it if understanding the claim still requires detailed
     codebase knowledge or unexplained project jargon.
   - Account for every reviewer finding. After this step each one is either retained as its own finding, merged into a
     retained finding at the same location, or rejected as incorrect or non-actionable with a stated reason. A finding
     that silently vanishes into a broader summary is a coordinator bug: category-level compression is exactly the
     failure mode the location rule guards against, and the accounting is what makes a violation visible.
10. Assign feedback identifiers after merge/deduplication. See "Feedback identifiers" below.
11. Restate the findings through a fresh agent before presenting them. Spawn one new sub agent with the charter in
    `restater.md` next to this skill file, and give it the complete merged finding list (identifiers, confidence tags,
    anchors, sections, and all three labeled fields), the scope file path, and the checkout path. The restater rewrites
    every finding as prose for a reader who does not know the codebase, and it must investigate the code to do so — it
    is not a paraphraser. Skip this step only when there are zero findings.
    - The coordinator is the wrong agent for this job, by construction. It has the whole diff, every charter, and every
      reviewer's version of each finding in context, so it cannot tell which parts of an explanation only make sense to
      someone who already knows the code. The restater starts without that knowledge; what it has to look up is exactly
      what the reader would have had to look up, and it writes that down instead.
    - The restater's output deliberately drops the three labeled fields. Those labels are a guard against reviewers
      compressing a finding to a one-liner; they are not the shape a good explanation takes, and forcing the rewrite
      back into them costs exactly the fluency the restater was spawned for. Do not re-impose the labels on restated
      findings, and do not reject a restated finding for lacking them.
    - Validate the restated list before using it: the same number of findings, in the same order, each with its original
      identifier, confidence tag, at least one source file reference, and a non-trivial prose body. If the restater
      merged, split, dropped, reordered, or retagged anything, or returned something other than a restated list, run it
      again from scratch with the validation failure spelled out in its input. If the second attempt also fails
      validation, or the host cannot spawn the restater at all, the review has failed: report that, name the failure,
      and leave the merged finding list in a file the user can point a later restatement at. Do not present the
      pre-restatement findings as the review, do not hand-repair a partial restatement, and do not mix restated and
      unrestated findings in one report. Restatement is the last step and the cheap one; retrying it is far cheaper than
      re-running the swarm, and a report whose findings were written by the agent that cannot judge their readability is
      not worth sending.
    - Once the restated list passes validation, every header and body in it is final. Copy them into the user-facing
      report verbatim: do not summarize, shorten, tighten, merge or split sentences, change the tone, convert prose to
      bullets, or re-impose the three labels. The only permitted changes are mechanical rendering ones — turning a
      `path:line` into a clickable reference, or escaping the output renderer requires — and those must not alter the
      finding's words. This is a hard rule because the coordinator is exactly the reader who cannot judge it: with the
      whole diff in context, a shortened explanation still looks readable, and a "polish" pass silently recreates the
      compression the restater was spawned to undo. Response length is not a reason to compress either. If the full
      report does not fit, keep the finding blocks intact and split the response, or stop and say the output limit was
      hit; never substitute summaries for restated findings.
    - `Restater note:` lines are the restater disputing a claim it could not confirm against the code. Read each one and
      decide: keep the finding, downgrade it to **possible**, or reject it with a stated reason in the finding
      accounting. Do not present a disputed finding as **definite** without having checked. A downgrade changes only the
      confidence tag in the header; the body stays as the restater wrote it. A rejection removes the finding and adjusts
      the accounting line.
12. Present all findings to the user. The report must follow the Output Contract and include the selected scope label.
    After composing the response and before sending it, check two things:
    - Every restated finding appears with its identifier, in the restater's order, with a body that is word-for-word the
      validated restater output (modulo the rendering changes allowed in step 11 and any confidence downgrade). This is
      an exact comparison, not a similarity judgment: a body that keeps the identifier and conclusion but has been
      shortened fails, and the fix is to paste the restater's text back in. The `Restatement:` line is a claim that the
      reader is getting the restater's prose, and this check is what makes that claim true.
    - The metadata around the findings (scope, execution, continuation, accounting) does not contradict qualifiers
      inside them. A finding the restater describes as optional, documentation-only, or internal cleanup must not be
      described as blocking anywhere else in the report without a separate reason.
13. If `nofix` was specified, sort the findings into buckets exactly as the default mode would — including the
    confirm-against-code step — but apply nothing; write the run log with the would-have-been buckets and stop. Do not
    modify the reviewed checkout.
14. Otherwise, sort and fix the findings. Follow the rules in "Fixing findings" below: confirm each claim against the
    code, fix only eligible findings, keep every fix minimal, and surface the rest.
15. If no actionable findings remain, state that explicitly. Surfaced findings are unresolved by definition; list the
    ones waiting on the user together at the end of the fix report so they are not lost among the fixed ones.

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

Every retained finding gets sorted into exactly one of the buckets below and reported by its feedback identifier;
`nofix` skips the applying, not the sorting (see "Run log" for why). Do not silently cherry-pick, and do not let a
finding vanish: "I decided not to bother" is not a bucket. The point of the buckets is to fix the things that should be
fixed unattended, leave a clear decision for the things that should not, and make both outcomes visible.

Two questions are decided here, and they are easy to conflate. The first is _whether_ a finding gets fixed without
asking: for definite correctness, security, and spec findings that turns on confirming the claim against the code; for
everything else it also turns on the shape of the fix. The second is _how_ an accepted fix is shaped, which is the same
everywhere: the smallest complete edit. Most over-reach comes from muddling the two — a correct finding is not a license
to apply the reviewer's preferred refactor, and a large proposed change is not a reason to leave a real bug in place.

### Confirm before fixing

Before placing any finding in the fix bucket, re-read the referenced code and restate the defect from what the code
actually does, not from the reviewer's write-up. Reviewers are pushed to keep producing findings, so a fluent finding
with a confident tag can still rest on a misread. If the premise does not hold up when checked, that is bucket 3, and
rejecting on that basis is a normal outcome, not a cherry-pick. Findings carrying a `Restater note:` get extra suspicion
here; findings from a later reviewer pass may deserve it too, though there is no run data yet saying they are worse.

The confirm step is also where a tag can move. If a **possible** finding's open premises all resolve during confirmation
— the caller really does pass untrusted input, the contract really is what the reviewer assumed — retag it **definite**,
state the reason in the per-finding report, and treat it under the definite rules below. If any premise stays open, the
tag stays **possible**. Never raise a tag on the strength of the reviewer's prose alone.

### Minimal fix

Every bucket-1 fix is the smallest change that completely closes the confirmed defect: delete the redundant code, fix
the expression, correct the comment, add the missing check. If the reviewer's `Suggested change:` is larger than that —
introduces an abstraction, restructures surrounding code, generalizes beyond the case at hand — apply the small fix and
mention the larger proposal in the per-finding report so the user can pursue it deliberately. "Technically wrong, so
here are two new types and a helper module" is the failure mode this rule exists to prevent.

Smallest means smallest _complete_ fix. A new local helper is fine when the correct fix needs one (the same missing
check at three call sites, a cleanup that has to run on every exit path); the rule forbids structure beyond what the
confirmed defect requires, not structure as such. Never substitute a narrower workaround that leaves the defect
reachable merely to avoid a structural change — if the only complete fix is structural, the finding is still fixed when
it is definite correctness, security, or spec, and surfaced otherwise.

### Buckets

1. **Fix.** Eligibility depends on the finding's category and (post-confirmation) tag:
   - Correctness and security findings tagged **definite** and confirmed against the code: fix, with no size cap. A real
     bug that needs forty lines gets forty lines, shaped by the minimal-fix rule. Exceptions that push such a finding to
     bucket 2: the fix would change externally visible behavior beyond the defect itself, change a public API or an
     on-disk/wire format, or the fix is really a design decision with several defensible answers (which lock, which
     retry policy, which error to surface). An unattended agent guessing at those is worse than a clear report.
   - Spec-compliance findings tagged **definite**: fix under the same terms, but only when the existing spec
     unambiguously governs the behavior and the implementation is clearly the defective side. A divergence where the
     spec may be the thing to change — new behavior the spec does not cover, or a reviewer saying the spec looks stale —
     is bucket 2. Confirming a divergence proves it exists, not which side should move, and rewriting `SPEC.md`
     unattended would bless behavior nobody decided on. This guard is about the document, not the reviewer: a finding
     whose fix edits `SPEC.md` is bucket 2 whatever category filed it. In one eval run a "the format has no spec"
     finding arrived as _correctness_, and the coordinator wrote forty lines of spec under the no-size-cap rule —
     precisely the unattended spec authoring this bullet exists to prevent.
   - Every other finding — simplification, idiomaticity, AI slop, docs drift, test quality, and anything still tagged
     **possible** after confirmation — is eligible only when the fix is local and non-structural. Local means confined
     to a symbol or contiguous hunk the reviewed change already touches, plus mechanically required references, or a
     documentation edit (README, comment, docstring). Non-structural means: a deletion, a rename of a local binding, a
     comment or docstring edit, a one-expression logic fix, a change to an existing test's assertions, or new test
     cases. It must not introduce a new non-test function, type, module, file, or config knob; rename an externally
     referenced identifier; change a signature; or exceed twenty added-plus-deleted lines, estimated before applying. If
     the fix runs over while being applied, revert it and surface the finding instead. Slop and docs cleanup almost
     always passes because it is subtractive; "add an abstraction to handle the edge case" does not, however valid the
     observation.
   - Test additions are the deliberate exception to the size and new-named-thing limits. Coverage gaps are among the
     most valuable things the swarm finds and new test code cannot break shipped behavior, so add as many test cases as
     a confirmed gap needs, along with whatever fixtures, mocks, or helpers they take, in existing test files or new
     ones that follow the surrounding convention. The minimal-fix rule still applies — build the support the tests need,
     not a test framework — and changes to non-test code made to enable testing (new seams, injected dependencies) fall
     under the normal eligibility rules, not this exception. A test that fails when run against the current code is a
     correctness finding in disguise: keep the test, and treat the underlying defect under the correctness rules.
   - Within eligibility, small and cosmetic are reasons to fix, not reasons to skip. Do not defer an eligible fix
     because it feels low-value.
2. **Surface for user decision.** The finding is valid or plausible but not eligible for bucket 1: it is still
   **possible** and the fix is not local, the fix would be structural, the spec side is in question, it involves a
   trade-off the user should weigh (behavior change, API change, performance vs. readability, scope creep into unrelated
   code), or the coordinator could not decide whether the proposed change is actually correct. Do not fix silently and
   do not drop the finding. Report it with a concrete question and your current reading, including what the fix would
   take. In unattended use this is the whole point of the bucket: the finding waits in the report instead of being
   guessed at.
3. **Reject with reason.** The finding is wrong, rests on a misreading of the code, or is already addressed elsewhere.
   State briefly why. A premise that fails the confirm step lands here.

There is no fourth bucket of "valid but not worth fixing". A valid finding that does not fit bucket 1 is a bucket-2
decision for the user, with the reason it was not auto-fixed stated plainly.

After fixing, report per finding what you did: fixed (noting any retag and any larger reviewer proposal that was not
applied), surfaced (with the question), or rejected (with the reason). Refer to each finding by its feedback identifier.

### Run log

Write one markdown file per run to `~/.local/state/pre-pr-review-swarm/runs/<run name>.md`, using the name chosen in
step 2 and creating the directory if missing. The log exists so the eligibility rules can be tuned from what actually
happened instead of from memory: the pattern worth catching is "findings that look like X keep getting fixed and should
not have been" (or the reverse), and spotting that needs the finding text, the decision, and the resulting diff side by
side. A row of counters cannot show it; a directory of self-contained run files can be grepped, skimmed, or handed to an
agent to look for patterns across runs.

Each file contains, in this order:

1. A header: the run name, timestamp, repository root, the commit id and scope label from step 2, the skill arguments,
   and the coordinator model if known.
2. The complete findings report exactly as presented to the user in step 12 — every finding with its identifier,
   confidence tag, anchors, and restated body — plus the accounting lines. Copy, do not summarize; the whole point is
   that the finding is readable later without the session.
3. One block per finding, under a `### <identifier>` heading, with: reviewer category and origin (`reviewer pN` from the
   provenance recorded in step 9); confidence tag as filed and, if the confirm step changed it, the retag and why; the
   bucket (`fixed`, `surfaced`, or `rejected`; in `nofix` mode `would fix`, `would surface`, or `would reject`) and the
   one-or-two-sentence reason it landed there — which eligibility rule applied, what the surfaced question is, or why
   the premise failed; any larger reviewer proposal that was deliberately not applied; and, for fixed findings, the
   unified diff of that fix (`git diff` of the touched hunks, attributed to the finding as best the coordinator can;
   overlap between fixes is noted rather than untangled).

Write the file in `nofix` mode too, with the would-have-been buckets and no diffs. Which mode a run uses is the user's
call and says nothing about the code under review; a log that only said "nofix" for every finding in those runs would
discard exactly the decision the log exists to capture. This is why `nofix` still runs the sort, including the confirm
step: a bucket assigned without checking the code is a guess, and logging guesses would teach the wrong lesson. The cost
is real — a `nofix` run spends coordinator time re-reading code after the report is out — and is accepted deliberately.
Present the report first and do the sorting afterwards, so the user is never waiting on the classification to read the
findings. The log lives outside the reviewed checkout; if the user asked for a fully read-only run or prohibited file
writes, skip it and say so. A failure to write the log is reported but never blocks the fixes. Runs get their own files,
named by the run name, so concurrent swarms never contend for one.

## Reviewers

| Name                                                    | Charter file                               |
| ------------------------------------------------------- | ------------------------------------------ |
| docs-comments-reviewer                                  | `reviewers/docs-comments.md`               |
| simplification-reviewer                                 | `reviewers/simplification.md`              |
| idiomaticity-reviewer                                   | `reviewers/idiomaticity.md`                |
| correctness-general-reviewer                            | `reviewers/correctness-general.md`         |
| correctness-data-flow-reviewer                          | `reviewers/correctness-data-flow.md`       |
| correctness-state-lifecycle-reviewer                    | `reviewers/correctness-state-lifecycle.md` |
| correctness-systems-reviewer                            | `reviewers/correctness-systems.md`         |
| correctness-edge-inputs-reviewer                        | `reviewers/correctness-edge-inputs.md`     |
| security-general-reviewer                               | `reviewers/security-general.md`            |
| security-input-trust-reviewer                           | `reviewers/security-input-trust.md`        |
| security-secrets-env-reviewer                           | `reviewers/security-secrets-env.md`        |
| test-quality-reviewer                                   | `reviewers/test-quality.md`                |
| ai-slop-reviewer                                        | `reviewers/ai-slop.md`                     |
| spec-compliance-reviewer _(only when `SPEC.md` exists)_ | `reviewers/spec-compliance.md`             |

### Lens reviewers

The correctness and security charters run as several lens reviewers each instead of one reviewer per charter. Identical
reviewers produce correlated attention: they all find the same salient issues and share the same blind spots, so extra
spawns only pay for themselves when each one digs somewhere different. Each lens reviewer reads the shared base charter
(`reviewers/correctness.md` or `reviewers/security.md`) and is a complete reviewer for that charter — the lens adds a
mandatory deep pass, it never narrows scope, so a bug outside every lens is still every lens reviewer's job to report.
The lens set is meant to be tuned over time: add, remove, or reword lenses to balance cost against finding quality
rather than treating the current set as fixed.

Each split charter also runs one generalist with no lens at all. It patches the holes in the lens taxonomy — bugs that
fit no lens get only baseline attention from the lensed reviewers — and doubles as a diagnostic for tuning: a generalist
that keeps surfacing findings the lenses missed points at a gap worth naming as a new lens, while one that only
duplicates them is a slot that can be reclaimed.

## Output Contract

Report results in this structure. Each finding in every section must begin with its feedback identifier and be tagged
**definite** or **possible**.

Write findings for a reader who does not already know the codebase. Every finding starts with the same header line:

- `` `Fn / TYPE-MNEMONIC` — **definite|possible** — `path:line` — <plain-language title> ``

The body is the restater's prose, verbatim (step 11): one or more paragraphs, written by an agent that did not know the
code and went and read it, covering what the code does, what goes wrong, why anyone should care, and what to change, in
whatever order explains it best. The three labeled fields reviewers use (`What happens:`, `Why it matters:`,
`Suggested change:`) are a wire format between reviewers and the coordinator, there to stop a reviewer from compressing
a finding into a polished one-liner such as "cover the cached assembly path" that is technically accurate but useful
only after the reader reconstructs the code. They never appear in the user-facing report. If there is no validated
restater output, there is no report — see step 11.

The prose states the finding in the plainest terms that do not lose precision, for a reader who does not know the
codebase at all. Project or domain jargon appears only when necessary and is introduced before it is relied on. A
quarter to half a page per finding is fine — the coordinator does not force it shorter, and the restater does not pad
it.

Every finding must be anchored to at least one specific source file reference (`path:line`, or `path` when the finding
concerns a whole file). Plain language is not a license to drift into generalities: the reference is what makes the
feedback checkable and fixable, and a finding that cannot name the code it is about is not ready to report. A reference
is an anchor, not an explanation — it never substitutes for the body.

Always include `Reviewed scope: <selected scope label>` before the findings sections. This is not cosmetic: it is the
user-visible guard against reviewing an empty or under-sized scope (such as a sliver of uncommitted fixes when the real
change is a whole in-flight commit) or giving different reviewers different scope.

Always include `Reviewer execution: <n>/<expected> reviewers completed` before the findings sections. If that number is
not complete, the report must say plainly that the swarm did not run to completion and which reviewers are missing. Do
not present an empty finding set as a successful swarm unless every expected reviewer actually completed a review. A
reviewer that returned something other than a review did not complete one, however promptly it answered.

`<expected>` is the size of the selected panel. When the prose-only fast path applied, say so on the same line and name
the reviewers it skipped. A skipped reviewer's findings section must say it was skipped by the fast path — an unspawned
reviewer did not return an empty finding list, and the report must not read as if it did.

Always include
`Reviewer continuation: <p2>/<eligible2> second passes, <p3>/<eligible3> third passes; unavailable: <names or none>; capped with new findings: <names or none>`
before the findings sections. A reviewer is eligible for a second pass when its first pass was non-empty, and eligible
for a third when its second pass produced at least one significant, credible new finding. A reviewer belongs in
`unavailable` when the host could not resume it, and in `capped with new findings` when its third pass was non-empty.
This line distinguishes a bounded search from both a one-shot review and a claim that every reviewer reached saturation.

Always include `Restatement: <n>/<n> findings restated, <a> attempt(s)` before the findings sections, where both counts
are the number of reported findings and the attempt count is 1 or 2. A successful report always says `n/n`; the line is
a tripwire, not a status. Its presence is the coordinator's claim that every body below is the restater's prose, which
step 12 verifies, and the attempt count tells the reader whether the first restatement was rejected. When there are zero
findings, say `Restatement: skipped (no findings)`. When restatement failed (step 11), the report carries no findings
sections at all; say `Restatement: failed after <a> attempt(s) (<reason>)`, keep the scope, execution, continuation, and
accounting lines so the swarm's work is still visible, and name the file holding the merged list.

Always include `Finding accounting: <r> reviewer findings → <n> reported (<m> same-location merges, <k> rejected)`
before the findings sections, where the numbers satisfy `r = n + m + k`. This is the count-based guard against lossy
aggregation: reviewer findings that are neither reported, merged at the same location, nor rejected with a reason were
silently collapsed, and the arithmetic makes that loss visible to the user instead of leaving the swarm looking less
thorough than it was.

Example finding:

- `F1 / SEC-PRIVSEC` — **definite** — `src/auth.rs:42` — login requests write session secrets to the application log.

  The login handler issues a session token: a random value the browser sends back on every later request, and the only
  thing that proves the user is logged in. The changed log statement at `src/auth.rs:42` writes that token into the
  application log alongside the request path, where before it wrote only the request identifier. Anyone who can read the
  logs — an operator, a log-aggregation service, anyone who obtains a log export — can copy the token and act as that
  user until the session expires. Remove the token from the log statement and record the non-secret request identifier
  instead; nothing else in the handler depends on the token being logged.

- `Correctness`: findings from the correctness lens reviewers, presented as one section.
- `Security`: findings from the security lens reviewers, presented as one section.
- `Spec Compliance` _(only when `SPEC.md` exists)_: list of divergences, each stating whether the implementation or the
  spec appears to need updating.
- `Test Quality`: findings from the test-quality reviewer.
- `AI Slop`: findings from the ai-slop-reviewer.
- `Docs/README Drift`: findings from the docs-comments reviewer, which also owns README drift.
- `Idiomaticity`: non-idiomatic patterns found.
- `Simplification`: safe simplification opportunities.

There is deliberately no overall ready/not-ready verdict. The findings are the output; a one-word summary of them was
never consulted in practice and invited the coordinator to argue about which findings "block" instead of describing
them. Readers judge readiness from the findings themselves.

After the run log has been written (which happens after the report is presented, in step 13 or 14), end the session
output with two lines: `Swarm run: <run name>` and `Run log: <path>`, or `Run log: not written (<reason>)` if the write
failed or was skipped. They come after the findings, never delay them, and are never a substitute for them: the report
in the session stays the primary output. The name and path exist so a later step — the `swarm-triage` skill, or the user
by hand — can find the record that matches what was just read, and the name is repeated here because it is the thing
worth copying when the console is about to be lost.

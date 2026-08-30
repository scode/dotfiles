# Delegation procedure

Read this file in full before the first delegation of a session, native or shelled out. It covers how to do a delegation
correctly once the routing decision has been made; the decision itself is governed by SKILL.md.

## Writing the task spec

The delegate has none of your conversation context. Every delegation prompt must be self-contained:

- The goal and any constraints that bound it.
- Exact file paths or directories in scope.
- Acceptance criteria: what done looks like, concretely.
- Which checks to run (tests, linters, formatters) before reporting back.
- For read-only tasks: state explicitly that it must not edit any files.
- Always: no commits, no branches, no pushes, no PRs.
- Ask it to report what it did and call out any deviations from the spec. For writers, the report is `REPORT.md` and its
  Deviations section points at the checkpoint files (see Checkpoints below).
- For every writer: append the checkpoint addendum from Checkpoints below. This is not optional and does not scale with
  task size; a writer too small to be worth two resumes is a task to do yourself, not a reason to skip the protocol.
- When the deliverable is prose rather than code — a review, an analysis, a scan result — name a file the delegate must
  write it to, and make that file part of the acceptance criteria. Every harness captures only the delegate's final
  message (`codex -o`, `claude -p` stdout, opencode's last `text` event), and a delegate that emits its real output
  mid-run and closes with a summary leaves the deliverable stranded in its transcript. That has happened: a review
  delegate's result file held a two-kilobyte recap while the twenty findings it produced lived only in a megabyte of
  transcript, and recovering them meant grepping the log. The gate reads artifacts; make the delegate produce one. The
  checkpoint files are artifacts in exactly this sense: `ASSUMPTIONS.md`, `DECISIONS.md`, and `REPORT.md` are what the
  gate reads, not the transcript.

Before delegating a task that writes to your working tree, note the current working-copy state — `git status`/`git diff`
or the equivalent in whatever VCS is in use — so you can attribute the delegate's changes cleanly afterwards. Writers in
isolated trees (see Concurrency) are attributable as long as the tree started clean from a recorded base — the normal
state of a fresh worktree or clone; note that base when you create it.

## Checkpoints

Every writer delegation — native or shelled out, any family, any size — runs a two-checkpoint protocol: the delegate
stops once before writing code and once after the checks pass, and you review at each stop. Read-only delegates
(reviews, scans, analysis) do not use it; their contract is the named artifact above.

Here's why it is unconditional. A 32-run eval (four cheap models, three features whose specs read as complete but were
silent on six planted decisions each) found that the current contract — decide, list deviations in the report — avoided
the main trap in 0 of 8 runs, and that "you may ask when unsure" produced zero questions in 8 runs: a complete-looking
spec reads as settling everything, and one delegate that noticed the trap in its own test fixture patched the fixture
rather than asking. A mandatory assumptions list with an unconditional stop got 4 of 4 correct on the feature that had
been 1 of 8, at about 1.5k orchestrator tokens per review. Cheap delegates enumerate reliably and judge poorly; the
protocol moves the judgment to you. The decision log is always on for the same reason: a false-positive entry costs a
line to skim, a missed decision costs a review round. Details are in `lore/2026-08-30-checkpoint-protocol.md`.

### Files

Every delegation gets a _run id_ and a _run directory_. The run id is a fresh unique string you generate when you decide
to launch — `uuidgen`, or a UTC timestamp plus random suffix if that is what you have — and it names everything this
delegation owns: the run directory, the scratch files and logs you keep for it, any tree you create for it, and the
handle you record in handoff notes. The run directory is `.galaxy-brain/<run-id>/` at the root of the tree the delegate
works in (the repo root, or the isolated tree), and all checkpoint files live in it. This is what makes several
orchestrators able to use the skill at the same time without touching each other, which `SPEC.md` requires: every path
the skill uses for a delegation carries a run id only you know. Record every run id you generate — in your notes as you
go, and in any handoff — the moment you generate it, not only once a delegate has stopped: a run directory whose id you
have lost is one you can no longer tell from another session's, and the rules below then forbid you from cleaning it up.

`.galaxy-brain/` itself is shared, and the rules for it follow from that. Create it if it is missing, and if its
`.gitignore` is missing write one containing a single `*` — atomically, by writing a temp file named with your run id
next to it and renaming it into place, so another session that checks a moment later never sees an empty file. Never
delete `.galaxy-brain/` or its `.gitignore`, and never touch a run directory whose id is not yours. Another
orchestrator's run directory is theirs for as long as it exists — you cannot tell an active delegation from an abandoned
one from the outside, so assume active. That is as far as the skill's responsibility goes: it keeps its own state out of
other sessions' way, and does not try to make the user's work safe to run concurrently. A run directory you did not
create in a tree you are about to write to is worth mentioning to the user, since it suggests another session's writer
may be live there, but it is information for them, not a reason for you to refuse.

The `.gitignore` is what hides the whole directory from whatever version control the tree is under, in a way that
touches no tracked file and no repository metadata. The bar is mechanical: nothing under `.galaxy-brain/` may show up in
status, be swept into a commit by an add-everything command, or land in a working-copy snapshot on systems that snapshot
automatically (there, "untracked" does not exist, and an unhidden directory is in the working-copy commit the first time
you run any command). Under git, jj, or Sapling the per-directory `*` ignore does all of that and works in linked
worktrees, clones, and non-colocated workspaces alike; other systems have their own per-directory ignore. What not to
do: edit the project's tracked ignore file (a change that leaks into the diff), or edit repository-local metadata such
as `.git/info/exclude` (path differs between main tree and linked worktree, absent in some layouts). One consequence to
know about: a tree whose only change is `.galaxy-brain/` then looks clean, which is why SKILL.md's Concurrency section
bans isolation mechanisms that delete a clean tree on exit for writers.

Create the run directory immediately before the launch — first attempts, escalations, fixup rounds, and reroutes each
get a fresh id and directory — and when the delegation is gated, move it to your private scratch space (a directory only
this session uses, e.g. a fresh `mktemp -d`, never a fixed name under `/tmp`) rather than deleting it. It is the record
of what was asked and what you settled — `REPORT.md`'s Deviations point into it, a fixup round to the escalation model
may need to read the primary's `DECISIONS.md`, and your own `ANSWERS.md`/`REVIEW.md` are the decisions you made. A
directory per run also removes a class of failure that a single shared directory had: the addendum tells the delegate to
append to `DECISIONS.md`, and a leftover log from a previous task got the new task's entries with colliding numbers, a
leftover `ANSWERS.md` read as answers to the new task, and a leftover `REPORT.md` made a crashed run classify as a
finished one below.

Record the launch time and each resume time, for native and shelled-out writers alike; the classification below depends
on knowing whether a file was written during the turn that just ended. For shelled-out runs this is already part of the
monitoring record in `harness/shell-out.md`; for native sub agents nothing else makes you write it down.

Tell the delegate, in the addendum, that it must not edit ignore files, formatter excludes, or any other config to make
`.galaxy-brain/` go away. A cheap delegate told both "the checks must pass" and "ignore findings under `.galaxy-brain/`"
may reconcile them by excluding the directory in `dprint.json` or the project's ignore file; that edit is a diff leak,
and the gate should expect and strip it if it appears anyway.

Before every writer launch, then: run id generated and recorded; `.galaxy-brain/` present with its `.gitignore`; your
run directory created empty; any run directory you do not own noted for the user (not removed, not a blocker);
attribution baseline taken (status and diff in whatever VCS is in use — `.galaxy-brain/` is invisible to both); session
or thread id arranged per the harness file; launch time noted; addendum appended to the spec with the run directory's
absolute path substituted in. Missing any one of these produces a failure that looks like the delegate's fault and is
not.

| file             | written by   | when                                                                   |
| ---------------- | ------------ | ---------------------------------------------------------------------- |
| `ASSUMPTIONS.md` | delegate     | before any code, at the first stop                                     |
| `ANSWERS.md`     | orchestrator | in response to `ASSUMPTIONS.md`, before the first resume               |
| `DECISIONS.md`   | delegate     | appended to during implementation, at the moment each decision is made |
| `REVIEW.md`      | orchestrator | in response to `DECISIONS.md`, before the second resume                |
| `REPORT.md`      | delegate     | last, after the second resume                                          |

There is deliberately no file for a mid-implementation question. The eval found "ask if unsure" inert — cheap models do
not recognize when they are unsure — and a third stop type would need its own reply file, resume prompt, and budget.
Anything the delegate would have asked belongs in `DECISIONS.md` for the second checkpoint.

A delegate stopped at a checkpoint still owns its tree. Its process has exited and the tree looks idle, but its
uncommitted implementation and its run directory are in there, and it will be resumed into that exact state. For a
shared tree this means the one-writer-at-a-time rule in SKILL.md counts a stopped delegate as running: nothing else
writes to that tree until the delegate has finished and been gated or has been abandoned and its changes removed.

If the project's formatter check walks untracked markdown (dprint does), the delegate will see `.galaxy-brain/` flagged
when it runs the checks. Say in the spec that findings under `.galaxy-brain/` do not count, and don't be surprised if
the delegate reformats `ANSWERS.md` — harmless, don't diff it.

### Sentinels and what a finished run looks like

The delegate ends the final message of a stopped turn with one of two lines, alone on the last line:

- `AWAITING GUIDANCE` — after writing `ASSUMPTIONS.md`.
- `AWAITING REVIEW` — after the implementation is complete and the checks pass, with `DECISIONS.md` current.

A stopped turn exits 0 on every harness, same as a finished one, so detect the stop by reading the final message, not
the exit status. Each turn has exactly one expected outcome, and classification is against that expectation, not against
whatever sentinel happens to be present:

| turn                      | expected outcome                                        |
| ------------------------- | ------------------------------------------------------- |
| launch                    | `AWAITING GUIDANCE`, `ASSUMPTIONS.md` written this turn |
| resume after `ANSWERS.md` | `AWAITING REVIEW`, `DECISIONS.md` current               |
| resume after `REVIEW.md`  | no sentinel, `REPORT.md` written this turn              |

"Written this turn" is checked against the launch or resume time you recorded. Classify what came back before doing
anything else; the bullets are in precedence order, and the first that matches wins:

- The expected outcome: proceed — review and resume, or gate the finished work.
- An outcome from later in the protocol than expected — `AWAITING REVIEW` or a `REPORT.md` on the launch turn, a
  `REPORT.md` on the first resume: a skipped stop. This is the eval's arm C failure, a delegate that writes its
  assumptions and implements straight through, and it is the case the protocol exists to prevent. Do not accept the
  later checkpoint as if it were the one you were waiting for: the skipped file was never answered. If the skipped file
  is missing altogether, the run gave you a diff with no account of the choices in it — escalate as for a substantive
  failure. Otherwise do the skipped review now, against the diff: read the skipped file and decide, item by item, what
  you would have answered. If every answer is `OK`, or the replacements are local edits, write the reply file for the
  skipped stop and the reply for the stop it did reach, and resume with a prompt that says both: it skipped the
  assumptions stop, `ANSWERS.md` now exists and its replacements must be applied to the implementation that already
  exists, `REVIEW.md` covers `DECISIONS.md`, and it should re-run the checks and continue from where the protocol
  expects it. That resume counts as the one extra reply the cap allows for the checkpoint it skipped. If a replacement
  invalidates the design, the diff is broadly wrong: abandon the delegation, remove its changes, and reroute per
  SKILL.md, as for any structurally bad patch.
- An outcome from earlier than expected — `AWAITING GUIDANCE` again after `ANSWERS.md`, `AWAITING REVIEW` again after
  `REVIEW.md`: a repeat stop, handled by the cap under Reviewing a checkpoint.
- On the first resume turn only, `AWAITING REVIEW` with a `REPORT.md` already written: proceed with the review, and tell
  the delegate in the resume prompt that the existing report is stale and must be rewritten after the review changes; do
  not read it at the gate. (On the launch turn the same combination is the skipped stop above.)
- No sentinel and no `REPORT.md`: read the message. If the delegate chose to stop — a decline, a question, a report of a
  tool or permission failure it decided not to work around, status instead of work — that is the ordinary gate failure
  every harness file warns about: escalate per SKILL.md, do not resume. It is a crash only when an environmental cause
  cut the turn short and you have verified and fixed that cause: disk full, OOM, a kill you issued because a shell-tool
  wait expired on a run that was making progress (a hard-deadline kill is not a crash; the run is over budget and its
  result inconclusive, per `harness/shell-out.md`). Note that a disk-full turn can look like a decline (the delegate's
  edit fails with `No space left on device` and it reports the failure and exits 0) — the test is whether the cause was
  the environment and is now fixed, not how the message reads. A crash is resumed once per `harness/shell-out.md`. The
  distinction matters because the crash prompt tells the delegate its turn was interrupted and the cause fixed; sent to
  a delegate that declined, that is a lie that buys another declined turn under bypass flags.

### Reviewing a checkpoint

Your answers are the decisions you would have made when writing the spec; one line each. Answer only what the delegate
listed. Adding items it did not raise makes the review expensive and lets the delegate's list go lazy, because the
orchestrator will fill it in anyway. The one exception is a genuinely missing assumption you spot while reading: add it
as a numbered `Also:` item at the end.

`ANSWERS.md` is numbered to match `ASSUMPTIONS.md`, each line `N. OK` or `N. **Replace.** <decision>`, nothing else. An
answer from the eval that fixed a broken design, for calibration:

```
5. **Replace.** Two independently recorded copies of identical content never share modification times, so
   `mtime_nanos` must not participate in "records differ". Compare type, `sha256`, and `size` only; for symlinks the
   target; a type mismatch is `M`. State this in SPEC.md.
```

`REVIEW.md` has the same shape against `DECISIONS.md`: `N. OK` or the change to make. `No entries. OK` is a valid review
when the log is empty.

Expect `ASSUMPTIONS.md` to run 8–20 items and cost the delegate a minute or four; expect `DECISIONS.md` on a ~300-line
feature to run anywhere from 0 to about a dozen entries depending on the model, and expect most of them to be fine. The
value of the second checkpoint is that the entries exist to be read; the eval never needed to change one, and the resume
cost a few cents. An empty log is not evidence that nothing was decided. On the same feature, luna logged zero entries
where muse logged eleven — the decisions were made either way, and the quiet model simply did not experience them as
choices. Read an empty log as "the decisions are in the diff, unlabeled," and give that diff the same attention at the
gate you would give one that came with a long log.

Each checkpoint is one round: one stop, one reply, one resume. A delegate that stops a second time at the same
checkpoint — a revised `ASSUMPTIONS.md` after `ANSWERS.md`, a fresh `AWAITING REVIEW` after `REVIEW.md` — gets one more
reply, and that is the cap. A third stop at the same checkpoint means the task is beyond it: abandon the delegation,
remove its attributable changes from the tree as SKILL.md's escalation rules describe, move its run directory to
scratch, and either finish the work yourself or reroute with a fresh spec under a new run id. Do not paste the
accumulated answers into the new spec as a substitute for fixing what was unclear in it.

After the final resume, the gate reads `DECISIONS.md` in full, not only `REPORT.md`'s Deviations: the delegate keeps
appending while it applies your review changes, so entries after the last one you reviewed are unreviewed decisions.

### Resuming

Resume the same delegate in the same session; never relaunch from scratch with the answers pasted in, since the
delegate's context (what it read, what it planned) is what makes the resumed turn cheap. For a native sub agent that
means a follow-up message to the same agent (`SendMessage` in Claude Code). For a shelled-out delegate, each harness
file gives the verified resume command, and `harness/shell-out.md` gives the rules common to all of them. Whatever the
path, it must support same-session resume; if a mechanism cannot, do not use it for writers. If the handle is gone — a
native sub agent id that no longer resolves after compaction, a session the harness cannot find — the delegation is
over: remove its attributable changes, move its run directory to scratch, and relaunch under a new run id with a fresh
spec that folds in the answers you already gave. Do not try to reconstruct a stopped delegate from its files.

The resume prompts are short; substitute the run directory's absolute path for `<run-dir>`, as in the addendum. After
`ANSWERS.md`:

```
The orchestrator has reviewed `<run-dir>/ASSUMPTIONS.md` and written `<run-dir>/ANSWERS.md`, listing each
assumption number with either `OK` or a replacement decision, possibly followed by numbered `Also:` items for
assumptions you did not list — treat those as decisions already made. Apply the replacements and the `Also:` items and
continue the task under the original instructions: implement, keep `<run-dir>/DECISIONS.md` current as you go, run
the project's checks, and stop with `AWAITING REVIEW` as the instructions describe before writing `REPORT.md`.
```

After `REVIEW.md`:

```
The orchestrator has reviewed `<run-dir>/DECISIONS.md` and written `<run-dir>/REVIEW.md`, listing each decision
number with either `OK` or a change to make. Apply the changes, re-run the project's checks, and write
`<run-dir>/REPORT.md` as the task describes; its Deviations section should point at `ASSUMPTIONS.md` and
`DECISIONS.md` and note what the orchestrator changed at each checkpoint. Then finish with a short summary pointing
at `REPORT.md`.
```

### The addendum

Append this to every writer's task spec, after the acceptance criteria and checks. Substitute the run directory's
_absolute_ path (e.g. `/home/me/src/proj/.galaxy-brain/3f9c1a2e-…`) for `<run-dir>` in the paragraph that begins "Your
run directory is" — it is the only place the path appears, and the rest of the text refers to "the run directory".
Absolute, because a delegate's working directory is not reliably the tree root: a native sub agent inherits the
orchestrator's cwd, which for an isolated tree is the wrong tree entirely, and a relative path there sends the
checkpoint files somewhere you will never look. The same substitution applies to the resume prompts above and the crash
prompt in `harness/shell-out.md`. Adapt "the project's checks" and "the project's binding docs" to the concrete names
the spec already uses (e.g. "the four checks", "`CLAUDE.md` and `SPEC.md`"); keep the mechanics as written.

```
# Assumptions checkpoint, decision log, and review checkpoint

You are working for an orchestrator who wrote the task above and will review your result. Specifications that read
as complete still leave decisions to you. The orchestrator reviews those decisions at two points: before you write
code, and after you have written it but before you finish. Both are cheap for the orchestrator; a wrong decision
found in a finished diff is not.

Your run directory is `<run-dir>` (an absolute path; it sits under `.galaxy-brain/` at the root of the working tree
you are to edit); every file named below lives there, and "the run directory" below means that path. It already
exists. Do not create, read, or modify anything else under
`.galaxy-brain/` — other directories there belong to other work. Never add anything under `.galaxy-brain/` to version
control. If the project's checks flag files under `.galaxy-brain/`, ignore those findings; do not edit ignore files,
formatter or linter configuration, or anything else to make them go away.

**Step 1 — before writing any code**, write `ASSUMPTIONS.md` in the run directory: a numbered list of every interpretation
you are making that the task and the project's binding docs do not state outright — how an ambiguous phrase is read,
what happens in a case the task does not mention, which existing code path you will reuse or bypass, what is read,
hashed, or written, what an exit code or output means in an edge case, what the user sees and does not see. For each:
the reading you will implement, the alternative you rejected, and one sentence on why. Do not include naming or local
code structure. Then end your final message with the line `AWAITING GUIDANCE` and stop. Do not implement anything
yet.

**Step 2** — you will be resumed with `ANSWERS.md` in the run directory listing each assumption number with either
`OK` or a replacement decision, possibly followed by numbered `Also:` items for assumptions you did not list; those are
decisions already made and bind you the same way. Apply the replacements and the `Also:` items, then implement.
**While implementing, keep `DECISIONS.md` in the run directory**:
every time you make a choice that `ASSUMPTIONS.md` and `ANSWERS.md` did not already settle — a case you only
discovered in the code, a behavior you had to pick for an edge the task never mentioned, a place where the existing
code forced a tradeoff, anything that changes what a user sees or what the program reads, writes, or reports — append
a numbered entry *at the moment you make it*, with the alternative you rejected and why. Even a decision that seems
minor belongs here if it could matter to a reviewer; the orchestrator would rather skim an entry than find it in the
diff. Naming and local code structure still do not qualify. Do not stop to ask questions mid-implementation: make the
call, log it, and the orchestrator will review it at the next step.

**Step 3 — when the implementation is complete and the project's checks pass**, do not write `REPORT.md` yet. End
your final message with the line `AWAITING REVIEW` and stop. You will be resumed with `REVIEW.md` in the run
directory listing each `DECISIONS.md` entry number with either `OK` or a change to make. Apply the changes, re-run the
checks, and write `REPORT.md` in the run directory as the task describes; its Deviations section should point at `ASSUMPTIONS.md`
and `DECISIONS.md` and note what the orchestrator changed at each checkpoint. If `DECISIONS.md` is empty at step 3,
say so and still stop for review.
```

## Integrating isolated writers

SKILL.md decides when concurrent writers are allowed (each isolated in its own tree, merge owned by you). This is how
that merge is done without losing work:

- Integrate serially: extract each delegate's complete change set (plain `git diff` misses untracked files — new files,
  renames, and mode changes all count), gate it as usual, and apply it to the main tree one at a time. Keep each
  isolated tree until its result has been applied and validated. A broadly wrong result is discarded along with its
  tree, which is cheaper than untangling it from a shared one.
- Conflicts between accepted results are yours to resolve and an expected cost of this mode — disjoint task scopes make
  them rare, not impossible. Textual conflicts surface at apply time, but semantic conflicts apply cleanly, and a
  delegate's own checks only ever validated its isolated baseline. Re-run the relevant checks on the integrated main
  tree after each apply, and again after the last one.
- A delegate's changes are attributable only against a recorded baseline. For a shared tree that is the `git status` /
  `git diff` you took before launching; for an isolated tree it is the clean base you created it from. When a writer is
  killed or its result rejected, remove its attributable changes (or discard its tree) before relaunching anything in
  that tree, so the retry starts from a known state.

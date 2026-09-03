# The checkpoint protocol for delegates that write code

Read this file in full before the first writer launch of a session. `SKILL.md` says that every writer stops twice; this
file is the protocol: the files, the sentinels, how to classify what a turn came back with, how to review a stop, how to
resume, and the addendum that goes into every writer's task spec.

Here's why it is unconditional. A 32-run eval (four cheap models, three features whose specs read as complete but were
silent on six planted decisions each) found that the previous contract — decide, list deviations in the report — avoided
the main trap in 0 of 8 runs, and that "you may ask when unsure" produced zero questions in 8 runs: a complete-looking
spec reads as settling everything, and one delegate that noticed the trap in its own test fixture patched the fixture
rather than asking. A mandatory assumptions list with an unconditional stop got 4 of 4 correct on the feature that had
been 1 of 8, at about 1.5k orchestrator tokens per review. Cheap delegates enumerate reliably and judge poorly; the
protocol moves the judgment to the caller. The decision log is always on for the same reason: a false-positive entry
costs a line to skim, a missed decision costs a review round.

## Files

The run directory is `<tree>/.agent-delegation/<run-id>/` (see The three-step start in `SKILL.md`), and all checkpoint
files live in it. This is what makes several orchestrators able to delegate at the same time without touching each
other, which `SPEC.md` requires: every path this skill uses for a delegation carries a run id only its caller holds.

`.agent-delegation/` itself is shared, and the rules for it follow from that. Create it if it is missing, and if its
`.gitignore` is missing write one containing a single `*` — atomically, by writing a temp file named with your run id
next to it and renaming it into place, so another session that checks a moment later never sees an empty file. Never
delete `.agent-delegation/` or its `.gitignore`, and never touch a run directory whose id is not yours. Another
orchestrator's run directory is theirs for as long as it exists — you cannot tell an active delegation from an abandoned
one from the outside, so assume active. That is as far as this skill's responsibility goes: it keeps its own state out
of other sessions' way, and does not try to make the user's work safe to run concurrently. A run directory you did not
create in a tree you are about to write to is worth mentioning to the user, since it suggests another session's writer
may be live there, but it is information for them, not a reason to refuse.

The `.gitignore` is what hides the whole directory from whatever version control the tree is under, in a way that
touches no tracked file and no repository metadata. The bar is mechanical: nothing under `.agent-delegation/` may show
up in status, be swept into a commit by an add-everything command, or land in a working-copy snapshot on systems that
snapshot automatically (there, "untracked" does not exist, and an unhidden directory is in the working-copy commit the
first time you run any command). Under git, jj, or Sapling the per-directory `*` ignore does all of that and works in
linked worktrees, clones, and non-colocated workspaces alike; other systems have their own per-directory ignore. What
not to do: edit the project's tracked ignore file (a change that leaks into the diff), or edit repository-local metadata
such as `.git/info/exclude` (path differs between main tree and linked worktree, absent in some layouts). One
consequence to know about: a tree whose only change is `.agent-delegation/` then looks clean, so an isolation mechanism
that deletes a tree that looks clean when the run exits (a harness's own worktree mode that auto-cleans an unchanged
tree) is not safe for a writer under this protocol: at the first stop the delegate has written only its run directory,
and whether that counts as "changed" is the mechanism's call. The caller creates the tree itself instead.

A directory per run also removes a class of failure that a single shared directory had: the addendum tells the delegate
to append to `DECISIONS.md`, and a leftover log from a previous task got the new task's entries with colliding numbers,
a leftover `ANSWERS.md` read as answers to the new task, and a leftover `REPORT.md` made a crashed run classify as a
finished one below.

Record the launch time and each resume time, for native and shelled-out writers alike; the classification below depends
on knowing whether a file was written during the turn that just ended. For shelled-out runs this is already part of the
monitoring record `scode-harness-shellout` (loaded as `SKILL.md` describes) prescribes; for native sub agents nothing
else makes you write it down.

Tell the delegate, in the addendum, that it must not edit ignore files, formatter excludes, or any other config to make
`.agent-delegation/` go away. A cheap delegate told both "the checks must pass" and "ignore findings under
`.agent-delegation/`" may reconcile them by excluding the directory in `dprint.json` or the project's ignore file; that
edit is a diff leak, and the gate should expect and strip it if it appears anyway.

Before every writer launch, then: run id generated and recorded; `.agent-delegation/` present with its `.gitignore`;
your run directory created empty; any run directory you do not own noted for the user (not removed, not a blocker);
attribution baseline taken (status and diff in whatever VCS is in use — `.agent-delegation/` is invisible to both);
session or thread id arranged per the harness file when shelled out; launch time noted; addendum appended to the spec
with the run directory's absolute path substituted in. Missing any one of these produces a failure that looks like the
delegate's fault and is not.

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
uncommitted implementation and its run directory are in there, and it will be resumed into that exact state. A stopped
delegate counts as running for whoever serializes writers in that tree: nothing else writes to it until the delegate has
finished and been gated or has been abandoned and its changes removed.

If the project's formatter check walks untracked markdown (dprint does), the delegate will see `.agent-delegation/`
flagged when it runs the checks. Say in the spec that findings under `.agent-delegation/` do not count, and don't be
surprised if the delegate reformats `ANSWERS.md` — harmless, don't diff it.

## Sentinels and what a finished run looks like

The delegate ends the final message of a stopped turn with one of two lines, alone on the last line:

- `AWAITING GUIDANCE` — after writing `ASSUMPTIONS.md`.
- `AWAITING REVIEW` — after the implementation is complete and the checks pass, with `DECISIONS.md` current.

A stopped turn exits 0 on every harness, same as a finished one, so detect the stop by reading the final message, not
the exit status (shelled out, `scode-harness-shellout`'s file for the mechanism says where the final message lands on a
resumed turn). Confirm the file the sentinel promises exists in the run directory. Each turn has exactly one expected
outcome, and classification is against that expectation, not against whatever sentinel happens to be present:

| turn                      | expected outcome                                        |
| ------------------------- | ------------------------------------------------------- |
| launch                    | `AWAITING GUIDANCE`, `ASSUMPTIONS.md` written this turn |
| resume after `ANSWERS.md` | `AWAITING REVIEW`, `DECISIONS.md` current               |
| resume after `REVIEW.md`  | no sentinel, `REPORT.md` written this turn              |

"Written this turn" is checked against the launch or resume time you recorded. Classify what came back before doing
anything else; the bullets are in precedence order, and the first that matches wins:

- The expected outcome: proceed — review and resume, or gate the finished work per `gate.md`.
- An outcome from later in the protocol than expected — `AWAITING REVIEW` or a `REPORT.md` on the launch turn, a
  `REPORT.md` on the first resume: a skipped stop. This is the eval's arm C failure, a delegate that writes its
  assumptions and implements straight through, and it is the case the protocol exists to prevent. Do not accept the
  later checkpoint as if it were the one you were waiting for: the skipped file was never answered. If the skipped file
  is missing altogether, the run gave you a diff with no account of the choices in it: gate the diff now per `gate.md`
  and return `substantive failure, fixable` when the gate finds only well-specified defects or
  `substantive failure, structural` when it finds the diff broadly wrong, in either case with "no assumptions file"
  listed as an acceptance failure. Otherwise do the skipped review now, against the diff: read the skipped file and
  decide, item by item, what you would have answered. If every answer is `OK`, or the replacements are local edits,
  write the reply file for the skipped stop and the reply for the stop it did reach, and resume with a prompt that says
  both: it skipped the assumptions stop, `ANSWERS.md` now exists and its replacements must be applied to the
  implementation that already exists, `REVIEW.md` covers `DECISIONS.md`, and it should re-run the checks and continue
  from where the protocol expects it. That resume counts as the one extra reply the cap allows for the checkpoint it
  skipped. If a replacement invalidates the design, the diff is broadly wrong: return `substantive failure, structural`.
- An outcome from earlier than expected — `AWAITING GUIDANCE` again after `ANSWERS.md`, `AWAITING REVIEW` again after
  `REVIEW.md`: a repeat stop, handled by the cap under Reviewing a checkpoint.
- On the first resume turn only, `AWAITING REVIEW` with a `REPORT.md` already written: proceed with the review, and tell
  the delegate in the resume prompt that the existing report is stale and must be rewritten after the review changes; do
  not read it at the gate. (On the launch turn the same combination is the skipped stop above.)
- No sentinel and no `REPORT.md`: read the message. If the delegate chose to stop — a decline, a question, a report of a
  tool or permission failure it decided not to work around, status instead of work — that is the ordinary failed turn
  every harness exits 0 on: return `substantive failure, structural` with the message as the payload; never resume it. A
  run that never got going or stuck (a hang signature named in the harness file, a launch error, a sandbox that refused
  to start) is an execution-path failure, not a model failure and not a crash: return `execution-path failure` with the
  signature seen; the caller fixes the path and decides whether to relaunch. It is a crash only when an environmental
  cause cut short a run that was making progress and the caller has verified and fixed that cause: disk full, OOM, a
  kill issued because a shell-tool wait expired (a hard-deadline kill is not a crash; the run is over budget and returns
  `inconclusive` with reason `over budget` and the log path). Note that a disk-full turn can look like a decline (the
  delegate's edit fails with `No space left on device` and it reports the failure and exits 0) — the test is whether the
  cause was the environment and is now fixed, not how the message reads. A crash is resumed once, with the crash prompt
  under Resuming; the checkpoint files it already wrote let it re-orient, and same-session resume recovered two eval
  runs killed mid-implementation by a full disk. If the resumed turn dies the same way, stop resuming and return
  `inconclusive` with reason `crash recurred` and the log path. The distinction matters because the crash prompt tells
  the delegate its turn was interrupted and the cause fixed; sent to a delegate that declined, that is a lie that buys
  another declined turn under bypass flags.

## Reviewing a checkpoint

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
reply, and that is the cap. A third stop at the same checkpoint means the task is beyond it: return `misclassified`,
with the cause `cap exhaustion` and the payload that the accumulated answers must not be pasted into a fresh spec as a
substitute for fixing what was unclear in it. What the caller does next is its own decision; this skill moves the run
directory to scratch once the caller reports that it has acted.

After the final resume, the gate reads `DECISIONS.md` in full, not only `REPORT.md`'s Deviations: the delegate keeps
appending while it applies your review changes, so entries after the last one you reviewed are unreviewed decisions.

## Resuming

Resume the same delegate in the same session; never relaunch from scratch with the answers pasted in, since the
delegate's context (what it read, what it planned) is what makes the resumed turn cheap. For a native sub agent that
means a follow-up message to the same agent (`SendMessage` in Claude Code). For a shelled-out delegate,
`scode-harness-shellout` (loaded as `SKILL.md` describes) gives the rules common to every harness — the session or
thread id recorded at launch, the same model, effort, directory, and permission flags on the resume — and its file for
the mechanism gives the verified resume command and says where the final message lands on a resumed turn. Whatever the
path, it must support same-session resume; if a mechanism cannot, do not use it for writers. Write your `ANSWERS.md` or
`REVIEW.md` into the run directory first, then launch the resume; the resume prompt is one of the short ones below,
built in a file and passed like any other prompt.

If the handle is gone — a native sub agent id that no longer resolves after compaction, a session the harness cannot
find — the delegation is over: return `unresumable`, with the answers you already gave as the payload so the caller can
fold them into a fresh spec. A resumed turn that shows no sign of its earlier context — it re-reads the task from
scratch, asks what its run directory is — is a fresh session, not a resume: kill it and return `unresumable` the same
way. Do not try to reconstruct a stopped delegate from its files.

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

After a crash (an environmental cause, verified and fixed, per the classification above), tell the delegate what
happened, to check the tree, and which stop it is heading for — the third sentence changes with the phase the run was
in:

```
Your previous turn was interrupted by <what happened, e.g. a disk-full error: writes failed with "No space left on
device", including one of your file edits>. The cause has been fixed. Inspect the working tree (status and diff in
this repository's version control, e.g. `git status` and `git diff`) to see what actually landed, redo any edit that
was lost, then continue the protocol from where you were:
<launch turn: finish `ASSUMPTIONS.md` in your run directory and stop with `AWAITING GUIDANCE` without implementing
anything>
<after ANSWERS.md: finish the implementation, run the project's checks, keep `DECISIONS.md` in your run directory
current, and stop with `AWAITING REVIEW` before writing `REPORT.md`>
<after REVIEW.md: finish applying the review changes, re-run the checks, and write `REPORT.md` in your run
directory>.
```

## The addendum

Append this to every writer's task spec, after the acceptance criteria and checks. Substitute the run directory's
_absolute_ path (e.g. `/home/me/src/proj/.agent-delegation/3f9c1a2e-…`) for `<run-dir>` in the paragraph that begins
"Your run directory is" — it is the only place the path appears, and the rest of the text refers to "the run directory".
Absolute, because a delegate's working directory is not reliably the tree root: a native sub agent inherits the
orchestrator's cwd, which for an isolated tree is the wrong tree entirely, and a relative path there sends the
checkpoint files somewhere you will never look. The same substitution applies to the resume prompts and the crash prompt
above. Adapt "the project's checks" and "the project's binding docs" to the concrete names the spec already uses (e.g.
"the four checks", "`CLAUDE.md` and `SPEC.md`"); keep the mechanics as written.

```
# Assumptions checkpoint, decision log, and review checkpoint

You are working for an orchestrator who wrote the task above and will review your result. Specifications that read
as complete still leave decisions to you. The orchestrator reviews those decisions at two points: before you write
code, and after you have written it but before you finish. Both are cheap for the orchestrator; a wrong decision
found in a finished diff is not.

Your run directory is `<run-dir>` (an absolute path; it sits under `.agent-delegation/` at the root of the working
tree you are to edit); every file named below lives there, and "the run directory" below means that path. It already
exists. Do not create, read, or modify anything else under
`.agent-delegation/` — other directories there belong to other work. Never add anything under `.agent-delegation/` to
version control. If the project's checks flag files under `.agent-delegation/`, ignore those findings; do not edit
ignore files, formatter or linter configuration, or anything else to make them go away.

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

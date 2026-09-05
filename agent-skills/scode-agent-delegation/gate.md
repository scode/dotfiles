# The gate

Read this file the first time in a session that a delegate stops or finishes. The caller is the quality gate for
everything a delegate produces; this file is the procedure, and it ends in exactly one of the verdicts listed in
`SKILL.md`. Never accept a delegate's self-report as evidence the work is good.

## Writers

For code output, the gate starts while the delegate is still running. Every writer stops twice under the checkpoint
protocol in `checkpoints.md`, and each stop is a gate step:

1. At `AWAITING GUIDANCE`: read `ASSUMPTIONS.md` in the delegation's run directory (`.agent-delegation/<run-id>/`),
   write `ANSWERS.md` there (one line per item, `OK` or a replacement), resume the same delegate.
2. At `AWAITING REVIEW`: read `DECISIONS.md` in the run directory, write `REVIEW.md` the same way, resume again.
3. When it finishes: inspect the actual change set yourself, not just the report — status plus diff in whatever VCS is
   in use (under git, `git status` and `git diff`; a plain diff misses new files, renames, and mode changes, and a new
   file is where a delegate's surprises tend to live). Strip any edit to ignore files or formatter configuration made to
   hide `.agent-delegation/`; it is a diff leak, not part of the work.
4. Re-run the relevant checks yourself.
5. Read `DECISIONS.md` in full (entries added while applying your review are unreviewed) and `REPORT.md`'s Deviations
   section, which must point at `ASSUMPTIONS.md` and `DECISIONS.md` and say what you changed at each checkpoint. A
   delegate that produced `REPORT.md` without stopping is a failure to catch by content — there is no sentinel in its
   final message — and its work is unreviewed until you have read the two files after the fact; `checkpoints.md` says
   how to classify what came back.
6. Go back to what the user actually asked for — their own words where you still have them, the retained record of the
   request plus their later clarifications and decisions after a compaction — and check that the work delivers it. Later
   explicit user direction wins over the first phrasing; the point is to catch intent that was silently dropped, not to
   resurrect a superseded reading. The spec is one interpretation, and it can narrow the request without anyone
   noticing: the acceptance checks are derived from the spec, so a spec that quietly dropped the point produces a diff
   that passes them all. This is not hypothetical; in an eval, a request to make a slow command fast was gated to a
   correct change never shown to make it fast, and a request to clean up leftover directories was gated to a correct
   hardening of an adjacent edge case. Judge the right unit: a delegation that is one step of a decomposed goal only
   owes its step, and the whole-request question is asked of the integrated result. And answer with evidence — point at
   the line, the test, or the measurement that delivers each thing the user asked for — not with the diff looking
   plausible. When something is missing, find the layer before deciding: a spec that dropped it, a delegate that missed
   a correct spec, a later unit that owns it, or a blocker only the user can resolve.

Then return the verdict:

- Everything holds: `accepted`, with a note of anything a later unit still owes.
- Small defects only (naming, comments, minor logic): fix them yourself — a fixup round-trip costs more than doing it —
  and return `accepted with local fixes`, saying what you fixed.
- The diff is correct against its spec but the spec dropped something the user asked for: `spec defect`, naming what the
  spec dropped.
- Wrong or incomplete against the spec, but the defects are well specified and one fixup round could close them:
  `substantive failure, fixable`, with the concrete acceptance failures as the payload.
- Broadly wrong, so that repairing the patch is worse than starting over: `substantive failure, structural`, with the
  acceptance failures; add the reason `lost context` when the delegate lost track of earlier context mid-task, since the
  caller's next attempt depends on knowing that, and it is not a judgment about the model.
- The work delivers the spec but the request needs a decision only the user can make: `blocked on user`, with the
  question. This is the gate's own finding at step 6; a delegate that itself asked a question instead of working is a
  decline, classified in `checkpoints.md`.

## Read-only findings

For read-only findings (reviews, scans, analysis): confirm the named artifact exists and holds the deliverable, then
spot-verify against the cited code or data before relaying. When the caller reports to the user, what was confirmed must
be separated from delegate claims that were not verified. The verdict is `accepted` when the artifact meets the
acceptance criteria; `substantive failure, fixable` when it falls short in well-specified ways; and
`substantive failure, structural` when the delegate declined, gave status, or answered a different question.

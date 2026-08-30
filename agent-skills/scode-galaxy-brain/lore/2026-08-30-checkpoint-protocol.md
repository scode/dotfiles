# 2026-08-30: why every writer stops at two checkpoints

NOTE: Point-in-time record of the evidence behind the checkpoint protocol in `delegating.md`. It is not maintained; the
skill text is authoritative for what the protocol currently is.

Primary evidence: the Treeward Guidance-Protocol Eval, round 7
(https://claude.ai/code/artifact/ef01172f-812c-4c8f-9742-68ebe1a8a0f1). Supporting: the recipe with harness quirks and
resume mechanics (https://claude.ai/code/artifact/f18b4c9e-8f82-4cdd-9c83-7a190b963343) and the overview
(https://claude.ai/code/artifact/43a3d4f1-fd32-41df-84bc-d62d6fb1f248). The handoff that turned the eval into skill text
is at https://gist.github.com/scode/eff527725d536e62c6b622c69065a7b9.

## The question

The delegation contract before this change was: decide, then list deviations in the report. The question was whether a
cheap delegate could be made to surface the decisions a complete-looking spec silently leaves to it, and if so, how.

## The setup

32 runs across four cheap models — gpt-5.6-luna at medium and high via codex, muse-spark-1.2-contributor high via
`muse exec`, glm-5.3-flash high via opencode — on three treeward features whose specs read as complete but were silent
on six planted decisions each. Five arms:

- A, the existing contract: decide and list deviations. Main trap avoided 0/8.
- B, spec plus "you may ask when a decision is hard to reverse and unsettled", questions batched into `QUESTIONS.md`, at
  most two rounds. Zero questions in 8 runs. Models read a complete-looking spec as settling everything; one noticed the
  trap in its own test fixture and patched the fixture instead of asking.
- C, mandatory `ASSUMPTIONS.md` with a self-test ("if the orchestrator assumed the opposite, would the result be wrong
  for the feature's purpose?"). All four models wrote the trap down; one asked anything, about other items; one applied
  the test backwards.
- D, list plus unconditional stop, orchestrator judges. 4/4 correct on the feature that was 1/8 under A and B, one
  `Replace` line per run, about 1.5k orchestrator tokens to read 13–20 items.
- E, D plus an always-on `DECISIONS.md` and a second stop for review. 4/4. Sixteen genuine implementation-time entries
  across four runs (validation placement, one-sided subtree expansion, dedup in a union walk, a regression test pinning
  the correction), none needing change. The second checkpoint cost one cheap resume, $0.01–0.03 and 34–84 seconds.

Same-session resume worked on every harness and also recovered two runs killed mid-implementation by a full disk.

## The conclusion

Cheap delegates enumerate reliably and judge poorly. "Ask if unsure" is inert because the delegate's judgment of when it
is unsure is exactly the faculty that is weak. The fix is to make the enumeration mandatory and move the judgment to the
orchestrator through an unconditional stop. The decision log is always on rather than gated on task size because a
false-positive entry costs a line to skim and a missed decision costs a review round; that was the user's call.

## Decisions made when writing the skill text

The gist proposed applying the protocol to every delegation; the user narrowed it to writers, since read-only delegates
already have the named-artifact contract and an assumptions stop before a review is overhead without a diff to protect.
No size-based opt-out: a writer too small to be worth two resumes is a task the orchestrator does itself. The checkpoint
files moved from the repo root to `.galaxy-brain/` to keep them out of the way of formatters and `git status`; the eval
used the root, so that location is untested in the strict sense, but nothing in the protocol depends on the path.

## What the delegates actually produced

`ASSUMPTIONS.md` ran 8–20 numbered items, written in 46–230 seconds for $0.01–0.02. Every model included the planted
main decision as an explicit item with the correct alternative rejected — that pattern, right answer listed and
discarded, is what the checkpoint exists to catch. `DECISIONS.md` on a ~300-line feature ran 0 entries (luna medium), 1
(luna high), 4 (glm), 11 (muse), all genuine. After the protocol, every delegate's `REPORT.md` Deviations cited "changed
by orchestrator (ANSWERS.md #N)" correctly. Delegates reformatted `ANSWERS.md` with the project formatter, which is
harmless.

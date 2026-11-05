# Instructions for agents changing this skill

## SPEC.md

The skill must conform to `SPEC.md` next to this file. Read it before changing any file in this directory. If the change
you are making, or the text you find, disagrees with `SPEC.md`, treat that as a bug: fix the skill, or update `SPEC.md`
explicitly in the same change with the reason.

`SPEC.md` opens with a `Dependencies:` line that `tests/skill_deps.rs` at the repository root parses, and `SKILL.md`
carries the matching marked stanza whose wording the same test checks against its canonical template. Change the stanza
only by changing the template in the test, and then in every skill that carries one.

## Checks for the runner

After changing `scripts/shepherd`, run from the repository root:

```bash
deno fmt --line-width=120 --ext=ts agent-skills/scode-shepherd/scripts/shepherd
deno check agent-skills/scode-shepherd/scripts/shepherd
deno test -A agent-skills/scode-shepherd/scripts/shepherd
```

and `deno lint` on a `.ts` copy of the script (`deno lint` skips extensionless files).

The unit tests drive `runLoop` through a fake `Ops`. When you change how the loop sequences its steps, add a test that
pins the new sequence through the fake's `calls`, and confirm it fails with the change reverted.

## Smoke run against real codex

The unit tests never start codex, gh, or git. After changing the real operations (`realOps`, `runLogged`, the hook
installation, or `cmdHook`), do a smoke run. It costs one cheap worker session and writes nothing to GitHub; it only
reads `scode/repotesting`:

1. Create a scratch Git repository with an empty commit on `main` and `origin` set to
   `https://github.com/scode/repotesting.git` (the runner asks `gh` which repository the checkout belongs to before
   anything else), and a run directory outside it.
2. Write a `plan.json` with one item that asks for one file, a workflow that forbids version control, the check `test -f
   <that file>`, `"worker": {"model": "gpt-6.1-terra", "effort": "low"}`, `"limits": {"sessionsPerItem": 1}`, and a spec
   that tells the worker to report done with `https://github.com/scode/repotesting/pull/999999`, which does not exist.
3. Run `shepherd init`, put a message file in `inbox/` that asks for a second file, then `shepherd run`.

Expected: both files exist; the message moved to the session's `inbox-delivered/`; the session's `hooks.jsonl` shows a
`PostToolUse` that replied and a `Stop`, all from one thread; the session has a valid `report.json`; and the run exits
10 with a `sessions-exhausted` escalation whose detail says the PR does not exist. Changes to verification or review
need a real stack, on `scode/repotesting` or a repository the user names.

After changing `cmdDrive`, `realDriverOps`, or the "Headless mode" section of `SKILL.md`, also drive one small goal
headless on a fresh jj clone of `scode/repotesting`: a one-PR goal file, and an answerer script that approves whatever
it is asked and keeps a copy of each question. Expected: the first question is a complete plan summary; the drive ends
with `DONE` and the final report; one draft PR exists; and `supervisor/` holds one `next-<n>.json` per turn. This puts
up a real draft PR on that repository, which exists for this kind of testing.

## Evaluating skill text

After changing `SKILL.md`, eval it with fresh-context sub agents before calling the change done: tell each only where
`SKILL.md` lives, state the situation in full, and judge whether the answer reflects the intended behavior. Unless a
situation says otherwise, the session is interactive Codex on gpt-6.1-sol high, the skill was invoked with a goal in a
Git repository with a GitHub remote, and no models were named.

| Situation                                                                   | Expected answer                                                                                                                          |
| --------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| invoked with "build a stack of 5 PRs adding X"                              | run directory next to the checkout, plan written, stack shown to the user and approval awaited before `init`                             |
| invoked with a path to a goal file the user wrote                           | the same, including waiting for approval                                                                                                 |
| the session is Claude Code                                                  | says v1 needs a Codex session and stops                                                                                                  |
| the plan is approved                                                        | `init`, journal started, runner armed under codex-wake-on-exit with a fresh wake directory, turn ended; no polling                       |
| woken with exit 10, `review-rounds-exhausted` on item 3                     | status read; a read-only sub agent investigates the review and sessions; guidance written, model changed, takeover, or user asked; rearm |
| woken with `replan`: the worker says item 4 should be split in two          | the split is a change to the stack's shape, so the user approves it before `plan.json` is edited and the runner restarted                |
| the user asks mid-run to tell the worker to use a different crate           | status checked; message names the item, written elsewhere and renamed into `inbox/`; user told it reaches the next tool call, once       |
| the supervisor wants to fix one item itself                                 | waits for the runner to stop, does the work, `shepherd adopt`, runs again                                                                |
| item 2 has escalated for the third time                                     | asks the user instead of running again                                                                                                   |
| the runner exited 0                                                         | final report with PRs, sessions, rounds, escalations, tokens, minor findings, run directory; PRs left as drafts                          |
| scode-powerhorse is active when the skill is invoked                        | says so and asks which one governs                                                                                                       |
| after compaction, the summary mentions a shepherd run directory             | rereads the skill, the journal, and status; reconciles the last watcher before arming anything                                           |
| headless: started by `shepherd drive` with a goal and a run dir             | uses that run directory; after planning writes `next.json` with an `ask` holding the full plan summary, ends the turn; no skillette      |
| headless: the answer to the approval question asks for a change             | revises the plan and asks again with another `ask`; no `init` until an answer approves                                                   |
| headless: the runner exited 10 with `blocked` the supervisor cannot resolve | writes an `ask` with the worker's question and the context needed to answer it; ends the turn; nothing to acknowledge                    |

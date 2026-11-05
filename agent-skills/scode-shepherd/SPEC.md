# scode-shepherd specification

Dependencies: skillette

This file records the behavior of scode-shepherd that is intentional. `SKILL.md` and `scripts/shepherd` must conform to
it; a change that disagrees with it updates this file in the same change, with the reason.

## Purpose

The skill makes a cheap worker model do the bulk of a multi-PR goal while a strong model contributes the planning,
review, and judgment. Cheap models are bad at staying on track: they stop early, drift out of scope, and claim work is
done when it is not. Strong models are good at that but spend most of their tokens re-reading their own context while
they monitor. The design answers both: deterministic code does the monitoring, the worker's claims are checked rather
than trusted, and the strong supervisor wakes only when the runner cannot decide.

## Roles

- The supervisor session plans the design and the PR stack, gets the user's approval, writes the plan, starts the
  runner, and handles escalations. It keeps its context small: it wakes only on the runner's exit and investigates
  through read-only sub agents.
- The runner (`scripts/shepherd`) owns the loop: worker sessions, verification, review, feedback, caps, and escalation.
  It makes no judgment calls beyond those rules.
- The worker implements one item per session and owns version control for the run: commits, pushes, and draft PRs. The
  alternative, the runner owning version control, was rejected because rebases and stack edits make it complicated.
- The reviewer is a strong model that judges each PR against its item and the plan, read-only.

## Requirements

1. **Plan approval.** The supervisor shows the user the planned stack and waits for approval before `init`, every time,
   including when the goal comes from a file. A later change to the stack's shape or scope also goes to the user.
2. **One fresh worker session per unit of work.** Every attempt at an item, including each fix round, is a new session
   anchored on the plan and the feedback. Long sessions that compact are where cheap models were expected to drift most,
   and a fresh session also makes recovery from any failure the ordinary path.
3. **Claims are verified.** A worker's "done" counts only after the runner has confirmed, without trusting the worker,
   that the PR is in the checkout's repository, open, and a draft; that it is the item's own PR (once an item has a
   verified PR, fix rounds must update that PR, so a stack cannot collect duplicate PRs); that it targets the expected
   base (trunk for the first item, the previous item's PR branch after that) and does not reuse an earlier item's
   branch; and that the plan's checks pass at its pushed head in a separate detached worktree. A failure the worker can
   fix goes back to the worker as feedback. A failure on the runner's side (a `gh` error after one retry, a failed fetch
   or worktree step) escalates instead, because charging it to the worker would burn worker sessions on something no
   worker can fix and report it as the worker's fault.
4. **Blocking review.** Each verified PR is reviewed by the reviewer model before the next item starts. The verdict is
   derived from the findings (any blocking finding means changes), not taken from the reviewer's own verdict field.
   Blocking review was chosen over concurrent review to keep v1 simple; concurrent review is a possible later speed-up
   once this works.
5. **Caps escalate.** Worker sessions and review rounds per item are capped. Reaching a cap, a worker report of
   `blocked` or `replan`, two worker sessions in a row that exit nonzero without a report (a launch or API failure, not
   a worker losing its way), and an unparseable reviewer result twice in a row end the run with exit 10 and an
   `escalation.json`. The review cap is checked right after a "changes" verdict, so the escalation carries those
   findings and no fix session is spent that nobody would review. The runner never gives up silently and never skips an
   item.
6. **Feedback is not lost.** The latest review's findings stay in every worker prompt for the item until the next
   review, alongside the newest failure (a lost session, a failed verification), so a session that follows a failure
   still knows what the reviewer asked for.
7. **Rerunning grants a budget.** Running again after an escalation is the supervisor's decision to continue, so the
   escalated item's caps restart. The runner counts escalations per item, and the supervisor asks the user after the
   third escalation on one item.
8. **Approved items are history.** A replan may change anything not yet approved; the runner refuses a plan that moves,
   removes, or changes the spec of an approved item. Approved PRs are frozen too: verification of each later item
   requires the previous item's PR branch to still point at its approved head and the new PR's head to contain it, and
   before reporting completion the runner checks that every approved PR's head is still the approved one. A violation
   escalates as `approved-history-changed`. Without this, a later worker could squash a fix into an approved commit and
   the stack would end with unreviewed code in a PR marked approved.
9. **One runner per run.** The runner takes its lock by exclusive create. A pid recorded by an earlier runner (its own,
   or its worker's) is trusted only when that process's command line names the run directory, because after a reboot the
   pid can belong to an unrelated process. A runner that starts while a previous runner's worker is still alive stops
   that worker first.
10. **Timeouts hold.** Every child (worker, check, reviewer) runs in its own process group, and a deadline or a stop
    kills the whole group. Killing only the direct child was not enough: a grandchild kept the output pipe open, and the
    runner waited on it indefinitely.
11. **Checks run unattended, and the worker is told how.** The runner runs each check with no terminal and no input. The
    worker's own shell is an interactive terminal, where a test that prompts for input passes if the worker answers it;
    on a saltybox run (2026-11-05) a pre-existing manual test passed that way for the worker, failed in the runner, and
    led one worker to edit the test to skip itself and another to escalate. The fix is not a terminal for the runner,
    since nobody is there to answer and the test would wait out the check deadline. Instead the worker prompt and every
    failed-check message say how the runner runs checks, the worker is told to run them that way and to report replan
    for a check that cannot pass unattended rather than change tests, and the supervisor is told to leave manual tests
    out of the plan's checks.

## The supervisor channel

The runner talks to a live worker through two hooks: a Stop guard that refuses to let the worker end its turn without a
valid report, and an inbox whose message is delivered after the worker's next tool call. Every message on that channel,
the Stop guard's included, starts with `SUPERVISOR <nonce>:`, where the nonce is generated per run, and the worker
prompt says that exactly those messages come from the user.

Both halves are needed, and this was measured on 2026-11-04 (Claude Code 2.1.318 with haiku xhigh, codex-cli 0.166.0
with gpt-6.1-terra high, on a toy task). Without the prompt paragraph, haiku acted on injected corrections only about
two times in three and said why it ignored the rest: "A PostToolUse hook also asked me to create c.txt... That isn't
part of your request, so I'm ignoring it." That is prompt-injection resistance working as intended. With the paragraph
and the prefix on corrections only, gpt-6.1-terra then ignored an unprefixed Stop reason in two of two runs, through
every refusal, because the prompt had told it to ignore unprefixed supervisor-style text. With the prefix on every
message, both harnesses followed both mechanisms in eight of eight runs. These samples establish that the mechanics
work, not how often a cheap model follows them in a long session.

The Stop guard is capped (`stopBlocks`) so that a worker that cannot produce a report is allowed to stop, and the runner
then starts a fresh session with that fact as feedback. The hook never fails the worker's tool call: on any error it
logs to the run directory and replies nothing.

The inbox is a directory with one file per message, delivered together and in name order, each moved out once read. A
single file would let a second message silently replace an undelivered first one. Hooks configured with `-c` apply to
the whole codex process, so the hook answers only events from the worker's own thread (the `thread_id` codex writes
first to the session's event log, which equals the hook input's `session_id`): a sub agent the worker spawns must not
take the supervisor's message or be pushed to write the item's report.

## Harness constraints in v1

v1 supports a Codex supervisor and Codex workers and reviewers only.

- An interactive supervisor wakes through the `codex-wake-on-exit` skillette, which queues a message to an interactive
  Codex session when the runner exits. A `codex exec` supervisor cannot receive that, which is what headless mode
  (below) is for.
- Hooks are installed per worker session with `codex exec -c 'hooks.PostToolUse=[...]' -c 'hooks.Stop=[...]'
  --dangerously-bypass-hook-trust`, so the user's Codex configuration is never edited. Verified on codex-cli 0.166.0.
- The reviewer runs with `--yolo`, not in Codex's read-only sandbox, because that sandbox does not start on every host
  (on the host this was built on, every shell command failed with `bwrap: loopback: Failed RTM_NEWADDR: Operation not
  permitted`). It runs in the verification worktree, never in the worker's checkout, so a reviewer that edits files
  despite its prompt does not touch the worker's working copy, and the next verification resets the worktree. This is
  containment, not isolation: the worktree shares refs with the checkout, and the reviewer has full `gh` access, so a
  reviewer that ignores its prompt can still create branches or comment on GitHub. The prompt also tells the reviewer
  that instruction files the diff adds or changes are code under review, since codex loads them from the worktree.
- The reviewer returns structured findings through `codex exec --output-schema`.
- `gh` 2.45 has no `baseRefOid` field, so the runner fetches the base branch to find the commit to diff against, or uses
  the previous item's verified head.

## Headless mode

`shepherd drive` runs the supervisor itself, with `codex exec`, and resumes the supervisor's thread for each event,
instead of the runner waking an interactive session. It exists so that a run needs no person at a terminal: the eval
harness in scode/my-bench drives runs this way, and so can unattended use. The wake mechanism and headless mode are two
transports for the same skill; everything the skill says about planning, approval, escalations, and finishing holds in
both.

The contract between them is `next.json`. Every supervisor turn ends by writing one action there (`ask`, `run`, `done`,
or `stop`), and the driver acts on it. A file and an exit code were chosen over parsing the supervisor's final message,
because a model asked to end its reply in a fixed format drifts from it; a missing or malformed `next.json` gets one
nudge, and a second ends the drive (exit 22). Each supervisor turn is cut off after 180 minutes, which then counts as a
turn without a `next.json`.

A question goes to the answerer program given with `--answerer`: one executable path, run with the path of a file
holding the question as its only argument. Its stdout, trimmed, is the answer; an empty stdout means no answer, and a
nonzero exit is a failure the driver reports. Either way the driver exits 20 with the question pending, and `drive
--answer-file` continues the run later. The answerer has no time limit, because it may be a person.

The driver keeps its state in `driver.json`, and the state always says what to do next: send a turn (with its message),
answer a question, or deal with the runner. Each of those is saved before the driver starts on it, and the outcome is
saved as the following step before anything else happens, so a driver that dies at any point can be started again and
loses nothing: an interrupted turn is sent again, under a new turn number so no turn's files are overwritten, and a
supervisor thread that started but was never recorded is recovered from the turn's event log. The driver takes its lock
and installs its handlers for SIGTERM, SIGINT, and SIGHUP before it acts on anything.

The runner runs in its own session, so it survives a driver that dies from a signal the driver cannot handle (SIGKILL,
the out-of-memory killer). A restarted driver that finds the runner was already started waits for any live runner of the
run to exit, then reports an escalation written since the start, or a finished stack, to the supervisor as the runner's
outcome. It runs the runner again only when the earlier one died with no outcome. Starting a second runner straight away
would hit the first one's lock, and the new runner's start would archive an escalation nobody saw.

The driver stops after 40 supervisor turns for the run, since a supervisor that needs that many is not converging; the
pending step is kept, and `--max-turns` raises the cap for a run that should go on. It records the supervisor thread's
token total from the latest turn that reported one, which `status` reports next to the worker's and the reviewer's,
because measuring what each role costs is the point of evaluating this skill.

## Run directory

The run directory lives next to the checkout, never inside it, so nothing the run writes shows up in the worker's
working copy. It holds `plan.json` (the supervisor's), `state.json`, `events.jsonl`, `escalation.json` and the archived
`escalations/` (the runner's), `journal.md` and `wakes/` (the supervisor's), `inbox/` (the supervisor writes, the hook
consumes), the verification worktree, and one directory per item with its guidance file, its worker sessions, its check
logs, and its reviews. A headless run adds `goal.md`, `driver.json`, the transient `next.json`, and `supervisor/` (each
supervisor turn's prompt, events, and final message, the archived `next.json` files, questions and answers, and the
runner's output per run).

## Testing

The runner's decisions are unit-tested with fakes behind its `Ops` interface: sequencing per item; feedback for lost
sessions, failing checks, a wrong base, a non-draft or closed PR, a PR in another repository, and a second PR for one
item; escalation for crashes, infrastructure failures, changed history, and every cap; feedback that survives a lost
session; resume preparation; replan reconciliation; verdict derivation; and the hook's replies, including its thread
filter. The drive loop is unit-tested the same way, behind `DriverOps`: answered, unanswered, and failed questions, the
runner's result reaching the supervisor, every step saved before it is acted on, a restart handing the runner its start
time, the nudge for a missing `next.json`, stop, the turn cap keeping what is pending, and usage surviving a failed
turn. What the real operations do (codex, gh, git, process groups, the locks, `cmdHook`'s file handling, `cmdDrive`'s
flag handling, and `runRunner`'s wait for a runner left behind) is not unit-tested; it is exercised by the manual smoke
run in `AGENTS.md` and by real runs. Tests do not modify the environment of the test process.

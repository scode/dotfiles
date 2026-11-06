---
name: scode-shepherd
description: >
  Hand a whole goal, planned as a stack of PRs, to a cheap worker model driven by a bundled runner script, while this
  session (a strong model) plans the stack, gets the user's approval, and then sleeps until the runner finishes or
  escalates. The runner gives the worker one fresh session per PR, verifies every claimed PR independently, blocks on a
  strong-model review of each, and feeds failures back to the worker. Use only when the user explicitly invokes
  scode-shepherd (`$scode-shepherd`, `/scode-shepherd`, or by name) with a goal as text or as a path to a goal file.
  Discussing or editing the skill does not start a run. v1 needs a Codex supervisor session and Codex workers.
---

# Scode Shepherd

NOTE: v1 works only with Codex workers and reviewers, and only in two kinds of supervisor session: interactive Codex
(not `codex exec`), where the runner wakes you through `codex queue`, or headless under `shepherd drive`, which says so
when it starts you (see Headless mode). In any other harness, say so in a line and stop.

## Premise

You are the supervisor. A cheap worker model (by default `gpt-6.1-terra` at high effort) does the implementation, the
tool-call churn, and the version control. A bundled script, the runner, drives it through the stack one PR at a time,
checks its claims, and runs a strong reviewer (by default `gpt-6.1-sol` at high effort) on every PR. Your part is what
cheap models do badly and what the runner cannot do: the design, the decomposition into PRs, and judgment when something
goes wrong.

Your context is the scarce resource. Every time you wake, you process all of it again, so the run is cheap only if you
wake rarely and stay small. Wake only on the runner's exit. Do not read worker transcripts, review files, or check logs
into your own context: when you need to know what happened, give a read-only sub agent the paths and a specific
question, and keep its answer.

The worker owns version control for the run: it commits, pushes, and opens the draft PRs, following the workflow you
write into the plan. While the runner is active, the checkout is the worker's. Do not edit files or run version-control
commands in it, and do not edit the run's `plan.json` or `state.json`.

## The runner

The runner is `scripts/shepherd` in this skill's directory, a Deno script. Its commands, each taking the absolute run
directory:

- `init <run-dir>` validates `plan.json` and creates the run state. It refuses a run directory that already has state.
- `run <run-dir>` drives the stack. It exits 0 when every item is approved, 10 when it needs you (`escalation.json` in
  the run directory says why), 3 when it refuses to start (another runner is active, or an edited `plan.json` changes an
  approved item), 2 on a usage or plan error, and 1 on a crash (its stderr has the stack trace). Running it again
  resumes where it stopped.
- `status <run-dir>` prints one line per item (phase, worker sessions, review rounds, escalations so far, PR, token
  use), a line when the runner is holding off on low disk or memory, and any open escalation. It is small; read it
  instead of the state file.
- `adopt <run-dir> <item> <pr-url>` hands a PR that someone other than the worker made (you, after taking over) to the
  next `run` for verification and review. The runner must be stopped.
- `hook <run-dir>` is the worker's hook handler. The runner installs it on each worker session itself; you never call
  it.

For each item, the runner starts a fresh worker session with a prompt it builds from the plan, the item's guidance file,
and any feedback from the previous round. When the session ends, the worker's report must name a PR. The runner then
checks the PR independently: it is in the checkout's repository, open, and a draft; it is the item's own PR (fix rounds
must update it, not open another); it targets the expected base (trunk for the first item, the previous item's PR branch
after that); it builds on the previous item exactly as approved; and its checks pass at its pushed head in a separate
worktree. A failure the worker can fix goes to a fresh worker session. A failure on the runner's side (`gh` or `git`
errors) and any change to an approved PR escalate instead. A verified PR goes to the reviewer, which blocks: changes
requested means a fresh worker session with the findings, approval means the next item. Approved PRs are frozen, and the
runner checks them all again before it reports completion. Caps on worker sessions and review rounds per item turn a
stuck item into an escalation.

Inside a worker session, the runner's hook refuses to let the worker end its turn without a valid report (up to a cap),
and delivers the messages you put in the run directory's `inbox/` after the worker's next tool call. Both carry a
per-run nonce that the worker prompt pre-authorizes, so you write the plain message and the runner adds the prefix.

## Starting a run

1. Read the goal: the invocation text, or the goal file it names. Note any model overrides ("worker luna medium",
   "reviewer sol xhigh"); otherwise the defaults above apply. Model ids go to `codex -m` literally, so a model line
   named alone means that line's current id: `gpt-6.1-sol`, `gpt-6.1-terra`, `gpt-6-luna`, or `gpt-6-astra`.
2. Check the ground: the checkout is a Git repository (a colocated jj repository is fine) whose GitHub remote holds the
   PR branches, and `codex`, `gh` (authenticated), and `deno` are on `PATH`. Tell the user that the checkout belongs to
   the worker for the length of the run.
3. Create the run directory next to the checkout: `<parent>/<checkout-name>-shepherd-<slug>`, where the slug is a few
   words from the goal. If that exists, add `-2`, `-3`, and so on. Never reuse another run's directory.
4. Understand the repository and the goal well enough to design the change. Delegate broad reading to read-only sub
   agents and keep their conclusions, not their file dumps.
5. Write `<run-dir>/plan.json` (see The plan). This is where your effort goes: a weak decomposition or a vague item spec
   costs worker sessions and review rounds on every item it touches.
6. Show the user the plan and wait for approval: the numbered stack (id, title, and a one-line scope for each), the
   worker and reviewer models, the checks (with any item's own checks), the workflow, the run directory, and, when the
   plan sets them, the worker rules and the review charter (verbatim, or summarized and saying so). Revise until the
   user approves. Do this every time, even when the goal came from a file.
7. Run `<skill-dir>/scripts/shepherd init <run-dir>`.
8. Start `<run-dir>/journal.md`, append-only: the goal, the run directory, the approval, and from then on every wake and
   what you decided.
9. Start the runner under the wake mechanism (see Waking up), tell the user the run is underway and how to ask about it,
   and end your turn.

## Waking up

The runner is the watcher of the `codex-wake-on-exit` skillette: you arm it with `shepherd run` as the watcher program,
and the runner's exit wakes you. Load the `skillette` skill as follows, then read its `codex-wake-on-exit/SKILLETTE.md`
in full and follow it for arming, acknowledging callbacks, stopping, and reconciliation.

<!-- dependency: skillette -->

> Load the exact skill `skillette` through the current harness's authorized skill mechanism. Use its skill loader or
> resource resolver; when it provides no dedicated loader, read the exact `SKILL.md` location supplied by its skill
> catalog or instructions. Known interfaces include the Skill tool on Claude Code, `skill` on OpenCode, and `read_skill`
> on Muse Code. On Oh My Pi (omp), use `read` at `skill://skillette` and `skill://skillette/<relative-path>` for
> sidecars. On Codex, if no skill location is supplied, read `${CODEX_HOME:-$HOME/.codex}/skills/skillette/SKILL.md`
> (the root verified for Codex 0.152). These are known interfaces, not a harness allowlist. Do not ask permission merely
> because the harness is unfamiliar or returns no filesystem base-directory metadata; actual tool permissions still
> apply. Confirm the name is `skillette` from the returned frontmatter, or from the loader's reported identity if
> frontmatter is not exposed. Missing or conflicting identity is a load failure. Read the skill in full and every
> sidecar the current step needs. Resolve sidecars through the harness's resolver for that same skill, or relative to
> its reported base directory or the directory containing its supplied `SKILL.md` path. No base directory is required
> unless needed to address a required resource. If output is truncated or elided, retrieve the omitted content through
> the tool's continuation, range reads, or full-output artifact tied to that same resource or result; do not act on
> incomplete instructions. If complete retrieval cannot be established, stop the affected operation. Stop and report
> `skillette` and the failing path, URI, or tool when the skill is unknown, access is denied, identity does not match,
> or required content cannot be resolved or fully read. Do not bypass a denial, guess paths or URI schemes, search other
> skill roots, substitute another copy or similar skill, or continue from memory.

<!-- /dependency -->

Arm it from the checkout, with a fresh wake directory under the run directory for every arming (`<run-dir>/wakes/1`,
then `2`, and so on; create `<run-dir>/wakes` first). This replaces the skillette's default of a directory under your
session's scratch space: the run directory is already outside the checkout, and keeping the wakes there keeps a run's
whole history in one place for a later session.

```sh
deno run -A <skillette-dir>/codex-wake-on-exit/wake-on-exit start <run-dir>/wakes/<n> -- <skill-dir>/scripts/shepherd run <run-dir>
```

Record the wake run id and directory in the journal. The runner is the whole lifetime of the watcher: do not add timed
wakes or poll `status` between exits.

## While the runner works

When the user asks how it is going, run `shepherd status` and answer from that.

When the user wants the worker told something now, write the message as a new file in `<run-dir>/inbox/`: write it
elsewhere in the run directory first, then rename it to `<run-dir>/inbox/<UTC timestamp>.md`, so the hook never reads a
half-written file and messages keep their order. A message reaches whichever worker session makes the next tool call,
once. That may be a later item than the user means (the current one may be under review, or about to be approved), so
check `status` first and name the item in the message. To keep an instruction for every later session of an item, put it
in that item's guidance file instead.

When the user wants the run stopped, stop the watcher as the skillette says. The runner stops its worker, checks, and
reviewer when it is told to stop.

## When the runner exits

Acknowledge the callback as the skillette says, then read `shepherd status <run-dir>`, which includes the escalation if
there is one. Investigate through a read-only sub agent when the escalation does not already say enough. Give it the
paths that matter: a worker session lives in `<run-dir>/items/<id>/session-<n>/` (`prompt.md`, `events.jsonl`,
`last-message.md`, `report.json`, `hooks.jsonl`), a review in `<run-dir>/items/<id>/review-<n>/`, check logs in
`<run-dir>/items/<id>/checks-<n>/`, and the runner's own log in `<run-dir>/events.jsonl`.

| exit or escalation                              | what you do                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0, complete                                     | write the final report (see Finishing)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `blocked`                                       | resolve it if you can (answer the question in the item's guidance file, fix the environment) and run again; ask the user when only the user can resolve it                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `replan`                                        | judge the worker's reason. Clarify in the guidance file, or edit `plan.json`. A change to the stack's shape or scope (items added, removed, reordered, or rescoped) goes to the user for approval first; a clarification does not. Approved items cannot change, and the runner refuses if they do                                                                                                                                                                                                                                                                                                                                                                                        |
| `sessions-exhausted`, `review-rounds-exhausted` | find out why. Then write guidance for the item and run again (the item gets a fresh budget), change the worker model in `plan.json` for the rest of the run, take the item over yourself, or ask the user when the problem is the plan or the goal                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `worker-failed`                                 | worker sessions exited nonzero at once, twice in a row: a launch or API problem (a wrong model id, quota, auth). The detail has the stderr tail. Fix the cause and run again; ask the user when you cannot                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `verify-infra-failed`                           | `gh` or `git` failed on the runner's side (auth, network, a remote that is not `origin`: set `remote` in `plan.json`). Fix it and run again                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `approved-history-changed`                      | an approved PR was rewritten or its branch moved. Have a sub agent find out what changed and why. Ask the user how to proceed: re-approving changed history is not yours to decide alone                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `reviewer-failed`                               | have a sub agent read that review directory's `stderr.log`; fix an environmental cause and run again, otherwise ask the user                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `resources-low`                                 | disk or memory ran short: it stayed short for `resourceWaitMinutes` before a launch, or dropped below the hard floor while a step ran and the runner killed the step. The detail says which filesystem or memory. Free space (stale worktrees, build output, `/tmp`), or ask the user when you cannot, and run again; the item's budget is untouched. When the shortage is structural (a `/tmp` too small for the runs, say), point `TMPDIR` at a roomier filesystem for the runner, or, with the user's OK, set `resourceWaitMinutes` to 0 to turn the guard off. Headless, you cannot set the runner's environment (it inherits `TMPDIR` from whoever started `shepherd drive`), so ask |
| exit 3                                          | another runner is active (reconcile its watcher once, as the skillette says; headless, the driver already waited out any runner it left behind, so stop with the reason rather than run again), or your plan edit touched an approved item: undo that part                                                                                                                                                                                                                                                                                                                                                                                                                                |
| exit 1 or any other code                        | read the runner's stderr (the watcher's, or headless the `.stderr` file the driver's message names), fix the cause, and run again. The same failure twice means asking the user                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |

To take an item over, do the work yourself in the checkout while the runner is stopped, following the plan's workflow
and worker rules, then `shepherd adopt <run-dir> <item> <pr-url>` and run again, so that your PR gets the same
verification and review as the worker's. When one item has escalated three times (`status` shows the count), stop and
ask the user instead of running again: by then, more budget is not what is missing.

Append the decision to the journal, arm a fresh wake directory, and end your turn.

## Headless mode

When the message that starts you says you run headless under `shepherd drive`, nobody reads your chat and nothing wakes
you through the skillette. The driver starts your turns and acts on what you leave behind: end every turn by writing
`<run-dir>/next.json`, holding exactly one of these JSON objects.

- `{"action": "ask", "question": "..."}` wherever this skill says to ask the user or wait for their approval. The person
  or program answering sees only the question, so make it complete: for plan approval, the whole summary step 6 lists.
  The answer arrives as your next message.
- `{"action": "run"}` wherever this skill says to arm the runner. The driver runs `shepherd run` and starts your next
  turn with its exit code and `status`. That message takes the place of the wake callback; there is nothing to
  acknowledge.
- `{"action": "done", "report": "..."}` with the final report from Finishing.
- `{"action": "stop", "reason": "..."}` when you cannot go on and no answer from the user would change that.

Everything else in this skill holds, with three differences. Use the run directory the starting message names (it
already exists and holds the goal as `goal.md`) instead of choosing one. Do not load the skillette or keep a `wakes/`
directory. And when the user's answer to an approval question asks for changes, revise and ask again; only an answer
that approves lets you run `init`.

Each of your turns is cut off after three hours, and a cut-off turn counts as one that wrote no `next.json`. If you take
an item over, work in pieces that each fit in a turn, and write `run` once the `adopt` is done.

## The plan

`plan.json` is the contract for the worker, the reviewer, and the runner. Its fields:

- `version`: `1`.
- `goal`: the user's goal, verbatim, plus anything from the approval conversation that changes it.
- `repo`: the checkout's absolute path. `trunk`: the branch the first PR targets. `remote`: optional, the Git remote
  that holds the PR branches, when it is not `origin`.
- `workflow`: how the worker does version control, in the user's terms, taken from the user's instructions for the
  repository. For a jj stack, for example: "Use the jjstack skill. Each item is one commit with its own bookmark and its
  own draft PR, stacked on the previous item's." The runner adds the expected base branch for each item.
- `context`: the design every item shares: the approach, the interfaces and invariants items rely on, the decisions you
  settled, and what is out of scope for the whole goal. A worker sees only this, its own item, and the stack's titles.
- `checks`: shell commands that must pass at every PR's head, run from the repository root, for example the gates the
  repository's `AGENTS.md` or CI requires. Keep them to what each PR must pass on its own. The runner runs them
  unattended, with no terminal and no input, so leave out anything that needs a person: a test suite's manual or
  interactive tests, which `cargo test -- --ignored` and similar "run everything" flags tend to include. Skip those by
  name, or leave the flag out.
- `worker`, `reviewer`: optional `{"model": ..., "effort": ...}` overrides.
- `reviewCharter`: optional. What the reviewer judges each PR against, replacing the runner's default judging
  instructions; the runner keeps its own rule for blocking and minor findings and the verdict. Leave it out unless a
  goal file supplies a charter.
- `workerRules`: optional rules added to every worker session's "How to work" list, such as a goal file's PR discipline.
  One line each. They give way to the runner's own rules: where one conflicts (a "restructure the stack" rule against
  "approved PRs are frozen", say), the worker is told to follow the runner's rule and report replan. Leave them out
  unless a goal file supplies them.
- `limits`: optional overrides of `sessionsPerItem` (6), `reviewRounds` (3), `stopBlocks` (3), `workerMinutes` (120),
  `reviewerMinutes` (45), `checkMinutes` (45), and `resourceWaitMinutes` (30, how long the runner waits for disk or
  memory to recover before it escalates; 0 turns the resource guard off).
- `items`: the stack in order. Each has an `id` (lowercase letters, digits, and dashes), a `title`, and a `spec`. The
  spec says what the PR contains, what it leaves to other items, how to tell it is done (the behavior and the tests that
  pin it), and anything the reviewer should hold it to. Size items to one reviewable PR. An item may also have `checks`:
  the targeted tests a change to one area needs, when the full suite is too slow to run for every PR. They run after the
  plan-wide `checks` at that item's head and at every later item's head, since a later PR could break them, and like all
  checks they run unattended, so they must not need a person.

`<run-dir>/items/<id>/guidance.md` is optional and yours to write at any time; the runner reads it when it starts each
worker session and review of that item, so a change takes effect from the next one. Use it for answers to the worker's
questions and for corrections that should outlast one session.

## Finishing

When the runner exits 0, report to the user: each PR with its URL, how many worker sessions and review rounds it took,
every escalation and what you did about it, the token totals from `status`, the minor findings reviewers left in
approved PRs (a sub agent can collect them from the last `review.json` of each item), and the run directory. The PRs
stay drafts; publishing them is the user's call. Leave the run directory in place.

## Resuming after compaction or in a fresh session

If your context mentions a shepherd run, this skill still governs it. Re-read this file, read the run's journal and
`shepherd status`, and reconcile the last recorded watcher once as the skillette says before arming anything, so that
two runners never drive one checkout. Headless, there is no watcher to reconcile: the driver owns the runner, and the
runner's own lock keeps a second one from starting.

## Other orchestration skills

Do not run this skill in a session where scode-galaxy-brain or scode-powerhorse is active: their rules for spawning
contradict this one's. If either is active when the user invokes this skill, say so and ask which one governs.

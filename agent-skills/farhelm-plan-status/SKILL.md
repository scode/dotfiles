---
name: farhelm-plan-status
description: >
  Report the status of the farhelm plan queue on this host: the plan executors, the landing monitor, and the queue on
  GitHub main. Use when the user asks for "farhelm plan status", to "check status" or "check now" on the farhelm
  executors, how the executors or a farhelm plan are doing, or similar wording about the farhelm plan queue.
---

# farhelm-plan-status

[farhelm](https://github.com/scode/farhelm) drains its plan queue (`plans/queue/INDEX.md` on main) with agent sessions
on one host. This skill assumes it runs on that host, with this layout:

- Executors work in `~/git/farhelm-fh-exec-N`, each driven by an interactive Claude Code or Codex session. Either
  harness can drive any checkout, and a checkout can switch between them.
- The landing monitor works in `~/git/farhelm-fh-complete-monitor` and logs each round to
  `~/git/farhelm-plans-monitor-log.md`.
- Executors keep their own logs beside the checkouts: `~/git/farhelm-plans-log-farhelm-fh-exec-N.md`, and one per plan
  as `~/git/farhelm-plan-<plan>-log.md`.

## Hard rule: hands off the checkouts

Do not fetch, edit, or otherwise touch any executor or monitor checkout. That means no `git` or `jj` commands in them,
no reading or writing files inside them, and no running their scripts. Everything this skill needs comes from places
that are safe to read: the harness transcripts under `~/.claude/projects` and `~/.codex/sessions`, the logs beside the
checkouts, `/proc`, and GitHub main through `gh api`. Do not message an executor or the monitor, and do not answer a
blocked plan, except for the nudge below.

## Collecting

Run `scripts/collect` from this skill's directory. It is a read-only Deno script and takes about 20 seconds. It prints:

- `NOW` in the host's local time zone.
- The queue lines from `INDEX.md` on main, the claim time of every in-flight or landing claim, and the latest queue
  commits (claim, deliver, start landing, record the landing, block, answer) with local times. If GitHub is unreachable
  it prints `QUEUE unavailable` and carries on with the sessions.
- For the landing monitor and each executor, the live interactive Claude and Codex processes in that checkout, then for
  each session driving it a line `[harness] <id> state=<state> live|NO LIVE PROCESS last write …`, the transcript path,
  which active claims the session mentions (that is how a plan maps to an executor), its latest assistant messages and
  tool calls, and for the monitor the newest entries of its log.

Turn states are `running`, `idle`, `aborted at …`, `error=<kind> at …`, and `unknown` (nothing to decide from). An idle
executor is normal; executors sleep on a queue watcher between plans. `NO LIVE PROCESS` means no interactive process of
that harness runs in the checkout, so the session shown is only the most recent one on disk: whatever its state says,
nothing is working there, and that is a problem to report.

To read further (what a worker has been doing for the last hour, why something is slow), read the transcript path the
collector prints and the plan log. A blocked plan's question is in the `Blocked` section of its plan file on main: `gh
api repos/scode/farhelm/contents/plans/queue/<plan>.md -H 'Accept: application/vnd.github.raw'`.

## Nudging an overloaded Codex executor

Codex does not retry a turn that fails with `server_overloaded` ("Selected model is at capacity"). The executor then
sits idle until something messages it, which has cost hours of plan time. When the collector shows a Codex session in
`error=server_overloaded` and `live`, nudge it by typing its thread id literally. Never nudge a session marked `NO LIVE
PROCESS`: it is not the one working in that checkout.

```sh
codex queue --thread <thread id> --message .
```

Then run `scripts/collect --states` (a few seconds) and confirm that session shows `running`. Report the nudge and
whether the new turn started. Any other error, an aborted turn, or a Claude Code API error goes to the user without a
nudge.

## The report

All times are in the host's local zone, never UTC; transcripts, logs and commits are in UTC, and the collector already
converts what it prints. Write:

1. A heading `## Executor status, HH:MM <zone>` and one bolded lead sentence with the most important news.
2. A worker table with columns Worker | Plan | On plan for | State. The first row is always `landing`, then one row per
   executor. Use `—` for an idle worker's plan.
   - Plan: the active claim the executor's session mentions; when it mentions none, take the plan from its latest
     messages.
   - On plan for: that claim's time from the collector and the elapsed time since, for example `since 06:39 (2h10m)`.
   - The landing row's plan and stage come from the monitor log (round start, review, checks, merging), not from
     `INDEX.md`: a plan reads `[complete]` there until the monitor writes `landing` after its checks pass. The monitor's
     own turn state says little, since it waits on background jobs between turns.
3. A queue table with columns Status | Plan | Description, one row per `- [` line of `INDEX.md`, each description cut to
   about 60 characters.
4. Short notes on what changed since the last report and on problems only: blocks and the decision being asked for,
   send-backs, landings, stalls, errors and nudges. End with `No executor shows an error.` when that is true.

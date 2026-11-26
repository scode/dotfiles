---
name: farhelm-plan-status
description: >
  Report the status of the farhelm plan queue: the plan executors (on this host and on other tailnet hosts), the
  landing monitor, and the queue on GitHub main. Use when the user asks for "farhelm plan status", to "check status" or
  "check now" on the farhelm executors, how the executors or a farhelm plan are doing, or similar wording about the
  farhelm plan queue.
---

# farhelm-plan-status

[farhelm](https://github.com/scode/farhelm) drains its plan queue (`plans/queue/INDEX.md` on main) with agent sessions,
mostly on the maintainer's host, where this skill runs, and possibly on other hosts on the tailnet (VMs running one
executor slot each). On this host the checkouts are:

- Executors in `~/git/farhelm-fh-exec-N`. Each claims a plan from the queue, does it, and delivers it as `[complete]`.
- The landing monitor, also called the completion monitor, in `~/git/farhelm-fh-complete-monitor`. It is not an executor
  and never claims a plan: it takes completed plans, checks and fixes them, and merges them in rounds. Its state belongs
  in the report all the same, so it is collected and reported as `landing`, apart from the executors.
- Each checkout is driven by an interactive Claude Code or Codex session. Either harness can drive any checkout, and a
  checkout can switch between them.
- The executors' and the monitor's logs are records in Farhelm's `agent-records` repository, and on this host each
  slot's clone of it, under `~/.fh-records/clones/<slot>/`, holds the live copy (Farhelm's `.agents/agent-records.md`
  describes the layout; read it from main with `gh api repos/scode/farhelm/contents/.agents/agent-records.md -H 'Accept:
  application/vnd.github.raw'`, never from a checkout). The slot is `exec-N` for `~/git/farhelm-fh-exec-N` and `landing`
  for the monitor. The monitor logs each round to `farhelm/landing/log.md` in the `landing` clone; each executor keeps
  its own log at `farhelm/executors/<slot>/log.md` in its clone, and a plan's log is `farhelm/plans/<plan>/log.md` in
  the clone of whichever slot holds or last held it.
- An executor slot can also run on another host. Every records commit carries a `Slot:` trailer and, from a records tool
  new enough, a `Host: <login>@<tailnet name>` trailer naming the machine it was pushed from; a slot's newest commit
  says where it runs now. The collector reads these from its own fetch-only clone, `~/.fh-records/clones/status` (bare;
  never a slot's clone), and collects a slot on another host by running itself there over ssh. On that host the slot's
  live records clone and its transcripts are the remote machine's, so read them there.

## Hard rule: hands off the checkouts

Do not fetch, edit, or otherwise touch any executor or monitor checkout. That means no `git` or `jj` commands in them,
no reading or writing files inside them, and no running their scripts. Everything this skill needs comes from places
that are safe to read: the harness transcripts under `~/.claude/projects` and `~/.codex/sessions`, the log files in the
slot clones under `~/.fh-records/clones/` (read the files; never run `git` there, since those clones belong to the
records tool), `/proc`, and GitHub main through `gh api`. Do not message an executor or the monitor, and do not answer a
blocked plan, except for the nudge below.

The same holds on another host: read only those same kinds of places there, over ssh, and run nothing else. The
collector prints a remote transcript as `<login>@<host>:<path>`, which is exactly the ssh target and the path:

```sh
ssh <login>@<host> tail -c 200000 <path>
```

`grep` and `cat` of the slot clone's log files work the same way.

## Collecting

Run `scripts/collect` from this skill's directory. It is a read-only Deno script and takes about 20 seconds. It prints:

- `NOW` in the host's local time zone.
- The queue lines from `INDEX.md` on main, the claim time of every in-flight or landing claim with its elapsed time
  (`claimed 11-22 15:49 (35m ago)`), and the latest queue commits (claim, deliver, start landing, record the landing,
  block, answer) with local times. If GitHub is unreachable it prints `QUEUE unavailable` and carries on with the
  sessions.
- `LANDINGS`: every landing recorded in the last 24 hours, newest first, with counts for that window and since local
  midnight. Use it for landing counts, and for what landed during a long gap since the previous report, rather than
  counting from the 15 queue events.
- For the landing monitor and each executor, the live interactive Claude and Codex processes in that checkout, then for
  each session driving it a line `[harness] <id> state=<state> live|NO LIVE PROCESS last write …`, the transcript path,
  which active claims the session mentions and how often, and its latest assistant messages and tool calls. The counts
  map a plan to its executor: the owner names its claim constantly, while other executors mention it a few times in
  passing ("another executor holds …"). The monitor mentions every claim it is landing or watching, so its counts say
  nothing about ownership.
- For the monitor only, a line `round rN started|ended <time> (<elapsed> ago)` from its log, and the log's newest
  entries.
- For a slot on another host, the same section collected there, with its paths prefixed `<login>@<host>:` and a closing
  line naming the host it runs on. If ssh or the remote run fails, the section says the remote collection failed and
  why: report that slot as not checked, not as stalled.
- A slot with no checkout here whose newest records commit names no host gets a line saying so: it was pushed by a
  records tool from before the `Host:` trailer, and it shows up properly after its next push with a current tool.
- A `RECORDS unavailable` line means the records clone could not be fetched, so slots on other hosts are missing from
  the report: say so.

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

For a session on another host, run the same command there over ssh:

```sh
ssh <login>@<host> codex queue --thread <thread id> --message .
```

Then run `scripts/collect --states` (a few seconds) and confirm that session shows `running`. Report the nudge and
whether the new turn started. Any other error, an aborted turn, or a Claude Code API error goes to the user without a
nudge.

## The report

All times are in the host's local zone, never UTC; transcripts, logs and commits are in UTC, and the collector already
converts what it prints. Write:

1. A heading `## Executor status, HH:MM <zone>` and one bolded lead sentence with the most important news.
2. A worker table with columns Worker | Plan | On plan for | State. The first row is always `landing`, the landing
   monitor, then one row per executor. Use `—` for an idle worker's plan. A slot on another host names it in the Worker
   column by its short tailnet name, e.g. `exec-5 (boatworker-fh-exec-5)`.
   - Plan: the active claim the executor's session mentions far more often than the others do; when that is unclear,
     take the plan from its latest messages.
   - On plan for: that claim's time and elapsed time from the collector, for example `since 06:39 (2h10m)`.
   - The landing row's plan and stage come from the monitor's round line and log (round start, review, checks, merging),
     not from `INDEX.md`: a plan reads `[complete]` there until the monitor writes `landing` after its checks pass. Its
     On plan for is the round's start. Between rounds the row reads `—` and says since when. When the collector prints
     `monitor log not found`, the stage is unknown: say so rather than reporting the monitor as between rounds. The
     monitor's own turn state says little, since it waits on background jobs between turns.
3. A queue table with columns Status | Plan | Description, one row per `- [` line of `INDEX.md`, each description cut to
   about 60 characters.
4. Short notes on what changed since the last report and on problems only: blocks and the decision being asked for,
   send-backs, landings, stalls, errors and nudges. End with `No executor shows an error.` when that is true.

# Token usage in a completed galaxy-brain goal

Historical artifact, written September 16, 2026, about a Codex run spanning September 15–16. Records an observational
token audit and proposed improvements, not an independent review of the delivered code. Not maintained.

## What prompted the investigation

This discussion concerns the cost of unattended coding through `scode-build-goal`, `scode-galaxy-brain`, and their
underlying delegation skills. `scode-build-goal` prepares a goal for execution; galaxy-brain keeps a capable model in
charge of planning, review, and integration while assigning suitable work to cheaper models. The question was whether
the orchestration machinery itself was consuming excessive tokens, especially when waiting for processes or workers.

Earlier experiments with `scode-build-blueprint` had identified polling as a substantial source of cached-input token
usage. Blueprint already had improvements to reduce that overhead when this discussion began. A cached input token is
still billed: waking a model repeatedly to check whether a process has finished can repeatedly charge for a large
conversation context, even when the answer contains almost nothing.

The subsequent changes to the shared delegation, harness-shellout, and goal-building skills favored verified completion
delivery over polling, with bounded waits as a fallback. They retained resource monitoring and deadlines. The discussion
also rejected a blanket prohibition on model-based watchers: a small-context, cheaper watcher can be economical if its
notifications work and its total cost is lower than repeatedly waking the orchestrator. Those changes were merged in
[dotfiles PR #390](https://github.com/scode/dotfiles/pull/390/changes), commit
`803e16f564521839fe3da1bec89d29025ea0847a`.

The user then supplied a real farhelm session to see what happened in practice. An initial audit covered about 54
minutes while it was running. After completion, the user supplied a larger codex-lb usage table and requested a second
audit. This entry records that completed-run analysis. It is one observational run, not a controlled before/after
comparison of the skill changes.

## Session and accounting

The audited Codex session was `01a0a7cb-8572-78a1-b642-7b7c5c4eaa8e`, working in `~/git/farhelm` with a native Astra
medium orchestrator and native Astra, Sol, Terra, and Luna delegates. Its goal covered ten near-term fixes, including
sidebar presentation, rename behavior, the launch composer, CLI safeguards, activity detection, and stable ordering.
The user added an RC release during execution.

Execution ran from September 15 at 19:11 PDT to September 16 at 00:12 PDT, about five hours. The dashboard window ended
at 04:30:55, when the user returned to the keyboard; the session had already finished more than four hours earlier.

The supplied codex-lb table was:

| Model | Uncached input | Cached input | Output | Estimated cost | Cost share |
| --- | ---: | ---: | ---: | ---: | ---: |
| Astra | 9,034,112 | 160,484,608 | 406,671 | $271.16 | 74.91% |
| Sol | 4,825,342 | 111,979,904 | 326,657 | $70.63 | 19.51% |
| Terra | 2,620,693 | 62,470,656 | 195,324 | $20.08 | 5.55% |
| Luna | 314,830 | 1,430,272 | 14,582 | $0.11 | 0.03% |
| Total | 16,794,977 | 336,365,440 | 943,234 | $361.97 | 100% |

The audit followed recorded parent-thread IDs recursively and counted unique per-response `token_usage_record` records,
not cumulative token counters. It found 2,582 responses, 335,039,232 cached input tokens, 15,883,796 uncached input
tokens, and 933,150 output tokens in the target tree. Reasoning output is already included in output.

Sol, Terra, and Luna cached-input and output totals matched the dashboard exactly. Almost all the Astra difference
matched the earlier audit in the separate dotfiles session: 1,319,552 cached input and 9,866 output tokens. That leaves
6,656 cached input and 218 output unreconciled. Uncached accounting also differed; the dashboard's dollar total should
not be treated as an exact cost attribution to the target session alone.

| Work | Model responses | Cached input | Share of target tree |
| --- | ---: | ---: | ---: |
| Main Astra orchestrator | 1,017 | 144.35M | 43.1% |
| CLI implementation and helper | 440 | 61.19M | 18.3% |
| Other Terra implementation | 444 | 59.75M | 17.8% |
| Four Sol follow-up repair workers | 405 | 52.55M | 15.7% |
| Independent Astra reviews | 206 | 14.81M | 4.4% |
| Wording readers and surveys | 70 | 2.39M | 0.7% |

## Polling remained a meaningful minority of usage

The narrow classifier used in the initial audit counted responses containing only native waiting/status tools. Across
the completed run, those 187 responses consumed 28.16M cached tokens, or 8.4%, compared with 5.9% in the initial sample.

Manual inspection added 40 shell resource/process-status responses and 30 GitHub release-status responses. Together,
the identifiable wait/status set contained 257 responses and 37.71M cached tokens, or 11.3% of the target tree. Calls
that mixed useful work with status checks were excluded. These figures attribute usage to observed actions; they are
not an estimate of fully recoverable savings. Collecting results and checking process health can be necessary.

The CLI Sol worker made 71 responses consisting solely of process polling, carrying 12.20M cached tokens. Of those,
68 requested one-second waits and three requested 30 seconds. Some returned compiler errors or completion; others
returned nothing. In the initial sample, two consecutive empty polls each carried about 185K cached tokens. Later
ordering and activity repair workers mostly requested 30-second waits, so short polling was not universal.

The final release watch showed a particularly clear duplication. At 23:38:38 PDT the parent launched
`gh run watch --exit-status --interval 45`, redirecting output to a file. It then independently alternated sleeps with
`gh run view` and resource checks. From about 23:39 to 00:09, thirty sleep responses plus thirty status responses
consumed 4.67M cached tokens. The whole release-watch and closeout phase consumed 6.44M; neither figure is entirely
avoidable, but the existing process watcher did not prevent repeated model wakeups.

The initial audit's relatively favorable observation about native subagent waiting still held: the orchestrator was
not trapped in a constant `list_agents` loop. The remaining issue was broader, including workers polling their own
builds and the parent monitoring external release jobs.

## The orchestrator and follow-up implementation dominated

The parent generated 1,017 responses with a median input context of 150.6K tokens. It performed investigation,
validation, integration, direct fixes, and bookkeeping in addition to delegation. It consumed about 91% of the target
tree's Astra cached input; independent Astra reviews accounted for the other 9%. Given Astra's roughly 75% share of
the dashboard's dollar total, reducing expensive parent work is a more consequential target than removing cheap
wording readers.

The first CLI Terra worker left a partial, noncompiling protocol migration. Sol took over and continued through
constructor, dispatcher, and test-fixture corrections. Four other Terra tasks also received Sol follow-up workers:

| Task | Initial Terra cached input | Sol follow-up cached input |
| --- | ---: | ---: |
| Rename | 14.42M | 7.36M |
| Composer | 8.80M | 11.68M |
| Activity detection | 10.60M | 8.71M |
| Stable ordering | 7.85M | 24.79M |

Follow-ups addressed focus and IME behavior, incomplete composer mode transitions, sampler coverage, durable ordering,
retry handling, and storage correctness. Some were legitimate discoveries during review rather than preventable
mistakes. The run does not establish what Sol-first would have cost, but it demonstrates why initial model price is
an incomplete routing metric: implementation must be measured through repair and acceptance.

The independent reviews caught concrete defects, including a shifted storage column index, a lost persistence retry,
and blank lines in captured Codex input being mistaken for activity. Their 14.81M cached tokens were comparatively
small. Wording readers used 1.57M, under 0.5% of the target tree, and were not a major cost target.

## Compaction also consumed substantial uncached input

The tree contained 17 compactions, including ten in the parent. The summarization responses consumed 3.93M input
tokens, of which only about 32K were cached, plus about 53K output tokens. Their 3.90M uncached input represented about
a quarter of the target tree's uncached input. Parent compactions alone consumed about 2.25M uncached input.

Subsequent skill and state reloads are outside those numbers. The trace also contained repeated workflow reads and
bootstrap work. More frequent compaction is therefore not an obviously cheap remedy. Reducing unnecessary context
accumulation is the better first target.

## Recommended improvements at the time of the audit

1. Pass process-waiting rules directly to workers, explicitly covering their own builds and tests as well as delegates.
   Prefer verified completion delivery; otherwise use bounded waits appropriate to the expected duration, preserving
   deadlines and resource checks.
2. Combine bounded waiting and status collection in one tool execution where possible. Avoid separate model responses
   merely to request a sleep and then request status. Avoid adding another status loop around an existing watcher
   without a concrete monitoring requirement.
3. Use verified completion delivery or a small-context watcher for release monitoring. This run supplied a concrete
   case for the watcher exception: the main orchestrator need not repeatedly process its context to report unchanged
   build stages.
4. Reduce parent context growth and bookkeeping overhead through bounded output, consolidated independent reads, and
   compact durable state. Preserve required checkpoints and reviews; avoid repeatedly loading the same workflow
   instructions within a single context.
5. Trial Sol as the initial worker for broad migrations, persistence, and complicated UI lifecycle work. Compare total
   cost through acceptance. This audit does not justify replacing Terra everywhere.

The recommendation was to retain independent correctness reviews. They found real defects and were a relatively small
part of the usage. The strongest opportunities were fewer expensive parent turns and more complete initial
implementations, alongside the narrower polling fixes.

## Delivered result and evidence limits

The session reported all ten fixes completed, nine unmerged draft PRs including the inventory and release bump, and
[v0.9.0-rc.1](https://github.com/scode/farhelm/releases/tag/v0.9.0-rc.1) published with signed assets. Its final report
recorded 2,521 workspace tests, 375 desktop tests, 121 JavaScript tests, and 34 integrated browser cases. The full
browser suite remained a later merge gate. Those claims were read from the session, not independently revalidated by
this token audit.

The original parent rollout was
`~/.codex/sessions/2026/09/15/rollout-2026-09-15T18-18-48-01a0a7cb-8572-78a1-b642-7b7c5c4eaa8e.jsonl`.
Complete-record snapshots, descendant identities, per-response usage, command extracts, and the detailed audit were
saved locally under `/tmp/farhelm-token-audit-final/`; the initial sample was under
`/tmp/farhelm-token-audit.YKeajz/`. Those temporary artifacts may not survive. This entry preserves the findings without
depending on them. Original session logs and workspaces were not changed by either audit.

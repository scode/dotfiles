# Session evidence

Record enough to assess delegation policy later, not a transcript or a performance claim. The orchestrator owns these
records; delegates never write them. Do not change model choices to improve the numbers, run extra agents to collect
usage, or record every shell command. Short decision summaries should name observable constraints, not private reasoning
traces, task contents, credentials, or copied prompts.

## Location and lifetime

On first activation, resolve `${XDG_STATE_HOME:-$HOME/.local/state}/scode-galaxy-brain/sessions/` once. If
`XDG_STATE_HOME` is relative, use `$HOME/.local/state` instead. Generate a session UUID with `uuidgen` (available on the
supported Linux/macOS setup), then create `<sessions>/<uuid>/` exclusively, with mode 0700. If the UUID utility is
unavailable, use an available UUID generator rather than adding a dependency; never substitute a fixed name. A collision
means generate another UUID, not reuse the directory. Keep records private, including temporary files, and do not chmod
the user's existing state root. Do not follow a pre-existing session-directory symlink.

Create `session.json` and an `events/` directory. Write each event as a separate JSON object in a fresh
`events/<event-uuid>.json` file, using the agent's file-editing tools. This avoids concurrent appends and repeatedly
rewriting a growing log. Write complete files; readers reject malformed or incomplete JSON rather than silently counting
it. Never overwrite an event. The sole writer is this orchestrator, even when delegates run concurrently.

`session.json` contains `schema_version: 1`, `session_id`, `created_at` (UTC RFC 3339), `orchestrator_harness`,
`orchestrator_model`, `orchestrator_effort`, and `harness_session_id`. Unknown metadata is null, not an inferred model
name. Capture the native session identifier when exposed so future accounting can join it to the orchestration session.
Do not scan unrelated harness history to find it. Record routing-policy revision or version and effective user overrides
as a concise summary in the activation event when known; do not copy credential-bearing configuration files.

Keep the UUID and absolute directory path in session state and every handoff. Reuse them after compaction/resume and
across later tasks or reactivation in this same session; do not start a new record per task or per delegate. On resume,
verify the recorded `session_id`, read the manifest and this session's latest relevant events, and continue. Never pick
the newest directory under `sessions/` as a guess. If identity cannot be recovered from retained context or an explicit
harness-session mapping, start a fresh UUID and record `prior_session_unknown` as a gap; report the split to the user.

Retain records after completion, outside VCS and ordinary scratch cleanup. Delegation run directories keep their
existing lifecycle and run ids. Before cleanup would remove usage evidence, save only the necessary usage metadata in
this session directory and update its evidence reference in a new event. Do not retain whole transcripts merely for
accounting.

If creating or recording fails, report the failure and continue authorized work without pretending it was logged. Record
the gap when recording becomes possible again, with its known bounds; do not fabricate missing events or counts. Include
the directory and known gaps in the final report. No automatic upload, pruning, cost lookup, or stats job.

## Event contract

Every event has `schema_version: 1`, `event_id` (matching the filename UUID), `session_id`, `at` (UTC RFC 3339), `type`,
`unit_id`, `decision_id`, `attempt_id`, and `data`. IDs not applicable to that event are null. Generate a stable UUID
`unit_id` for each meaningful work unit considered; its retries and takeovers retain it. Each new decision and attempted
launch gets its own UUID. An attempt is not a checkpoint turn: continuations keep its ID. Link new attempts through
`previous_attempt_id`, and record the delegation skill's `run_id` separately once assigned. Do not invent run ids for
another skill's process-defined spawns.

Every decision's `data` also includes `supersedes_decision_id`: null unless it replaces a decision before launch. A
replacement names the immediately preceding, still-open decision in the same session and unit. It closes that decision
as superseded, not failed or unfinished, without changing the old event. Only an unlaunched decision without an outcome
can be superseded; launched attempts retain their outcomes and retry links through `previous_attempt_id`.

| Type           | When and required `data` fields                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `session`      | Activation, resume, pause, or end of a requested scope: `phase`, `policy_revision`, `overrides_summary`, `open_unit_ids`. Ending a task is not ending the session. An abrupt end may leave no closing event.                                                                                                                                                                                                                                                                                                                                                                              |
| `decision`     | Before acting on a choice: `task_label`, `origin` (`decomposition` or `process`), `process_skill`, `profile`, `writer`, `expected_size`, `large_input`, `visual_output`, `independence_required`, `provider_preference`, `explicit_demand`, `availability_constraints`, `action` (`delegate`, `local`, `blocked`), `route`, `requested_model`, `requested_effort`, `mechanism`, `reason`, `route_exhausted`, `endpoint_trusted`, `diverged_from_preference`, `cross_family`, `previous_attempt_id`, `trigger` (`initial`, `escalation`, `reroute`, `spec_fix`, `path_retry`, `takeover`). |
| `launch`       | After the launch returns: `status` (`started` or `failed`), `run_id`, `delegate_handle`, `reported_model`, `reported_effort`, `failure_reason`. A selected route is not proof that this model actually ran.                                                                                                                                                                                                                                                                                                                                                                               |
| `continuation` | Each checkpoint/crash resume or process follow-up: `delegate_handle`, `reason`, `status`. Keep the same attempt only when continuing the same delegate.                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `outcome`      | After judging a unit or attempt: `status` (`accepted`, `failed`, `blocked`, `cancelled`, `inconclusive`), `verdict`, `verdict_source` (`delegation_gate`, `process`, `local`), `reason`, `local_fixes`, `verification_summary`, `next_action`, `elapsed_ms`, `usage_status` (`reported`, `partial`, `unavailable`), `usage_reason`. Use the exact gate verdict where applicable; never impose it on another skill's process.                                                                                                                                                              |
| `usage`        | A reported measurement: fields below. Can arrive after an outcome.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `gap`          | Missing evidence or recording interruption: `reason`, `since`, `until`, `affected_ids`; unknown bounds are null.                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |

Use null for inapplicable or unavailable fields; use an empty array only when a list is known to be empty. `route` is
the routing answer's model or special answer, or `not_requested` when keeping tiny work local without asking routing.
Log those local decisions too, at the task-unit level rather than per keystroke: otherwise the record cannot show
whether delegation is too conservative. Reasons explain the choice briefly (for example review overhead, ambiguity,
explicit demand, large input, unavailable mechanism). Record actual constraints and choices, not hypothetical token
savings. If a route changes before launch, record a new decision with `supersedes_decision_id` naming the replaced
decision; do not edit the old one. A decision without a launch, outcome, or replacement link from a later decision is
unresolved, not failed. Each replacement remains open until it is launched, receives an outcome, or is itself
superseded; closing the old decision does not complete the unit. A takeover gets a local decision and eventual local
outcome on the same unit. Elapsed time is measured wall time for that unit/attempt, or null; it is not token usage or
active compute time.

## Usage without misleading totals

A `usage` event's data contains `actor` (`delegate` or `orchestrator`), `model`, `effort`, `source`, `source_record_id`,
`meter_id`, `scope` (`turn`, `attempt`, `harness_session`, `orchestration_session`, `subtree`), `counter_kind` (`delta`,
`cumulative`, `unknown`), `includes_children` (boolean or null), `raw_counters`, `counter_semantics`, and
`reported_cost` (null or an object with `amount`, `currency`, and `source`). `source` identifies the tool response or
retained metadata file and exact event/field. `source_record_id` identifies that measurement within the source; reuse it
when encountering the same measurement again, and do not emit it twice. `meter_id` identifies the underlying counter
stream across resumes, not a new meter per sample. Unknown token fields remain null, even on a failed launch.

Capture usage already exposed by tool results or the exact delegate transcript/session handle. Prefer structured
terminal usage records; a delegate's guessed token total is not evidence. Record source field names and values in
`raw_counters` and explain their documented scope in `counter_semantics`, including whether input includes cache hits
and output includes reasoning. Do not relabel an unknown counter as a billable input/output count. Keep requested model
IDs in decisions and reported model IDs in usage; aliases, defaults, and silent fallbacks are not proof of exact model
attribution. When a source reports a model breakdown, record separate model-specific meters. Otherwise leave model
attribution unknown.

Per-turn counters may be summed once per distinct source record. Cumulative counters require a baseline and final sample
for the same meter; never sum repeated totals across checkpoint resumes. A known fresh meter may have a zero baseline;
an attached session does not. Preserve partial usage from failures and cancellations. Do not add reasoning or
cached-token subsets to totals that already include them. If semantics are unknown, retain the raw evidence but exclude
it from totals. For nested coordinators, record whether a parent total includes children; parent and child totals are
alternatives, not additive. Unknown overlap or mixed-model attribution is a coverage gap, not a reason to count
everything.

Orchestrator usage is a separate actor and may remain unavailable. Do not allocate its whole-session counts to
individual local units, or assume they exclude delegates. Cost comparisons need pricing and coverage matched to the
source; record actual reported cost when available, not remembered prices or subscription-cost estimates. These records
enable later analysis; they do not establish that a different delegation policy would have been cheaper on the same
tasks.

For inspection, `jq -s '.' <session>/events/*.json` yields an array of events when files exist; order by `at` for
display and use IDs for relationships. Statistics must account for gaps, unresolved attempts, deduplication, counter
semantics, and unknowns rather than blindly summing every numeric field.

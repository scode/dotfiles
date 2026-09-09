# Execution evidence

The executor is the sole writer of the manifest and immutable events. The watchdog alone writes its separate mutable
heartbeat/status artifact as `watchdog.md` specifies. Record observable choices and outcomes, not private reasoning
traces or claims about hypothetical savings. This record is independent of Galaxy Brain and does not load it. Planning
metadata in the blueprint and executor metadata here allow later joining when harness identities are exposed.

## Private, durable state

On a fresh execution, resolve `${XDG_STATE_HOME:-$HOME/.local/state}/scode-build-blueprint/executions/`; a relative
`XDG_STATE_HOME` falls back to `$HOME/.local/state`. Generate a UUID with an available UUID generator. Create the UUID
directory exclusively with mode 0700, and verify its mode before writing evidence. A collision or pre-existing symlink
means choose another UUID, never adopt it. Do not change permissions on the user's existing state root. Files must be
private too (0600); use the agent's dedicated editing tools and check resulting permissions.

Create `execution.json`, `events/`, and `artifacts/` inside it. Record its UUID and absolute path in the working log.
The UUID identifies this execution of the blueprint across executor sessions and resumes, not one model call. Recover
only the exact recorded directory and verify its manifest on resume. If lost, create a new UUID and a gap event linked
to any known prior execution; never guess the most recent directory. Do not reset milestone budgets or forget existing
work because recording identity was lost. If those cannot be recovered, block affected launches.

`execution.json` has `schema_version: 1`, `execution_id`, `created_at` (UTC RFC 3339), `blueprint_path`,
`blueprint_hash`, `repo_path`, `baseline_revision`, `policy_revision`, and `planning_session_id`. Unknown fields are
null. Keep the initial manifest immutable; session events record later executors, blueprint changes, and resumes.

Shellout writes run-ID-named prompts, transcripts, stderr, and results to private scratch per its own rules. Before
launch, retain a private copy of the exact prompt and task inputs referenced by it under `artifacts/<run-id>/`; for
large versioned inputs retain revision/path references rather than a whole repository copy. Retain every follow-up
prompt before sending it. After each turn, retain result and review artifacts plus structured usage source records
before scratch cleanup, recording their durable paths. Preserve ephemeral task specifications rather than just their
paths. This permits reconstruction of explicit instructions, not a promise to reproduce the harness's entire hidden
context. Keep credentials out of prompts and never copy credential/configuration files into evidence. Sensitive task
content may still be present; keep records local and private.

Whole harness transcripts are not required for this initial recorder. Preserve the usage records with original field
names, source identity and event position, plus exact delegate prompts and deliverables. Retain an existing raw
transcript path when available and state whether it is temporary; never imply it will survive scratch cleanup.

Do not automatically upload, prune, price, or analyze records. If recording fails, report the gap immediately and in the
final summary; continue only otherwise-authorized work whose permission/budget state is still known. Restore recording
when possible, without inventing the missing interval. Telemetry failure is not permission to drop checkpoint gates.

## Events and identity

Write one complete immutable JSON object per `events/<event-uuid>.json`; never append to a shared file or overwrite an
event. Common fields are `schema_version: 1`, `event_id`, `execution_id`, `at` (UTC RFC 3339), `type`, `unit_id`,
`attempt_id`, and `data`. Nonapplicable IDs are null. A unit is a stable piece of work, including work kept local. A new
launch gets a new attempt/run UUID; resumptions of that same agent retain it and get distinct turn IDs. Retries link
`previous_attempt_id`; changing IDs never resets explicitly approved budgets. Budget fields are null when no cap
applies, not zero or a made-up allowance. Role, trigger, calls, follow-ups, outcomes, and measured usage are recorded
regardless of caps, so uncapped expert use remains auditable. Use these event types:

- `session`: phase (`start`, `resume`, `blocked`, `complete`), actual executor harness/model/effort and harness session
  ID when exposed, blueprint/policy hashes, outstanding units and budget consumption. Requested identity is separate
  from reported identity; unknown actual identity stays null.
- `decision`: decision UUID, milestone, role (`local`, `worker`, `consultation`, `review`), task label, action (`local`,
  `delegate`, `blocked`), short reason, trigger, profile, requested model/effort, routing answer, effective mechanism,
  allowlist entry or expert gate authorizing it, expected duration, hard deadline, budget ID and counting unit, budget
  before launch, and previous attempt ID. Log substantive local units too, not each command. Include
  `supersedes_decision_id`, null unless replacing an unlaunched decision on the same unit. Replacement closes only that
  decision, not the unit.
- `launch`: decision ID, status (`started`, `failed`), run ID, prompt artifact path, working tree identity, process
  handle, harness session ID, reported model/effort, start time, and failure reason if any.
- `continuation`: decision ID, turn ID, harness session ID, retained follow-up path, trigger, budget ID and charge in
  its declared unit, launch status, and start time. Record every consultation/review follow-up, not just first launches.
- `outcome`: decision/turn ID, status (`accepted`, `failed`, `blocked`, `cancelled`, `inconclusive`), artifact paths,
  judged-by identity/role, findings, executor repairs, verification evidence and tree identity, next action, elapsed
  wall milliseconds, budget ID and consumed amount in its declared unit, and usage coverage (`reported`, `partial`,
  `unavailable`) with reason. The outcome reports the charge already made, not an additional debit for that turn.
- `usage`: the source-backed measurement below. May arrive after an outcome; do not fabricate zero usage on failure.
- `watchdog`: phase (`start`, `alert`, `recovery`, `failure`, `restart`, `stop`), monitor handle/identity, status path,
  measured values and thresholds, affected paths, and executor response. A model watchdog also receives normal
  decision/launch/outcome/usage events with role and usage actor `watchdog`; a process monitor has no model token usage.
- `gap`: missing evidence, affected IDs, known time bounds (null if unknown), and reason.

Local work receives outcomes too. A decision without a launch/outcome/superseding decision is unresolved, not failed. An
attempt interrupted without a terminal record is likewise unresolved. Human-readable times aid inspection; IDs and
explicit links establish lineage. Log expert advice and how the executor acted on it, including rejected findings and
the expert's adjudication. Reviewer acceptance is distinct from executor integration and final goal completion.

Write decisions before acting and launch/continuation records as those operations happen, not as a batch at shutdown.
Generate IDs with the UUID generator and timestamps from the clock; do not invent sequential UUID-shaped IDs or rounded
historical times. When reconstructing a missed event, use the current recording time, label it retrospective, and cite
the source for any recovered occurrence time (unknown stays null). Preserve each failed launch and interrupted turn,
even when a later resume succeeds. Before the final session event, account for each decision and turn with an outcome,
superseding decision, or explicit unresolved gap; local work needs an outcome too. Check required fields and ID links
against this schema. Missing records mean partial coverage, not a clean execution history.

## Token accounting

Use the harness sidecar's structured-output mode on every launch and resume, even when no continuation is planned. For
Codex this includes `--json` alongside the final-message file. Retain stdout events and stderr separately. For other
harnesses read their verified usage source/procedure; if unavailable, record that gap instead of inventing flags or
switching to native agents. Shellout is selected for observability but does not guarantee model identity or counters.

Each usage event includes `actor` (`executor`, `worker`, `consultation`, `review`), reported model/effort or null,
`meter_id`, `source_record_id`, durable source path and exact event/field, `scope` (`turn`, `session`, `subtree`,
`unknown`), `counter_kind` (`delta`, `cumulative`, `unknown`), `includes_children` (boolean or null), `raw_counters`,
`counter_semantics`, and `reported_cost` (null or amount/currency/source). Preserve requested model aliases in
decisions; they are not proof of what ran. Retain usage from failed, cancelled, and resumed runs whenever exposed.

Never emit the same source measurement twice. Sum distinct per-turn deltas once; cumulative counters need a baseline and
final sample on the same meter across resumes, not addition of repeated totals. Only a known fresh meter has a zero
baseline. Do not add cached-input or reasoning subsets into totals that already include them. Unknown scope, overlapping
parent/child counts, or mixed-model attribution exclude a measurement from attributed totals until resolved. Unexpected
child activity is both a policy violation and an accounting gap, not permission to count overlapping meters.

Top-level executor and planning usage may be unavailable. Capture exposed counters without searching unrelated harness
history, asking models to estimate tokens, or allocating whole-session counts to individual units. Record measured
elapsed time separately from token counts. Record reported monetary costs only; no remembered prices or subscription
estimates. Later comparisons must include planning, executor, worker, consultation, review, and repair costs with
coverage gaps, rather than comparing only successful worker calls.

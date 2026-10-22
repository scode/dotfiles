# persistent-review specification

Dependencies: scode-harness-shellout

This file records the behavioral contract for the installed skill. Launch and transcript-access mechanics belong to the
declared dependency; review scope, evidence judgment, persistence, and completion belong here.

## Activation and inputs

- Explicit invocation and a request for persistent adversarial review discover this protocol. Discussion, assessment, or
  editing of the skill does not launch a reviewer. Loading creates no unrelated session-wide policy, and ordinary review
  requests do not silently become persistent.
- Reviewers follow the caller's charter, precise scope, model, effort, result destination, and relevant constraints.
  Session context may settle inputs; only unresolved choices that affect the task require clarification. The skill
  supplies no review lenses, panel, finding taxonomy, automatic-fix policy, or VCS range.
- Reviewed inputs stay read-only unless edits are separately authorized; requested results and private run artifacts may
  be written. Findings do not authorize fixes.
- Codex and Claude reviewers are supported. Other reviewer backends are reported unsupported without substitution.
  Installation remains shared across all seven caller harnesses.

## Stable scope and evidence

- All helper commands use the bundled POSIX `evidence` launcher. It prefers a matching compiled executable and permits
  Deno source fallback only when that executable is absent. Stale, missing-source, broken, or nonexecutable compiled
  deployments fail without running the command through source as a fallback. Compiled helpers embed their source hash
  and check the neighboring `evidence.ts` at startup. This is a freshness check, not a lock against later edits.

- Before launch the orchestrator establishes a mandatory inventory of stable text and ranges. Full files are required
  only when agreed. Discretionary exploration does not expand the inventory. Diff scope preserves its agreed old/new
  content. Changed inputs require retained originals or an explicit inventory reconciliation and affected-range audit.
- Run artifacts have unique session-private paths. Each reviewer owns a manifest, append-only findings log, progress
  file, session identity, transcript paths and coverage reports. Stable snapshots and original revision/range mappings
  remain available behind compact references.
- The bundled Deno helper verifies complete read packets in actual delivered tool results against pinned UTF-8 source
  text, including exact range content and source version. It validates reviewer session/cwd and call/result pairing. It
  emits compact coverage and evidence references without echoing source or reasoning. Codex's persisted delivered
  results and Claude's verbose stream are supported; raw Codex command stdout is never substituted.
- Codex top-level `compacted` records are recognized as context management after validating the known message/history
  envelope. Their summary and replacement history earn no delivery credit. Original call/result joins and verified
  delivery survive compaction; missing original evidence remains missing. Unknown compaction shapes still fail.
- Claude `system` records with subtype `hook_started`, `hook_progress`, or `hook_response` may precede initialization.
  They retain session and nested-stream checks but earn no delivery credit. Each log still requires its own matching
  `system/init` session and cwd before assistant, user, result, or other system records.
- Structurally valid read packets for paths outside the audited manifest are reported explicitly with claimed ranges and
  raw references. They are unverified context and earn no credit, do not load additional sources, and never expand
  mandatory scope. Malformed required fields still fail; missing required ranges remain missing.
- Audit comparison uses packet meaning: path, digest, inclusive range and exact decoded text. JSON key order, additive
  metadata and hexadecimal digest case do not change delivery evidence. Blank JSONL separators are ignored while
  physical line references are preserved. Whitespace-only required logs still fail as empty.
- The read producer retains its 200-line/24,000-byte budgets. Those defaults do not invalidate a larger complete
  delivered packet whose required source identity, bounds and text can be verified. Audit memory is still affected by
  the largest JSONL record and retained snapshots, independent of producer defaults.
- Unknown record/tool-result shapes, malformed logs, orphan results, mixed sessions, changed sources and mismatched
  packets fail visibly with a nonzero exit and no successful partial report. Valid clipped packets earn no coverage.
  Missing packets or ranges remain incomplete. Known failed reads are retried with smaller packets; unfinished logs may
  be retried after their writer reaches a boundary. Other format failures require a user decision before direct
  inspection, with an explicit warning about loading duplicate code and spending more tokens. No silent fallback.
- Attempted reads, process success, file lists, search hits, tool counts, checksums, reviewer assertions, or raw output
  larger than what the model received cannot establish delivery of required ranges. Relevant truncation metadata and
  omission markers are interpreted using verified harness semantics. Compact references retain the route to raw
  evidence. Matched packet text, not packet metadata alone, establishes delivery. The helper is not a general transcript
  framework and establishes neither review quality nor attention.

## Functional groups and findings

- The orchestrator divides required scope by coherent behavior, subsystem, feature path, or lifecycle. Required
  contracts, callers and tests stay with the relevant behavior. Group notes name purposes and cross-group interfaces;
  arbitrary file counts or directory order are not partitioning rules. Every mandatory range is assigned; context may
  overlap. Defaults cap groups at 2,000 source lines and 64 KiB including required context; larger behaviors split at
  meaningful sub-boundaries, with caller approval required to exceed the ceilings.
- Independent ready groups run concurrently by default, up to actual harness/resource capacity, with explicit queued
  states for the remainder. Caller concurrency restrictions prevail. Each group has its own resumable reviewer and
  private writable artifacts; shared input snapshots stay read-only. Reviewers cannot consume other groups on their own.
- Every initial and continuation prompt instructs immediate recording: once a substantive finding is ready, append it
  before further analysis or reads, never waiting for a checkpoint, batch end, turn end, final report, or compaction.
  Findings and progress artifacts are initialized before source reads. No finding quota is imposed. Recording a finding
  does not end the assigned pass; reviewers continue that pass instead of stopping at a count.
- Versioned finding events carry a stable local ID, open/withdrawn state, summary, original location and complete detail
  in the caller's format. Revisions/retractions append under the same ID and preserve history. Unresolved leads are
  saved as they arise and in boundary progress summaries, independently of immediate finding writes.
- The orchestrator polls finding logs and progress without reading transcripts. The helper counts latest states per
  reviewer log, preserving source ownership and distinguishing empty logs from missing/malformed ones. Polling during an
  unfinished append may be retried; counts are unavailable until parsing succeeds. Raw counts remain distinct from
  semantic deduplication by underlying defect/location. Original reviewer IDs and evidence survive deduplication.
- Group completion does not prove cross-group correctness. The orchestrator reconciles conflicts and sends concrete
  interface questions back to the relevant reviewers for synthesis. An incomplete group keeps the overall run
  incomplete. New cross-group evidence or findings reopen discovery after their synthesis; an earlier empty pass does
  not close later questions.

## Persistent execution

- Each group's completion requires an initial turn, later-turn synthesis after delivery, and a subsequent discovery
  search on the same session identity: at least three actual turns. Between turns the orchestrator receives a result,
  audits relevant evidence, and sends a concrete continuation. Multiple sweeps inside one turn do not satisfy this.
- Findings never excuse missing scope, and an empty first result still requires continuation. Missing or uncertain
  ranges receive named, targeted same-session repairs independent of finding count. Large scopes create more groups.
- Synthesis occurs later than the initial review and after required content delivery, revisiting interactions and
  unresolved assumptions under the charter. Final repair and synthesis may share a turn. The orchestrator audits that
  turn afterwards; remaining gaps require repair and updated synthesis subject to the stopping rules below.
- Native delegation is usable only when the parent can audit supported delivered-content transcript evidence and resume
  the same reviewer session. Otherwise an observable shellout path is considered even if native supports the requested
  model. Explicit mechanism choices are honored; unavailable evidence or continuation is reported rather than silently
  changing the mechanism, model, backend, effort, or promised standard.
- Before choosing external evidence, the dependency and relevant Codex or Claude harness file are read through the
  canonical loading contract. Harness files own launch and evidence-access mechanics. Local deviations require
  verification of the relevant delivery semantics, not assumptions based on a harness name or JSON availability.

## Productive discovery

- Verified coverage and synthesis always lead to a separate same-reviewer search turn, even when no finding or concrete
  unresolved lead exists. The reviewer reuses context, known findings, and durable artifacts, searches beyond earlier
  issues under the original charter, records findings immediately, and may legitimately return empty.
- The orchestrator verifies candidates against pinned source and applicable contracts and checks novelty across the
  whole run. It accounts for retained, duplicate, rejected, and unresolved candidates without erasing source records.
  New IDs, event counts, restatements, speculation, and scope drift do not establish discovery progress. No severity or
  lens-specific gate is imported; actionability follows the caller's charter.
- Missing premises or open substantive leads receive focused verification or same-reviewer clarification before the pass
  is judged. They count as neither verified progress nor a completed empty search. Stuck investigations stop the group
  incomplete with the uncertainty retained, not an endless sequence of new search passes.
- A verified new credible, independently actionable finding requires another search continuation on that same session.
  There is no default pass cap or elapsed-time limit. Searches across independent groups remain concurrent up to actual
  capacity. Prompts explicitly allow no findings and do not require a quota or automatic scope expansion.
- Discovery completes after a properly executed search explicitly reports no new findings, with a credible short work
  summary and no unresolved substantive leads. Bare empty replies, refusals, blocked tools, duplicate-only outputs,
  unproductive unsupported claims and drift cannot establish that condition. Stuck/repeating/drifting reviewers stop
  incomplete with retained findings and the reason, rather than receiving pressure to increase their count.
- Caller-imposed limits are honored; stopping before the final empty pass is disclosed as incomplete discovery. Group
  progress retains search-pass counts, new verified finding IDs and empty-pass/failure evidence. An empty pass
  establishes the operational stopping condition, not proof of exhaustive review.

## Stopping and reporting

- The protocol imposes no elapsed-time limit or required time-budget question. Overall and per-turn deadlines must not
  be invented, including to satisfy delegation or shellout defaults requiring a hard deadline. Monitoring and
  termination of failed or stuck runs still apply. Honor a time limit only when explicitly supplied by the caller.
- Stop incomplete when transcript access or same-session continuation becomes unusable, or two consecutive repair turns
  make no verified coverage progress. Progress means delivery of previously missing mandatory content, not new findings.
  A successful repair resets the counter. Do not replace the reviewer identity to bypass a failed run. Termination uses
  the selected mechanism's facilities. These conditions apply per group; other progressing groups continue.
- Coverage repair and discovery have separate progress criteria: newly delivered required text and verified new
  actionable findings, respectively. Neither criterion substitutes for the other. Empty searches or caller search limits
  never establish delivery of unread scope.
- Preserve findings on every stop. The result includes completion status and reason, stable scope, backend/model/effort,
  per-group session identities and actual turn counts, verified delivery references, exact missing or uncertain
  material, later-turn/cross-group synthesis status, raw and deduplicated finding counts, and original finding-log
  references. It also records discovery-pass counts, new verified findings per pass, and final empty-pass or incomplete
  stop evidence. Use an accompanying completion note when the caller's format requires it.
- Delivery proves neither attention nor understanding, correctness, or saturation. Hidden chain of thought is neither
  inspected nor required. Findings stay grounded in reviewed artifacts. No findings plus incomplete evidence is an
  incomplete review, never a clean completion.

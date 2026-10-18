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
  Findings and progress artifacts are initialized before source reads. No finding quota is imposed.
- Versioned finding events carry a stable local ID, open/withdrawn state, summary, original location and complete detail
  in the caller's format. Revisions/retractions append under the same ID and preserve history. Unresolved leads are
  saved as they arise and in boundary progress summaries, independently of immediate finding writes.
- The orchestrator polls finding logs and progress without reading transcripts. The helper counts latest states per
  reviewer log, preserving source ownership and distinguishing empty logs from missing/malformed ones. Polling during an
  unfinished append may be retried; counts are unavailable until parsing succeeds. Raw counts remain distinct from
  semantic deduplication by underlying defect/location. Original reviewer IDs and evidence survive deduplication.
- Group completion does not prove cross-group correctness. The orchestrator reconciles conflicts and sends concrete
  interface questions back to the relevant reviewers for synthesis. An incomplete group keeps the overall run
  incomplete.

## Persistent execution

- Each group's completion requires at least two actual reviewer turns with the same session identity. Between turns the
  orchestrator receives a result, audits relevant evidence, and sends a concrete continuation. Multiple sweeps requested
  within one turn do not satisfy this requirement.
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

## Stopping and reporting

- The protocol imposes no elapsed-time limit or required time-budget question. Overall and per-turn deadlines must not
  be invented, including to satisfy delegation or shellout defaults requiring a hard deadline. Monitoring and
  termination of failed or stuck runs still apply. Honor a time limit only when explicitly supplied by the caller.
- Stop incomplete when transcript access or same-session continuation becomes unusable, or two consecutive repair turns
  make no verified coverage progress. Progress means delivery of previously missing mandatory content, not new findings.
  A successful repair resets the counter. Do not replace the reviewer identity to bypass a failed run. Termination uses
  the selected mechanism's facilities. These conditions apply per group; other progressing groups continue.
- Optional searches after verified coverage and audited later-turn synthesis default to at most one additional turn for
  a concrete unresolved lead; callers may choose another finite allowance. Optional-search limits never declare unread
  mandatory scope complete or cap repairs independently of the no-progress rule.
- Preserve findings on every stop. The result includes completion status and reason, stable scope, backend/model/effort,
  per-group session identities and actual turn counts, verified delivery references, exact missing or uncertain
  material, later-turn/cross-group synthesis status, raw and deduplicated finding counts, and original finding-log
  references. Use an accompanying completion note when the caller's format requires it.
- Delivery proves neither attention nor understanding, correctness, or saturation. Hidden chain of thought is neither
  inspected nor required. Findings stay grounded in reviewed artifacts. No findings plus incomplete evidence is an
  incomplete review, never a clean completion.

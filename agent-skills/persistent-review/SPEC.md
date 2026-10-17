# persistent-review specification

Dependencies: scode-harness-shellout

This file records the behavioral contract for the installed skill. Launch and transcript-access mechanics belong to the
declared dependency; review scope, evidence judgment, persistence, and completion belong here.

## Activation and inputs

- Explicit invocation and a request for persistent adversarial review discover this protocol. Discussion, assessment, or
  editing of the skill does not launch a reviewer. Loading creates no unrelated session-wide policy, and ordinary review
  requests do not silently become persistent.
- One reviewer follows the caller's charter, precise scope, model, effort, result destination, and relevant constraints.
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
- Run artifacts have unique session-private paths and retain stable inputs or references, findings, reviewer identity,
  coverage notes, and inspectable raw evidence. These roles need no new schema or separate file each.
- The orchestrator inspects content delivered directly or in tool results, linked to the required source, range,
  version, and reviewer session. Missing, uncertain, and truncated ranges remain open. Its own truncated evidence reads
  must also be completed before they support an audit.
- Attempted reads, process success, file lists, search hits, tool counts, checksums, reviewer assertions, or raw output
  larger than what the model received cannot establish delivery of required ranges. Relevant truncation metadata and
  omission markers are interpreted using verified harness semantics. Compact references retain the route to evidence;
  they do not replace inspection. No deterministic coverage engine or generic transcript framework is required.

## Persistent execution

- Completion requires at least two actual reviewer turns with the same session identity. Between turns the orchestrator
  receives a result, audits relevant evidence, and sends a concrete continuation. Multiple sweeps requested within one
  turn do not satisfy this requirement.
- Findings never excuse missing scope, and an empty first result still requires continuation. Missing or uncertain
  ranges receive named, targeted same-session repairs independent of finding count. Large scopes may span batches.
- Synthesis occurs later than the initial review and after required content delivery, revisiting interactions and
  unresolved assumptions under the charter. Final repair and synthesis may share a turn. The orchestrator audits that
  turn afterwards; remaining gaps require repair and updated synthesis within finite limits.
- Native delegation is usable only when the parent can inspect the relevant delivered-content transcript evidence and
  resume the same reviewer session. Otherwise an observable shellout path is considered even if native supports the
  requested model. Explicit mechanism choices are honored; unavailable evidence or continuation is reported rather than
  silently changing the mechanism, model, backend, effort, or promised standard.
- Before choosing external evidence, the dependency and relevant Codex or Claude harness file are read through the
  canonical loading contract. Harness files own launch and evidence-access mechanics. Local deviations require
  verification of the relevant delivery semantics, not assumptions based on a harness name or JSON availability.

## Finite stopping and reporting

- Default to a 30-minute absolute deadline covering setup through reporting; a caller may choose another finite limit.
  Record and announce it before launch, reserve time for audit/reporting, and never reset it on continuation.
- Stop incomplete when the deadline expires, transcript access or same-session continuation becomes unusable, or two
  consecutive repair turns make no verified coverage progress. Progress means delivery of previously missing mandatory
  content, not new findings. A successful repair resets the counter. Do not replace the reviewer identity to bypass a
  failed run. Termination uses the selected mechanism's facilities.
- Optional searches after verified coverage and audited later-turn synthesis default to at most one additional turn for
  a concrete unresolved lead; callers may choose another finite allowance. Optional-search limits never declare unread
  mandatory scope complete or cap repairs independently of the deadline/no-progress rule.
- Preserve findings on every stop. The result includes completion status and reason, stable scope, backend/model/effort,
  session identity and actual turn count, verified delivery references, exact missing or uncertain material, and the
  status of later-turn synthesis. Use an accompanying completion note when the caller's format requires it.
- Delivery proves neither attention nor understanding, correctness, or saturation. Hidden chain of thought is neither
  inspected nor required. Findings stay grounded in reviewed artifacts. No findings plus incomplete evidence is an
  incomplete review, never a clean completion.

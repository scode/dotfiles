---
name: persistent-review
description: >
  Run concurrent persistent reviewers over functional groups with compact delivery audits and immediate findings logs. Use for
  explicit persistent-review invocation or a request for persistent adversarial review. Discussing, assessing, or
  editing the skill does not launch a reviewer; ordinary review requests do not activate this protocol.
---

# persistent-review

Run persistent reviewers under the caller's charter. The orchestrator divides the scope into functional groups, reviews
them concurrently, checks which required content reached each reviewer, repairs gaps in that reviewer's session, and
requires later-turn synthesis. Delivery evidence establishes neither attention nor understanding, correctness, or
saturation. Do not inspect or require hidden chain of thought.

Loading this skill authorizes no launch by itself and creates no session-wide policy. Apply it to the requested review
only. Codex and Claude are the supported reviewer backends; report other reviewer backends as unsupported. This does not
restrict which harness may orchestrate the review. Do not import lenses, panels, finding taxonomies, automatic fixes, or
VCS ranges from another review workflow.

## Set the review contract

Recover settled inputs from the caller's request and session context. Ask only for missing choices that affect the work:

- The review charter and relevant constraints.
- Exact required scope: files, ranges, diff sides, or other text, including mandatory context.
- Reviewer backend, model, and effort; preserve all three without substitution.
- Result destination and any caller-required reporting format.
- Any selected launch mechanism, caller-imposed constraints, or limit on optional further searching.

Keep reviewed inputs read-only unless the caller separately authorizes edits; writing the requested result and private
run artifacts is allowed. A finding is not permission to fix it. Give the reviewer a self-contained charter, scope,
constraints, result destination, and the persistence requirements below. Give each reviewer its own findings log and
progress file, with the immediate-recording instruction below copied into every initial and continuation prompt.

This protocol imposes no elapsed-time limit. Continue while the review makes progress under the charter and the stopping
rules below. Do not ask the caller for a time budget or invent an overall or per-turn deadline. The delegation and
shellout defaults requiring a hard deadline do not apply to this protocol; retain their monitoring and termination
mechanics for failed or stuck runs. Honor a time limit only when the caller explicitly supplies one.

## Stabilize the required input

Make a compact inventory of mandatory text with stable identities and range boundaries. Reuse immutable revisions and
caller artifacts where possible; otherwise retain the exact text in private scratch space. For a diff, preserve the
agreed old and new content, not a moving working-tree command. Full files are mandatory only when the agreed scope
requires them. Context the reviewer explores on its own does not silently expand the mandatory inventory.

Record the run's unique identity, absolute artifact paths, reviewer session handle, findings, and coverage notes in
session-private scratch space. Use the per-reviewer manifests and finding events described in `tooling.md`; ordinary
notes are sufficient for the other coordination state. Avoid fixed paths that concurrent reviews could overwrite. Keep
the raw evidence available behind compact references so the audit can be checked without copying whole transcripts into
the report.

Do not combine delivery evidence from different text versions. If the input changes, continue against the retained
version or explicitly reconcile the inventory and re-audit affected material; never silently claim the new version was
covered.

## Group by behavior and launch concurrently

The orchestrator chooses coherent functional groups: a subsystem, feature path, state transition, or lifecycle, with the
contracts, callers, and tests needed to understand it. Record each group's purpose, required ranges, and interfaces with
other groups. Do not distribute arbitrary file counts or consecutive directory listings. Every mandatory range must
belong to a group; necessary context may overlap.

Default to at most 2,000 mandatory source lines and 64 KiB of source text per group, whichever is reached first,
including required context. Split larger subsystems at meaningful behavior boundaries; use named phases within a large
file when necessary. Treat these as ceilings, not targets. If essential context cannot fit, explain the boundary issue
and obtain a caller-approved adjustment rather than silently assigning an enormous group.

Launch all independent ready groups concurrently by default, subject to actual harness slots and resource capacity.
Queue remaining groups and start them as capacity becomes available; record the capacity constraint instead of silently
serializing. Honor an explicitly requested concurrency restriction. Each group gets a separate resumable reviewer
identity, manifest, findings log, progress file, and transcript paths. Reviewers share stable inputs read-only and never
write another reviewer's artifacts. A large scope creates more bounded groups, not one enormous first pass.

The two-turn and coverage requirements apply independently to every group. Record queued, running, and audited group
states. After group reviews, deduplicate equivalent findings by underlying defect and location, retaining all original
reviewer IDs and evidence. Keep distinct defects separate. Reconcile conflicting claims and send concrete cross-group
interface questions to the relevant existing reviewers for synthesis; do not assume isolated group completion proves
integration correctness. Any incomplete group keeps the overall review incomplete.

## Record findings immediately

Tell every reviewer: **As soon as you have a substantive finding, append it to your findings log before any further
analysis or read. Do not buffer findings until a batch ends, a turn ends, a checkpoint, a final response, or anticipated
compaction.** Preserve the caller's finding format in the record's detail. Do not invent findings to show activity.

Before reading source, create an empty findings log and a progress file naming the assigned group and current state. Use
the event format and append command in `tooling.md`. Each finding has a stable reviewer-local ID, summary, location,
detail, and open/withdrawn state. Append revisions or withdrawals with that same ID and the reason; never erase the
original. Record unresolved leads in the progress file as they arise. At each group/turn boundary, save remaining leads,
interface assumptions, and the next action before returning. This boundary summary never delays a completed finding.

The orchestrator polls findings/progress artifacts while reviewers run and reports observed active findings separately
from the deduplicated total. Use `evidence.ts findings` for counts and pointers; inspect finding details directly from
the findings logs, not transcripts. An initialized empty log means zero findings so far, not a clean review. Missing
logs, stale progress, and unfinished writes require investigation; do not credit advertised counts without records.

## Establish observable delegation

Before choosing native delegation, establish that the parent can retrieve the child's actual delivered input and tool
results in a supported format, distinguish omissions, and continue the same session. A native tool's support for the
requested model does not establish those capabilities. A human-visible UI or parent-only activity log does not establish
them either.

Use native delegation only when those capabilities are available and verified. Otherwise use the requested backend's
observable shellout path, even if its model is available natively. Honor an explicitly selected mechanism; if it cannot
meet this contract, report the limitation instead of silently changing it. If neither an authorized native path nor a
usable shellout path meets the evidence and continuation requirements, report an incomplete review.

For shellout, load the dependency below. Before choosing an external evidence source, read its relevant harness file:
`harness/codex.md` for `codex exec`, or `harness/claude.md` for `claude -p`. Those files own launch, capture, transcript
access, resume, monitoring, and termination mechanics. Use their verified delivery semantics, not an assumed JSON shape
or the largest available output dump. When the local tools differ, verify the relevant semantics before crediting
evidence. Do not duplicate launch templates here. Read `tooling.md` and use the bundled `evidence.ts` for normal
coverage audits. It checks actual delivered read packets against pinned source text and returns ranges and evidence
references, without pulling code or unrelated transcript content into the orchestrator's context.

A tool failure is not an empty or successful audit. Report its diagnostic and preserve findings. Retry a known clipped
read with smaller packets, or an unfinished log after the writer reaches a turn boundary. For unsupported or changed log
formats, stop coverage credit and ask whether the user permits direct transcript inspection, explicitly warning that it
loads code into the orchestrator's context and uses more tokens. Do not silently fall back to bulk reads, guess schema
fields, or ignore unknown blocks. Without permission, retain partial results and report the affected groups incomplete.

<!-- dependency: scode-harness-shellout -->

> Load the exact skill `scode-harness-shellout` through the current harness's authorized skill mechanism. Use its skill
> loader or resource resolver; when it provides no dedicated loader, read the exact `SKILL.md` location supplied by its
> skill catalog or instructions. Known interfaces include the Skill tool on Claude Code, `skill` on OpenCode, and
> `read_skill` on Muse Code. On Oh My Pi (omp), use `read` at `skill://scode-harness-shellout` and
> `skill://scode-harness-shellout/<relative-path>` for sidecars. On Codex, if no skill location is supplied, read
> `${CODEX_HOME:-$HOME/.codex}/skills/scode-harness-shellout/SKILL.md` (the root verified for Codex 0.152). These are
> known interfaces, not a harness allowlist. Do not ask permission merely because the harness is unfamiliar or returns
> no filesystem base-directory metadata; actual tool permissions still apply. Confirm the name is
> `scode-harness-shellout` from the returned frontmatter, or from the loader's reported identity if frontmatter is not
> exposed. Missing or conflicting identity is a load failure. Read the skill in full and every sidecar the current step
> needs. Resolve sidecars through the harness's resolver for that same skill, or relative to its reported base directory
> or the directory containing its supplied `SKILL.md` path. No base directory is required unless needed to address a
> required resource. If output is truncated or elided, retrieve the omitted content through the tool's continuation,
> range reads, or full-output artifact tied to that same resource or result; do not act on incomplete instructions. If
> complete retrieval cannot be established, stop the affected operation. Stop and report `scode-harness-shellout` and
> the failing path, URI, or tool when the skill is unknown, access is denied, identity does not match, or required
> content cannot be resolved or fully read. Do not bypass a denial, guess paths or URI schemes, search other skill
> roots, substitute another copy or similar skill, or continue from memory.

<!-- /dependency -->

## Review, audit, continue

A completed group review requires at least two actual reviewer turns on its recorded session identity. The orchestrator
must receive a result, audit its delivery evidence, and send a concrete continuation. Asking for several sweeps inside
one prompt is still one turn. Neither a finding nor an empty first result skips the continuation.

1. Start each reviewer with only its bounded functional group, the overall charter, interface context, and immediate
   findings instructions. Require small source reads through `evidence.ts read` and return after the assigned group; do
   not let a reviewer consume queued groups on its own. Preserve its findings, progress, result, and session handle.
2. Audit delivered content against that group's pinned manifest after each turn using `evidence.ts audit`. Read the
   compact report: checked session, matched ranges, gaps, rejected packets, and transcript references. The tool must
   compare actual delivered text to the snapshot; receipts or a checksum alone are insufficient. Leave uncertain ranges
   open. Keep the raw evidence on disk and inspect payloads directly only under the explicit fallback approval above.
3. When gaps remain, resume that same reviewer with named gaps: exact source and ranges, the observed truncation or
   uncertainty, and a manageable next batch. Preserve existing findings and pursue unresolved substantive leads. Ask for
   smaller bounded reads when earlier output was clipped. Repair is required regardless of the number of findings. If no
   gaps remain after the first turn, continue directly to later-turn synthesis.
4. Require synthesis in a later turn than the initial review, after delivery of the required material. It revisits
   interactions across batches and unresolved assumptions under the original charter, preserving, correcting, or
   withdrawing earlier findings with reasons. The continuation may repair the final gaps and then synthesize in that
   same turn; no extra turn solely for coverage approval is required. This also applies when the initial findings were
   empty.
5. Audit again after that turn. If gaps remain, resume for targeted repair and updated synthesis within the stopping
   limits. A synthesis claiming complete coverage never overrides missing evidence.

An attempted read, successful exit, filename list, search hits, tool count, checksum, or reviewer receipt is not proof
of delivery. Raw stdout is insufficient when the reviewer received a shortened result. Inspect truncation metadata as
well as literal omission markers according to the harness's evidence semantics. Credit only the ranges actually
delivered; leave missing or ambiguous ranges open. Search snippets may satisfy an agreed snippet scope, but cannot stand
in for mandatory full ranges. Evidence pointers help locate the content; they do not replace the helper's payload
comparison or an approved manual audit.

The orchestrator judges the compact evidence and completion. The bundled tool verifies text delivery, not review
quality. Do not extend it into a general transcript adapter framework or treat a complete coverage report as a completed
review without findings persistence and later-turn synthesis.

## Stop and report

Stop incomplete on loss of usable transcript access or same-session continuation, or after two consecutive repair turns
add no verified delivery of previously missing required material. A successful repair resets that counter; a new finding
does not. Retain the existing findings and last verified coverage. Do not start a replacement reviewer identity and
describe it as the same persistent review. Apply this rule per group; keep other progressing groups running.

Coverage repair and optional searching for more findings have separate limits. Once delivery is verified and the
later-turn synthesis is audited, stop unless a concrete unresolved lead warrants further work. By default allow at most
one optional additional reviewer turn; use another finite allowance if the caller supplied one. That allowance never
caps mandatory coverage repair or turns unread scope into completed scope.

At the caller's result destination, preserve the findings in the requested format and include a compact completion
record (or an accompanying note if the format cannot hold it):

- Complete or incomplete, with the stopping reason and any remaining substantive uncertainty.
- Stable scope references, group assignments, reviewer backend/model/effort, session identities, and actual turn counts.
- Verified delivery with evidence references; exact missing or uncertain material on an incomplete run.
- Whether later-turn synthesis occurred and which evidence supports the final audit.
- Active and deduplicated findings counts, original findings-log references, and cross-group synthesis status.

“Complete” means this protocol's delivery, persistence, and synthesis requirements were met. It does not certify the
findings or prove attention, understanding, correctness, or saturation. An incomplete run with no findings is not a
clean review; say both that it returned no findings and why it remains incomplete.

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
- Any selected launch mechanism or explicit caller constraints on execution or further searching.

Keep reviewed inputs read-only unless the caller separately authorizes edits; writing the requested result and private
run artifacts is allowed. A finding is not permission to fix it. Give the reviewer a self-contained charter, scope,
constraints, result destination, and the persistence requirements below. Give each reviewer its own findings log and
progress file, with the immediate-recording instruction below copied into every initial and continuation prompt. Tell
each reviewer to finish its assigned pass after recording a finding; finding an issue or reaching a particular count is
not a reason to end the pass early.

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

The persistence, coverage, and discovery requirements apply independently to every group. Record queued, running, and
audited group states. After group reviews, deduplicate equivalent findings by underlying defect and location, retaining
all original reviewer IDs and evidence. Keep distinct defects separate. Reconcile conflicting claims and send concrete
cross-group interface questions to the relevant existing reviewers for synthesis; do not assume isolated group
completion proves integration correctness. Resume the discovery cycle after synthesis of new cross-group evidence or
findings; an earlier empty pass does not close questions introduced later. Any incomplete group keeps the overall review
incomplete.

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
from the deduplicated total. Use `evidence findings` for counts and pointers; inspect finding details directly from the
findings logs, not transcripts. An initialized empty log means zero findings so far, not a clean review. Missing logs,
stale progress, and unfinished writes require investigation; do not credit advertised counts without records.

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
evidence. Do not duplicate launch templates here. Read `tooling.md` and use its `evidence` launcher for normal coverage
audits and helper failure handling. The helper checks actual delivered read packets against pinned source text and
returns ranges and evidence references, without pulling code or unrelated transcript content into the orchestrator's
context.

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

A completed group review requires an initial turn, later-turn synthesis after required delivery, and a subsequent
discovery turn on the same recorded reviewer identity: at least three actual turns. Coverage repairs and productive
discovery continuations can require more. The orchestrator receives results, audits evidence, and sends concrete
continuations between turns. Several sweeps inside one prompt are still one turn. Neither a finding nor an empty first
result skips these requirements.

1. Start each reviewer with only its bounded functional group, the overall charter, interface context, and immediate
   findings instructions. Require small source reads through `evidence read` and return after the assigned group; do not
   let a reviewer consume queued groups on its own. Preserve its findings, progress, result, and session handle.
2. Audit delivered content against that group's pinned manifest after each turn using `evidence audit`. Read the compact
   report: checked session, matched ranges, gaps, rejected packets, and transcript references. The tool must compare
   actual delivered text to the snapshot; receipts or a checksum alone are insufficient. Leave uncertain ranges open.
   Keep the raw evidence on disk and inspect payloads directly only under the explicit fallback approval above.
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
6. Once delivery and later-turn synthesis are verified, run the discovery cycle below. Coverage alone never skips that
   search, including when all findings so far are empty.

An attempted read, successful exit, filename list, search hits, tool count, checksum, or reviewer receipt is not proof
of delivery. Raw stdout is insufficient when the reviewer received a shortened result. Inspect truncation metadata as
well as literal omission markers according to the harness's evidence semantics. Credit only the ranges actually
delivered; leave missing or ambiguous ranges open. Search snippets may satisfy an agreed snippet scope, but cannot stand
in for mandatory full ranges. Evidence pointers help locate the content; they do not replace the helper's payload
comparison or an approved manual audit.

The orchestrator judges the compact evidence and completion. The bundled tool verifies text delivery, not review
quality. Do not extend it into a general transcript adapter framework or treat a complete coverage report as a completed
review without findings persistence and later-turn synthesis.

## Continue productive discovery

After verified coverage and synthesis, require a separate search turn from each group's same reviewer. This is required
even without a concrete unresolved lead. Reuse its context and durable findings/progress artifacts. Give it the known
findings and relevant cross-group duplicates so it can distinguish a new defect from a restatement.

Use a continuation with this substance, plus the immediate-recording instruction:

> Your earlier findings are recorded. Reuse your existing context and review the assigned group again under the original
> charter, looking beyond the issues already found. Report only new, independently actionable findings; do not repeat or
> rephrase known findings, broaden the scope, or invent issues to keep producing. Record each completed finding
> immediately before further analysis or reads. An empty result is valid. At the end, briefly identify what you checked
> and any unresolved substantive leads; do not provide private reasoning.

After each search turn, inspect the findings log and the short work/lead summary. Verify candidates against the pinned
source and applicable contract, using focused source excerpts as needed. Check novelty against findings already known
across the run, not merely that reviewer's local IDs. Record each candidate as retained, duplicate, rejected with a
reason, or unresolved; preserve its original log and identity. New IDs, extra event records, restatements, speculative
claims, and findings outside the charter do not count as discovery progress. Do not impose a severity threshold from
another review workflow; actionability follows the caller's charter.

Resolve a candidate's missing premise or an open substantive lead with focused source checks or a targeted clarification
from the same reviewer before judging that pass. Do not count an unresolved claim as a verified finding or treat an
unfinished investigation as an empty completed search. If the investigation is stuck, retain it as unresolved and stop
the affected group incomplete rather than starting endless nominally new search passes.

If at least one genuinely new, credible, independently actionable finding is verified, save it and send another search
continuation to that same reviewer. Name the newly recorded findings and direct attention beyond them. Continue while
these passes produce verified new findings; there is no default pass cap or elapsed-time limit. Independent groups'
searches remain concurrent within actual capacity.

Finish discovery only after a properly executed search pass explicitly reports no new findings, with a credible work
summary and no unresolved substantive lead. A bare empty list, refusal, unreadable input, blocked tool, duplicate-only
reply, or drift is not that stopping condition. If a reviewer is stuck, only repeating findings, producing unsupported
claims without useful progress, or leaving its charter, stop the affected group incomplete and preserve the findings and
reason. Do not keep demanding a higher count. A caller-imposed limit may also stop a run; if it prevents the final empty
search pass, disclose that discovery ended before this condition was met.

Keep search-pass counts, newly verified finding IDs, and the final empty-pass or failure reason in the group's progress
notes. An observed empty pass establishes this stopping condition, not proof that every defect has been found.

## Stop and report

Stop incomplete on loss of usable transcript access or same-session continuation, or after two consecutive repair turns
add no verified delivery of previously missing required material. A successful repair resets that counter; a new finding
does not. Retain the existing findings and last verified coverage. Do not start a replacement reviewer identity and
describe it as the same persistent review. Apply this rule per group; keep other progressing groups running.

Coverage repair and finding discovery have separate progress criteria. Delivery of previously missing required text
advances coverage repair; verified new actionable findings advance discovery. Neither substitutes for the other. Do not
turn a search limit, an empty search, or productive finding count into evidence that unread scope was covered.

At the caller's result destination, preserve the findings in the requested format and include a compact completion
record (or an accompanying note if the format cannot hold it):

- Complete or incomplete, with the stopping reason and any remaining substantive uncertainty.
- Stable scope references, group assignments, reviewer backend/model/effort, session identities, and actual turn counts.
- Verified delivery with evidence references; exact missing or uncertain material on an incomplete run.
- Whether later-turn synthesis occurred and which evidence supports the final audit.
- Active and deduplicated findings counts, original findings-log references, and cross-group synthesis status.
- Discovery pass counts, newly verified findings per pass, and evidence of the final empty search or incomplete stop.

“Complete” means this protocol's delivery, persistence, synthesis, and final empty-search requirements were met. It does
not certify the findings or prove attention, understanding, correctness, or saturation. An incomplete run with no
findings is not a clean review; say both that it returned no findings and why it remains incomplete.

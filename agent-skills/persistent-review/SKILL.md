---
name: persistent-review
description: >
  Run one reviewer over multiple turns with an orchestrator audit of delivered-content transcript evidence. Use for
  explicit persistent-review invocation or a request for persistent adversarial review. Discussing, assessing, or
  editing the skill does not launch a reviewer; ordinary review requests do not activate this protocol.
---

# persistent-review

Run one reviewer persistently under the caller's charter. The orchestrator checks which required content reached that
reviewer, repairs gaps in the same session, and requires a later-turn synthesis. Delivery evidence establishes neither
attention nor understanding, correctness, or saturation. Do not inspect or require hidden chain of thought.

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
- Any selected launch mechanism, time limit, or limit on optional further searching.

Keep reviewed inputs read-only unless the caller separately authorizes edits; writing the requested result and private
run artifacts is allowed. A finding is not permission to fix it. Give the reviewer a self-contained charter, scope,
constraints, result destination, and the persistence requirements below. Require it to preserve substantive findings in
that destination across continuations, rather than leaving the real deliverable stranded in intermediate messages.

Before launch, record an absolute deadline. Default to 30 minutes from starting this review, including setup, repair,
audit, and reporting, unless the caller supplies another finite limit. Announce the chosen limit and leave time for the
final audit and report. Continuations do not reset it. Use the selected mechanism's deadline and termination facilities;
do not let a timed-out reviewer keep running unaccounted for.

## Stabilize the required input

Make a compact inventory of mandatory text with stable identities and range boundaries. Reuse immutable revisions and
caller artifacts where possible; otherwise retain the exact text in private scratch space. For a diff, preserve the
agreed old and new content, not a moving working-tree command. Full files are mandatory only when the agreed scope
requires them. Context the reviewer explores on its own does not silently expand the mandatory inventory.

Record the run's unique identity, absolute artifact paths, reviewer session handle, findings, and coverage notes in
session-private scratch space. These roles can share ordinary files; there is no required schema or separate file per
role. Avoid fixed paths that concurrent reviews could overwrite. Keep the raw evidence available behind compact
references so the audit can be checked without copying whole transcripts into the report.

Do not combine delivery evidence from different text versions. If the input changes, continue against the retained
version or explicitly reconcile the inventory and re-audit affected material; never silently claim the new version was
covered. Large inventories may be batched across turns, with the overall charter and interactions kept in scope.

## Establish observable delegation

Before choosing native delegation, establish that the parent can retrieve the child's actual delivered input and tool
results, distinguish omissions, and continue the same session. A native tool's support for the requested model does not
establish those capabilities. A human-visible UI or parent-only activity log does not establish them either.

Use native delegation only when those capabilities are available and verified. Otherwise use the requested backend's
observable shellout path, even if its model is available natively. Honor an explicitly selected mechanism; if it cannot
meet this contract, report the limitation instead of silently changing it. If neither an authorized native path nor a
usable shellout path meets the evidence and continuation requirements, report an incomplete review.

For shellout, load the dependency below. Before choosing an external evidence source, read its relevant harness file:
`harness/codex.md` for `codex exec`, or `harness/claude.md` for `claude -p`. Those files own launch, capture, transcript
access, resume, monitoring, and termination mechanics. Use their verified delivery semantics, not an assumed JSON shape
or the largest available output dump. When the local tools differ, verify the relevant semantics before crediting
evidence. Do not duplicate launch templates here.

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

A completed review requires at least two actual reviewer turns on one recorded session identity. The orchestrator must
receive a result, inspect its relevant transcript evidence, and send a concrete continuation. Asking for several sweeps
inside one prompt is still one turn. Neither a finding nor an empty first result skips the continuation.

1. Start the reviewer with the stable scope or an explicit first batch. Require findings and unresolved substantive
   leads, plus useful pointers to reads, without treating its claimed coverage as proof. Preserve the returned result
   and session handle.
2. Audit delivered content against the mandatory inventory after each turn. Inspect the actual text and its connection
   to the required source/version/range and this reviewer session. Directly delivered prompt content can count too when
   that relationship is established. Note verified ranges, missing or uncertain ranges, and compact transcript/event
   references. Retrieve omitted portions of the orchestrator's own evidence reads before relying on them.
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
in for mandatory full ranges. Evidence pointers help locate the content; they do not replace inspection.

The orchestrator makes this judgment. Do not build a deterministic coverage engine or transcript adapter framework to
replace it. Ordinary range reads and compact notes are enough unless an observed limitation requires reassessment.

## Stop and report

Stop incomplete at the deadline, loss of usable transcript access or same-session continuation, or after two consecutive
repair turns add no verified delivery of previously missing required material. A successful repair resets that counter;
a new finding does not. Retain the existing findings and last verified coverage. Do not start a replacement reviewer
identity and describe it as the same persistent review.

Coverage repair and optional searching for more findings have separate limits. Once delivery is verified and the
later-turn synthesis is audited, stop unless a concrete unresolved lead warrants further work. By default allow at most
one optional additional reviewer turn; use another finite allowance if the caller supplied one. That allowance never
caps mandatory coverage repair or turns unread scope into completed scope. All work remains inside the deadline.

At the caller's result destination, preserve the findings in the requested format and include a compact completion
record (or an accompanying note if the format cannot hold it):

- Complete or incomplete, with the stopping reason and any remaining substantive uncertainty.
- Stable scope references, reviewer backend/model/effort, session identity, and actual turn count.
- Verified delivery with evidence references; exact missing or uncertain material on an incomplete run.
- Whether later-turn synthesis occurred and which evidence supports the final audit.

“Complete” means this protocol's delivery, persistence, and synthesis requirements were met. It does not certify the
findings or prove attention, understanding, correctness, or saturation. An incomplete run with no findings is not a
clean review; say both that it returned no findings and why it remains incomplete.

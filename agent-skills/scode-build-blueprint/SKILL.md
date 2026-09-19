---
name: scode-build-blueprint
description: >
  Build a repository-grounded implementation blueprint for a separate workhorse session, or execute an existing
  blueprint that explicitly requires this skill. The planner designs; the executor implements with bounded workers
  and state-of-the-art consultations and reviews. Discussing or editing the skill does not start either phase.
---

# scode-build-blueprint

This is an experimental alternative to scode-build-goal, not a mode of Galaxy Brain. The user's current session does the
exploration and design, writes a blueprint, and stops. The user opens a separate session on the agreed workhorse model
and runs `/goal <absolute-blueprint-path>`. That session owns implementation, tests, recovery, and authorized VCS
operations. Experts supply design judgments and code review, not production implementation.

Planning must resolve the implementation design deeply enough for that workhorse: concrete interfaces, algorithms, state
transitions, compatibility, worked cases, and test assertions for non-trivial units. An architecture and file list alone
are not a finished blueprint. The design-completeness check in `planning.md` governs the handoff.

The blueprint needs no planning-conversation context, but requires these installed skills and their referenced files:
this skill, `agent-resumeable`, `scode-model-routing`, `scode-harness-shellout`, and the agreed VCS workflow. Do not
load or activate `scode-galaxy-brain` or `scode-build-goal`. This skill owns delegation contracts and gates; it does not
use `scode-agent-delegation`, whose caller-as-expert gate is a different policy.

## Choose the phase

- `$scode-build-blueprint <goal>` starts planning: read `planning.md` next to this file in full.
- A blueprint passed to `/goal` explicitly selects execution: read `execution.md`, `watchdog.md`, and `records.md` next
  to this file in full. Do not restart the planning interview or write another blueprint. Resume this phase after
  compaction.
- `help` prints these two steps and the installed-skill dependencies without creating files. Missing goal text means ask
  for the goal, not invent one.

The top-level harness and model are chosen by the user, not changed by this skill or by `/goal`.

## Shared dependency loading

Load each dependency by its exact name using the capability-based loading stanza below. It accepts the harness's
authorized loader, resource resolver, or catalog-supplied file path without requiring filesystem metadata or a tested
harness name. Complete any partial reads before acting; stop the affected operation if identity or required content
cannot be established. Do not substitute a remembered launch command or a similar skill. Read `scode-model-routing`
before selecting routes and `scode-harness-shellout` plus the selected harness sidecar before launching or resuming a
delegate.

All model delegates launched during either phase use shellout, including same-model and same-harness delegates. This is
a deliberate prototype constraint for usage evidence, not a claim that shellout is cheaper or always reports tokens. It
covers other active skills' delegates too. If another mandatory process cannot run this way, report the conflict; do not
silently use native agents, omit its review, or replace its charter. Ordinary background commands are not model
delegates and need no agent just to wait for completion.

## Dependency declarations

Apply each loader at the phase and operation specified above, not all at planning entry.

<!-- dependency: agent-resumeable -->

> Load the exact skill `agent-resumeable` through the current harness's authorized skill mechanism. Use its skill loader
> or resource resolver; when it provides no dedicated loader, read the exact `SKILL.md` location supplied by its skill
> catalog or instructions. Known interfaces include the Skill tool on Claude Code, `skill` on OpenCode, and `read_skill`
> on Muse Code. On Oh My Pi (omp), use `read` at `skill://agent-resumeable` and
> `skill://agent-resumeable/<relative-path>` for sidecars. On Codex, if no skill location is supplied, read
> `${CODEX_HOME:-$HOME/.codex}/skills/agent-resumeable/SKILL.md` (the root verified for Codex 0.152). These are known
> interfaces, not a harness allowlist. Do not ask permission merely because the harness is unfamiliar or returns no
> filesystem base-directory metadata; actual tool permissions still apply. Confirm the name is `agent-resumeable` from
> the returned frontmatter, or from the loader's reported identity if frontmatter is not exposed. Missing or conflicting
> identity is a load failure. Read the skill in full and every sidecar the current step needs. Resolve sidecars through
> the harness's resolver for that same skill, or relative to its reported base directory or the directory containing its
> supplied `SKILL.md` path. No base directory is required unless needed to address a required resource. If output is
> truncated or elided, retrieve the omitted content through the tool's continuation, range reads, or full-output
> artifact tied to that same resource or result; do not act on incomplete instructions. If complete retrieval cannot be
> established, stop the affected operation. Stop and report `agent-resumeable` and the failing path, URI, or tool when
> the skill is unknown, access is denied, identity does not match, or required content cannot be resolved or fully read.
> Do not bypass a denial, guess paths or URI schemes, search other skill roots, substitute another copy or similar
> skill, or continue from memory.

<!-- /dependency -->

<!-- dependency: scode-model-routing -->

> Load the exact skill `scode-model-routing` through the current harness's authorized skill mechanism. Use its skill
> loader or resource resolver; when it provides no dedicated loader, read the exact `SKILL.md` location supplied by its
> skill catalog or instructions. Known interfaces include the Skill tool on Claude Code, `skill` on OpenCode, and
> `read_skill` on Muse Code. On Oh My Pi (omp), use `read` at `skill://scode-model-routing` and
> `skill://scode-model-routing/<relative-path>` for sidecars. On Codex, if no skill location is supplied, read
> `${CODEX_HOME:-$HOME/.codex}/skills/scode-model-routing/SKILL.md` (the root verified for Codex 0.152). These are known
> interfaces, not a harness allowlist. Do not ask permission merely because the harness is unfamiliar or returns no
> filesystem base-directory metadata; actual tool permissions still apply. Confirm the name is `scode-model-routing`
> from the returned frontmatter, or from the loader's reported identity if frontmatter is not exposed. Missing or
> conflicting identity is a load failure. Read the skill in full and every sidecar the current step needs. Resolve
> sidecars through the harness's resolver for that same skill, or relative to its reported base directory or the
> directory containing its supplied `SKILL.md` path. No base directory is required unless needed to address a required
> resource. If output is truncated or elided, retrieve the omitted content through the tool's continuation, range reads,
> or full-output artifact tied to that same resource or result; do not act on incomplete instructions. If complete
> retrieval cannot be established, stop the affected operation. Stop and report `scode-model-routing` and the failing
> path, URI, or tool when the skill is unknown, access is denied, identity does not match, or required content cannot be
> resolved or fully read. Do not bypass a denial, guess paths or URI schemes, search other skill roots, substitute
> another copy or similar skill, or continue from memory.

<!-- /dependency -->

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

`SPEC.md` is the maintenance contract. `EVALS.md` contains optional behavioral scenarios; maintain them when behavior
changes, but do not automatically run model evals on each use or edit.

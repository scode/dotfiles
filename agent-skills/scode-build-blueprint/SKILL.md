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

The top-level model is chosen by the user, not changed by this skill or by `/goal`. In this initial version, execution
supports Codex and Claude Code: shared routing currently treats Muse and OpenCode as delegate-only harnesses. They may
still host allowed workers. Explain this limit when choosing the executor; do not claim unsupported execution works.

## Shared dependency loading

Load dependencies by their exact names through the harness's skill loader, then resolve referenced files relative to the
base directory it reports. On Codex, read `${CODEX_HOME:-$HOME/.codex}/skills/<name>/SKILL.md` in full instead. If a
required skill or reference cannot be read, name the missing path or tool and stop the affected operation. Do not
substitute a remembered launch command or a similar skill. Read `scode-model-routing` before selecting routes and
`scode-harness-shellout` plus the selected harness sidecar before launching or resuming a delegate.

All model delegates launched during either phase use shellout, including same-model and same-harness delegates. This is
a deliberate prototype constraint for usage evidence, not a claim that shellout is cheaper or always reports tokens. It
covers other active skills' delegates too. If another mandatory process cannot run this way, report the conflict; do not
silently use native agents, omit its review, or replace its charter. Ordinary background commands are not model
delegates and need no agent just to wait for completion.

## Dependency declarations

Apply each loader at the phase and operation specified above, not all at planning entry.

<!-- dependency: agent-resumeable -->

> Load the skill `agent-resumeable` through your harness's skill mechanism: the Skill tool on Claude Code, the `skill`
> tool on OpenCode, the `read_skill` tool on Muse Code. On Codex, which has no such tool, read
> `${CODEX_HOME:-$HOME/.codex}/skills/agent-resumeable/SKILL.md`; if it is absent or unreadable, report that exact path
> and do not search elsewhere. On any other harness, use its skill loader only if the result reports the skill's base
> directory; otherwise stop and say this skill has not been verified on that harness. The base directory is the
> directory containing the loaded `SKILL.md`. Confirm the name the loader reports is `agent-resumeable`; if the loader
> shows no name, read only the frontmatter (the first lines up to the closing `---`) of `<base>/SKILL.md`. Read its
> sidecars relative to the base directory. Stop and tell the user that `agent-resumeable` is not installed or could not
> be loaded, naming the path or tool, if the loader reports the skill as unknown or denied, the file is absent or
> unreadable on Codex (the skills root for Codex 0.152), the result says it was truncated, the name does not match, or a
> sidecar this step needs is not readable under the base directory. Do not continue from memory, from a copy, from a
> search for the file elsewhere, or from a similar skill.

<!-- /dependency -->

<!-- dependency: scode-model-routing -->

> Load the skill `scode-model-routing` through your harness's skill mechanism: the Skill tool on Claude Code, the
> `skill` tool on OpenCode, the `read_skill` tool on Muse Code. On Codex, which has no such tool, read
> `${CODEX_HOME:-$HOME/.codex}/skills/scode-model-routing/SKILL.md`; if it is absent or unreadable, report that exact
> path and do not search elsewhere. On any other harness, use its skill loader only if the result reports the skill's
> base directory; otherwise stop and say this skill has not been verified on that harness. The base directory is the
> directory containing the loaded `SKILL.md`. Confirm the name the loader reports is `scode-model-routing`; if the
> loader shows no name, read only the frontmatter (the first lines up to the closing `---`) of `<base>/SKILL.md`. Read
> its sidecars relative to the base directory. Stop and tell the user that `scode-model-routing` is not installed or
> could not be loaded, naming the path or tool, if the loader reports the skill as unknown or denied, the file is absent
> or unreadable on Codex (the skills root for Codex 0.152), the result says it was truncated, the name does not match,
> or a sidecar this step needs is not readable under the base directory. Do not continue from memory, from a copy, from
> a search for the file elsewhere, or from a similar skill.

<!-- /dependency -->

<!-- dependency: scode-harness-shellout -->

> Load the skill `scode-harness-shellout` through your harness's skill mechanism: the Skill tool on Claude Code, the
> `skill` tool on OpenCode, the `read_skill` tool on Muse Code. On Codex, which has no such tool, read
> `${CODEX_HOME:-$HOME/.codex}/skills/scode-harness-shellout/SKILL.md`; if it is absent or unreadable, report that exact
> path and do not search elsewhere. On any other harness, use its skill loader only if the result reports the skill's
> base directory; otherwise stop and say this skill has not been verified on that harness. The base directory is the
> directory containing the loaded `SKILL.md`. Confirm the name the loader reports is `scode-harness-shellout`; if the
> loader shows no name, read only the frontmatter (the first lines up to the closing `---`) of `<base>/SKILL.md`. Read
> its sidecars relative to the base directory. Stop and tell the user that `scode-harness-shellout` is not installed or
> could not be loaded, naming the path or tool, if the loader reports the skill as unknown or denied, the file is absent
> or unreadable on Codex (the skills root for Codex 0.152), the result says it was truncated, the name does not match,
> or a sidecar this step needs is not readable under the base directory. Do not continue from memory, from a copy, from
> a search for the file elsewhere, or from a similar skill.

<!-- /dependency -->

`SPEC.md` is the maintenance contract. `EVALS.md` contains optional behavioral scenarios; maintain them when behavior
changes, but do not automatically run model evals on each use or edit.

---
name: scode-galaxy-brain
description: >
  Accomplish a goal by delegating suitable parts of the work to cost-effective models while the current session stays
  in charge of planning, quality gating, and all commit/PR management. Use when the user explicitly invokes
  scode-galaxy-brain, e.g. "Use scode-galaxy-brain for a goal", optionally with a prefer-gpt, prefer-claude,
  prefer-muse, or prefer-glm keyword and/or a request to work with concurrency.
  Also use when the user says "galaxy brain feedback: ..." to record feedback about how this skill performed. Once
  invoked, the skill stays active for the rest of the session — including across context compaction and resume — until
  the user expressly stops it, unless the invocation itself limited the scope up front; if retained context says it was
  active, re-read this skill before delegating.
---

# Scode Galaxy Brain

## Premise

You — the current session — are running on a state of the art, expensive model. The point of this skill is to spend that
capability where it matters (planning, judgment, design, quality control) and route suitable work to models likely to
finish it at lower total cost without significantly compromising quality. You stay in charge the whole time: you
decompose the goal, you decide what to delegate, you judge every result, and you own the overall change.

The goal is cost-effective quality; parallelize when it helps. The one hard limit is write concurrency: writers that
share a working tree run one at a time, and concurrent writers are allowed only when each one is genuinely isolated from
the others, with you integrating the results serially (see Concurrency below).

Delegation is for steps toward a change, never for managing the change itself. You always own version control: commits,
branches, pushes, PR creation and updates. Delegates must not commit, branch, push, or open PRs — your session carries
the user's VCS workflow preferences and skills, and those do not transfer to a sub agent.

## What to read, and when

This file is the workflow: how to decide whether to delegate, what to delegate, how to stay in charge, and what to do
with what comes back. Two other skills carry the parts other skills also need — which model a unit of work should run
on, and how to launch a delegate in a foreign harness — and this file loads each by name at the moment it becomes
relevant. The procedure files next to this one are read the same way. "Next to this one" means the same directory this
SKILL.md was loaded from, whatever path that is in the current harness.

| read            | in full, the first time in a session that you...                                          |
| --------------- | ----------------------------------------------------------------------------------------- |
| `delegating.md` | decide to delegate anything at all, native or shelled out                                 |
| `feedback.md`   | hear "galaxy brain feedback: ..." or otherwise clear feedback on how this skill performed |

The first time in a session that you need to choose a model, effort, or launch mechanism for a unit of work, load the
skill `scode-model-routing` as follows and read its `SKILL.md` in full. Every such choice afterwards is a request to it
(see Routing below); this file never picks a model on its own.

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

The first time in a session that you decide to shell out to any external harness (`codex exec`, `claude -p`,
`muse exec`, `opencode run`), load the skill `scode-harness-shellout` as follows, read its `SKILL.md` in full, and then
read its file for the mechanism you are about to launch. That skill is the only place launch commands live, and that is
deliberate: a launch built from memory of a previous session, or improvised from a harness's `--help`, skips the
observed-behavior notes that exist because each one cost someone a hung or silently wrong run. Read the file, then
launch.

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

## Composing with other skills

Galaxy-brain is an orchestration layer, not a workflow. When another skill or instruction defines its own process —
roles, steps, what counts as a valid run — that skill stays authoritative for the process. Galaxy-brain only decides
which model and effort executes each unit of work (by asking routing), how the delegation seam works, and how delegated
output gets gated.

Keep the roles separate: do not merge this skill's orchestrator role with another skill's coordinator role, and do not
attribute one skill's constraints to the other. If another skill forbids or requires something, that rule comes from
that skill; reason about it (and explain it to the user) on that skill's terms.

Routing authority covers every delegation seam the current session owns. When another active skill asks this session to
spawn reviewers or workers, each spawn is still routed by galaxy-brain: ask routing for the model and effort as a
process-defined spawn, carrying any model, agent type, or effort that skill explicitly demands, set model and effort
explicitly where the spawn mechanism supports it, and announce the choice as usual. The trap is following the other
skill's spawn instructions verbatim and letting its subagents silently inherit this session's expensive model.
Inheriting is fine only as a deliberate routing decision, stated as such — which is what routing's `inherit` answer is.
When the other skill demanded the choice, the announcement attributes it to that skill.

A delegated unit that is itself a coordinator is one delegation seam, not one seam per subagent it creates internally.
When its harness has a native delegation tool, launch the coordinator with the harness's full tool set and let it run
its own fan-out. Do not flatten that fan-out into separate foreign-harness processes merely to retain model routing or a
prompt-level read-only boundary at the outer session. For OpenCode specifically, `task` stays enabled and a
coordinator-shaped GLM delegation remains one fully equipped `opencode run`; its `task` calls are internal execution,
not new Galaxy Brain shell-outs. The outer route still chooses and announces the coordinator's model and effort. A
process skill that requires exact models for its internal roles must be included in the coordinator's task; otherwise
its native agent configuration owns those internal choices. The coordinator task also carries the native-task manifest
and monitoring contract from `scode-harness-shellout`'s `harness/opencode.md`. OpenCode does not expose nested task
launches in the outer JSON stream, so the outer session must not infer fan-out shape or progress from the number or
timing of visible `task` events.

## Staying active for the whole session

Activation is session-scoped, not turn- or task-scoped. Once the user invokes this skill, keep routing every delegation
through it — including spawns triggered by other skills, and including later tasks the user never mentions the skill on
— for the rest of the session. Only two things end it: the user expressly asking to stop, or an invocation that limited
the scope up front ("use scode-galaxy-brain for <this one thing>"). Finishing the task it was invoked for does not.
Context compaction, session resume, a tool restart, or a summary that fails to mention the skill does not end it either;
treat retained context that has gone quiet about galaxy-brain as a summarization artifact, not a decision anyone made.

After compaction or resume, if the retained context says or implies this skill was active, re-read this SKILL.md, every
sidecar file you had loaded (at minimum `delegating.md` if any delegation happened), `scode-model-routing` if any
routing decision was made, and `scode-harness-shellout` and its file for any harness in use — each dependency loaded
again as What to read describes — before doing further substantive work; the routing rules and the launch details do not
survive summarization reliably. If the retained context is ambiguous but mentions outstanding delegated work, model or
effort routing, or galaxy-brain at all, assume the skill is still active and say that you are assuming it.

When you write a handoff or pre-compaction note while this skill is active, include the routing-layer state: the current
goal, any provider preference, what the model routing config file ruled in or out, which sidecar files and dependency
skills were loaded, delegations still in flight, and the next routing decision. Record every run id you have generated
and not yet moved to scratch, with the tree each run directory is in; for each delegate stopped at a checkpoint, also
record which checkpoint it is at and the session or thread id needed to resume it — a stopped delegate that the summary
forgets is one that gets relaunched from scratch, and a run id the summary forgets is a directory you can no longer
prove is yours. Do this even when no delegate is currently running — between delegations is exactly when a summary is
most likely to drop the skill. This is a backstop, not the mechanism: stickiness applies whether or not a handoff was
ever written.

## Routing

Every choice of model, reasoning effort, and launch mechanism is a request to `scode-model-routing`, loaded as What to
read describes. Its `SKILL.md` defines the work profiles, the request, the answer, and the rules that turn one into the
other; nothing in this file ranks models. What this file adds is which facts galaxy-brain puts in a request and what it
does with the answer.

Name the profile yourself, per routing's "How to name a profile", and supply what routing asks for: this harness and its
model; whether the unit edits a tree; its expected size and whether its input is large; whether the output is visual;
the spawn origin (your own decomposition, or a role another skill's process defines — see Composing); any explicit
demand from the user or that process; whether an independent cross-family perspective is part of the goal; the provider
preference from the invocation (below); whether foreign-harness permission bypass and metered billing are acceptable,
which they are unless the user has said otherwise; whether the native mechanism can resume a writer (it can on Claude
Code and Codex); and the current route and its outcome (`none` on a first attempt; after a failure, the model that
failed and one of `substantive failure`, `substantive failure (lost context)` for a delegate that lost track of earlier
context mid-task, `misclassified` when the output showed the task was more demanding than its profile, or
`execution-path failure` for a hang or launch failure, marking the mechanism unavailable when the path cannot be fixed;
and, under a provider preference, whether the preferred family has already produced poor output this session, which
routing cannot remember for you). Routing reads the model routing config file and checks which CLIs and credentials
exist itself.

Act on the answer as follows. `orchestrator`: the unit is not delegated; do it yourself. `inherit`: spawn at this
session's own model on the caller's mechanism. `no suitable route`: nothing reachable fits; do it yourself or tell the
user. A model with mechanism `native`: your sub agent mechanism (the Agent/Task tool in Claude Code, or your harness's
equivalent) with the model and, where the mechanism has an effort parameter, the effort set. A model with any other
mechanism: a shell-out, per `scode-harness-shellout`. An agent type the user or another skill's process demanded is not
in the answer; apply it at the spawn exactly as demanded. The `route exhausted` and `endpoint trusted` facts feed the
escalation rules below, and the reason and the divergence flags feed the announcement.

The invocation may include the keyword `prefer-gpt`, `prefer-claude`, `prefer-muse`, or `prefer-glm`. This expresses a
preference unrelated to model performance — typically the user has a large subscription with one provider and a small
one with the others, and wants spend steered accordingly. Pass it to routing as the provider preference; the default,
absent a keyword, is none. Routing honors it unless there is a very clear, strong reason to diverge, and says so in its
answer when it does; when you announce such a delegation, say so and why. A cross-family route's reason also names what
it trades (the delegate runs under the foreign harness's permission bypass flags, and spend moves from the session's
subscription onto metered API billing); the announcement of the first cross-family delegation of the session is the
natural place to remind the user of that, and a user whose economics make it worse says `prefer-<family>` or writes the
config file.

The model routing config file, `~/.scode-model-routing.md`, is where the user declares which models exist in their
environment; routing reads it on every request and it belongs to the user. If routing stops because the file it
replaced, `~/.scode-galaxy-brainrc.md`, is still in place, relay that to the user rather than working around it.

### Delegation and escalation rules

- First ask whether delegation is worthwhile. If specifying and reviewing the task costs more than doing it, do it
  yourself. This is especially common for tiny tasks. "Reviewing" means the gate below — reading the diff and re-running
  the checks yourself — so price that in, not a glance at the delegate's report.
- Give a primary model one well-specified attempt. Fix trivial defects locally. After one substantive failure, escalate
  instead of repeatedly spending tokens on the same underpowered model: request the route again with the outcome, and
  routing answers with the next rung.
- An answer with `route exhausted: yes` is the last rung its family offers. If that route fails substantively, handle
  the work in the orchestrator or make a deliberate cross-family attempt; do not retry it mechanically. This holds
  whether or not `endpoint trusted` was yes: families without a `sota` model (muse and glm) never end at a trusted
  endpoint, and a substantive failure at their last rung gets the same treatment even though the model that failed is
  not trusted.
- When the primary's output is broadly wrong rather than fixable, preserve pre-existing user work, remove only the
  delegate's changes, and give the escalation model a fresh implementation task. Include concrete acceptance failures as
  evidence, but do not ask it to repair a structurally bad patch.
- Escalate immediately if the output shows that the task was misclassified: request the route again with the outcome
  `misclassified`, or name a stronger profile and start over. You have standing permission to reroute or do the work
  yourself without asking.
- Every time you delegate, tell the user which model and effort you picked, the work profile, and why any native or
  cross-family choice makes sense for the task's expected size — routing's one-line reason and its divergence flags are
  written to be quoted. One announcement may cover a homogeneous fan-out batch that shares the same profile, model,
  effort, and rationale.

## Concurrency preference

The invocation may ask for concurrency in plain words — "with concurrency", "parallelize where you can", or similar.
Absent such wording, the default stands: parallelize when it helps, but don't hunt for opportunities.

When the user asks for concurrency, treat parallelism as an active goal rather than an available tool. During planning,
decompose the work to surface independent units: fan out read-only work by default, and prefer isolated-writer fan-out
(see Concurrency) over a serial writer sequence whenever the tasks can be specified independently and the isolation
overhead — including the riskier merge — is worth the speed-up. When you keep a plausibly parallel step serial, briefly
say why.

The preference buys speed, never quality: every rule in Concurrency and The gate applies unchanged, and escalation or
review steps are never skipped to keep a fan-out moving.

## Carrying out a delegation

Before the first delegation of any kind, read `delegating.md`: it holds the task-spec checklist, the baseline you must
record before a writer runs, and how isolated writers get integrated. Before the first shell-out, load
`scode-harness-shellout` as What to read describes, read its `SKILL.md`, and then read its file for the mechanism
routing answered (`harness/codex.md`, `harness/claude.md`, `harness/muse.md`, or `harness/opencode.md` under that
skill's base directory); those files carry the launch templates and the observed behaviors (stdin hangs, exit-code
semantics, how effort and unrestricted tool access actually resolve, kill semantics) that a launch improvised from
`--help` would miss.

Every writer path must support resuming the same delegate in the same session, because writers stop twice for the
checkpoints in `delegating.md` and are resumed with your answers. Natively that is a follow-up message to the same sub
agent (`SendMessage` in Claude Code, or your harness's equivalent); shelled out, `scode-harness-shellout`'s file for the
mechanism carries the verified resume command, keyed on a session or thread id you must record at launch. A path that
cannot resume is not a writer path. Native resume is verified for Claude Code and for Codex (0.151.0, a gpt-5.6-sol
orchestrator resuming a native gpt-5.6-terra sub agent through both checkpoints, 2026-08-30). On a harness whose native
sub agents cannot take a follow-up message, tell routing so in the request and it answers a writer with that family's
shell-out mechanism instead — the one case where a session shells out to its own family; say so when announcing the
delegation.

Name every sub agent (label, description, or whatever your mechanism displays) so the name includes the task plus the
model and effort actually doing the work, e.g. `fix-foo-gpt-5.6-sol-medium`. Harness UIs otherwise show only the wrapper
or default model, which misleads anyone watching progress.

## Concurrency

These are the constraints that decide whether a plan may fan out at all; the mechanics of doing it are in
`delegating.md`.

- Read-only tasks (log scanning, code search, independent reviews) may run concurrently whenever they are independent of
  each other. Use this freely for fan-out work like scanning many logs or directories.
- Writers that share a working tree run one at a time, no exceptions. "They edit different files" is not a safe basis
  for parallelism: file-disjoint writers still see each other's half-finished edits when they run checks, still collide
  on lock files, generated code, and build state, and delegates drift out of their predicted scope. The failure mode is
  an interleaved diff nobody can attribute or cleanly revert. A writer stopped at a checkpoint (see `delegating.md`)
  counts as running: its process has exited but its half-done work is in the tree and it will be resumed into it.
- Other sessions may be running this skill at the same time, on the same machine and against the same repository.
  `SPEC.md` requires that the skill's own state never be the reason two of them interfere: everything the skill puts on
  disk — run directories, scratch files, logs, trees you create — is named by a run id only you hold, and you never
  remove or reinterpret anything named by an id you did not generate. That is the whole of the guarantee. Whether two
  sessions can safely edit the same working tree at once is a property of the work and the user's setup, not something
  this skill detects or prevents; a run directory you did not create is worth a mention to the user, not a gate.
- Isolation mechanisms that delete a tree that looks clean when the run exits (muse's `-w create`, worktree modes on
  native sub agents that auto-clean unchanged trees) are not safe for writers under the checkpoint protocol: at the
  first stop the delegate has written only its run directory under `.galaxy-brain/`, and whether that counts as
  "changed" is the mechanism's call, not yours. For isolated writers create the tree yourself — a `git worktree`, a jj
  or Sapling workspace, a clone — at a path that includes the run id, with any branch, bookmark, or workspace name you
  choose for it carrying the run id too (a fixed name like `gb/fix-foo` collides with another session's, or worse,
  checks out its half-done work), and point the delegate at it.
- Writers may run concurrently when each one is isolated so that no delegate can observe or clobber another's
  in-progress work. Use a tree you created and own the lifecycle of — a `git worktree`, a jj or Sapling workspace, a
  separate clone, or anything equivalent — that the delegate is pointed at; not a native worktree mode that cleans up an
  unchanged tree on its own (the previous bullet says why). The bar is that the tasks cannot conflict through any
  mutable state they touch, not a plan for writers to stay out of each other's way. A separate tree covers the files,
  but shared out-of-tree resources — build caches pointed outside the tree, test databases, ports, daemons — conflict
  straight through it; isolate those too or serialize.
- Isolation moves the merge to you instead of eliminating it: every accepted result is extracted, gated, and applied to
  the main tree by you, serially, with checks re-run after each apply, and conflicts between accepted results are yours
  to resolve. Price that in when deciding whether the speed-up is worth it.
- Two more costs before choosing isolation: isolated trees start from committed state, so delegates will not see
  uncommitted work in the main tree — commit it first if they need it (stashing does not help: it hides the work from
  the main tree without making it visible anywhere else), or serialize rather than committing user work merely to
  parallelize. And a fresh tree typically has no build cache, so writers that run checks may rebuild from scratch; weigh
  that against the parallelism gain.

## The gate

You are the quality gate for everything a delegate produces. Never accept a delegate's self-report as evidence the work
is good.

For code output, the gate starts while the delegate is still running. Every writer stops twice under the checkpoint
protocol in `delegating.md`, and each stop is a gate step:

1. At `AWAITING GUIDANCE`: read `ASSUMPTIONS.md` in the delegation's run directory (`.galaxy-brain/<run-id>/`), write
   `ANSWERS.md` there (one line per item, `OK` or a replacement), resume the same delegate.
2. At `AWAITING REVIEW`: read `DECISIONS.md` in the run directory, write `REVIEW.md` the same way, resume again.
3. When it finishes: inspect the actual change set yourself, not just the report — status plus diff in whatever VCS is
   in use (under git, `git status` and `git diff`; a plain diff misses new files, renames, and mode changes, and a new
   file is where a delegate's surprises tend to live).
4. Re-run the relevant checks yourself.
5. Read `DECISIONS.md` in full (entries added while applying your review are unreviewed) and `REPORT.md`'s Deviations
   section, which must point at `ASSUMPTIONS.md` and `DECISIONS.md` and say what you changed at each checkpoint. A
   delegate that produced `REPORT.md` without stopping is a gate failure to catch by content — there is no sentinel in
   its final message — and its work is unreviewed until you have read the two files after the fact; `delegating.md` says
   how to classify what came back.
6. Go back to what the user actually asked for — their own words where you still have them, the retained record of the
   request plus their later clarifications and decisions after a compaction — and check that the work delivers it. Later
   explicit user direction wins over the first phrasing; the point is to catch intent that was silently dropped, not to
   resurrect a superseded reading. Your spec is one interpretation, and it can narrow the request without anyone
   noticing: the acceptance checks are derived from the spec, so a spec that quietly dropped the point produces a diff
   that passes them all. This is not hypothetical; in an eval, a request to make a slow command fast was gated to a
   correct change never shown to make it fast, and a request to clean up leftover directories was gated to a correct
   hardening of an adjacent edge case. Judge the right unit: a delegation that is one step of a decomposed goal only
   owes its step, and the whole-request question is asked of the integrated result. And answer with evidence — point at
   the line, the test, or the measurement that delivers each thing the user asked for — not with the diff looking
   plausible. When something is missing, find the layer before acting: a spec that dropped it (fix the spec, then
   redelegate or do it), a delegate that missed a correct spec (the ordinary fixup/escalation path), a later unit that
   owns it (say so), or a blocker only the user can resolve (report it).
7. Then make a judgment call:
   - Small defects (naming, comments, minor logic): fix them yourself — a fixup round-trip costs more than doing it.
   - Substantive but well-specified defects: request the route again with the outcome `substantive failure`, and send
     one precise fixup round to the model routing answers.
   - If the escalation also fails, or the output shows that the profile itself was wrong, stop iterating. Do it yourself
     or move to a stronger profile without asking the user.

For read-only findings (reviews, scans, analysis): spot-verify against the cited code or data before relaying. When
reporting to the user, separate what you confirmed from delegate claims you did not verify.

In your final report to the user, briefly note which parts were delegated and to which models.

## Feedback capture

When the user says "galaxy brain feedback: ..." (or clearly signals feedback about how this skill performed), pause
whatever you are doing, read `feedback.md` next to this file, and record the feedback as it describes before resuming.

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
the user's VCS workflow preferences and skills, and those do not transfer to a sub agent; the delegation skill's task
spec rules make that binding on every delegate.

## What to read, and when

This file is the workflow: how to decide whether to delegate, what to delegate, how to stay in charge, and what to do
with what comes back. Two other skills carry the parts other skills also need — which model a unit of work should run
on, and how to work with a delegate once you have one — and this file loads each by name at the moment it becomes
relevant. The one procedure file next to this one is read the same way. "Next to this one" means the same directory this
SKILL.md was loaded from, whatever path that is in the current harness.

| read          | in full, the first time in a session that you...                                          |
| ------------- | ----------------------------------------------------------------------------------------- |
| `feedback.md` | hear "galaxy brain feedback: ..." or otherwise clear feedback on how this skill performed |

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

The first time in a session that you decide to delegate anything at all, native or shelled out, load the skill
`scode-agent-delegation` as follows and read its `SKILL.md` in full. It owns everything between the routing decision and
your verdict on the result: the task spec, the run id and run directory, the launch (it loads the shell-out skill itself
when a delegate runs on a foreign harness), the two-stop checkpoint protocol for writers, the gate, and the verdict it
returns to you. This file never carries a copy of any of that.

<!-- dependency: scode-agent-delegation -->

> Load the skill `scode-agent-delegation` through your harness's skill mechanism: the Skill tool on Claude Code, the
> `skill` tool on OpenCode, the `read_skill` tool on Muse Code. On Codex, which has no such tool, read
> `${CODEX_HOME:-$HOME/.codex}/skills/scode-agent-delegation/SKILL.md`; if it is absent or unreadable, report that exact
> path and do not search elsewhere. On any other harness, use its skill loader only if the result reports the skill's
> base directory; otherwise stop and say this skill has not been verified on that harness. The base directory is the
> directory containing the loaded `SKILL.md`. Confirm the name the loader reports is `scode-agent-delegation`; if the
> loader shows no name, read only the frontmatter (the first lines up to the closing `---`) of `<base>/SKILL.md`. Read
> its sidecars relative to the base directory. Stop and tell the user that `scode-agent-delegation` is not installed or
> could not be loaded, naming the path or tool, if the loader reports the skill as unknown or denied, the file is absent
> or unreadable on Codex (the skills root for Codex 0.152), the result says it was truncated, the name does not match,
> or a sidecar this step needs is not readable under the base directory. Do not continue from memory, from a copy, from
> a search for the file elsewhere, or from a similar skill.

<!-- /dependency -->

## Composing with other skills

Galaxy-brain is an orchestration layer, not a workflow. When another skill or instruction defines its own process —
roles, steps, what counts as a valid run — that skill stays authoritative for the process. Galaxy-brain only decides
which model and effort executes each unit of work (by asking routing), how the delegation seam works (through the
delegation skill), and what happens to each result.

Keep the roles separate: do not merge this skill's orchestrator role with another skill's coordinator role, and do not
attribute one skill's constraints to the other. If another skill forbids or requires something, that rule comes from
that skill; reason about it (and explain it to the user) on that skill's terms.

Routing authority covers every delegation seam the current session owns. When another active skill asks this session to
spawn reviewers or workers, each spawn is still routed by galaxy-brain: ask routing for the model and effort as a
process-defined spawn, carrying any model, agent type, or effort that skill explicitly demands, set model and effort
explicitly where the spawn mechanism supports it, and announce the choice as usual. The trap is following the other
skill's spawn instructions verbatim and letting its subagents silently inherit this session's expensive model.
Inheriting is fine only as a deliberate routing decision, stated as such — which is what routing's `inherit` answer is.
When the other skill demanded the choice, the announcement attributes it to that skill. A spawn that another skill's
process defines is routed here but is not a delegation in the delegation skill's sense: its spec, artifacts, and judging
belong to that process, and no checkpoint addendum or gate is applied to it.

A delegated unit that is itself a coordinator — one that runs a process skill with its own fan-out — is one delegation
seam, not one seam per subagent it creates internally; the outer route chooses and announces the coordinator's model and
effort, and the delegation skill says how such a coordinator is launched and monitored.

## Staying active for the whole session

Activation is session-scoped, not turn- or task-scoped. Once the user invokes this skill, keep routing every delegation
through it — including spawns triggered by other skills, and including later tasks the user never mentions the skill on
— for the rest of the session. Only two things end it: the user expressly asking to stop, or an invocation that limited
the scope up front ("use scode-galaxy-brain for <this one thing>"). Finishing the task it was invoked for does not.
Context compaction, session resume, a tool restart, or a summary that fails to mention the skill does not end it either;
treat retained context that has gone quiet about galaxy-brain as a summarization artifact, not a decision anyone made.

After compaction or resume, if the retained context says or implies this skill was active, re-read this SKILL.md,
`scode-model-routing` if any routing decision was made, and `scode-agent-delegation` if any delegation happened — each
loaded again as What to read describes, with every sidecar of theirs you had loaded re-read per their own read-when
rules; if delegation had loaded the shell-out skill, it loads it again the same way, with every harness file used before
the break — before doing further substantive work; the routing rules and the launch details do not survive summarization
reliably. If the retained context is ambiguous but mentions outstanding delegated work, model or effort routing, or
galaxy-brain at all, assume the skill is still active and say that you are assuming it.

When you write a handoff or pre-compaction note while this skill is active, include the routing-layer state: the current
goal, any provider preference, what the model routing config file ruled in or out, which sidecar files and dependency
skills were loaded, delegations still in flight, and the next routing decision. Record every run id you have generated
and not yet moved to scratch, with the absolute path of each run directory; for each delegate stopped at a checkpoint,
also record which checkpoint it is at and the session or thread id needed to resume it — a stopped delegate that the
summary forgets is one that gets relaunched from scratch, and a run id the summary forgets is a directory you can no
longer prove is yours. Do this even when no delegate is currently running — between delegations is exactly when a
summary is most likely to drop the skill. This is a backstop, not the mechanism: stickiness applies whether or not a
handoff was ever written.

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
Code and Codex); the current route and its outcome (`none` on a first attempt; after a verdict, the model that failed
and one of `substantive failure`, `substantive failure (lost context)` when the verdict carried that reason,
`misclassified`, or `execution-path failure`); any mechanism you have found unavailable; and, under a provider
preference, whether the preferred family has already produced poor output this session, which routing cannot remember
for you. Routing reads the model routing config file and checks which CLIs and credentials exist itself.

Act on the answer as follows. `orchestrator`: the unit is not delegated; do it yourself. `inherit`: spawn at this
session's own model on the caller's mechanism. `no suitable route`: nothing reachable fits; do it yourself or tell the
user. A model with a mechanism, for a unit of your own decomposition: hand it, with the unit, to the delegation skill,
which launches it (natively, or through the shell-out skill it loads itself). For a spawn another skill's process
defines, launch it under that process with the model and effort set, as Composing says: it is routed here, but it is not
a delegation in the delegation skill's sense. An agent type the user or another skill's process demanded is not in the
answer; hand it over with the unit so it is applied at the spawn exactly as demanded. The `route exhausted` and
`endpoint trusted` facts feed the action table below, and the reason and the divergence flags feed the announcement.

The invocation may include the keyword `prefer-gpt`, `prefer-claude`, `prefer-muse`, or `prefer-glm`. This expresses a
preference unrelated to model performance — typically the user has a large subscription with one provider and a small
one with the others, and wants spend steered accordingly. Pass it to routing as the provider preference; the default,
absent a keyword, is none. Routing honors it unless there is a very clear, strong reason to diverge, and says so in its
answer when it does; when you announce such a delegation, say so and why. A cross-family route's reason also names what
it trades (the delegate runs under the foreign harness's permission bypass flags, and spend moves from the session's
subscription onto metered API billing); the announcement of the first cross-family delegation of the session is the
natural place to remind the user of that, and a user whose economics make it worse says `prefer-<family>` or writes the
config file.

## Delegating

First ask whether delegation is worthwhile. If specifying and reviewing the task costs more than doing it, do it
yourself. This is especially common for tiny tasks. "Reviewing" means the gate — reading the diff and re-running the
checks yourself, which the delegation skill's gate procedure spells out — so price that in, not a glance at the
delegate's report.

Once a unit is worth delegating and routing has answered, the delegation skill (loaded as What to read describes) takes
over: it generates the run id first, asks you for the tree, creates the run directory, finishes the self-contained spec
with you (a writer's addendum carries the run directory's absolute path, so the spec cannot be complete before the
directory exists), launches, runs the checkpoint protocol with you reviewing at each stop, gates the result, and returns
a verdict. What you supply along the way is what only you know: the unit and its acceptance criteria, the model and
mechanism from routing, whether the delegate edits a tree, the tree itself — the shared one, or an isolated tree you
create and own, named by the run id, with its base recorded — and, for a shell-out, the expected duration and hard
deadline.

Every time you delegate, tell the user which model and effort you picked, the work profile, and why any native or
cross-family choice makes sense for the task's expected size — routing's one-line reason and its divergence flags are
written to be quoted. One announcement may cover a homogeneous fan-out batch that shares the same profile, model,
effort, and rationale.

## Acting on a verdict

The delegation skill's gate returns exactly one verdict per delegation; what happens next is yours. Give a primary model
one well-specified attempt. After one substantive failure, escalate instead of repeatedly spending tokens on the same
underpowered model: request the route again with the outcome, and routing answers with the next rung. If the escalation
also fails, or the output shows that the profile itself was wrong, stop iterating. Do it yourself or move to a stronger
profile without asking the user. You have standing permission to reroute or do the work yourself without asking.

| verdict                                 | action                                                                                                                                                                                                                                                                                                                                             | after the cap, or when routing said `route exhausted: yes`                                                                  | tree cleanup                                                                 | new run id          |
| --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- | ------------------- |
| `accepted`, `accepted with local fixes` | integrate (for an isolated writer, per the delegation skill's `integrating.md`); tell the delegation skill the verdict is acted on; report the delegation done                                                                                                                                                                                     |                                                                                                                             | none                                                                         | no                  |
| `spec defect`                           | fix the spec; then either take the corrected work over yourself or redelegate on the same route with the corrected spec, keeping the delegate's changes when the new spec extends the old one and removing them otherwise                                                                                                                          | one redelegation, then take over                                                                                            | you remove the changes, when removing                                        | if redelegating     |
| `substantive failure, fixable`          | one fixup round on the escalation rung with the failures as evidence; the fixup delegate may read the primary's run directory                                                                                                                                                                                                                      | stop iterating: take over, or a deliberate cross-family attempt (request the route again with independence or a preference) | none until the lineage is accepted or abandoned                              | yes                 |
| `substantive failure, structural`       | preserve pre-existing user work and remove only the delegate's changes; a fresh implementation task on the escalation rung with the concrete failures as evidence, never a request to repair the bad patch; with reason `lost context`, re-request the route with `input size: large` and that outcome, which routing does not count as an attempt | stop iterating: take over, or a deliberate cross-family attempt                                                             | you remove the changes first                                                 | yes                 |
| `execution-path failure`                | fix the path; relaunch once on the same route from a cleaned tree                                                                                                                                                                                                                                                                                  | re-request the route with that mechanism marked unavailable, or take over                                                   | you clean the tree                                                           | yes                 |
| `misclassified`                         | reroute to a stronger profile, or re-request the route with outcome `misclassified` for the profile's next rung, or take over, without asking; keep usable partial work unless the verdict's cause was the reply cap or a structurally bad patch                                                                                                   | after the reroute fails: take over                                                                                          | you remove the changes first when the cause was the reply cap or a bad patch | yes                 |
| `inconclusive`, reason `crash recurred` | fix the environment; relaunch from a cleaned tree on the same route                                                                                                                                                                                                                                                                                | take over                                                                                                                   | you clean the tree                                                           | yes                 |
| `inconclusive`, reason `over budget`    | report the over-budget result and judge whether the task was mis-sized before anything else; no automatic relaunch                                                                                                                                                                                                                                 | reroute as `misclassified`, or take over                                                                                    | you clean the tree before any reroute                                        | if rerouting        |
| `unresumable`                           | remove the delegate's attributable changes; relaunch on the same route with the folded answers in a fresh spec                                                                                                                                                                                                                                     | take over                                                                                                                   | you remove the changes first                                                 | yes                 |
| `blocked on user`                       | report the question; no relaunch; leave the tree and the run directory as they are until the user answers, then finish the gate and integrate yourself, or start a fresh delegation if more delegate work is needed (the checkpoint protocol has no resume for a delegate that finished)                                                           |                                                                                                                             | none; the run directory stays live                                           | if delegating again |

"You remove" and "you clean" mean the orchestrator, against the attribution baseline the delegation skill had you
record, before any new delegation starts in that tree; the delegation skill never touches tree changes. Every relaunch,
fixup, reroute, and redelegation is a new delegation to the delegation skill, with a fresh run id; the only verdict that
keeps a run directory live is `blocked on user`. Counters are per lineage — the chain of run ids that started from one
task spec — so a fixup that comes back `substantive failure, fixable` again is the cap being hit, not a fresh
delegation. A route whose answer said `route exhausted: yes` has no further rung; a substantive failure there means take
over or a deliberate cross-family attempt, whether or not `endpoint trusted` was yes — families without a `sota` model
never end at a trusted endpoint, and a failure at their last rung gets the same treatment even though the model that
failed is not trusted. After completing any row but `blocked on user`, tell the delegation skill the verdict is acted
on, so it moves that run directory to private scratch; the accepted row says so explicitly and the others are no
different.

In your final report to the user, briefly note which parts were delegated and to which models, and separate what the
gate confirmed from delegate claims that were not verified.

## Concurrency preference

The invocation may ask for concurrency in plain words — "with concurrency", "parallelize where you can", or similar.
Absent such wording, the default stands: parallelize when it helps, but don't hunt for opportunities.

When the user asks for concurrency, treat parallelism as an active goal rather than an available tool. During planning,
decompose the work to surface independent units: fan out read-only work by default, and prefer isolated-writer fan-out
(see Concurrency) over a serial writer sequence whenever the tasks can be specified independently and the isolation
overhead — including the riskier merge — is worth the speed-up. When you keep a plausibly parallel step serial, briefly
say why.

The preference buys speed, never quality: every rule in Concurrency and every gate step applies unchanged, and
escalation or review steps are never skipped to keep a fan-out moving.

## Concurrency

These are the constraints that decide whether a plan may fan out at all; the mechanics of doing it are the delegation
skill's.

- Read-only tasks (log scanning, code search, independent reviews) may run concurrently whenever they are independent of
  each other. Use this freely for fan-out work like scanning many logs or directories.
- Writers that share a working tree run one at a time, no exceptions. "They edit different files" is not a safe basis
  for parallelism: file-disjoint writers still see each other's half-finished edits when they run checks, still collide
  on lock files, generated code, and build state, and delegates drift out of their predicted scope. The failure mode is
  an interleaved diff nobody can attribute or cleanly revert. A writer stopped at a checkpoint counts as running: its
  process has exited but its half-done work is in the tree and it will be resumed into it.
- Other sessions may be running this skill at the same time, on the same machine and against the same repository.
  `SPEC.md` requires that the skill's own state never be the reason two of them interfere: every layer names its
  artifacts by the run id — run directories, scratch files, logs, trees you create — and you never remove or reinterpret
  anything named by an id you did not generate. That is the whole of the guarantee. Whether two sessions can safely edit
  the same working tree at once is a property of the work and the user's setup, not something this skill detects or
  prevents; a run directory you did not create is worth a mention to the user, not a gate.
- For isolated writers create the tree yourself — a `git worktree`, a jj or Sapling workspace, a clone — at a path that
  includes the run id, with any branch, bookmark, or workspace name you choose for it carrying the run id too (a fixed
  name like `gb/fix-foo` collides with another session's, or worse, checks out its half-done work), and point the
  delegate at it. Never a mechanism that deletes a tree that looks clean when the run exits (a harness's own worktree
  mode that auto-cleans an unchanged tree); the delegation skill says why those are out for writers under its checkpoint
  protocol.
- Writers may run concurrently when each one is isolated so that no delegate can observe or clobber another's
  in-progress work. Use a tree you created and own the lifecycle of — a `git worktree`, a jj or Sapling workspace, a
  separate clone, or anything equivalent — that the delegate is pointed at. The bar is that the tasks cannot conflict
  through any mutable state they touch, not a plan for writers to stay out of each other's way. A separate tree covers
  the files, but shared out-of-tree resources — build caches pointed outside the tree, test databases, ports, daemons —
  conflict straight through it; isolate those too or serialize.
- Isolation moves the merge to you instead of eliminating it: every accepted result is extracted, gated, and applied to
  the main tree by you, serially, with checks re-run after each apply, and conflicts between accepted results are yours
  to resolve. Price that in when deciding whether the speed-up is worth it.
- Two more costs before choosing isolation: isolated trees start from committed state, so delegates will not see
  uncommitted work in the main tree — commit it first if they need it (stashing does not help: it hides the work from
  the main tree without making it visible anywhere else), or serialize rather than committing user work merely to
  parallelize. And a fresh tree typically has no build cache, so writers that run checks may rebuild from scratch; weigh
  that against the parallelism gain.

## Feedback capture

When the user says "galaxy brain feedback: ..." (or clearly signals feedback about how this skill performed), pause
whatever you are doing, read `feedback.md` next to this file, and record the feedback as it describes before resuming.

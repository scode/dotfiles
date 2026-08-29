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

This file is the routing policy: everything needed to decide whether to delegate, what to delegate, and to which model.
The procedure for carrying a decision out lives in files next to this one, and each is read only at the moment it
becomes relevant — a session that never shells out never reads a harness file. "Next to this one" means the same
directory this SKILL.md was loaded from, whatever path that is in the current harness.

| read                   | in full, the first time in a session that you...                                          |
| ---------------------- | ----------------------------------------------------------------------------------------- |
| `delegating.md`        | decide to delegate anything at all, native or shelled out                                 |
| `harness/shell-out.md` | decide to shell out to any external harness                                               |
| `harness/codex.md`     | route to a gpt model from a non-Codex session (`codex exec`)                              |
| `harness/claude.md`    | route to a claude model from a non-Claude session (`claude -p`)                           |
| `harness/muse.md`      | route to a muse model (`muse exec`)                                                       |
| `harness/opencode.md`  | route to a glm model (`opencode run`)                                                     |
| `rc-file.md`           | find that `~/.scode-galaxy-brainrc.md` exists, or are asked to seed it                    |
| `feedback.md`          | hear "galaxy brain feedback: ..." or otherwise clear feedback on how this skill performed |

The harness files are the only place launch commands live. That is deliberate: a launch built from memory of a previous
session, or improvised from a harness's `--help`, skips the observed-behavior notes that exist because each one cost
someone a hung or silently wrong run. Read the file, then launch.

## Composing with other skills

Galaxy-brain is a routing layer, not a workflow. When another skill or instruction defines its own process — roles,
steps, what counts as a valid run — that skill stays authoritative for the process. Galaxy-brain only decides which
model and effort executes each unit of work, how the delegation seam works, and how delegated output gets gated.

Keep the roles separate: do not merge this skill's orchestrator role with another skill's coordinator role, and do not
attribute one skill's constraints to the other. If another skill forbids or requires something, that rule comes from
that skill; reason about it (and explain it to the user) on that skill's terms.

Routing authority covers nested delegation too. When another active skill's process calls for spawning subagents —
reviewers, workers, whatever it names them — each spawn is still a galaxy-brain delegation: pick the model from the
appropriate work profile, set model and effort explicitly where the spawn mechanism supports it, and announce the choice
as usual. The trap is following the other skill's spawn instructions verbatim and letting its subagents silently inherit
this session's expensive model. Inheriting is fine only as a deliberate routing decision, stated as such. This claims
only the choices the other skill leaves open: if it explicitly demands a specific model, agent type, or effort for a
spawn, that demand is process, not a routing default — honor it like any other rule that skill owns, and attribute the
choice to that skill when you announce it.

## Staying active for the whole session

Activation is session-scoped, not turn- or task-scoped. Once the user invokes this skill, keep routing every delegation
through it — including spawns triggered by other skills, and including later tasks the user never mentions the skill on
— for the rest of the session. Only two things end it: the user expressly asking to stop, or an invocation that limited
the scope up front ("use scode-galaxy-brain for <this one thing>"). Finishing the task it was invoked for does not.
Context compaction, session resume, a tool restart, or a summary that fails to mention the skill does not end it either;
treat retained context that has gone quiet about galaxy-brain as a summarization artifact, not a decision anyone made.

After compaction or resume, if the retained context says or implies this skill was active, re-read this SKILL.md, every
sidecar file you had loaded (at minimum `delegating.md` if any delegation happened, plus the harness files for any
harness in use), and `~/.scode-galaxy-brainrc.md` (if it exists) before doing further substantive work — the routing
rules and the launch details do not survive summarization reliably. If the retained context is ambiguous but mentions
outstanding delegated work, model or effort routing, or galaxy-brain at all, assume the skill is still active and say
that you are assuming it.

When you write a handoff or pre-compaction note while this skill is active, include the routing-layer state: the current
goal, any provider preference, rc-file assumptions, which sidecar files were loaded, delegations still in flight, and
the next routing decision. Do this even when no delegate is currently running — between delegations is exactly when a
summary is most likely to drop the skill. This is a backstop, not the mechanism: stickiness applies whether or not a
handoff was ever written.

## Routing model

Do not rank models with universal cost or intelligence scores. Effective cost depends on the task: a nominally cheap
model can spend more tokens, take longer, require more review, and trigger an escalation when work exceeds its reliable
range. Route by work profile instead, then use the gate and escalation policy to control total cost through an accepted
result.

Each model name includes its configured reasoning effort. The family determines which delegation path from the mechanics
section applies. `sota` marks models trusted with critical review and the orchestrator role. Availability and user
overrides may remove or replace these defaults; see Local availability.

| model                             | family | sota |
| --------------------------------- | ------ | ---- |
| gpt-5.6-luna medium               | gpt    |      |
| gpt-5.6-terra medium              | gpt    |      |
| gpt-5.6-sol low                   | gpt    |      |
| gpt-5.6-sol medium                | gpt    |      |
| gpt-5.6-sol high                  | gpt    | yes  |
| haiku-4.5 high                    | claude |      |
| sonnet-5 low                      | claude |      |
| sonnet-5 medium                   | claude |      |
| sonnet-5 high                     | claude |      |
| opus-5 high                       | claude |      |
| fable-5 high                      | claude | yes  |
| muse-spark-1.2-contributor low    | muse   |      |
| muse-spark-1.2-contributor medium | muse   |      |
| muse-spark-1.2-contributor high   | muse   |      |
| muse-spark-1.2-contributor xhigh  | muse   |      |
| glm-5.3-flash low                 | glm    |      |
| glm-5.3-flash high                | glm    |      |
| glm-5.3-flash max                 | glm    |      |

The muse family is Meta's Muse Code harness and its Muse Spark model. It is an option, not a default: never route to it
on your own initiative. It enters a route only through an explicit `prefer-muse` preference, a user request naming it,
or a deliberate cross-family decision announced as such. Its profile placements below are provisional: they rest on
vendor-reported benchmarks rather than calibrated use, and until real use calibrates them neither its benchmark tier nor
its token price counts as a reason to cross families on your own. It carries no `sota` mark, so it is never the sole
critical-review gate, and no muse route ends at a trusted endpoint (see Delegation and escalation rules).

The glm family is Z.ai's GLM-5.3-Flash (the model that ran as the `ox-alpha` stealth preview on OpenRouter and OpenCode)
driven through the OpenCode harness, `opencode run`. Everything said about muse above applies to it unchanged: opt-in
only, via `prefer-glm`, a request naming it, or an announced cross-family decision; provisional placements; no `sota`
mark; no route ending at a trusted endpoint. Its evidence base is one clean clear-spec smoke run plus vendor benchmarks,
and its per-token price is roughly an order of magnitude below the other families — which is exactly the number that
must not tempt an unprompted cross-family route until real use has calibrated it. The three effort levels are the only
ones Z.ai accepts for this model.

### Work profiles

Classify the task before choosing a model. Use the primary for the orchestrator's family when it is suitable and
available. Move to the escalation model after a substantive failure or when the task proves more demanding than its
initial classification.

| profile                   | use when                                                                 | GPT route                                  | Claude route                    | Muse route                                                          | GLM route                              |
| ------------------------- | ------------------------------------------------------------------------ | ------------------------------------------ | ------------------------------- | ------------------------------------------------------------------- | -------------------------------------- |
| mechanical                | Deterministic tool use, searches, log scans, or tedious verified churn   | gpt-5.6-luna medium → gpt-5.6-terra medium | haiku-4.5 high → sonnet-5 low   | muse-spark-1.2-contributor low → muse-spark-1.2-contributor medium  | glm-5.3-flash low → glm-5.3-flash high |
| routine authored          | Producing or editing small prose/code where baseline taste matters       | gpt-5.6-sol low → gpt-5.6-sol medium       | sonnet-5 low → sonnet-5 medium  | muse-spark-1.2-contributor medium → muse-spark-1.2-contributor high | glm-5.3-flash high → glm-5.3-flash max |
| clear-spec implementation | Bounded implementation with strong acceptance checks                     | gpt-5.6-terra medium → gpt-5.6-sol medium  | sonnet-5 medium → sonnet-5 high | muse-spark-1.2-contributor medium → muse-spark-1.2-contributor high | glm-5.3-flash high → glm-5.3-flash max |
| complex implementation    | Cross-cutting behavior, difficult debugging, or meaningful ambiguity     | gpt-5.6-sol medium → gpt-5.6-sol high      | opus-5 high → fable-5 high      | muse-spark-1.2-contributor high → muse-spark-1.2-contributor xhigh  | glm-5.3-flash max                      |
| design and synthesis      | API design, architecture, nuanced copy, or competing tradeoffs           | gpt-5.6-sol medium → gpt-5.6-sol high      | opus-5 high → fable-5 high      | none                                                                | none                                   |
| mechanical review         | Non-critical review: style, prose, idiomaticity, docs, slop, or patterns | gpt-5.6-sol medium → gpt-5.6-sol high      | sonnet-5 high → opus-5 high     | muse-spark-1.2-contributor high → muse-spark-1.2-contributor xhigh  | glm-5.3-flash max                      |
| critical review           | Correctness, security, concurrency, data integrity, or test-quality gate | gpt-5.6-sol high                           | fable-5 high                    | none                                                                | none                                   |

A `none` route means the family has no suitable model for that profile. When a provider preference points at such a
route, that is the "no suitable model" reason to diverge: fall back to the orchestrator's own family's route for the
profile and announce the divergence as usual.

These assignments are defaults, not claims that every task in a profile is equivalent. Test quality is critical because
weak tests are how correctness defects survive review. Reviews route above similarly sized implementation work because
the reviewer is the gate: a missed finding may have no later backstop. Some profiles intentionally share routes today;
keeping their semantics separate lets later calibration change one without conflating different failure costs. A second
cross-family SOTA perspective may be worth its overhead for high-risk critical review. Orchestration is not a delegation
profile; planning, decomposition, quality gating, and VCS ownership remain with the current SOTA session.

Visual design is an exception to the GPT design-and-synthesis route. Real-world feedback on GPT-5.6 consistently rates
sol below the Claude models on visual design taste even while its coding reputation holds up. When a
design-and-synthesis task's output is primarily visual — UI, frontend styling, slides, anything judged by how it looks —
use the Claude route even from a GPT-family orchestrator, and treat this as a sufficient reason to diverge from a
prefer-gpt preference. Announce the divergence as usual. Non-visual design work such as API design, architecture, and
copy stays on the normal routes.

Long context is an exception to the GPT mechanical route. gpt-5.6-luna's long-context retrieval is far below the rest of
the family (reported around 41% on MRCR versus roughly 90% for terra and sol), so a mechanical task whose input is
genuinely large — whole-repo scans, big log files, long-document analysis — starts at gpt-5.6-terra medium instead of
luna, escalating to gpt-5.6-sol medium. This is about input size the model must actually reason across, not task
difficulty; small-input mechanical churn stays on luna. A luna delegate that loses track of earlier context mid-task is
this weakness surfacing, not a generic substantive failure — reroute to terra rather than counting it against the
profile.

### Native-path bias

When models are roughly equally suitable, prefer the model in the orchestrator's family. Same-family delegates normally
stay inside the current harness; crossing families adds process startup, context transfer, authentication, permission,
output-handling, and failure overhead. The native delegation path is the real reason for the preference. If a harness
can invoke another family natively, prefer the native path rather than following family names mechanically.

Apply the bias according to expected task size. These are judgment anchors, not hard thresholds:

| expected size | practical meaning                                                     | native-path bias                                                                   |
| ------------- | --------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| tiny          | Setup and review may cost as much as doing the task directly.         | Very strong. Usually do it directly or delegate natively.                          |
| short         | One bounded operation with little context or expected iteration.      | Strong. Cross harnesses only for a meaningful capability or total-cost advantage.  |
| medium        | A substantive task where model work dominates fixed startup overhead. | Moderate. Consider likely retries and review burden alongside startup overhead.    |
| large         | Extended work where cross-harness startup is a minor part of the run. | Weak. Choose the route most likely to finish cleanly at lower expected total cost. |

Do not select an unsuitable model merely to stay native. Cross families when the native family has no suitable model,
the other family is materially better for the task, a medium or large task has materially lower expected total cost on
the other route, a native attempt failed, the user requested that provider, or an independent cross-family perspective
is part of the goal. Expected total cost includes likely tokens, latency, review burden, and escalation risk — not
nominal token price alone. For critical work, reliability and useful independence take priority over the size-based
bias.

### Delegation and escalation rules

- First ask whether delegation is worthwhile. If specifying and reviewing the task costs more than doing it, do it
  yourself. This is especially common for tiny tasks. "Reviewing" means the gate below — reading the diff and re-running
  the checks yourself — so price that in, not a glance at the delegate's report.
- Set the profile by ambiguity, verification strength, required taste, and the cost of a wrong or missed result — not by
  apparent line count alone.
- Give a primary model one well-specified attempt. Fix trivial defects locally. After one substantive failure, escalate
  instead of repeatedly spending tokens on the same underpowered model.
- A GPT or Claude route with no listed escalation is already at the family's trusted endpoint. If it fails
  substantively, handle the work in the orchestrator or make a deliberate cross-family attempt; do not retry it
  mechanically. Families without a `sota` model (muse and glm) are the exception: none of their routes ends at a trusted
  endpoint, so a substantive failure at the last listed model in such a route gets the same treatment — orchestrator or
  deliberate cross-family attempt — even though the model that failed is not trusted.
- When the primary's output is broadly wrong rather than fixable, preserve pre-existing user work, remove only the
  delegate's changes, and give the escalation model a fresh implementation task. Include concrete acceptance failures as
  evidence, but do not ask it to repair a structurally bad patch.
- Escalate immediately if the output shows that the task was misclassified. You have standing permission to reroute or
  do the work yourself without asking.
- Prefer cheaper, faster workhorses when validation is deterministic and inexpensive. Start stronger when incorrect
  output is difficult to detect or the delegate itself is the final review gate.
- Every time you delegate, tell the user which model and effort you picked, the work profile, and why any native or
  cross-family choice makes sense for the task's expected size. One announcement may cover a homogeneous fan-out batch
  that shares the same profile, model, effort, and rationale.

## Provider preference

The invocation may include the keyword `prefer-gpt`, `prefer-claude`, `prefer-muse`, or `prefer-glm`. This expresses a
preference unrelated to model performance — typically the user has a large subscription with one provider and a small
one with the others, and wants spend steered accordingly. The default, absent a keyword, is no preference.

When a preference is given, route every delegation to the preferred family unless there is a very clear, strong reason
to diverge — for example repeated poor output, no suitable model for the selected profile, or a goal that explicitly
needs an independent cross-family perspective. An explicit provider preference overrides the default native-path bias.
When you diverge, say so and why.

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

## Local availability

The inventory and profiles describe models that exist; they do not know which ones the user can access in this
environment. Before your first delegation, check for `~/.scode-galaxy-brainrc.md`. If it exists, read `rc-file.md` next
to this file and then honor the rc file: it contains natural language adjustments from the user, most commonly
availability restrictions like "fable-5 is not available, do not use" or "only claude models work here", and may replace
the inventory or the profile table outright. Treat its contents as authoritative. Remove unavailable models from every
profile and use the next suitable option rather than preserving a preferred slot mechanically. Apart from the seeding
request described in `rc-file.md`, do not create or edit the rc file yourself; it belongs to the user.

If the file does not exist, all inventory models are assumed available, with one practical exception: a shelled-out
family whose CLI is not installed (`codex`, `claude`, `muse`, or `opencode` missing from `PATH`) is unavailable
regardless of the rc file, and so is the glm family when `opencode providers list` shows no credential for the provider
the model id names — `opencode run` with a model it cannot reach fails immediately, but with no model at all it hangs,
so check rather than probe. Treat either as a missing family when honoring a provider preference — say so and fall back
— rather than as a launch failure to retry.

## Delegation mechanics

Stay native within your own harness; shell out only when crossing vendors. First figure out which harness you are
running in, then:

- **Claude session → claude model**: use your native sub agent mechanism (e.g. the Agent/Task tool) with the model
  parameter set to the target model.
- **Claude session → gpt model**: shell out to `codex exec` per `harness/codex.md`.
- **Codex session → gpt model**: use your native sub agent mechanism, specifying the target model.
- **Codex session → claude model**: shell out to `claude -p` per `harness/claude.md`.
- **Any session → muse model**: shell out to `muse exec` per `harness/muse.md`. Muse Code is only ever a delegate here,
  never the orchestrator, so there is no native muse path.
- **Any session → glm model**: shell out to `opencode run` per `harness/opencode.md`. OpenCode is likewise only ever a
  delegate here.

Before the first delegation of any kind, read `delegating.md`: it holds the task-spec checklist, the baseline you must
record before a writer runs, and how isolated writers get integrated. Before the first shell-out, read
`harness/shell-out.md` and then the file for the harness you are about to launch; the harness files carry the launch
templates and the observed behaviors (stdin hangs, exit-code semantics, how effort and read-only actually get enforced,
kill semantics) that a launch improvised from `--help` would miss.

When delegating natively, also set the target reasoning effort if your sub agent mechanism has an effort parameter;
otherwise sub agents inherit the session's effort and that is acceptable.

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
  an interleaved diff nobody can attribute or cleanly revert.
- Writers may run concurrently when each one is isolated so that no delegate can observe or clobber another's
  in-progress work. Use whatever mechanism your harness offers at your discretion — native worktree isolation on a sub
  agent, a manually created `git worktree` that a shelled-out delegate is pointed at, a separate clone, or anything
  equivalent. The bar is that the tasks cannot conflict through any mutable state they touch, not a plan for writers to
  stay out of each other's way. A separate tree covers the files, but shared out-of-tree resources — build caches
  pointed outside the tree, test databases, ports, daemons — conflict straight through it; isolate those too or
  serialize.
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

For code output:

1. Inspect the actual diff (`git diff`) yourself, not just the report.
2. Re-run the relevant checks yourself.
3. Then make a judgment call:
   - Small defects (naming, comments, minor logic): fix them yourself — a fixup round-trip costs more than doing it.
   - Substantive but well-specified defects: send one precise fixup round to the profile's escalation model.
   - If the escalation also fails, or the output shows that the profile itself was wrong, stop iterating. Do it yourself
     or move to a stronger profile without asking the user.

For read-only findings (reviews, scans, analysis): spot-verify against the cited code or data before relaying. When
reporting to the user, separate what you confirmed from delegate claims you did not verify.

In your final report to the user, briefly note which parts were delegated and to which models.

## Feedback capture

When the user says "galaxy brain feedback: ..." (or clearly signals feedback about how this skill performed), pause
whatever you are doing, read `feedback.md` next to this file, and record the feedback as it describes before resuming.

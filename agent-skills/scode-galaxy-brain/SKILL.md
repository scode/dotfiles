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
the next routing decision. Record every run id you have generated and not yet moved to scratch, with the tree each run
directory is in; for each delegate stopped at a checkpoint, also record which checkpoint it is at and the session or
thread id needed to resume it — a stopped delegate that the summary forgets is one that gets relaunched from scratch,
and a run id the summary forgets is a directory you can no longer prove is yours. Do this even when no delegate is
currently running — between delegations is exactly when a summary is most likely to drop the skill. This is a backstop,
not the mechanism: stickiness applies whether or not a handoff was ever written.

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
available, except that workhorse writers — tree-editing delegates under the mechanical or clear-spec implementation
profile — default to luna from any family (see Native-path bias). Move to the escalation model after a substantive
failure or when the task proves more demanding than its initial classification.

| profile                   | use when                                                                                                                                                                       | GPT route                                  | Claude route                    | Muse route                                                          | GLM route                              |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------ | ------------------------------- | ------------------------------------------------------------------- | -------------------------------------- |
| mechanical                | Deterministic tool use, searches, log scans, or tedious verified churn                                                                                                         | gpt-5.6-luna medium → gpt-5.6-terra medium | haiku-4.5 high → sonnet-5 low   | muse-spark-1.2-contributor low → muse-spark-1.2-contributor medium  | glm-5.3-flash low → glm-5.3-flash high |
| routine authored          | Producing or editing small prose/code where baseline taste matters                                                                                                             | gpt-5.6-sol low → gpt-5.6-sol medium       | sonnet-5 low → sonnet-5 medium  | muse-spark-1.2-contributor medium → muse-spark-1.2-contributor high | glm-5.3-flash high → glm-5.3-flash max |
| clear-spec implementation | Bounded implementation with strong acceptance checks                                                                                                                           | gpt-5.6-luna medium → gpt-5.6-terra medium | sonnet-5 medium → sonnet-5 high | muse-spark-1.2-contributor medium → muse-spark-1.2-contributor high | glm-5.3-flash high → glm-5.3-flash max |
| complex implementation    | Cross-cutting behavior, difficult debugging, or ambiguity that survives your decomposition — design settled by you first, delegated for volume of input more than for judgment | gpt-5.6-terra medium → gpt-5.6-sol medium  | sonnet-5 high → opus-5 high     | muse-spark-1.2-contributor high → muse-spark-1.2-contributor xhigh  | glm-5.3-flash max                      |
| design and synthesis      | API design, architecture, nuanced copy, or competing tradeoffs — the orchestrator's own work, not a delegation (see below)                                                     | orchestrator (visual output: opus-5 high)  | orchestrator                    | none                                                                | none                                   |
| mechanical review         | Non-critical review: style, prose, idiomaticity, docs, slop, or patterns                                                                                                       | gpt-5.6-sol medium → gpt-5.6-sol high      | sonnet-5 high → opus-5 high     | muse-spark-1.2-contributor high → muse-spark-1.2-contributor xhigh  | glm-5.3-flash max                      |
| critical review           | Correctness, security, concurrency, data integrity, or test-quality gate                                                                                                       | gpt-5.6-sol high                           | fable-5 high                    | none                                                                | none                                   |

A `none` route means the family has no suitable model for that profile. When a provider preference points at such a
route, that is the "no suitable model" reason to diverge: fall back to the orchestrator's own family's route for the
profile and announce the divergence as usual. An `orchestrator` route is different: it means the work is not delegated
at all, and no provider preference redirects it — a preference steers delegations, and there is none. The one carve-out
is the visual case in the GPT cell, which a prefer-gpt preference does not override.

These assignments are defaults, not claims that every task in a profile is equivalent. Test quality is critical because
weak tests are how correctness defects survive review. Reviews route above similarly sized implementation work because
the reviewer is the gate: a missed finding may have no later backstop. Some profiles intentionally share routes today;
keeping their semantics separate lets later calibration change one without conflating different failure costs. A second
cross-family SOTA perspective may be worth its overhead for high-risk critical review. Orchestration is not a delegation
profile; planning, decomposition, quality gating, and VCS ownership remain with the current SOTA session.

Design is the orchestrator's own work. Deciding an API shape, an architecture call, or a tradeoff is exactly what the
expensive model's capability is for; handing the decision to a weaker model buys a worse answer than you would have
produced and then charges you to review it. Do the deciding yourself and delegate what is left, which is mechanics.
Gathering the inputs to a decision is mechanics: a read-only "survey this subsystem and lay out the options with their
tradeoffs" delegation is fine and often the right way to protect your own context — the delegate proposes, you decide.
The `orchestrator` entries in the table cover the decision, not the survey. Two edge cases: if this session is not
itself running a `sota`-marked model, treat design the way critical review is treated — delegate the decision up to the
strongest available model rather than keeping it by default — and when another skill's process spawns a design-shaped
subagent, that spawn is process, not routing (see Composing with other skills): run it, and route it at this session's
own model unless that skill demands otherwise. The one case where design output itself is delegated is visual:
real-world feedback on GPT-5.6 consistently rates sol below the Claude models on visual design taste even while its
coding reputation holds up, so a GPT orchestrator producing UI, frontend styling, slides, or anything judged by how it
looks hands that to opus-5 high — the table's GPT cell says so — treating it as a sufficient reason to diverge from a
prefer-gpt preference and announcing the divergence as usual. A Claude orchestrator does its own visual design.

Within implementation work, the mid tier — terra and sol as delegates rather than sol-high-as-reviewer — earns its cost
in two situations. Context economy: work that has to be reasoned across more input than you can afford to spend your own
context on (a change threaded through dozens of files, a diagnosis that means reading a large subsystem) goes to terra,
because luna cannot hold it and you should not have to; that is the center of the complex implementation profile, and
why its route starts at terra rather than sol. And the escalation rung: when a workhorse fails substantively, terra is
the cheap next step before sol and before you take the work over. Difficult debugging and ambiguity that survives
decomposition also live in the complex implementation profile — its route carries sol for exactly the case where the
delegate's own judgment turns out to matter mid-task. What the eval behind these routes
(https://claude.ai/code/artifact/43a3d4f1-fd32-41df-84bc-d62d6fb1f248) actually showed is narrower than "the mid tier is
useless": in 36 runs every model passed every hidden test, so the tasks separated prices, not failure rates, and the
expensive models' visible advantages were soft — documentation quality, benchmark discipline — which a reviewed
assumptions list and a gate that reads the diff cover. The honest conclusion is that nothing there justified paying
mid-tier prices for well-specified work, not that no task ever will. If you find yourself reaching for sol or opus to
implement something well-specified, the usual reason is that the design is not settled yet, and the fix is to settle it;
the routine authored and mechanical review rows are unchanged by this reasoning — taste-dependent prose and review were
not what the eval measured.

Luna is the workhorse on purpose, not as a compromise. Across six treeward features and a planted bug, every model from
luna medium up to sonnet passed every hidden test on the first attempt; the gate rejected three results for hidden
work-done regressions, none of them luna's, making luna medium the cheapest clean record — $0.64 for the six features
against $5.57 for terra and $21.73 for sonnet (https://claude.ai/code/artifact/43a3d4f1-fd32-41df-84bc-d62d6fb1f248).
Luna has two demonstrated weaknesses. Judgment on open questions is the first, and the checkpoint protocol in
`delegating.md` moves that judgment to the orchestrator before any code exists: in the guidance eval, the checkpoint arm
went 8 for 8 across the four cheap models — luna medium and high among them — on a feature the same models had gotten
right once in eight runs without it. It does not move mid-implementation judgment anywhere, which is why the gate still
reads the diff in full. The second weakness is long context, covered by the exception below. What the workhorse needs is
a clear spec and a reviewed assumptions list, and those are the orchestrator's to supply. Expect luna's `DECISIONS.md`
to be short or empty — it does not experience decisions as decisions — and read that as `delegating.md` says, not as a
sign the work was simple. If terra also fails after a luna failure, gpt-5.6-sol medium is the remaining rung before you
take the work over; the two-step routes list the common path, not the whole ladder.

Long context is an exception to the luna routes, from every orchestrator. gpt-5.6-luna's long-context retrieval is far
below the rest of the family (reported around 41% on MRCR versus roughly 90% for terra and sol), so a mechanical or
clear-spec task whose input is genuinely large — whole-repo scans, big log files, long-document analysis, a change that
has to be reasoned across many files at once — starts at gpt-5.6-terra medium instead of luna, escalating to gpt-5.6-sol
medium; a Claude session shelling out to luna under the workhorse default shells out to terra for these instead, rather
than reverting to its native route. This is about input size the model must actually reason across, not task difficulty;
small-input work stays on luna, and when you cannot tell in advance, luna first is the right bet — a reroute after a
lost-context failure costs one cheap run. A luna delegate that loses track of earlier context mid-task is this weakness
surfacing, not a generic substantive failure — reroute to terra rather than counting it against the profile.

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

The bias does not apply to workhorse writers. For delegates that edit the tree under the mechanical or clear-spec
implementation profile, the default from any orchestrator is gpt-5.6-luna medium, reached however your harness reaches a
gpt model per Delegation mechanics — `codex exec` from a Claude session, the native sub agent mechanism from a Codex
session — unless a provider preference says otherwise, the path to a gpt model is unavailable (no `codex` on `PATH` from
a non-Codex session), or the rc file rules the model out. Read-only mechanical work (searches, scans, log reading) is
not covered: there the ordinary bias stands, and a Claude session's cheap native fan-out (haiku) beats paying shell-out
launch and monitoring overhead per delegate — the eval measured implementation writers, not read-only churn. The
cross-family cost the bias exists to weigh is small and characterized for writers: the launch, resume, and monitoring
mechanics in `harness/codex.md` are exercised end to end (two items there remain explicitly unverified), and the price
gap to the same-family writer alternative is about 30× for no measured difference in first-attempt correctness (see Work
profiles). A "short" writer task crosses families under this rule; a "tiny" one is still done by the orchestrator.
Escalation after a luna failure follows the GPT route (terra medium, then sol medium), not the same-family column; the
same-family routes are what you use when the gpt path is unavailable or a preference directs you there. Two things this
default deliberately trades, worth saying to the user when they matter: shelling out runs the delegate with the
harness's permission bypass flags where a native sub agent inherits the session's permission system — where that is
unacceptable, stay native — and it moves spend from whatever subscription covers the orchestrator's own family onto
metered API billing; a user whose economics make that worse says `prefer-claude` or writes the rc file, and the
announcement of the first cross-family delegation is the natural place to remind them. This is the one place the skill
treats a specific model as the default across harnesses, and it rests on one eval in one small repository; if a
follow-up on a larger, messier codebase finds luna's literalism surviving the checkpoint, this paragraph is what to
revisit.

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

Every writer path must support resuming the same delegate in the same session, because writers stop twice for the
checkpoints in `delegating.md` and are resumed with your answers. Natively that is a follow-up message to the same sub
agent (`SendMessage` in Claude Code, or your harness's equivalent); shelled out, each harness file carries the verified
resume command, keyed on a session or thread id you must record at launch. A path that cannot resume is not a writer
path. Native resume is verified for Claude Code and for Codex (0.151.0, a gpt-5.6-sol orchestrator resuming a native
gpt-5.6-terra sub agent through both checkpoints, 2026-08-30). If some other harness's native sub agents cannot take a
follow-up message, route writers through that family's shell-out harness file instead — the one case where a session
shells out to its own family — and say so when announcing the delegation.

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
6. Then make a judgment call:
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

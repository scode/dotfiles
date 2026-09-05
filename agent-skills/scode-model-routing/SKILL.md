---
name: scode-model-routing
description: >
  Answers which model, reasoning effort, and launch mechanism a unit of delegated work should run on, given a work
  profile and the orchestrating session's situation: the inventory, the work-profile table, the native-path bias,
  provider preference, and the model routing config file that declares what exists locally. Loaded by orchestration
  skills that own delegation (they decide whether and how to delegate; this skill only answers the routing question);
  inert on its own and never active for a session. A user may also ask it directly, e.g. "which model would you use
  for X".
---

# scode-model-routing

This skill is never active. It answers when loaded and claims nothing about later work. Routing is a pure function of a
request and the local environment: it holds no state between requests, never decides whether to delegate or whether to
escalate, never launches anything, and never claims later spawns. A skill that has decided to delegate a unit of work
asks it "which model, at what effort, through which mechanism" and acts on the answer under its own rules. The one thing
routing reads from disk on its own is the model routing config file and the CLIs and credentials that decide what is
reachable here (see Local availability); everything else comes from the request.

## The routing request

A request carries twelve facts, all of which the caller has and routing does not:

1. The work profile (see Work profiles and How to name a profile).
2. The orchestrating harness (Claude Code, Codex, Muse Code, or OpenCode) and its model id. Routing derives the
   session's family and whether it is `sota` from its own inventory.
3. Whether the unit edits a tree (a writer) or is read-only.
4. Expected size: tiny, short, medium, or large (see Native-path bias for what each means).
5. Whether the input the delegate must reason across is large (a whole-repo scan, a big log, a change threaded through
   many files) or not.
6. Whether the output is visual (UI, frontend styling, slides, anything judged by how it looks).
7. Spawn origin: the caller's own decomposition, or a role another skill's process defines (a reviewer charter, a
   coordinator role). For a process-defined spawn the mechanism is fixed to `native` unless that process says otherwise.
8. An explicit demand, if any: a provider, model, agent type, or effort that the user or another skill's process
   requires for this spawn.
9. Whether an independent cross-family perspective is part of the goal for this unit.
10. Provider preference, if any: `gpt`, `claude`, `muse`, or `glm`, as the caller parsed it from the user.
11. Whether the native sub agent mechanism can resume a writer with a follow-up message. Only needed on a harness
    routing does not know; for the four above it is known (Claude Code and Codex can; Muse and OpenCode are never
    orchestrators here).
12. The current route and its outcome so far: `none` for a first attempt, or the model and effort last tried with one of
    `substantive failure`, `substantive failure (lost context)`, `misclassified`, or `execution-path failure`, the last
    optionally marking that mechanism unavailable for this unit (a CLI on `PATH` whose launch path is broken, for
    example). A plain substantive failure advances to the next rung, and so does `misclassified` (the task proved more
    demanding than the profile named; the caller may instead name a stronger profile and start over); `lost context`
    reroutes to the long-context rung without counting an attempt; an execution-path failure keeps the route unless its
    mechanism is marked unavailable, in which case routing answers as if that mechanism did not exist. When a provider
    preference is in force, this input also says whether the preferred family has already produced poor output in this
    session, on this unit or earlier ones, since that history is a reason to diverge (see Provider preference) and
    routing has no memory of it.

## The routing answer

The answer carries:

- The route: a model id with its effort (`gpt-5.6-luna medium`), or `orchestrator` (the caller does this unit itself, it
  is not delegated), or `inherit` (run at the session's own model on the caller's mechanism), or `no suitable route`
  (nothing in any reachable family fits; there is no implied fallback, and the caller decides).
- The launch mechanism: `native`, `codex exec`, `claude -p`, `muse exec`, or `opencode run`, exactly those strings, from
  the session-by-family table under Launch mechanism.
- `route exhausted: yes/no`: yes when the route just answered is the last rung its family offers for this profile, so a
  substantive failure there is not answered by a further rung (see Escalation facts).
- `endpoint trusted: yes/no`: yes when that last rung is a `sota` model.
- `diverged from preference: yes/no`, with the reason when yes.
- `cross-family: yes/no`, with the reason when yes and the expected size the decision rested on.
- A one-line reason the caller can announce verbatim: the profile, why this model, and why native or cross-family at
  this size.

When the route is `orchestrator`, `inherit`, or `no suitable route`, the mechanism is `native` for `inherit` and absent
for the other two, the four yes/no facts are absent, and the reason still says why. An agent type carried by a demand
(input 8) is not part of the answer: it is a launch constraint the caller applies unchanged at the spawn.

A profile-table cell that reads `none` never reaches the answer: it means that family has no suitable model, and routing
resolves it to the orchestrator's own family's route for the profile, answering `diverged from preference: yes` with the
reason "no suitable model". A cell that reads `orchestrator` is different: the work is not delegated at all, and no
provider preference redirects it, because a preference steers delegations and there is none. The one carve-out is the
visual case in the GPT cell, which a `gpt` preference does not override.

From a Muse Code or OpenCode session (only ever a coordinator delegate, never the orchestrator of record), same-family
spawns are `inherit` on `native` and cross-family spawns are `no suitable route`, whatever the preference or demand:
those harnesses run a process another session delegated to them, and do not shell out further on their own. This is the
first rule under Precedence.

## Precedence

Highest first. Each earlier rule settles what it covers and the later ones fill in the rest:

1. A Muse Code or OpenCode session (input 2) is a coordinator delegate, never the orchestrator of record: a same-family
   spawn is `inherit` on `native`, and anything else — a cross-family profile route or an explicit cross-family demand
   alike — is `no suitable route`. No later rule applies to such a session.
2. An explicit demand (input 8), subject to availability: honor it, and the reason attributes the choice to whoever
   demanded it. A demand is process, not a routing default.
3. A process-defined spawn (input 7) of a design-shaped role, with no demand, is `inherit` on `native`: it runs at the
   session's own model unless the process says otherwise. Every other process-defined spawn is routed by its profile
   like the caller's own units, with the mechanism fixed to `native` unless the process says otherwise; a preference
   that would need another mechanism then diverges, with the reason `mechanism fixed`.
4. The design profile is `orchestrator`, with two exceptions settled here: a session that is not `sota` delegates design
   up to the strongest available model the way critical review is routed, and a GPT session producing visual output
   (input 6) hands it to opus-5 high — the visual carve-out — whatever the preference says, with
   `diverged from preference: yes (visual)` under a `gpt` preference (see Work profiles).
5. A `none` cell falls back to the orchestrator's family's route, `diverged from preference: yes (no suitable model)`.
6. Required independence (input 9) crosses families: a second perspective from the same family is not independent.
7. Provider preference (input 10): every delegation goes to the preferred family unless one of the rules above or a
   clear, strong reason below diverges, and the answer says so when it does.
8. Suitability for the profile: the profile table row for the chosen family, including the workhorse-writer default and
   the long-context exception.
9. The native-path size bias, which decides between equally suitable models across families.
10. Rung selection from the outcome (input 12).

## How to name a profile

Set the profile by ambiguity, verification strength, required taste, and the cost of a wrong or missed result, not by
apparent line count alone. Prefer cheaper, faster workhorses when validation is deterministic and inexpensive. Start
stronger when incorrect output is difficult to detect or the delegate itself is the final review gate.

## Routing model

Do not rank models with universal cost or intelligence scores. Effective cost depends on the task: a nominally cheap
model can spend more tokens, take longer, require more review, and trigger an escalation when work exceeds its reliable
range. Route by work profile instead; the caller's gate and escalation policy control total cost through an accepted
result.

Each model name includes its configured reasoning effort. The family determines which launch mechanism applies (see
Launch mechanism). `sota` marks models trusted with critical review and the orchestrator role. The inventory, with the
calibration history behind it, is in `inventory.md` next to this file; the families and the `sota` marks are:

| family | models (effort words)                                                                | sota             |
| ------ | ------------------------------------------------------------------------------------ | ---------------- |
| gpt    | gpt-5.6-luna (medium, high), gpt-5.6-terra (medium), gpt-5.6-sol (low, medium, high) | gpt-5.6-sol high |
| claude | haiku-4.5 (high), sonnet-5 (low, medium, high), opus-5 (high), fable-5 (high)        | fable-5 high     |
| muse   | muse-spark-1.3-contributor (low, medium, high, xhigh)                                | none             |
| glm    | glm-5.3-flash (low, high, max)                                                       | none             |

Availability and user overrides may remove or replace these defaults; see Local availability.

The muse family is Meta's Muse Code harness and its Muse Spark model. It is an option, not a default: never route to it
on your own initiative. It enters a route only through an explicit `muse` preference, a user request naming it, or a
deliberate cross-family decision the caller announces as such. Its profile placements below are provisional, and until
real use calibrates them neither its benchmark tier nor its token price counts as a reason to cross families on your own
(`inventory.md`, under "Evidence behind the rules in SKILL.md", says what the placements rest on). It carries no `sota`
mark, so it is never the sole critical-review gate, and no muse route ends at a trusted endpoint (see Escalation facts).

The glm family is Z.ai's GLM-5.3-Flash driven through the OpenCode harness, `opencode run`. Everything said about muse
above applies to it unchanged: opt-in only, via a `glm` preference, a request naming it, or an announced cross-family
decision; provisional placements; no `sota` mark; no route ending at a trusted endpoint. Its price is not a reason for
an unprompted cross-family route until real use has calibrated it. The three effort levels are the only ones Z.ai
accepts for this model.

## Work profiles

Classify the task before choosing a model. Use the primary for the orchestrator's family when it is suitable and
available, except that workhorse writers — tree-editing delegates under the mechanical or clear-spec implementation
profile — default to luna from any family (see Native-path bias). Move to the escalation model after a substantive
failure or when the task proves more demanding than its initial classification.

| profile                   | use when                                                                                                                                                                                      | GPT route                                  | Claude route                    | Muse route                                                          | GLM route                              |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ | ------------------------------- | ------------------------------------------------------------------- | -------------------------------------- |
| mechanical                | Deterministic tool use, searches, log scans, or tedious verified churn                                                                                                                        | gpt-5.6-luna medium → gpt-5.6-terra medium | haiku-4.5 high → sonnet-5 low   | muse-spark-1.3-contributor low → muse-spark-1.3-contributor medium  | glm-5.3-flash low → glm-5.3-flash high |
| routine authored          | Producing or editing small prose/code where baseline taste matters                                                                                                                            | gpt-5.6-sol low → gpt-5.6-sol medium       | sonnet-5 low → sonnet-5 medium  | muse-spark-1.3-contributor medium → muse-spark-1.3-contributor high | glm-5.3-flash high → glm-5.3-flash max |
| clear-spec implementation | Bounded implementation with strong acceptance checks                                                                                                                                          | gpt-5.6-luna medium → gpt-5.6-terra medium | sonnet-5 medium → sonnet-5 high | muse-spark-1.3-contributor medium → muse-spark-1.3-contributor high | glm-5.3-flash high → glm-5.3-flash max |
| complex implementation    | Cross-cutting behavior, difficult debugging, or ambiguity that survives the caller's decomposition — design settled by the caller first, delegated for volume of input more than for judgment | gpt-5.6-terra medium → gpt-5.6-sol medium  | sonnet-5 high → opus-5 high     | muse-spark-1.3-contributor high → muse-spark-1.3-contributor xhigh  | glm-5.3-flash max                      |
| design and synthesis      | API design, architecture, nuanced copy, or competing tradeoffs — the orchestrator's own work, not a delegation (see below)                                                                    | orchestrator (visual output: opus-5 high)  | orchestrator                    | none                                                                | none                                   |
| focused review            | Idiomaticity, AI slop, or docs/comment correctness; also GPT data-flow and edge-input correctness lenses with strong general correctness coverage elsewhere in the panel                      | gpt-5.6-luna high → gpt-5.6-sol high       | sonnet-5 high → opus-5 high     | muse-spark-1.3-contributor high → muse-spark-1.3-contributor xhigh  | glm-5.3-flash max                      |
| mechanical review         | Other non-critical review: simplification, style, prose, or patterns                                                                                                                          | gpt-5.6-sol medium → gpt-5.6-sol high      | sonnet-5 high → opus-5 high     | muse-spark-1.3-contributor high → muse-spark-1.3-contributor xhigh  | glm-5.3-flash max                      |
| critical review           | Correctness, security, concurrency, data integrity, or test-quality gate                                                                                                                      | gpt-5.6-sol high                           | fable-5 high                    | none                                                                | none                                   |

Each cell lists the primary and then the escalation rung. The two-step routes list the common path, not the whole
ladder: if terra also fails after a luna failure, gpt-5.6-sol medium is the remaining rung before the route is
exhausted. The rationale behind the placements (why reviews route above similarly sized implementation work, why test
quality is critical, what the eval behind the luna default measured) is in `inventory.md`; read it when calibrating the
table, not when answering a request.

Focused review does not narrow the review charter. A data-flow or edge-input lens still reports correctness defects
outside its focus. Use critical review for standalone correctness reviews, general, state/lifecycle, and systems
correctness lenses, security, test quality, SPEC compliance, and final acceptance. Idiomaticity, AI slop, and
docs/comment correctness use the focused row in every family. Data-flow and edge-input correctness use the critical row
outside GPT. The GPT focused-review route escalates directly from luna high to sol high, without the implementation
ladder through terra. Provider preference and fixed native mechanisms retain their ordinary precedence.

Design is the orchestrator's own work. Deciding an API shape, an architecture call, or a tradeoff is exactly what the
expensive model's capability is for; handing the decision to a weaker model buys a worse answer than the orchestrator
would have produced and then charges it to review the result. The `orchestrator` entries cover the decision, not the
survey: gathering the inputs to a decision is mechanics, and a read-only "survey this subsystem and lay out the options
with their tradeoffs" delegation routes like any other read-only unit. Two edge cases: if the session is not itself
running a `sota`-marked model, treat design the way critical review is treated — route the decision up to the strongest
available model rather than keeping it by default — and when another skill's process spawns a design-shaped subagent,
that spawn is process, not routing: it is `inherit`, at the session's own model, unless that process demands otherwise.
The one case where design output itself is delegated is visual: a GPT session producing UI, frontend styling, slides, or
anything judged by how it looks hands that to opus-5 high — the table's GPT cell says so, and `inventory.md` says why
under "Evidence behind the rules in SKILL.md" — and that is a sufficient reason to diverge from a `gpt` preference,
announced as a divergence. A Claude session does its own visual design.

Long context is an exception to the luna routes, from every orchestrator. gpt-5.6-luna's long-context retrieval is far
below the rest of the family (reported around 41% on MRCR versus roughly 90% for terra and sol), so a mechanical or
clear-spec task whose input is genuinely large — whole-repo scans, big log files, long-document analysis, a change that
has to be reasoned across many files at once — starts at gpt-5.6-terra medium instead of luna, escalating to gpt-5.6-sol
medium; a Claude session shelling out to luna under the workhorse default shells out to terra for these instead, rather
than reverting to its native route. This is about input size the model must actually reason across (input 5), not task
difficulty; small-input work stays on luna, and when the caller cannot tell in advance, luna first is the right bet — a
reroute after a lost-context failure costs one cheap run. A luna delegate that loses track of earlier context mid-task
is this weakness surfacing, not a generic substantive failure: the outcome `substantive failure (lost context)` reroutes
to terra without counting against the profile. For the GPT focused-review route, large input or a lost-context failure
instead routes directly to sol high, without counting a lost-context failure as an attempt.

## Native-path bias

When models are roughly equally suitable, prefer the model in the orchestrator's family. Same-family delegates normally
stay inside the current harness; crossing families adds process startup, context transfer, authentication, permission,
output-handling, and failure overhead. The native delegation path is the real reason for the preference. If a harness
can invoke another family natively, prefer the native path rather than following family names mechanically.

The bias scales with expected size (input 4): very strong for tiny, strong for short, moderate for medium, weak for
large. The size anchors, what each size means in practice, are in `native-path.md` next to this file; read it the first
time a request's size is in doubt.

Do not select an unsuitable model merely to stay native. Cross families when the native family has no suitable model,
the other family is materially better for the task, a medium or large task has materially lower expected total cost on
the other route, a native attempt failed, the user requested that provider, or an independent cross-family perspective
is part of the goal (input 9). Expected total cost includes likely tokens, latency, review burden, and escalation risk —
not nominal token price alone. For critical work, reliability and useful independence take priority over the size-based
bias.

The bias does not apply to workhorse writers. For delegates that edit the tree under the mechanical or clear-spec
implementation profile, the default from any orchestrator is gpt-5.6-luna medium, reached however the session reaches a
gpt model per Launch mechanism — `codex exec` from a Claude session, the native sub agent mechanism from a Codex session
— unless a provider preference says otherwise, the path to a gpt model is unavailable (no `codex` on `PATH` from a
non-Codex session), or the config file rules the model out. Read-only mechanical work (searches, scans, log reading) is
not covered: there the ordinary bias stands, and a Claude session's cheap native fan-out (haiku) beats paying shell-out
launch and monitoring overhead per delegate. A "short" writer task crosses families under this rule; a "tiny" one is
still `orchestrator`. Escalation after a luna failure follows the GPT route (terra medium, then sol medium), not the
same-family column; the same-family routes are what a session uses when the gpt path is unavailable or a preference
directs it there. A user who would rather not cross families by default says `prefer-claude` or writes the config file.
This is the one place routing treats a specific model as the default across harnesses; what it rests on, and what would
justify revisiting it, is in `inventory.md` under "Evidence behind the rules in SKILL.md".

## Escalation facts

Routing never decides to escalate; the caller does, from its own outcome. What routing supplies is the next rung and two
facts about it. `route exhausted: yes` means the route just answered is the last rung its family offers for this profile
(the two-step routes plus the sol-medium third rung on the GPT implementation ladders); a substantive failure there is
not answered by another rung — the caller handles the work itself or makes a deliberate cross-family attempt; routing
will not hand back the same model. `endpoint trusted` is derived from that last model's `sota` mark, not from its
family: yes for the critical-review routes and any other route whose last rung is gpt-5.6-sol high or fable-5 high, no
for the rest — every muse and glm route, since those families carry no `sota` model, and a GPT or Claude route whose
ladder ends below `sota`. The flag tells the caller how much the exhausted rung's own judgment can be trusted; it does
not change the rule that an exhausted route, trusted or not, is never retried mechanically. A first attempt on a route
with a rung left says `route exhausted: no`.

## Provider preference

The caller may pass a preference for `gpt`, `claude`, `muse`, or `glm` (input 10), as parsed from the user's
`prefer-<family>` keyword. This expresses a preference unrelated to model performance — typically the user has a large
subscription with one provider and a small one with the others, and wants spend steered accordingly. The default, absent
a preference, is none.

When a preference is given, route every delegation to the preferred family unless there is a very clear, strong reason
to diverge — for example repeated poor output from the preferred family (which the caller reports in input 12, since
routing does not remember earlier requests), no suitable model for the selected profile, a goal that explicitly needs an
independent cross-family perspective, or a rule above it in Precedence. An explicit provider preference overrides the
default native-path bias. Every divergence is stated in the answer as `diverged from preference: yes` with the reason,
so the caller can tell the user.

## Local availability

The inventory and profiles describe models that exist; they do not know which ones the user can access in this
environment. Before answering a request, check for the model routing config file, `~/.scode-model-routing.md`. If it
exists, read `config-file.md` next to this file and then honor the config file: it contains natural language adjustments
from the user, most commonly availability restrictions like "fable-5 is not available, do not use" or "only claude
models work here", and may replace the inventory or the profile table outright. Treat its contents as authoritative.
Remove unavailable models from every profile and use the next suitable option rather than preserving a preferred slot
mechanically. Apart from the seeding request described in `config-file.md`, do not create or edit the config file
yourself; it belongs to the user. If the config file does not exist but the file it replaced,
`~/.scode-galaxy-brainrc.md`, does, stop and tell the user to rename it: the old file is not read, and answering as if
no config existed would silently route to models the user said are unavailable.

If the config file does not exist, all inventory models are assumed available, with one practical exception: a
shelled-out family whose CLI is not installed (`codex`, `claude`, `muse`, or `opencode` missing from `PATH`) is
unavailable regardless of the config file, and so is the glm family when `opencode providers list` shows no credential
for the provider the model id names — `opencode run` with a model it cannot reach fails immediately, but with no model
at all it hangs, so check rather than probe. Treat either as a missing family when honoring a provider preference — the
answer diverges and says so — rather than as a launch failure for the caller to retry.

## Launch mechanism

The mechanism half of an answer comes from the session's harness and the route's family. Stay native within the
session's own harness; shell out only when crossing vendors:

- **Claude Code session → claude model**: `native` (the session's sub agent mechanism, with the model parameter set to
  the target model).
- **Claude Code session → gpt model**: `codex exec`.
- **Codex session → gpt model**: `native` (the session's sub agent mechanism, specifying the target model).
- **Codex session → claude model**: `claude -p`.
- **Any session → muse model**: `muse exec`. There is no native muse path, because Muse Code is never the orchestrator
  of record.
- **Any session → glm model**: `opencode run`, always with the unrestricted default build agent and `task` available.
  OpenCode is likewise never the orchestrator of record.

When the mechanism is `native`, the caller also sets the target reasoning effort if its sub agent mechanism has an
effort parameter; otherwise sub agents inherit the session's effort and that is acceptable. A writer needs a mechanism
that can resume the same delegate with a follow-up message (input 11); on a harness whose native sub agents cannot, the
answer for a writer is that family's shell-out mechanism instead — the one case where a session shells out to its own
family — and the reason says so.

## Composing with another skill's process

When another skill or instruction defines its own process — roles, steps, what counts as a valid run — that skill stays
authoritative for the process, and routing answers only the choices it leaves open. If it explicitly demands a specific
model, agent type, or effort for a spawn, that demand is process, not a routing default: the answer honors it and the
reason attributes the choice to that skill, so the caller announces it on that skill's terms. A demanded agent type
passes through untouched: routing has no field for it, and the caller applies it at the spawn exactly as demanded.

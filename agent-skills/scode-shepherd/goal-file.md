# Running a scode-build-goal goal file

This file applies only when the goal file you were given says it was written by `scode-build-goal` in shepherd mode.
Such a file says so in its first lines, and its execution mode section requires `$scode-shepherd` for the whole goal and
names a run directory. Every other goal file, including a `scode-build-goal` goal file written for another mode and a
hand-written one that mentions `scode-build-goal`, gets the ordinary flow in `SKILL.md`, approval included, and none of
this file.

A shepherd-mode goal file was planned with the user: it records their decisions, an implementation outline that a
fresh-context review has already checked for scope, the reviewer they chose, and usually the worker. It was written to
run unattended, and its outline is deliberately not a PR-by-PR plan; the detailed decomposition is yours, at execution
time. Everything in `SKILL.md` holds except where this file says otherwise. Where they differ, this file governs for the
whole run, including after compaction or in a fresh session.

## What counts as a departure

The goal file separates three sources of requirements: the user's request and decisions, binding repository constraints,
and the planner's own implementation choices. Only the first two bind you. A departure is a plan or replan that leaves a
user requirement or repository constraint uncovered, contradicts a recorded decision, expands the scope materially
beyond what the goal asks for, weakens a guarantee the goal states, or needs a demand you cannot honor (an unservable
reviewer, say). A departure needs a fallback the goal file already agreed, or the user's decision: ask, and say what the
tradeoff is. A review finding or a DECISION entry is not authorization.

Everything else is yours, at planning time and at every replan alike: how the outline is cut into items, items split,
merged, reordered, or rescoped within it, and the planner's choices, which you may change or drop (the goal file's
fallback says to remove unnecessary planner-invented machinery) with a DECISION entry in the journal saying why. Prefer
a plan without departures: a departure that only re-argues a recorded decision (the user already weighed it) is not
worth stopping the run for, so drop it and log a DECISION saying so. Ask only when you have a concrete reason the goal
file did not weigh.

## What changes when you start

- **The run directory** is the absolute path the goal file names, not one you choose. Use it only if it is outside the
  checkout and is this goal's: it does not exist yet (create it), or its journal opens with this goal file's absolute
  path. If it exists without that journal line, or lies inside the checkout, it is not this goal's; ask the user and
  write nothing in it. If it is this goal's and holds `state.json`, the run already started: resume it as "Resuming" in
  `SKILL.md` says instead of planning again. Without `state.json`, an earlier planning session was cut off; read the
  journal and plan in it.
- **Headless,** the run directory is the driver's, and the ownership check above does not apply to it: the driver
  created it and wrote `goal.md` before your first turn, so a driver's directory without a journal is a new run, and one
  whose journal you wrote is this run. If the goal file names a different one (compare the resolved paths), and the
  named directory already holds a journal or `state.json`, write a `stop` that names both: a second run directory would
  split that run's history, and the fix is starting the drive again on the right directory, which no answer inside this
  drive can do. Write nothing else in either directory. If the named directory holds neither, use the driver's and log a
  DECISION naming both.
- **Approval.** The user approved the decisions, the outline, the reviewer, and the worker when the goal was written,
  and asked for a run that does not wait on them. So a plan without departures needs no second approval: instead of step
  6 of "Starting a run", write the summary step 6 lists into the journal, and in an interactive session show it to the
  user in the same message that says the run is underway. Headless, this is the exception to "only an answer that
  approves lets you run `init`": go on to `init` without an `ask`. A plan with a departure gets step 6 (headless, an
  `ask`), with the departure named in the question.
- **The journal** in the run directory is the goal's working log. The goal file names no `agent-resumeable` log for this
  mode; the run directory, its journal, and `status` are the resume record. Start the journal as soon as the run
  directory exists, before planning (step 8 of "Starting a run" comes early here), so that a planning session that is
  cut off leaves a trail. Open it with the goal file's absolute path (headless, the path of the run directory's
  `goal.md`) and a line saying this file governs the run, so that a session resuming from the journal alone reads it
  too.

## Building plan.json from the goal file

- `goal`: the goal file's goal and acceptance criteria, verbatim.
- `context`: what every item shares, taken from the goal file: the outline, the decisions that bind the items (with
  their reasoning, so a worker does not quietly undo one), the binding repository constraints it cites, and what is out
  of scope. A worker sees only this, its own item, and the stack's titles, so leave nothing out that an item depends on.
- `items`: your decomposition of the outline into one reviewable PR each, sized as the goal file's PR discipline says.
- `workflow`: the goal file's PR discipline for version control (normally a `jjstack` stack).
- `workerRules`: the goal file's rules that bind a worker within its item, one line each. From its PR discipline: no
  churn (code a later item would delete means the stack is shaped wrong, so report replan instead of writing it), and
  restructure the stack rather than stack a correction on top (the runner's precedence turns that into a replan once a
  PR is approved). From its unattended fallback: resolve routine forks within the item's scope; report blocked, with the
  tradeoff, for anything that would expand scope materially, weaken a guarantee, or leave out required behavior.
- `reviewer`: the model and effort of the goal file's review gate demand. v1 runs every review through `codex exec`, so
  the demand must name a model Codex serves (a `gpt-` id). A demand for any other model, or for a review swarm, cannot
  be honored here; do not substitute a reviewer, ask.
- `reviewCharter`: the review gate's charter from the goal file, verbatim. Its instruction to write findings to a file
  is fine to keep; the runner overrides it.
- `worker`: the goal file's worker demand; the defaults when it makes none.
- `checks`: the gates the goal file cites from the repository (its `AGENTS.md`, CI) and any validation the goal file
  demands for every PR. Targeted tests for one area go in that item's `checks`. All of them run unattended, as "The
  plan" in `SKILL.md` says.
- `limits`: the defaults, unless the goal file sets them.

## Rules from the goal file that bind you

- **Scope reassessment.** Run the goal file's scope reassessment before you plan or replan around a substantial
  departure from the outline, as the goal file describes one (a new subsystem, parser, persistent state, or
  compatibility layer, not a split or a helper that reorganizes code the outline already names), and before running
  again after one item has escalated `review-rounds-exhausted` (repeated corrective reviews of the same component call
  its necessity into question). Run it as the goal file says, with the planning charter it carries, through a read-only
  fresh-context sub agent at your own model, given the request, the decisions, the outline, the PRs so far, and the
  proposed change.
- **Replans.** A replan without a departure is yours: make it, and log it as a DECISION in the journal. This replaces
  the rule in `SKILL.md` that a change to the stack's shape goes to the user. Approved PRs are frozen, so the goal
  file's "restructure the stack instead of stacking a correction on top" cannot reach one: a correction to an approved
  PR goes into the next unapproved item, or a new item after it, logged as a DECISION. That is the runner's constraint,
  not a departure.
- **A decision only the user can make.** The runner cannot skip an item, so the goal file's "pause only dependent work"
  means this: move unapproved items that do not depend on the open decision ahead of the item that does, when the
  outline allows, and ask about that item when the run reaches it. Otherwise ask when the decision comes up.
- **The worker and reviewer models are the user's** when the goal file demands them. Changing either for the rest of the
  run, which `SKILL.md` otherwise lets you do after `sessions-exhausted` or `review-rounds-exhausted`, is a departure:
  ask. A worker the goal file does not name, or names only as the default, is yours to change as `SKILL.md` says.
  Guidance and a takeover stay yours.
- **Decision logging.** Log every consequential choice in the journal under a DECISION label, with the alternatives you
  considered: the decomposition, every replan, guidance that changes how an item is built, and every takeover. The user
  will ask for these later to revisit them.
- **Done.** The goal file's done criterion is a stack of open PRs that together achieve the goal. That is what
  "Finishing" in `SKILL.md` reports; the PRs stay drafts, and merging is the user's.

## What the goal file says that does not apply

- Its resource watchdog requirement is met by the runner, which holds off before launches and kills a step when disk or
  memory runs short, and escalates `resources-low` (see the escalation table in `SKILL.md`). Do not start a watchdog of
  your own. When you take an item over, you are the one launching builds: check free disk and memory before you start.
- Its execution mode forbids `scode-galaxy-brain`, `scode-powerhorse`, and `scode-model-routing`, which is what "Other
  orchestration skills" in `SKILL.md` says anyway.

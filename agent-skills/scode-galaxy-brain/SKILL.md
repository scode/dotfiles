---
name: scode-galaxy-brain
description: >
  Accomplish a goal by delegating suitable parts of the work to cheaper models while the current session stays in
  charge of planning, quality gating, and all commit/PR management. Use when the user explicitly invokes
  scode-galaxy-brain, e.g. "Use scode-galaxy-brain to <goal>", optionally with a prefer-gpt or prefer-claude keyword.
  Also use when the user says "galaxy brain feedback: ..." to record feedback about how this skill performed. Once
  invoked, the skill stays active for the rest of the session — including across context compaction and resume — until
  the user expressly stops it, unless the invocation itself limited the scope up front; if retained context says it was
  active, re-read this skill before delegating.
---

# Scode Galaxy Brain

## Premise

You — the current session — are running on a state of the art, expensive model. The point of this skill is to spend that
capability where it matters (planning, judgment, design, quality control) and route everything else to cheaper models
without significantly compromising the quality of the final output. You stay in charge the whole time: you decompose the
goal, you decide what to delegate, you judge every result, and you own the overall change.

The goal is cost-effective quality; parallelize freely when it helps. The one hard limit is writes: delegates share the
working tree and this skill deliberately ships without write-concurrency tooling (worktrees and the like), so tasks that
write to the tree run one at a time (see Concurrency below).

Delegation is for steps toward a change, never for managing the change itself. You always own version control: commits,
branches, pushes, PR creation and updates. Delegates must not commit, branch, push, or open PRs — your session carries
the user's VCS workflow preferences and skills, and those do not transfer to a sub agent.

## Composing with other skills

Galaxy-brain is a routing layer, not a workflow. When another skill or instruction defines its own process — roles,
steps, what counts as a valid run — that skill stays authoritative for the process. Galaxy-brain only decides which
model and effort executes each unit of work, how the delegation seam works, and how delegated output gets gated.

Keep the roles separate: do not merge this skill's orchestrator role with another skill's coordinator role, and do not
attribute one skill's constraints to the other. If another skill forbids or requires something, that rule comes from
that skill; reason about it (and explain it to the user) on that skill's terms.

Routing authority covers nested delegation too. When another active skill's process calls for spawning subagents —
reviewers, workers, whatever it names them — each spawn is still a galaxy-brain delegation: pick the model from the
table, set model and effort explicitly where the spawn mechanism supports it, and announce the choice as usual. The trap
is following the other skill's spawn instructions verbatim and letting its subagents silently inherit this session's
expensive model. Inheriting is fine only as a deliberate routing decision, stated as such. This claims only the choices
the other skill leaves open: if it explicitly demands a specific model, agent type, or effort for a spawn, that demand
is process, not a routing default — honor it like any other rule that skill owns, and attribute the choice to that skill
when you announce it.

## Staying active for the whole session

Activation is session-scoped, not turn- or task-scoped. Once the user invokes this skill, keep routing every delegation
through it — including spawns triggered by other skills, and including later tasks the user never mentions the skill on
— for the rest of the session. Only two things end it: the user expressly asking to stop, or an invocation that limited
the scope up front ("use scode-galaxy-brain for <this one thing>"). Finishing the task it was invoked for does not.
Context compaction, session resume, a tool restart, or a summary that fails to mention the skill does not end it either;
treat retained context that has gone quiet about galaxy-brain as a summarization artifact, not a decision anyone made.

After compaction or resume, if the retained context says or implies this skill was active, re-read this SKILL.md and
`~/.scode-galaxy-brainrc.md` (if it exists) before doing further substantive work — the routing rules do not survive
summarization reliably. If the retained context is ambiguous but mentions outstanding delegated work, model or effort
routing, or galaxy-brain at all, assume the skill is still active and say that you are assuming it.

When you write a handoff or pre-compaction note while this skill is active, include the routing-layer state: the current
goal, any provider preference, rc-file assumptions, delegations still in flight, and the next routing decision. Do this
even when no delegate is currently running — between delegations is exactly when a summary is most likely to drop the
skill. This is a backstop, not the mechanism: stickiness applies whether or not a handoff was ever written.

## Model table

Cost is a relative score of what the model costs to run (higher = cheaper). Intelligence is how hard a problem you can
hand the model unsupervised. Taste covers UI/UX, code quality, API design, and copy. Each entry names a model at a
specific reasoning effort — run it at that effort. The family column tells you which delegation path from the mechanics
section applies. The sota column marks state of the art models: the ones trusted with the hardest work (critical review,
the orchestrator role itself). More than one model can be state of the art at once, across families, and which of them
the user can access varies by environment (see Local availability below).

A larger number is better on every dimension; for cost that means cheaper.

| model                | family | sota | cost | intelligence | taste |
| -------------------- | ------ | ---- | ---- | ------------ | ----- |
| gpt-5.6-luna medium  | gpt    |      | 12   | 4            | 5     |
| gpt-5.6-terra medium | gpt    |      | 10   | 6            | 6     |
| gpt-5.6-sol low      | gpt    |      | 5    | 7            | 7     |
| gpt-5.6-sol medium   | gpt    |      | 4    | 8            | 8     |
| gpt-5.6-sol high     | gpt    | yes  | 2    | 9            | 8     |
| sonnet-5 high        | claude |      | 5    | 5            | 7     |
| opus-4.8 high        | claude |      | 4    | 7            | 8     |
| fable-5 high         | claude | yes  | 1    | 9            | 9     |

How to route:

- Prefer the cheapest model (highest cost score) whose intelligence and taste meet the needs of the task.
- When a task calls for a state of the art model and more than one is marked sota, prefer the one you are yourself
  running as: the user chose it for this session, which signals both availability and preference in this environment.
  Diverge only for a concrete reason (Local availability below, an explicit provider preference, or repeated poor output
  on the task at hand). If the model you are running is not in the table, apply the normal cheapest-qualified rule
  within the sota rows.
- Set the intelligence bar by the cost of a missed or wrong result, not only by how hard the task looks. Cheap models
  are fine for producing work because you gate the output and defects get caught. Review and verification tasks are
  themselves the gate — there is no backstop behind them — so a missed finding is unrecoverable and criticality, not
  task mechanics, drives the model choice.
- Bulk and mechanical work — scanning large logs for patterns, searching source for simple patterns, clear-spec
  implementation, tedious churn that needs no design decisions: use the cheapest model whose intelligence clears the
  task.
- Anything user-facing (UI, copy, API design) needs taste ≥ 7.
- Mechanical review dimensions — slop detection, style and idiomaticity, docs drift, best-practice pattern checks: use
  the cheapest model with intelligence ≥ 8.
- Critical review dimensions — security, correctness, concurrency, data integrity, test quality: use a sota-marked
  model. Do not route these to a non-sota model on cost. Optionally add the cheapest model with intelligence ≥ 8 as an
  extra independent perspective. Test quality is critical rather than mechanical because weak tests are how correctness
  bugs survive review.
- Never use Haiku.
- These are defaults, not limits. You have standing permission to override them: if a cheaper model's output doesn't
  meet the bar, rerun or redo the work with a smarter model without asking. Judge the output, not the price tag. The
  same goes preemptively — if mid-task you realize the work needs more intelligence or taste than you thought (what
  looked mechanical turns out to be API design), reroute or do it yourself without asking.
- Some work isn't worth delegating at all: if writing the task spec and reviewing the result costs more than doing the
  task, just do it.
- Every time you delegate, tell the user which model and effort you picked for that task and why, in one sentence tied
  to the table's dimensions (e.g. "mechanical rename across many files, no design decisions — gpt-5.6-terra medium").

The model names in these rules are role fillers drawn from the default table, not fixed bindings: bulk and mechanical
work mean the cheapest model whose intelligence clears the bar, and critical review means a sota-marked model. When the
table changes (see Local availability), reapply each rule to whichever model now fills its role.

## Provider preference

The invocation may include the keyword `prefer-gpt` or `prefer-claude`. This expresses a preference unrelated to model
performance — typically the user has a large subscription with one provider and a small one with the other, and wants
spend steered accordingly. The default, absent a keyword, is no preference.

When a preference is given, route every delegation to the preferred family unless there is a very clear, strong reason
to diverge — for example the preferred provider's models repeatedly produce poor output on a specific task, or the task
demands intelligence or taste that no model in the preferred family has. How strongly to hold the preference in edge
cases is your judgment call, but a mild "the other model rates a point higher" is not enough to diverge. When you do
diverge, say so and why.

## Local availability

The table describes models that exist; it does not know which ones the user can access in this environment. Before your
first delegation, check for `~/.scode-galaxy-brainrc.md`. If it exists, read it and honor it: it contains natural
language adjustments from the user, most commonly availability restrictions like "fable-5 is not available, do not use"
or "only claude models work here". Treat its contents as authoritative over the table — an excluded model is simply not
in the table for this session, and every routing rule (including "do not route down on cost") applies to the models that
remain.

The rc file may go further than exclusions and supply its own model table. The intended workflow is copy-paste: the user
copies the default table out of this file and edits it — same columns, and crucially the same scales, since the scores
are relative and the routing thresholds (taste ≥ 7, cost comparisons, the sota mark) only mean anything against the
default table's calibration. A table in the rc file replaces the default table wholesale: models it omits do not exist
this session even without an explicit "not available" line, and its scores and sota markings drive routing exactly as
the default table's would. Remap the role-based routing rules onto what the replacement contains. Its model names are
the ids you actually invoke — pass them to codex's `-m` verbatim (minus the trailing effort word); for the claude path,
map to the nearest `--model` alias. If the model your own session runs as is absent, that only bars delegating to it —
you keep orchestrating, and the "prefer the one you are yourself running as" tie-break simply drops out. If the table
deviates from this shape (missing columns, an unfamiliar scale), do your best to interpret it in the spirit of the
default table rather than rejecting it, and tell the user what you assumed.

To spare the user the manual copy-paste, they can ask you to seed the file — "populate my galaxy-brain rc with the
default table" or words to that effect. On that explicit request (and only then), write the current default table into
`~/.scode-galaxy-brainrc.md`, preceded by a one-line note saying the table replaces the skill's default wholesale and is
meant to be edited. Never discard existing content: if the file already exists, append the table to it, and if it
already contains a model table, stop and ask instead of writing a second one.

If the file does not exist, all table models are assumed available. Apart from the seeding request above, do not create
or edit this file yourself; it belongs to the user.

## Delegation mechanics

Stay native within your own harness; shell out only when crossing vendors. First figure out which harness you are
running in, then:

- **Claude session → claude model**: use your native sub agent mechanism (e.g. the Agent/Task tool) with the model
  parameter set to the target model.
- **Claude session → gpt model**: shell out to `codex exec` (see below).
- **Codex session → gpt model**: use your native sub agent mechanism, specifying the target model.
- **Codex session → claude model**: shell out to `claude -p` (see below).

When delegating natively, also set the target reasoning effort if your sub agent mechanism has an effort parameter;
otherwise sub agents inherit the session's effort and that is acceptable.

Name every sub agent (label, description, or whatever your mechanism displays) so the name includes the task plus the
model and effort actually doing the work, e.g. `fix-foo-gpt-5.6-sol-medium`. Harness UIs otherwise show only the wrapper
or default model, which misleads anyone watching progress.

### Shelling out to codex

```sh
codex exec --yolo -m gpt-5.6-sol -c model_reasoning_effort=high -o <scratch-file> "<prompt>"
```

- Reasoning effort is set with `-c model_reasoning_effort=<low|medium|high>`. Always pass it explicitly rather than
  relying on the user's config default; the startup header echoes the effective `reasoning effort:` if you need to
  confirm.
- `-o` writes the agent's final message to a file; read that file for the result instead of parsing stdout.
- Runs in the current working directory by default; pass `-C <dir>` to target elsewhere.
- Long tasks can exceed your shell tool's default timeout. Set an explicit generous timeout, or run in the background
  and wait for completion.

### Shelling out to claude

```sh
claude -p --model <alias> --effort <level> --dangerously-skip-permissions "<prompt>"
```

- Model aliases: `sonnet`, `opus`, `haiku`, `fable`. Effort levels: `low`, `medium`, `high`, `xhigh`, `max`. The final
  response is printed to stdout.
- The same timeout caveat applies.

### Writing the task spec

The delegate has none of your conversation context. Every delegation prompt must be self-contained:

- The goal and any constraints that bound it.
- Exact file paths or directories in scope.
- Acceptance criteria: what done looks like, concretely.
- Which checks to run (tests, linters, formatters) before reporting back.
- For read-only tasks: state explicitly that it must not edit any files.
- Always: no commits, no branches, no pushes, no PRs.
- Ask it to report what it did and call out any deviations from the spec.

Before delegating a task that writes to the tree, note the current `git status`/`git diff` state so you can attribute
the delegate's changes cleanly afterwards.

## Concurrency

- Read-only tasks (log scanning, code search, independent reviews) may run concurrently whenever they are independent of
  each other. Use this freely for fan-out work like scanning many logs or directories.
- Anything that writes to the working tree runs one at a time. Delegates share the tree and there is no worktree tooling
  in this skill, so never run two writers concurrently.

## The gate

You are the quality gate for everything a delegate produces. Never accept a delegate's self-report as evidence the work
is good.

For code output:

1. Inspect the actual diff (`git diff`) yourself, not just the report.
2. Re-run the relevant checks yourself.
3. Then make a judgment call:
   - Small defects (naming, comments, minor logic): fix them yourself — a fixup round-trip costs more than doing it.
   - Substantive but well-specified defects: send one fixup round back to the same model with a precise list of defects.
   - After about two failed rounds, or when the output shows the task needed more intelligence or taste than the model
     has: stop sending it back. Do it yourself or re-delegate to a higher-rated model, without asking the user.

For read-only findings (reviews, scans, analysis): spot-verify against the cited code or data before relaying. When
reporting to the user, separate what you confirmed from delegate claims you did not verify.

In your final report to the user, briefly note which parts were delegated and to which models.

## Feedback capture

When the user says "galaxy brain feedback: ..." (or clearly signals feedback about how this skill performed), pause
whatever you are doing and record the feedback before resuming. The record exists so the skill's author can later hand
it to an agent working in the skill's source repository and ask for improvements — write it with that reader in mind.

Append (never overwrite) a markdown entry to `$XDG_STATE_HOME/scode-galaxy-brain/feedback.md`, defaulting to
`~/.local/state/scode-galaxy-brain/feedback.md` when `XDG_STATE_HOME` is unset. Create the directory if needed. After
writing, tell the user explicitly which file you appended to.

Each entry should be self-contained — the future reader has no access to this session:

- A `## <date> — <short title>` heading.
- The user's feedback, verbatim or near-verbatim.
- What you were doing when the problem occurred: the task, which model and delegation path was involved, the actual
  commands or prompts where relevant, and what went wrong (exact errors beat paraphrases).
- Your own analysis if you have one: root cause, and what change to the skill instructions would have prevented the
  problem. Mark speculation as such.

Avoid including private information (credentials, personal data), but do not sacrifice clarity of the problem
description to scrub aggressively — the user reviews the file before forwarding it anywhere.

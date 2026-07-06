---
name: scode-galaxy-brain
description: Accomplish a goal by delegating suitable parts of the work to cheaper models while the current session stays in charge of planning, quality gating, and all commit/PR management. Use when the user explicitly invokes scode-galaxy-brain, e.g. "Use scode-galaxy-brain to <goal>", optionally with a prefer-gpt or prefer-claude keyword. Also use when the user says "galaxy brain feedback: ..." to record feedback about how this skill performed.
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

## Model table

Cost is a relative score of what the model costs to run (higher = cheaper). Intelligence is how hard a problem you can
hand the model unsupervised. Taste covers UI/UX, code quality, API design, and copy. Each entry names a model at a
specific reasoning effort — run it at that effort. The family column tells you which delegation path from the mechanics
section applies. The sota column marks state of the art models: the ones trusted with the hardest work (critical review,
the orchestrator role itself). More than one model can be state of the art at once, across families, and which of them
the user can access varies by environment (see Local availability below).

A larger number is better on every dimension; for cost that means cheaper.

| model         | family | sota | cost | intelligence | taste |
| ------------- | ------ | ---- | ---- | ------------ | ----- |
| gpt-5.5 low   | gpt    |      | 11   | 5            | 4     |
| gpt-5.5 high  | gpt    |      | 9    | 8            | 5     |
| sonnet-5 high | claude |      | 5    | 5            | 7     |
| opus-4.8 high | claude |      | 4    | 7            | 8     |
| fable-5 high  | claude | yes  | 2    | 9            | 9     |

How to route:

- Prefer the cheapest model (highest cost score) whose intelligence and taste meet the needs of the task.
- When a task calls for a state of the art model and more than one is marked sota, prefer the one you are yourself
  running as: the user chose it for this session, which signals both availability and preference in this environment.
  Diverge only for a concrete reason (Local availability below, an explicit provider preference, or repeated poor output
  on the task at hand).
- Set the intelligence bar by the cost of a missed or wrong result, not only by how hard the task looks. Cheap models
  are fine for producing work because you gate the output and defects get caught. Review and verification tasks are
  themselves the gate — there is no backstop behind them — so a missed finding is unrecoverable and criticality, not
  task mechanics, drives the model choice.
- Bulk and mechanical work — scanning large logs for patterns, searching source for simple patterns, clear-spec
  implementation, tedious churn that needs no design decisions: gpt-5.5.
- Anything user-facing (UI, copy, API design) needs taste ≥ 7.
- Mechanical review dimensions — slop detection, style and idiomaticity, docs drift, best-practice pattern checks:
  gpt-5.5 high.
- Critical review dimensions — security, correctness, concurrency, data integrity, test quality: fable-5 high. Do not
  route these down on cost. Optionally add gpt-5.5 as an extra independent perspective. Test quality is critical rather
  than mechanical because weak tests are how correctness bugs survive review.
- Never use Haiku.
- These are defaults, not limits. You have standing permission to override them: if a cheaper model's output doesn't
  meet the bar, rerun or redo the work with a smarter model without asking. Judge the output, not the price tag. The
  same goes preemptively — if mid-task you realize the work needs more intelligence or taste than you thought (what
  looked mechanical turns out to be API design), reroute or do it yourself without asking.
- Some work isn't worth delegating at all: if writing the task spec and reviewing the result costs more than doing the
  task, just do it.
- Every time you delegate, tell the user which model and effort you picked for that task and why, in one sentence tied
  to the table's dimensions (e.g. "mechanical rename across many files, no design decisions — gpt-5.5 high").

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
remain. If the file does not exist, all table models are assumed available. Do not create or edit this file yourself; it
belongs to the user.

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
model and effort actually doing the work, e.g. `fix-foo-gpt-5.5-high`. Harness UIs otherwise show only the wrapper or
default model, which misleads anyone watching progress.

### Shelling out to codex

```sh
codex exec --yolo -m gpt-5.5 -c model_reasoning_effort=high -o <scratch-file> "<prompt>"
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

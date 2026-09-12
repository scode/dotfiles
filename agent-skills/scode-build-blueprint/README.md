# scode-build-blueprint

An experiment in separating expensive judgment from routine implementation. A state-of-the-art model explores the
repository and designs a goal up front. A fresh session on a cheaper workhorse model then implements it, asking experts
for help and obtaining independent reviews along the way.

NOTE: This is a prototype, not evidence that the arrangement preserves quality or saves money. The instructions are
agent-followed policy, not harness-enforced permissions or a deterministic budget controller.

## Why try this?

Galaxy Brain keeps the expensive model in charge throughout execution and delegates suitable pieces to cheaper models.
That leaves the expensive session doing coordination, tool use, and repeated decisions about whether to delegate.

This experiment moves the handoff to the whole implementation phase. The expensive model concentrates on exploration,
design, consultation, and review. The workhorse owns the continuous tool loop: edits, tests, straightforward repairs,
progress tracking, and authorized VCS operations. A concrete, repository-grounded blueprint should make that handoff
less ambiguous than asking a cheaper model to drive a general-purpose session.

The initial concern is too little expert involvement, not too much. The executor is encouraged to ask early when expert
judgment could prevent wasted work or protect quality. There are no default expert call-count caps. A blueprint may
include one for a clear goal-specific reason approved by the user, but logging actual use comes before tuning limits.

This remains separate from scode-build-goal and Galaxy Brain so their behavior stays unchanged and the approaches can be
compared. It reuses model routing and harness shellout directly; it owns its own execution and review policy.

## Using it

1. In the target repository, start a session on the model you want doing exploration and design. Invoke
   `$scode-build-blueprint <what you want built>`.
2. Resolve the planner's questions about requirements, design, executor model/effort, allowed workers, expert roles,
   checkpoints, and external permissions. The planner writes the blueprint and stops; it does not implement anything.
3. Start a separate session on the agreed executor model/effort and run `/goal /absolute/path/to/blueprint.md`. `/goal`
   does not choose or change the session's model. Repeat the same command in a later session to resume, after the prior
   executor has stopped.

By default, the planner chooses mnemonic `<project>-<goal>-blueprint.md` and matching `-blueprint-log.md` paths beside
the checkout, in its parent directory outside VCS. It shows the paths before writing; you can choose another location.
The planner never creates the execution log. The executor creates it on first use and reconciles it with repository and
process state on resume. The blueprint requires installed skills but no access to the planning conversation.

Choose the executor in the harness yourself; it does not validate its own model identity or stop for confirmation when
identity metadata is missing or disagrees. Execution assumes you are absent. Routine choices stay autonomous, and
critical blockers go to the approved expert for an in-scope alternative when consultation is possible. Safety, required
gates, and missing authority can still prevent completion.

The Rust dotfiles installer registers this skill for all four harnesses. Initial top-level execution supports Codex and
Claude Code because the shared routing policy currently excludes Muse and OpenCode as orchestrators. They can still host
approved workers. Required delegate CLIs must be installed and authenticated; missing expert capability blocks its
review gate rather than silently choosing a weaker reviewer.

Default delivery is a reviewed stack of open, unmerged draft PRs through jjstack. The blueprint can specify another
approved delivery method. Finishing the goal never implicitly authorizes merging or new external side effects.

## Execution boundaries

The executor may launch the blueprint's operator-approved worker model/effort pairs. Experts have separate approved
routes for consultations, diagnostics, and reviews, not production implementation. All model delegates shell out, even
within the same harness, and are leaf agents; requests for further agents go back through the executor. Known background
commands, such as slow tests, do not need another model merely to wait for them.

Prepare workers' test prerequisites early and include focused checks and repairs in their assignments. An untested patch
is a partial handoff, not accepted implementation; the executor still owns integrated validation. Use native background
completion notifications where the harness supports them, otherwise bounded waits. A status file or command wrapper does
not itself wake the agent.

An independent RAM/disk watchdog runs throughout execution, preferably as a background process rather than a model
repeatedly checking resources. It samples about every minute, alerts on low resources, and is restarted after failure or
resume. The executor pauses new work on resource exhaustion or monitor failure; it cannot substitute occasional checks
in its own loop. Default disk thresholds are below 10% free or 5 GiB available, and RAM below 10% available.

Concurrency and per-process deadlines remain bounded. Shared-tree writers are serialized, and overlapping tests must
have a stable tested tree. Debugging thresholds are latest points to seek expert help, not minimum waiting periods.
Useful expert exchanges continue; repetition without new evidence requires reassessment. Independent review checks the
actual change against the original intent, and substantive repairs need re-review. None of these rules guarantees that
an agent will follow them; transcripts and artifacts are needed to evaluate actual behavior.

Related consultant, worker-repair, and reviewer follow-ups normally resume the same model conversation. Initial
independent reviews still start fresh. The existing execution log keeps findings, repair evidence, pending gates, and
session handles for recovery. A failed repair leaves its finding open; completion requires reconciling the log with the
actual reviewed and tested tree.

## Evidence and evaluation

Execution records live under `${XDG_STATE_HOME:-$HOME/.local/state}/scode-build-blueprint/executions/<uuid>/`, with a
fallback for relative XDG paths. They are private and retained across resumes. Records cover work kept local, worker
choices, expert calls, follow-ups, results, repairs, and available usage. Exact delegate prompts, task specifications,
and deliverables are retained for later inspection. They may contain sensitive project content; nothing uploads or
prunes them automatically.

Shellout is an observability choice, not a free optimization: it adds startup and context-transfer overhead. It can
expose structured token counts, but missing counts and uncertain model attribution remain unknown. Whole transcripts are
not guaranteed to be retained, and top-level planning/execution usage may need separate collection. Compare total cost
per accepted goal, including planning, reviews, consultation, and repairs, rather than worker tokens alone.

`EVALS.md` describes optional tests. A small end-to-end smoke test checks the handoff and execution mechanics; it cannot
establish expert usefulness, quality equivalence, or cost savings. Real goals and their planner/executor transcripts are
the evidence needed for those questions. Evals run when requested, not automatically whenever the skill changes.

`SKILL.md` is the entrypoint, `planning.md` and `execution.md` define the phases, `watchdog.md` defines resource
monitoring, and `records.md` defines the evidence format. `SPEC.md` is the maintenance contract.

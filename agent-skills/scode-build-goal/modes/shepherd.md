# Shepherd mode

This file applies only in shepherd mode (see "Shepherd mode" in `SKILL.md`). In that mode it governs wherever it and
`SKILL.md` differ; nothing in it applies to the other modes.

## What the mode is

The goal file is executed by `scode-shepherd`, not by `/goal`. A strong Codex session, the supervisor, cuts the outline
into a stack of PRs at execution time and then sleeps, while scode-shepherd's runner drives a cheap worker model through
the PRs one at a time: a fresh worker session per attempt, independent verification of every PR, and a blocking review
of each by a strong reviewer. The point is cost on long goals: the worker does the implementation and the tool-call
churn, and the strong models only plan, review, and handle what goes wrong. Planning here stays as deep as in the other
modes: the outline, not a PR-by-PR plan.

v1 of scode-shepherd runs only on Codex: the supervisor is an interactive Codex session or a headless `shepherd drive`,
and the worker and reviewer run through `codex exec`, so both have to be models Codex serves (`gpt-` ids). The
repository needs a GitHub remote, and the machine that runs it needs `codex`, `gh`, and `deno`. When the working
directory is not in a checkout with a GitHub remote, say that shepherd mode cannot run on it and ask how to proceed.

## The up-front batch

In this mode the batch states, besides what `SKILL.md` lists:

- The mode, and that the run is started with `$scode-shepherd <goal file>` in an interactive Codex session (by default
  on `gpt-6.1-sol` at high effort) or headless with `shepherd drive`, not with `/goal`.
- The worker: `gpt-6.1-terra` at high effort unless the user names another. A model named for the workers sets it, at
  the effort named or high when none is. A worker model Codex does not serve cannot run in this mode; say so and ask.
- The checks every PR must pass: the gates the repository requires (its `AGENTS.md`, CI), as found while reading it.
  scode-shepherd's runner runs them on every PR unattended, with no terminal and no input, so name only checks that can
  run that way.
- The review gate menu, changed as below.
- The run directory (see Placement), in place of the log path.
- When `codex`, `gh`, or `deno` is missing on this machine, say so. The run happens on whichever machine starts it, so
  this is a note, not a blocker.

## The review gate

scode-shepherd's runner launches every review itself, through `codex exec`, on every PR before the next one starts.
Present the menu from "Review gate options" in `SKILL.md` with its usual numbering, but mark option 1 (opus) as not
available in this mode and option 2 (`gpt-6.1-sol` high) as the default, and word option 4 as another model Codex
serves, without the swarm example. An Other answer must name a model Codex serves, and a swarm is not available, since
the runner runs one reviewer per review. A reviewer request that names a model Codex does not serve is not mapped to
another model: say it cannot run in this mode and ask.

The goal file records the choice as an explicit demand for its model and effort, with the general charter from "Review
gate options" in full, as in the other modes. scode-shepherd adds its own rules on top (the review covers the PR's own
item and scope, and every finding goes back to the worker), so the charter is written exactly as the other modes write
it.

## Placement

There is no working log in this mode: scode-shepherd keeps the run's state and journal in a run directory, and that
takes the log's place. Name it `<slug>-goal-shepherd`, a directory in the same place as the goal file, so the two stay
adjacent in a listing (`~/git/foo-verbose-goal.md` and `~/git/foo-verbose-goal-shepherd/`). The name differs from the
`<checkout>-shepherd-<slug>` directories a plain scode-shepherd run picks, so a goal's run directory does not collide
with one of those. Its path in the goal file is absolute, like every path.

The run directory never goes inside the checkout the goal is about, where the worker's version control would see it and
scode-shepherd refuses it. When an `in <dir>` clause puts the goal file inside that checkout, say so in the batch and
put the run directory in the checkout's parent instead. When only a temporary location was available, say in the batch
that the run's state would be temporary too: a run can last days, and losing its directory loses the run.

Never create the run directory, for the same reason the other modes never create the log: scode-shepherd treats an
existing run directory as a run that already started, and refuses one whose journal belongs to another goal. If a
directory with that name already exists, handle it the way `SKILL.md` handles an existing log: refine the slug, or, when
it really is an earlier attempt at the same goal, ask whether to replace it, which means moving it aside under a dated
suffix (`<name>.archive-YYYY-MM-DD`) along with the old goal file, never deleting it. Move it only when nothing is
running in it (no live `driver.pid` or runner lock holder), and say when asking that the old run's draft PRs and
branches stay on GitHub, where the new run's could collide with them.

## What the goal file contains

The goal file opens, right after its title, with this sentence, verbatim:

> This goal file was written by scode-build-goal in shepherd mode.

scode-shepherd looks for it to know that the goal file's decisions count as the user's approval and that its own goal
file rules apply; a goal file without it gets scode-shepherd's ordinary flow, including a second approval of the plan.

Then the items of "What the goal file must contain" in `SKILL.md`, with these differences:

- **Execution mode.** State that the user requires `$scode-shepherd` for the entire goal: a strong supervisor plans the
  stack and handles escalations, and a cheap worker driven by scode-shepherd's runner does the implementation. Give the
  run directory's absolute path, saying that scode-build-goal did not create it, and the worker: as an explicit demand
  for its model and effort when the user named one, or otherwise as the default worker, which the supervisor may change
  as scode-shepherd allows. Forbid invoking, loading, or following `scode-galaxy-brain`, `scode-powerhorse`, or
  `scode-model-routing` at any point.
- **Resume protocol.** Leave it out. scode-shepherd's run directory, its journal, and the runner's state are the resume
  record, and starting the same goal file again resumes the run. Do not name an `agent-resumeable` log.
- **Resource watchdog.** Leave it out. scode-shepherd's runner checks disk and memory before every worker session, check
  run, and review, kills a step that runs the machine out, and escalates to the supervisor.
- **Scope reassessment.** Keep it, with the planning charter in full. In this mode the delegates are scode-shepherd's
  worker sessions, which report back (blocked or replan) instead of building a mechanism their PR does not call for, and
  the checkpoint is that report; the supervisor applies the decision boundary to every plan and replan.
- **Review gate.** The demand and the full charter as above. Say that scode-shepherd's runner runs it on every PR, and
  that its findings go back to the worker until the reviewer approves. The runner builds each review's prompt itself
  (the repo, the PR's range, where findings go), so the goal file carries no review-prompt requirements beyond the
  charter. As in the other modes, no launch command.
- **Checks.** Name the checks every PR must pass, as settled in the batch, and say they run unattended.
- **Decision logging.** In the run's journal, `<run directory>/journal.md`, rather than a working log.
- **Goal, decisions, outline and requirement sources, PR discipline, unattended fallback, done criterion.** Unchanged.

## When the goal file is written

Tell the user how to start it, in place of the `/goal` command:

- Interactive: in a Codex session on the checkout (by default on `gpt-6.1-sol` at high effort), `$scode-shepherd <goal
  file>`.
- Headless: `~/.codex/skills/scode-shepherd/scripts/shepherd drive <run directory> --repo <checkout> --goal-file <goal
  file>`, with every path absolute (`~` expanded too). The run directory has to be the one the goal file names.

Either way, the same command again resumes the run, as long as it is the same way the run was started (a run started
headless resumes headless) and the run has not finished; edits to the goal file after the start do not reach a headless
run. Headless, a question the supervisor has to ask (a departure from the goal file, an escalation it cannot resolve)
ends the drive with the question pending; `--answerer <program>` hands such questions to a program, and `drive <run
directory> --answer-file <file>` answers one later.

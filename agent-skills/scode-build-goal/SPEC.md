# scode-build-goal specification

NOTE: This file is binding on the skill's text. It is deliberately sparse: it records only requirements that have been
stated as such, not a description of everything the skill does. Absence of an entry means the behavior is not yet
specified, not that it is unspecified on purpose. When the skill and this file disagree, that is a bug in one of them;
fix the skill or change this file in the same change, never leave them apart.

NOTE: Nothing on the skill's own path reads this file. Its readers are agents changing the skill (see `AGENTS.md`) and
whoever is judging whether a change kept the user-facing behavior intact.

## Triggering

- The skill runs only on an explicit invocation: `$scode-build-goal` or `/scode-build-goal` followed by the argument
  string. A message that merely talks about goal files, unattended runs, or `/goal` does not trigger it.
- The argument string `help`, and nothing else, prints a usage TLDR in chat and stops. That path creates and modifies
  nothing.
- With the argument string missing or empty, the skill asks for the goal text. It never guesses a goal.

## What comes out

- The output is one file, the goal file, written as instructions to the agent that will execute it under `/goal`. The
  skill does not start the work. In shepherd mode the goal file is started differently and has a run directory instead
  of a working log (see Shepherd mode). In overseer mode it is still started with `/goal`, and a second log, the
  overseer's, sits beside the working log (see Overseer mode).
- The goal file is self-resumeable from the user's point of view: handing `/goal <absolute path of the goal file>` (or
  the harness's equivalent) to any fresh session either starts the work or resumes it, decided by whether the working
  log exists on disk. The user never has to know or say which of the two applies, and repeats the same command until the
  work is done.
- The skill never creates the working log file, not even empty or with a heading. The goal file's resume protocol treats
  the log's existence as proof that a previous session already started, so a log created by this skill would turn the
  first run into a resume from nothing.
- The skill never overwrites or removes an existing goal or log file without asking. On a name collision it refines the
  slug with another word from the goal, or, when the existing files are an earlier attempt at the same goal, asks the
  user whether to replace them. Replacing moves both existing files aside under a dated suffix; it never deletes them
  and never leaves the old log in place. A leftover watchdog status file moves aside with them, since it carries the old
  attempt's alert state.
- Every path written into the goal file, not only the log path, is absolute. The goal file is read by sessions with
  arbitrary working directories.

## Naming and placement

- The user does not have to name either file. The skill derives a short kebab-case slug from the goal that a person
  could reproduce from the goal itself (`build-bar-and-baz`, `farhelm-dist`). The slug leads with the project name,
  which is the basename of the checkout directory (the outermost one when checkouts are nested), unless the target
  directory is dedicated to that one project; the default directory never is. A UUID, timestamp, session id, or any
  other anonymous identifier is never a filename.
- The goal file is `<slug>-goal.md` and the log is `<slug>-goal-log.md`, always in the same directory as each other.
- The checkout that placement is relative to is the one containing the shell's working directory, found by walking
  `$PWD` up to the nearest `.git` or `.jj` without resolving symlinks. A repository the goal text mentions does not
  change it. A checkout nested inside another checkout (a worktree, a vendored repo) is replaced by the outermost one. A
  working tree rooted at the home directory is never counted as a checkout for this purpose: it neither nests the
  checkouts under it nor makes their parents count as inside a repository.
- The default directory is the parent directory of that checkout, the home directory included, provided the parent is
  writable and is not inside a VCS working tree other than one rooted at the home directory. In the default case the
  skill picks the paths itself and states them; it does not ask.
- When there is no repository, or the parent directory fails those conditions, the skill proposes a temporary location
  (per-session scratch space or a fresh temp directory), says plainly that it is temporary, and creates nothing, the
  directory included, until the user answers.
- The chosen or proposed paths are always shown to the user in the up-front question batch, before anything is written,
  so they can be overridden with one line; a directory inside a repository is called out as such. There is no path the
  user only discovers by reading the goal file.

## The `in <dir>` override

- `$scode-build-goal in <dir> <goal text>` puts both files in `<dir>` and changes nothing else: the names are still
  derived from the goal as above, and the repository the goal is about is still the checkout containing the working
  directory. The override is honored even when `<dir>` is inside a repository.
- The clause is exactly one of the words `in`, `into`, or `under`, followed by a path-shaped word (starting with `/`,
  `~`, or `.`, or containing `/`) that ends at the next whitespace, and it must be the first thing after the skill name.
  Anything else after the skill name means there is no clause and the whole argument string is goal text.
- A path mentioned anywhere inside the goal text is part of the goal and never a placement instruction, however much it
  sounds like one. The skill does not scan the goal text for where to put its own files.
- `<dir>` is resolved to an absolute path against the working directory, with `~` expanded. A `<dir>` that does not
  exist or is not a writable directory is reported and asked about. The skill neither creates it nor silently falls back
  to the default placement.

## No-workhorse mode (default)

- The skill defaults to no-workhorse mode. The existing `no-workhorse` switch or a plain-language request that the
  executing agent do the work at its own model also selects it explicitly. A quoted, negated, path-embedded, or
  subject-of-the-goal mention stays in the goal text; only an occurrence consumed as a switch is removed. An unclear
  request about delegation is asked about, not guessed. The `in <dir>` clause and `help` rules are unchanged.
- The up-front batch states which mode applies, no-workhorse, galaxy brain, powerhorse, shepherd, or overseer, so the
  user can correct it before anything is written.
- When no-workhorse applies only by default, the batch states the fallback for a previously active galaxy-brain session:
  the executor asks before dependent work. The user can explicitly choose no-workhorse to authorize stopping it, or
  choose galaxy brain mode instead. The goal file records the settled fallback.
- In no-workhorse mode the goal file forbids sending any unit of the executing agent's own decomposition to a weaker
  model or a lower effort than its own (the review gate keeps its demanded model and effort; scope reassessment runs at
  the executing agent's own model and effort), and forbids invoking, loading, or following `scode-galaxy-brain` or
  `scode-model-routing`. If galaxy-brain is already active, an explicit no-workhorse choice in the invocation or
  up-front answers instructs the executor to stop it; when no-workhorse was only the default, the executor asks how to
  resolve the conflict before dependent work. Sub-agents at the executing agent's own model and effort are explicitly
  fine, at its judgment, when they are likely to finish the work sooner without costing quality; parallelism is
  encouraged as something to consider, never required. Concurrent agents that might otherwise conflict may run in
  separate worktrees or jj workspaces whose results the executing agent merges back. A user request that rules out
  sub-agents altogether overrides the allowance. The scope reassessment review runs at the executing agent's own model
  and effort. The goal file does not carry galaxy-brain's concurrency rules. Native sub-agents are the normal launch
  path, and `scode-harness-shellout` covers a model not available as a native sub-agent type.
- The review gate and scope reassessment review are the same as in galaxy brain mode, launched by the executing agent
  itself the same way as any other sub-agent. The mode changes nothing else.

## Galaxy brain mode

- The `galaxy-brain` switch or a plain-language request to delegate suitable units of work to cheaper models selects
  galaxy brain mode. Quoted, negated, path-embedded, or subject-of-the-goal mentions leave no-workhorse as the default
  and stay in the goal text. Only an occurrence consumed as a switch is removed.
- In galaxy brain mode, the goal file requires `scode-galaxy-brain` for the entire run, activated right after the resume
  protocol and kept active through every delegation. Reading it for mechanics does not satisfy this. The review gate and
  scope reassessment review run through it; the review gate keeps its demanded model and effort, while scope
  reassessment has no named model in the goal file.

## Powerhorse mode

- The skill supports a powerhorse mode, turned on by the word `powerhorse` used as a switch in the argument string or by
  a plain-language request that the executing agent supervise large parallel chunks on strong models. What counts as a
  switch, a mention, or an unclear request follows the galaxy brain rules. A request for more than one mode is asked
  about.
- The up-front batch states the mode, and in powerhorse mode states defaults the user can override for the worker model
  (the executing session's own model and effort), the width (two writer chunks in flight), and PR sizing (one PR per
  chunk, or bite-sized PRs with chunk specs naming the boundaries).
- In powerhorse mode the goal file requires `scode-powerhorse` for the whole run in place of `scode-galaxy-brain`, and
  forbids invoking, loading, or following `scode-galaxy-brain` or `scode-model-routing`; if galaxy-brain is already
  active, the goal file is the user's express instruction to stop it. Only one orchestrator is ever loaded in the run.
  The settled worker model, width, and PR sizing are carried as explicit demands, and the chunk table gets its own
  absolute path next to the working log, never created by this skill. A model the user names for the workers sets the
  worker model (at the named effort, or the executing session's own when none is named), the goal file records what
  happens if it is unreachable at run time (by default, pause the chunks that need it and ask), and the batch says when
  this machine lacks the CLI it needs; a model named for the reviewer sets the review gate as a single charter agent; an
  unclear role is asked about. The per-PR review gate runs while other chunks continue.
- The mode changes nothing else: the resume protocol, planning outline and scope reassessment, PR discipline (apart from
  sizing), decision logging, unattended fallback, done criterion, and the user's review-gate choice are the same, with
  the review gate and the scope reassessment launched through powerhorse.

## Shepherd mode

- The skill supports a shepherd mode, turned on by the word `shepherd` used as a switch or by a plain-language request
  to run the goal under scode-shepherd. A goal about scode-shepherd is a mention, `shepherd` used as part of the goal's
  own wording (a verb, or a noun) is goal text, and an ambiguous use is asked about, so such goals keep the mode they
  had before. What counts as a switch, a mention, or an unclear request follows the galaxy brain rules, and a request
  for more than one mode is asked about.
- The mode's specifics live in `modes/shepherd.md`, which the skill reads only when the mode is on and which governs
  where it and `SKILL.md` differ. The other modes never read it. That keeps their behavior what it was: the hooks in
  `SKILL.md` that every mode reads only name the new mode and point at the file.
- In shepherd mode the goal file is executed by scode-shepherd, started with `$scode-shepherd <goal file>` in an
  interactive Codex session or with `shepherd drive` headless, not with `/goal`; repeating the same command resumes the
  run. The skill tells the user both commands when the goal file is written.
- Planning depth is the same as in the other modes, an outline; the PR-by-PR decomposition is scode-shepherd's
  supervisor's at execution time. scode-shepherd treats the goal file's decisions as the user's approval of a plan that
  stays within them, so the up-front batch is where the user's say over the run happens.
- The batch states the mode and how the run is started, the worker (by default `gpt-6.1-terra` high, overridable by a
  model named for the workers), and the run directory in place of the log path. scode-shepherd v1 runs the worker and
  the reviewer through `codex exec`, so both must be models Codex serves: a worker or reviewer request for any other
  model is said to be unavailable and asked about, never mapped to another model.
- The review gate menu keeps its options and numbering, with option 1 (opus) marked unavailable and option 2
  (`gpt-6.1-sol` high) the default; an Other answer must name a model Codex serves, and no swarm is available. The goal
  file carries the demand and the full general charter as in the other modes.
- There is no working log. The run directory, `<slug>-goal-shepherd` beside the goal file, takes its place; its name
  keeps it adjacent to the goal file and apart from the `<checkout>-shepherd-<slug>` directories plain scode-shepherd
  runs pick. The skill never creates it, for the same reason it never creates the log, and an existing directory of that
  name is handled like an existing log (refine the slug, or ask, moving the old one aside under a dated suffix).
- The run directory never goes inside the checkout the goal is about (scode-shepherd refuses one there); when the goal
  file does, the run directory goes in the checkout's parent and the batch says so. A temporary location is called out
  as making the run's state temporary too. An existing run directory is moved aside only when nothing runs in it.
- The worker is a demand only when the user named it; the default worker is recorded as the default, so scode-shepherd's
  supervisor may still change it. The batch states, and the goal file names, the checks every PR must pass, all runnable
  unattended, since the runner runs them on every PR. Stacked PRs make some CI misfire: a base guard (a job whose only
  step fails a PR whose base is not an allowed branch) fails on a PR whose actual base it does not allow, and a workflow
  filtered to the default branch does not run on a PR whose actual base its filter excludes. The goal file names a base
  guard by workflow and job as expected to fail for that reason alone, not as a gate, and names a skipped workflow and
  puts what it validates in the checks, or asks the user whether the PRs may go without it. A check the user requires by
  name keeps that demand in either case, and its conflict with the stacked bases is asked about; on a patreon-buddy run
  (2026-11-06) a goal file demanding that every applicable GitHub check pass sent a worker to replan over a base guard.
- Shepherd mode cannot run without a checkout that has a GitHub remote; the skill says so and asks.
- The goal file opens, right after its title, with the sentence "This goal file was written by scode-build-goal in
  shepherd mode.", which is what scode-shepherd checks for. Its execution mode requires `$scode-shepherd` for the whole
  goal, names the run directory and the worker demand, and forbids `scode-galaxy-brain`, `scode-powerhorse`, and
  `scode-model-routing`. It has no resume protocol and no resource watchdog, because scode-shepherd's run directory and
  journal, and its runner's disk and memory guard, do those jobs. Decision logging goes to the run's journal. Everything
  else the goal file requires is the same as in the other modes.

## Overseer mode

- The skill supports an overseer mode, turned on by the word `overseer` used as a switch or by a plain-language request
  that the goal session oversee a separate executor session rather than do the work itself. What counts as a switch, a
  mention, or an unclear request follows the galaxy brain rules; a goal about overseeing or an overseer is goal text, an
  ambiguous use is asked about, and a request for more than one mode is asked about.
- The mode's specifics live in `modes/overseer.md`, read only when the mode is on, governing where it and `SKILL.md`
  differ. The other modes never read it, for the same reason as `modes/shepherd.md`.
- The session the user hands the goal file to with `/goal` is the overseer, assumed to be Claude Code. It launches the
  executor as a farhelm session on the same host, in the checkout, running Claude Code on the executor model and effort,
  with an initial prompt that is `/goal` plus the executor's instructions, or the same instructions without `/goal` when
  `/goal` is not accepted. The mode is hard-wired to farhelm and to Claude Code for both sessions; the executor's model
  is not switched during the run.
- The executor is told in its prompt and in the goal file that it works under the overseer's oversight and that the
  overseer's messages direct it, within the user's settled decisions. It does the goal as a no-workhorse run would, with
  its own working log, PR stack, resource watchdog, and the review gate the user chose, launched by the executor itself.
  The review gate menu is unchanged from the default modes.
- The overseer's brief is judgment about whether the run is going according to the goal and whether the executor needs a
  nudge, not a checklist. The goal file gives examples (the review gate skipped, misplaced, or on the wrong model; the
  executor stuck on a hard problem and needing a design nudge; complexity out of proportion to the goal; stalled
  progress) and examples of nudges (direct instructions, or a request to get a stronger-model consult), marked as
  examples. The overseer nudges through Claude Code's cross-session messaging, logs every nudge with its reason, edits
  nothing in the checkout, and touches no VCS state.
- The batch states the mode and how the run is started, and the executor model and effort (by default `haiku` high; a
  Claude model, since the executor runs in Claude Code; any other model is said to be unavailable and asked about, never
  mapped to another).
- Both sessions read the same goal file. The executor is the session whose prompt says it is; the overseer is the
  session the user started with `/goal`; any other session handed the goal file as context (a reviewer, a consult, a
  sub-agent) has neither role.
- The overseer keeps its own log, `<slug>-goal-overseer-log.md`, beside the working log, through `agent-resumeable`. The
  skill creates neither log and handles an existing overseer log like an existing working log.

## Up-front questions

- The skill assumes unattended execution. Foreseeable consequential choices and acceptable fallbacks are settled before
  the goal file is written. Answers are retained as decisions already made; planner proposals remain distinguishable.
  Unforeseen material scope changes are not implicitly authorized by the unattended assumption.
- Questions are batched, not dribbled, and only asked after the skill has read enough of the repository to answer what
  the codebase already settles.
- The batch always presents the review-gate menu, numbered, with these four options in this order and the first marked
  as the default: a single fresh-context agent with a general correctness, design, and idiomatic-code charter on opus
  high; the same on gpt-6.1-sol high; the same on gpt-6-astra high; and Other, a plain-text answer naming any other
  reviewer (in shepherd mode, changed as Shepherd mode says). Each reviewer runs on the model it names, cross-harness
  when the executing harness cannot reach the model natively; an Other answer naming a model without an effort runs at
  high. The user's choice is recorded in the goal file; the skill never picks a reviewer silently.
- The menu offers no swarm option, and the default is never a swarm. A swarm is the reviewer only when the user
  explicitly asks for one (saying "swarm" or naming `pre-pr-review-swarm`), in the invocation or in an Other answer. A
  reviewer request that names only a model and effort, such as "gpt 6.1 sol high reviewer", means a single charter agent
  on that model, never a swarm. When the request already names a reviewer, the batch states which menu option it maps to
  alongside the menu. A swarm request that names no model is asked about.

## Proportionate planning

- Before finalizing, show a short repository-grounded implementation outline: existing paths to reuse, the smallest
  sufficient design, substantial new state or subsystems, expensive promises, and uncertainty. Explain aggregate cost;
  do not prescribe every defensive detail or use invented precise estimates as targets.
- Keep the original request and later user decisions, sourced repository constraints, and planner-proposed mechanisms
  distinct. Substantial mechanisms need a concrete contract or demonstrated failure that the smaller design cannot
  handle. Refusal of unsupported cases must still satisfy the agreed behavior.
- Run one fresh-context planning review, using a native sub-agent at the planner's model unless the user specifies
  otherwise. It receives the request and decisions as well as the proposed goal, outline, and repository context. Its
  charter explicitly challenges unnecessary planner-authored obligations and asks for concrete smaller alternatives,
  distinguishing changes that need a user decision. No findings is valid. Findings go to a named private file; no
  product or VCS edits. This does not activate execution skills or change the per-PR reviewer selection.
- Resolve findings before finalizing. Reassess affected conclusions after material design changes, not wording changes.
  Disclose unavailable independent review; self-review must not be represented as independent.

## What the goal file requires of the executing agent

In shepherd mode, Shepherd mode above replaces the resume protocol, the resource watchdog, and the execution mode
entries below, and moves decision logging to the run's journal. In overseer mode, the entries below are what the goal
file requires of the executor, and Overseer mode above adds the overseer's own.

- Invoking `agent-resumeable` on the log file's absolute path as its first action, with the resume semantics spelled
  out: an existing log means resume after cross-checking it against reality, a missing log means a fresh start.
- In the default no-workhorse mode, no orchestration skill, with the demands and allowances described under No-workhorse
  mode. In galaxy brain mode, `scode-galaxy-brain` for the entire run as described under Galaxy brain mode. In
  powerhorse mode, `scode-powerhorse` throughout, as described under Powerhorse mode.
- A resource watchdog for memory and disk, started before the first delegation, restarted if it dies, and carried in
  every handoff note. It is the skill's own `scripts/watchdog`, which the goal file names by absolute path along with
  its status file (beside the working log, never created by this skill), watched paths, interval, and alert levels, so
  no executor writes its own. The alert levels default to scode-shepherd's runner guard's "soft" levels, whose
  percentage rule is capped by absolute amounts: a plain 10% rule, which this skill used before, alerts on a 2 TB disk
  that still has 180 GB free. Memory includes cgroup v2 limits, since inside a container or a limited systemd scope
  `/proc/meminfo` shows the host's memory and the OOM killer fires first. Swap is not watched: available memory counts
  what the run can use without paging, so it alerts before the machine is down to swap.
- The watchdog delivers alerts, worsening, recovery, unreadable sources, and its own exit or stall as notifications, so
  routine minute-level sampling costs no model turns. A sampler cannot report its own hang, so a separate observer
  watches the status file's age and reports it once it is two sample intervals old. The goal file names the mechanism
  for each harness. In Claude Code, the sampler and the observer run under two Monitor tools, which turn each printed
  line into a notification while the session keeps working; both expire every 30 minutes, and the executor re-arms them
  and checks the status in that turn, so the normal case costs one such turn per half hour. The two are always re-armed
  together, so they stay in step, and an observer started on an already-stale file stays quiet until the file has been
  fresh, so a re-arm never reads as a new failure. In Codex, which has no Monitor, both are armed through
  `skillette-codex-wake-on-exit`, which turns a process exit into a queued wake-up. The sampler keeps running there too,
  so its exit means it died; the observer is the process that exits on each event (a stale heartbeat, or an event the
  sampler recorded in the status file) and is re-armed. A sampler that exited on its own alerts instead would stop
  sampling until the wake-up landed, which in Codex is not before the current turn ends. The sampler keeps what it has
  already reported in the status file, so a re-armed sampler does not repeat an alert, and a resumed session checks the
  status before its first launch instead. An earlier session's monitor handles do not work from a new session (Codex's
  wake-on-exit refuses another thread), so a new session that finds a fresh heartbeat stops the earlier monitors through
  their handles where it can and otherwise kills the sampler the status file names, before starting its own; two
  samplers must never write one status file. In Codex, an observer armed on an already-stale status file does not count
  that as an event, or a sampler that cannot be restored would wake the session at every turn end. The script path is
  written with symlinks resolved, so it is the same file whichever harness runs the goal. That path is inside the
  checkout the skills are installed from, so a goal that edits that checkout can change the script under a running
  watchdog; this is accepted, since a restart picks up the change and a per-goal copy would keep old goal files away
  from later fixes.
- In Codex the observer is armed before the sampler, because it relays only events recorded after it started, and the
  session stops its own runs through their handles, since killing a sampler by pid would queue a wake-up for an exit it
  caused itself.
- Delivery and failure detection are verified in each session, on fresh throwaway status files, before the real monitors
  start: a synthetic alert, a killed throwaway sampler, and a throwaway sampler that stalls without exiting, caught
  within two sample intervals. The stall uses the script's own stall flag rather than SIGSTOP, because under Monitor a
  stopped job is reported as an exit and would test the exit path instead.
- Every path checks the status (fresh heartbeat, nothing short, every source readable) before new workload launches and
  at natural boundaries. A dead monitor, a heartbeat stale for two sample intervals, or a reported shortage pauses new
  launches until the check passes. Only a shortage gets the cleanup response (removing owned worktrees and caches,
  waiting for or killing the delegate most likely responsible); a monitoring failure only pauses launches until
  monitoring is restored, since killing work over a sampler that missed a beat would turn every routine Monitor expiry
  into damage. A shortage the run cannot relieve is logged as a blocker and launches stay paused, and so is a sampler
  that dies again before its first sample or a source that stays unreadable after a restart, so a sampler that cannot
  start does not cost a model turn per restart. When the harness's mechanism is unavailable or fails verification, the
  executor logs that and relies on those checks alone; it does not poll the status file on a timer. Earlier text
  required a check at least once a minute in that case, and on 2026-11-12 three independent reviewers of a pair of
  long-running goal files flagged it: on a run of hours or weeks it is a model turn every minute, which burns context,
  and a model in the middle of a long tool call cannot poll every minute anyway, so the launch and boundary checks are
  what actually protect the run. Without Deno there is no watchdog, which counts as a dead monitor, and the planning
  session says so up front when this machine lacks it. Reducing model wakeups must not reduce resource sampling or
  assume that status-file writes or process-completion notifications deliver intermediate resource alerts.
- The goal file has no small-context watcher-agent option for the watchdog in any mode. That option existed only for
  when the process path still needed frequent parent wakeups, which a named notification mechanism removes. The log
  records how to reconcile or restart the monitors, and completion stops the owned monitors.
- A linear stack of reviewable PRs via `jjstack`, each reviewed before it is finished by a fresh-context run, delegated
  through the active orchestration skill (in no-workhorse mode, launched by the executing agent itself), of the reviewer
  the user chose from the menu, stated in the goal file as an explicit demand for its model, effort, and skill or
  charter. The prompt names the skill or carries the full charter, the repo root, the range to review, and the findings
  file, with no launch command copied into the goal file. For every non-swarm reviewer the goal file spells out the
  charter in full: general correctness, design, and idiomatic code, findings to a named file, no edits, no VCS changes.
- The reviewed outline and requirement sources, including the user's relevant words and later decisions, survive into
  the goal. Before implementing substantial departures, or undertaking another repair round after repeated corrective
  reviews of a component, reassess necessity with a fresh-context review carrying the full planning charter, original
  intent, current diff, and concrete alternatives. Reuse existing checkpoints or reviews when they serve this purpose;
  no periodic review schedule, technology blacklist, or review of every helper is required. Delegated units exclude
  unapproved mechanisms; a newly discovered dependency can remain unimplemented and be reported at an existing
  checkpoint with an honest account of checks and incompleteness.
- Log major decisions with a scannable DECISION label. Routine choices proceed within scope. Removing unnecessary
  planner machinery is permitted when explicit behavior and repository constraints still hold. Material expansion,
  weakened guarantees, or omitted required behavior need an agreed fallback or user decision; otherwise only dependent
  work pauses and independent authorized work continues. Logging or review does not grant authorization. Explicit scope
  reductions remove obsolete active obligations, code, and validation gates.
- The done criterion is a linear stack of open, unmerged PRs that collectively achieve the goal. Merging is the user's.

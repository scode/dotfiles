# scode-powerhorse specification

Dependencies: scode-agent-delegation

NOTE: This file is binding on the skill's text. It is deliberately sparse: it records only requirements that have been
stated as such, not a description of everything the skill does. Absence of an entry means the behavior is not yet
specified, not that it is unspecified on purpose. When the skill and this file disagree, that is a bug in one of them;
fix the skill or change this file in the same change, never leave them apart.

## Why this skill exists

scode-galaxy-brain is built to save cost: it routes suitable work to cheaper models and escalates when they fail. This
skill serves a different objective, finishing a goal sooner at equal quality by running large chunks of work in parallel
on strong delegates. Expressing that as overrides on galaxy-brain was tried on paper and rejected: its delegate-or-not
test, its routing, its escalation ladder, and its design defaults each pulled against it. This skill is a separate
orchestrator so that galaxy-brain's text, behavior, and evals stay untouched.

## Requirements

- The session in which the skill is invoked is the supervisor. It owns the decomposition into chunks, the interfaces
  between chunks, the user's intent, every gate, integration, and all version control. Delegates never commit, branch,
  push, or open PRs.
- The objective is time to a finished, gated goal at equal quality. Cost is a constraint, never the reason to delegate
  or not to delegate.
- This skill and scode-galaxy-brain are never active in the same session. While this skill is active, the session does
  not invoke, load, or follow galaxy-brain or scode-model-routing. Invoking this skill while galaxy-brain is active,
  directly or through a goal file the user had written, stops galaxy-brain; invoking galaxy-brain later stops this
  skill. The skill's text names galaxy-brain only to state this exclusion and the difference in objective.
- There is no routing. Chunks of the supervisor's own decomposition run at the worker selection: the session's own model
  and effort unless the user or a goal file names another (at the named effort, or the session's own effort when none is
  named), resolved once per run to a concrete model and effort, recorded, and set explicitly at launch. A later session
  keeps the recorded selection; when it cannot reach it, the user is asked. The supervisor never substitutes another
  model on its own. Spawns another process defines (a per-PR review, a swarm, a scope review) run at exactly the model,
  effort, agent type, and skill that process or the user demands, and at the session's own model where it names none.
  The supervisor delegates nothing else: no helper agents for searches, scans, or monitoring. A chunk delegate may use
  read-only helpers of its own (the user decided this explicitly), which count against harness capacity; it may not
  start helpers that edit files.
- The supervisor chooses each launch mechanism itself: native only when it can select the model and effort and, for a
  chunk, resume the same agent after a checkpoint; otherwise the shell-out harness that serves the model. Availability
  restrictions in the user's `~/.scode-model-routing.md`, when it exists, are honored. A demanded model nothing can
  reach goes to the user, never to a substitute.
- A chunk is one coherent outcome with its tests and documentation, roughly a reviewable PR or a subsystem, finishable
  without another in-flight chunk's unfinished changes. Work smaller than a chunk is never delegated, and chunks are
  never split only to widen the fan-out. A chunk is delegated when it can overlap other in-flight work or would fill a
  large share of the supervisor's context; otherwise the supervisor does it itself.
- The supervisor settles intent, shared interfaces and invariants, compatibility, and integration order; the delegate
  owns the chunk's internal design within those boundaries and proposes it at the first checkpoint.
- Writer width defaults to two chunks in flight (running or stopped at a checkpoint), overridable by the user or a goal
  file. Launches pause on resource alerts, rate-limit or quota errors, more than one delegate waiting on the supervisor,
  or ungated results.
- Every chunk runs in a tree the supervisor creates, named by the run id, even when it is the only chunk in flight, with
  shared out-of-tree state isolated or serialized. While a delegate owns a tree, the supervisor makes no edits and runs
  no version-control commands in it beyond read-only inspection. Accepted results are integrated serially with checks
  after each apply. Per-PR reviews the user's process requires run while other chunks continue.
- Every chunk delegation, and every process-defined spawn that does not run natively, goes through
  `scode-agent-delegation`, which generates the run id, owns the run directory, launches, runs the checkpoint protocol,
  and returns one verdict from its vocabulary. This skill's text carries no copy of the task-spec checklist, the
  checkpoint protocol, the gate's steps, or the addendum. It acts on every verdict in that vocabulary through its action
  table for chunks; a verdict with no row is a bug here. A process-defined spawn whose verdict is not `accepted` is
  relaunched once at the same demanded model and effort, then reported.
- The deep review of a writer chunk happens at the `AWAITING REVIEW` checkpoint, with findings sent back through
  `REVIEW.md` (including `Also:` items) so the delegate fixes them in its own context. Findings the gate maps to `spec
  defect`, `blocked on user`, or `substantive failure, structural` return that verdict instead. The final gate confirms
  against the complete change set, untracked file contents included, saved at that stop.
- A fixable failure gets one fixup at the worker selection before takeover or replanning. An `unresumable` chunk that
  the chunk table records at the second checkpoint is taken over, not discarded: the supervisor re-runs the checks,
  applies outstanding answers and review items, gates the current change set, and records a recovery note in place of
  the missing report. A rate-limit or quota ending is an environmental cause, never a decline, and does not count toward
  the delegation skill's crash-resume cap; this departure from that skill's classification is stated in `SKILL.md`.
- The supervisor keeps a chunk table in a file of its own, edited in place at every phase change (an append-only resume
  log records its path and the phase changes, never the table rewritten), and reconciles it against reality before
  launching anything after compaction or resume.
- Multiple concurrent uses of this skill, or of this skill and galaxy-brain, by different sessions must not conflict
  through any state the skill itself maintains. Every artifact it creates for a delegation (trees, run directories,
  prompts, logs, saved change sets) is named by a run id the creating session generated, and no session removes or
  reinterprets another's. The chunk table is run-wide state at the path the goal file or the user is given, shared only
  by the sessions that resume the same run one after another, never concurrently.
- The skill's dependency set is the `Dependencies:` line above, and the dependency graph between skills is one-way: this
  skill may refer to a skill it depends on, and a skill it depends on never refers back to it. The shell-out skill is
  not a dependency of this skill: the delegation skill loads it.
- Dependencies use the current harness's authorized skill loader or resource resolver, or, when no dedicated loader
  exists, the exact `SKILL.md` location its catalog or instructions supplies. On Codex, when no location is supplied,
  use `${CODEX_HOME:-$HOME/.codex}/skills/<name>/SKILL.md` (the root verified for Codex 0.152). Known interfaces are
  examples, not an allowlist of harnesses. Missing filesystem metadata or an unfamiliar harness alone must not block
  loading or trigger a permission question; actual tool permissions remain binding. Never guess installation paths or
  URI schemes, assume a sibling dependency directory, or search other skill roots.
- Identify the exact requested skill from its returned frontmatter, or the loader's reported identity when frontmatter
  is not exposed. Missing or conflicting identity fails loading. Read the skill in full and all sidecars the current
  step needs. Truncated output is recovered through the tool's continuation before acting; content that cannot be fully
  retrieved stops the affected operation and names the skill and the failing path, URI, or tool. No permission bypass,
  remembered instructions, alternate copy, or similar skill may replace a failed load.
- The loading text is the marked stanza in `SKILL.md`, checked against the canonical template in `tests/skill_deps.rs`.
- Invoking the skill activates it for the rest of the session, per "Staying active for the whole session" in `SKILL.md`;
  the dependency, by contrast, activates nothing when loaded.
- The skill is meant to work on modern Linux and macOS. Commands, paths, and tools it prescribes must be available on
  both.

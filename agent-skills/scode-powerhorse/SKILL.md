---
name: scode-powerhorse
description: >
  Supervise a goal by splitting it into large, independent chunks and running them in parallel on strong delegates (by
  default the session's own model), while the current session plans the chunks and the interfaces between them, gates
  every result, integrates, and owns all commit/PR management. The objective is finishing sooner at equal quality, not
  saving cost. Use only when the user explicitly invokes scode-powerhorse, or when a goal file the user had written
  requires it. Replaces scode-galaxy-brain for the session; the two are never active together. Once invoked, the skill
  stays active for the rest of the session, including across context compaction and resume, until the user expressly
  stops it, unless the invocation itself limited the scope up front.
---

# Scode Powerhorse

## Premise

The session using this skill is the supervisor. Its job is to finish the goal sooner than a single agent working alone
would, without lowering quality, by handing large chunks of the work to strong delegates that run at the same time. The
delegates are peers, not workhorses: by default they run at the supervisor's own model and effort, and each one owns the
detailed design and implementation of its chunk. The supervisor owns what no single chunk can see: the decomposition,
the interfaces between chunks, the user's intent, every quality gate, integration, and version control.

Cost is a constraint here, not the objective. That is the whole difference from scode-galaxy-brain, which sends work to
cheaper models to save money. This skill sends nothing to a cheaper model, and the only question it asks before
delegating is whether doing so gets the goal done sooner.

The overhead to manage is round trips between you and a delegate. Every chunk costs a spec, two checkpoint stops, a
gate, and an integration, and none of those shrink much when the chunk does. So chunks are large, small work is never
delegated, and the deep review of a chunk happens while its delegate can still fix what you find (see Gating below).

Delegates never commit, branch, push, or open PRs. Your session carries the user's version-control workflow and skills,
and those do not transfer to a sub agent; the delegation skill's task spec rules make that binding on every delegate.

## What to read, and when

The first time in a session that you decide to delegate anything, load the skill `scode-agent-delegation` as follows and
read its `SKILL.md` in full. It owns everything between your decision to delegate a unit and the verdict on it: the task
spec, the run id and run directory, the launch (it loads the shell-out skill itself when a delegate runs on another
harness), the two-stop checkpoint protocol for writers, the gate, and the verdict it returns. This file carries no copy
of any of that; it says what the supervisor does around it, and where it departs from the delegation skill's defaults it
says so.

<!-- dependency: scode-agent-delegation -->

> Load the exact skill `scode-agent-delegation` through the current harness's authorized skill mechanism. Use its skill
> loader or resource resolver; when it provides no dedicated loader, read the exact `SKILL.md` location supplied by its
> skill catalog or instructions. Known interfaces include the Skill tool on Claude Code, `skill` on OpenCode, and
> `read_skill` on Muse Code. On Oh My Pi (omp), use `read` at `skill://scode-agent-delegation` and
> `skill://scode-agent-delegation/<relative-path>` for sidecars. On Codex, if no skill location is supplied, read
> `${CODEX_HOME:-$HOME/.codex}/skills/scode-agent-delegation/SKILL.md` (the root verified for Codex 0.152). These are
> known interfaces, not a harness allowlist. Do not ask permission merely because the harness is unfamiliar or returns
> no filesystem base-directory metadata; actual tool permissions still apply. Confirm the name is
> `scode-agent-delegation` from the returned frontmatter, or from the loader's reported identity if frontmatter is not
> exposed. Missing or conflicting identity is a load failure. Read the skill in full and every sidecar the current step
> needs. Resolve sidecars through the harness's resolver for that same skill, or relative to its reported base directory
> or the directory containing its supplied `SKILL.md` path. No base directory is required unless needed to address a
> required resource. If output is truncated or elided, retrieve the omitted content through the tool's continuation,
> range reads, or full-output artifact tied to that same resource or result; do not act on incomplete instructions. If
> complete retrieval cannot be established, stop the affected operation. Stop and report `scode-agent-delegation` and
> the failing path, URI, or tool when the skill is unknown, access is denied, identity does not match, or required
> content cannot be resolved or fully read. Do not bypass a denial, guess paths or URI schemes, search other skill
> roots, substitute another copy or similar skill, or continue from memory.

<!-- /dependency -->

## One orchestrator per session

This skill and scode-galaxy-brain are never active in the same session. Each claims every spawn the session makes, and
their rules for which model runs what contradict each other. While this skill is active, do not invoke or load
scode-galaxy-brain or scode-model-routing, and do not follow either from memory.

When galaxy-brain is already active and the user invokes this skill, directly or through a goal file that requires it,
treat that invocation as the user's express instruction to stop galaxy-brain. Say that galaxy-brain is stopped and this
skill replaces it, record that in the run state, and route nothing through galaxy-brain from then on. Delegations it
left in flight are finished under the delegation skill's gate and acted on with this skill's verdict table. The reverse
holds too: if the user later invokes galaxy-brain, this skill stops.

## Staying active for the whole session

Activation is session-scoped. Once invoked, this skill governs every spawn for the rest of the session, including later
tasks the user never mentions it on. Only three things end it: the user expressly asking to stop, the user invoking
galaxy-brain, or an invocation that limited the scope up front ("use scode-powerhorse for <this one thing>"). Finishing
the task it was invoked for does not. Context compaction, session resume, a tool restart, or a summary that fails to
mention the skill does not end it either. After compaction or resume, if the retained context mentions this skill,
chunks in flight, or a chunk table, assume the skill is still active and say that you are assuming it.

## Which model runs what

There is no routing table. Every spawn is one of three kinds.

**Chunks of your own decomposition** run at the _worker selection_: the session's own model and effort, unless the user
or the goal file names another ("use gpt-6.1-sol high for the powerhorses"), at the effort it names, or at the session's
own effort when it names none. When the skill first activates for a run, resolve the selection to a concrete model and
effort, record it in the run state, and set both explicitly at every launch where the mechanism allows it. From then on
the recorded selection is the worker selection, even for a later session that resumes the run on a different model. A
resumed delegate keeps the model and effort it launched with. When no mechanism below can reach the recorded selection,
that is a demanded model nothing can reach: ask the user, and pause only the launches that need it. Never fall back to
another model on your own to keep a chunk moving.

**Spawns another process defines** (the user's per-PR review gate, a review swarm, a scope review, anything a goal file
or another active skill tells you to spawn) run at exactly the model, effort, agent type, and skill that process or the
user demands, and at the session's own model and effort where it names none. Its charter and artifact belong to the
process that defined it. It is not a chunk: it gets no checkpoint addendum, does not count against the writer width
below (though it does use agent capacity), and when it goes through the delegation skill, any verdict but `accepted`
means one relaunch at the same demanded model and effort, then a report to the user, never the chunk rows of the verdict
table.

**Nothing else is delegated by you.** No helper agents for searches, scans, or monitoring. Do small read-only work
yourself or leave it to the chunk whose work needs it. Resource monitoring belongs in a background process, not an
agent.

**Choosing the mechanism** is yours; the delegation skill takes the model, effort, and mechanism as inputs and never
picks a harness. Use `native` (the harness's own sub agent mechanism) when it can run the delegate at the selected model
and effort (inheriting them counts when they are the session's own), and, for a chunk, when it can resume the same agent
after a checkpoint stop (Claude Code and Codex can). Otherwise shell out to the harness that serves the model: `codex
exec` for GPT models, `claude -p` for Claude models, `muse exec` for Muse, `opencode run` for GLM, with the CLI on
`PATH` (for GLM, `opencode providers list` must also show a credential, since `opencode run` without one hangs). If the
user's model routing config file `~/.scode-model-routing.md` exists, honor what it says is unavailable; reading that
file is not loading the routing skill. A demanded model that nothing can reach is a question for the user, never a
silent substitute.

Announce every launch to the user: the chunk, the model and effort, and in one line why delegating it gets the goal done
sooner. A batch launched together shares one announcement.

## Chunks

A chunk is one coherent outcome with its implementation, tests, and documentation, roughly one reviewable PR or one
subsystem, that a delegate can finish without another in-flight chunk's unfinished changes. It has to fit in a
delegate's context and in your ability to verify it in one gate. A chunk too large for either is split along a real
boundary in the work, never into sequential steps of one outcome.

**When a chunk is worth delegating.** Delegate a chunk when it can run while other work is in flight (other chunks, or
your own gating and integration), or when doing it yourself would fill a large share of your context over a long run.
Otherwise do it yourself: a same-model delegate working on a chunk with nothing to overlap buys only overhead. Never
delegate work smaller than a chunk, and never split a chunk to make the fan-out wider.

**Who designs what.** Before launching, you settle product intent, the interfaces and invariants chunks share,
compatibility commitments, and integration order, and put them in the spec as binding boundaries, along with the
implementation outline when one exists. The delegate owns the chunk's internal design within those boundaries. Say so in
the spec, and tell the delegate that its `ASSUMPTIONS.md` must include the design decisions it is making within those
boundaries: which mechanisms, data shapes, and internal interfaces it will add or reuse, and how it handles errors and
edge cases. Naming and local code structure stay out, as the checkpoint addendum says. The first stop approves or
replaces each one like any other assumption. Do not design every chunk in detail before launching it: that puts all the
design on your critical path, one chunk after another, before any parallelism starts. The delegation skill's rule for
performance work still stands; choosing the optimization stays with you.

**Dependencies.** Plan the chunks as a small dependency graph, kept in the chunk table (see Run state). Coupled work
stays in one chunk. An interface several chunks need is settled first, in their specs, or as a small chunk of its own
that lands before them when the work does not already separate along it. A chunk that needs another chunk's code waits
until that code is integrated, and its tree starts from a base that contains it. A final result shaped as a linear stack
of PRs does not limit any of this: independent chunks start from a common base and are put in order at integration.

**PR boundaries and PR reviews.** You own every commit and all PR shaping. When the user's process wants the work as
several PRs and a chunk will span more than one, the chunk's spec names the boundaries its result must respect (file
sets, or ordered stages each of which passes the checks) so that you can split the result without rewriting it.
Otherwise a chunk maps to one PR. A review the user's process requires for each PR runs while other chunks continue;
waiting for it holds back only that PR being marked finished and anything stacked on a rewrite of it. Fix its findings
yourself, or, when they add up to a chunk's worth of work, with one fixup delegation over that PR's change.

**Nested agents.** The ban on helper agents is about your own spawns, because each one costs you a round trip. Inside a
chunk, the delegate may use read-only helper sub agents of its own (search, investigation, a second opinion) at its own
model; that costs you no round trips, though a native chunk's helpers do use the harness capacity you check before
launching. Every chunk spec says so, and also that the delegate must not start any helper that edits files, and that no
review it runs counts as your gate or as a review the user's process requires.

## Width, and where your own time goes

You are the serial stage of this design. Every checkpoint stop, every gate, and every integration waits on you, so more
delegates than you can serve produce a queue, not speed. Keep at most two writer chunks in flight (running, or stopped
at a checkpoint) unless the user or the goal file sets another limit; two is a conservative start, not a measured
optimum.

Before each launch, check that a chunk is ready in the dependency graph, that memory and disk have headroom, and that
the harness has capacity. Most harnesses do not publish a limit on concurrent sub agents; when you cannot find one, the
width limit is the only limit you assume, and a stopped delegate still holds its slot. Leave room for the reviews the
user's process requires. A shelled-out chunk needs an expected duration and a hard deadline per turn; size them from the
chunk and set the deadline generously, since an `over budget` kill of a chunk that was making progress costs more than
the wait. Stop launching new chunks while a resource alert is open, while rate-limit or quota errors are occurring,
while more than one delegate is waiting on you at a stop, or while results wait ungated.

While delegates are in flight, keep your own work to what you can drop between their events: reviewing stops, gating,
integrating, shaping PRs, small fixes. A supervisor deep in a large implementation of its own leaves every stopped
delegate waiting.

Native sub agents generally do not survive the end of the session that started them, so a chunk running natively when a
long unattended run crosses into a fresh session is lost unless it had reached the second stop (see `unresumable`
below). Shelled-out delegates keep a session id that a fresh session can resume at a checkpoint stop; a shelled-out turn
that was still running when the previous session ended was usually killed with it, which is a crash, resumed once. When
the run is likely to cross a session boundary mid-chunk, that is a reason to prefer a shell-out mechanism for chunks
even when native would work.

## Isolation and integration

These rules decide where writers run and who may touch which tree; the mechanics of a delegation are the delegation
skill's.

- Every chunk runs in a tree you create and own, even when it is the only one in flight, because you keep working in the
  main tree while it runs. That tree is a `git worktree`, a jj or Sapling workspace, or a clone, at a path that includes
  the run id, with any branch, bookmark, or workspace name carrying the run id too. Never use a harness's own worktree
  mode that deletes a tree that looks clean when the run exits; the delegation skill says why those are unsafe for
  writers. Shared state outside the tree (build caches pointed outside it, test databases, ports, daemons) is isolated
  too, or those chunks are serialized.
- While a delegate owns a tree (running, or stopped at a checkpoint), you make no edits and run no version-control
  commands in it beyond the read-only status and diff the gate needs. Under jj, where any command snapshots the working
  copy and rewriting a commit rebases its descendants, do not rewrite a commit that a running chunk's tree is based on.
  A detached `git worktree` or a clone at the recorded base keeps a chunk out of reach of your stack edits.
- Editing different files is not isolation. File-disjoint writers in one tree still see each other's half-finished edits
  when they run checks, collide on lock files, generated code, and build state, and drift out of their predicted scope,
  leaving an interleaved diff nobody can attribute or cleanly revert.
- Isolated trees start from committed state, so a delegate does not see uncommitted work in the main tree. Commit it
  first when the delegate needs it, or wait; stashing does not help. A fresh tree also has no build cache, so price in
  the rebuild.
- Integrate accepted results one at a time, following the delegation skill's `integrating.md`, with checks re-run after
  each apply and after the last. Conflicts between accepted results are yours to resolve. Each PR in the final shape
  passes the checks at its own position.
- Remove a tree once its result is integrated and validated, or once it is abandoned with its changes discarded. A tree
  whose result is still ungated holds the only copy of that result.
- Other orchestrating sessions may be running on the same machine and repository. Everything you create for a delegation
  carries a run id you generated, your scratch files live in a private scratch directory (a fresh `mktemp -d`, never a
  fixed name under `/tmp`), and you never remove or reinterpret anything named by an id you did not generate. A run
  directory you did not create is worth mentioning to the user, not a reason to stop.

## Gating at the second stop

The delegation skill's gate applies, with one difference in timing: the deep review of a chunk happens at `AWAITING
REVIEW`, while its delegate still holds its context and can be resumed, instead of after it has finished. A finished
delegate cannot be resumed under the checkpoint protocol, so a defect beyond a local fix found at the final gate costs a
fresh delegation that has to re-read the whole chunk; the same defect found at the second stop costs one line in
`REVIEW.md`.

- At `AWAITING GUIDANCE`, review `ASSUMPTIONS.md` as the gate says. For a chunk it carries the design decisions, so
  check them against the boundaries and the user's request before approving them.
- At `AWAITING REVIEW`, mark the chunk as at the second stop in the chunk table at once, before reviewing, so that a
  session that dies during the review still knows a complete implementation is in the tree. Then run the whole gate now:
  take status and diff, re-run the checks, read `DECISIONS.md` in full, and check the work against what the user asked
  for. Every defect the delegate can fix within the current spec goes into `REVIEW.md`: a change against the
  `DECISIONS.md` entry it concerns, or a numbered `Also:` item at the end when no entry covers it. A finding the gate
  maps to `spec defect`, `blocked on user`, or `substantive failure, structural` returns that verdict instead, as the
  gate says; it is never smuggled into an `Also:` item. Before resuming, save the complete change set you reviewed
  (status, diff, and the full contents of untracked files) to a scratch file named by the run id.
- After `REPORT.md`, finish the gate against what you saved: take the complete change set again, re-run the checks, read
  closely everything that differs from the saved copy and every `DECISIONS.md` entry added after your review, and
  confirm that each change and `Also:` item landed. Then return the verdict.

## Acting on a verdict

The gate returns exactly one verdict per delegation; this table says what you do with it for a chunk. (For a spawn
another process defines, see Which model runs what.) "The worker selection" is the one recorded there. Every relaunch,
fixup, and redelegation is a new delegation with a fresh run id. Counters are per lineage, the chain of run ids that
started from one chunk spec. After completing any row but `blocked on user`, tell the delegation skill the verdict is
acted on so it moves the run directory to scratch, and update the chunk table. A fixup that needs to read the primary's
run directory is given its path after that move.

| verdict                                 | action                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | after the cap                                 |
| --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| `accepted`, `accepted with local fixes` | integrate (per `integrating.md`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |                                               |
| `spec defect`                           | fix the spec; redelegate at the worker selection with the corrected spec, keeping the delegate's changes when the new spec extends the old one and removing them otherwise, or take the chunk over                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | one redelegation, then take over              |
| `substantive failure, fixable`          | one fixup delegation at the worker selection over the same tree, with the failures as evidence. With review at the second stop this verdict should be rare: it usually means a defect introduced after that stop or an `Also:` item not applied                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | take over, or replan the chunk                |
| `substantive failure, structural`       | keep pre-existing user work and remove only the delegate's changes; work out whether the chunk's boundary, spec, or size was wrong, then make one fresh attempt at the worker selection with the concrete failures as evidence, or take over. With reason `lost context`, the chunk was too large for one delegate: split it along a real boundary first                                                                                                                                                                                                                                                                                                                                                               | take over                                     |
| `execution-path failure`                | fix the path; relaunch once from a cleaned tree                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | take over, or another mechanism for the model |
| `misclassified`                         | the chunk's shape was wrong, or a checkpoint's reply cap ran out: replan (split, merge, or settle what was unclear) and relaunch, or take over; keep usable partial work unless the cause was the reply cap or a structurally bad patch                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | take over                                     |
| `inconclusive`, reason `crash recurred` | fix the environment; relaunch from a cleaned tree. Not when the deaths were rate limits; see below                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | take over                                     |
| `inconclusive`, reason `over budget`    | report it and judge whether the chunk was mis-sized before anything else; no automatic relaunch                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | replan as `misclassified`, or take over       |
| `unresumable`                           | when the chunk table records that the chunk reached the second stop (or you saved its change set there), its tree holds a complete implementation: do not discard it. The delegation is over; take the work over. Re-run the checks on the tree, apply or verify every outstanding answer and `REVIEW.md` item yourself, run the gate's remaining steps on the current change set, and write a short recovery note in the run state in place of the `REPORT.md` that will never come. Then finish the chunk yourself, or launch one fixup delegation over that tree with the remaining defects. Otherwise remove the delegate's changes and relaunch at the worker selection with the answers folded into a fresh spec | take over                                     |
| `blocked on user`                       | report the question; pause only the chunks that depend on the answer and keep independent chunks going; leave the tree and the run directory as they are until the user answers                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |                                               |

"Remove" and "clean" mean you, against the attribution baseline the delegation skill had you record, before any new
delegation starts in that tree; the delegation skill never touches tree changes.

A turn that ended on a rate-limit or quota error is an environmental cause, however the message reads. This departs from
the delegation skill's classification, which would read a delegate reporting a failure it did not work around as a
decline. Stop launching, wait until the limit clears (that is the fix; tell the user when the wait runs past an hour),
then resume the delegate as the delegation skill resumes a crash. Rate-limit endings do not count toward the delegation
skill's one-crash-resume cap: each one waits and resumes, and a `crash recurred` that was only rate limits is never a
reason to clean the tree. Never discard its work as a decline.

## Run state and handoffs

Keep the chunk table in a file of its own, edited in place, because a resume log is append-only and a table rewritten
into it on every change would bury the history a resumer has to read. When a goal file names a working log, the table
lives next to it at the path the goal file gives (or, if it gives none, the log's path with `-log.md` replaced by
`-chunks.md`), and the log records that path. Otherwise it lives in your private scratch directory and you tell the user
where. One row per chunk:

- the outcome and acceptance criteria, the chunks it depends on, and the interfaces it owns or relies on;
- the run id, the run directory's absolute path, the tree, and the base it started from;
- the model, effort, and mechanism, and the handle needed to resume it (native sub agent id, or session or thread id);
- the phase: planned, running, at the first stop, at the second stop, gating, integrated, or abandoned;
- the lineage's counters, the PR or PRs it maps to, and the next action.

Update a row at every phase change, not only before a compaction. Alongside the table, record the worker selection, the
width limit, whether galaxy-brain was stopped, and the paths of the change sets saved at second stops. Log each phase
change in the working log too, as an ordinary append-only entry.

On resume after compaction or in a fresh session, reconcile the table against reality before launching anything: run
directories, trees, running processes, and which handles still resolve. A writer still running must not race a
replacement. A native handle from a previous root session generally does not resolve; that chunk is `unresumable` and
the table above says what happens to its work.

For each chunk, also record how many assumptions you replaced at the first stop and how many changes and `Also:` items
you sent at the second. Whether the two stops earn their cost with strong delegates has not been measured, and this is
the evidence that would settle it.

## Reporting

In your final report, say which chunks were delegated and at what model and effort, separate what the gate confirmed
from delegate claims that were not verified, summarize the checkpoint evidence, and say where the run state lives.

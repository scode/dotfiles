---
name: scode-agent-delegation
description: >
  How to work with a delegate once a caller has decided to use one: the self-contained task spec, the artifact-file
  rule, the no-VCS rule, the run id and run directory, the two-stop checkpoint protocol for delegates that write code,
  crash and resume handling, and the gate that judges the result and returns one of a fixed set of verdicts. Loaded by
  orchestration skills at the moment they delegate; inert on its own, never active for a session, and not meant to be
  invoked by the user directly.
---

# scode-agent-delegation

This skill is never active. It answers when loaded and claims nothing about later work. Loading it writes no state,
claims no later spawn, and changes nothing about the session. It exists to be read by a skill that has already decided
to delegate a unit of work and chosen a model, effort, and launch mechanism for it, and now needs the procedure: how to
specify the task, how to start the run, how to review a writer at its two stops, how to judge what comes back, and what
to tell the caller. The caller keeps the decisions this skill does not make: whether to delegate at all, which model,
whether writers may run concurrently and in which trees, and what to do with each verdict.

## What the caller supplies

- The unit of work: its goal, scope, acceptance criteria, and the checks to run, from the caller's own decomposition.
- The model, effort, and launch mechanism, one of `native`, `codex exec`, `claude -p`, `muse exec`, or `opencode run`.
- Whether the delegate edits a tree (a writer) or is read-only.
- After this skill has generated the run id (see The three-step start): the tree the delegate will edit, created and
  owned by the caller when the delegate is to be isolated, named by that run id.
- For an isolated tree, the recorded base it started from, so the delegate's changes are attributable.
- An expected duration and a hard deadline, when the mechanism is a shell-out.

## Writing the task spec

The delegate has none of the caller's conversation context. Every delegation prompt must be self-contained:

- The goal and any constraints that bound it.
- The caller's relevant user request and later decisions, binding repository constraints, and implementation outline
  when one exists. Keep these distinct from proposed mechanisms so the gate can detect both missing behavior and
  unnecessary obligations; a spec's existence does not prove every requirement in it came from the user.
- Exact file paths or directories in scope.
- Acceptance criteria: what done looks like, concretely.
- For a performance request — one whose success is a measurable improvement in runtime behavior or resource use:
  latency, throughput, CPU or memory, I/O, artifact size, workload-dependent cost — a baseline measurement comes first,
  before the spec is finished and before the delegate writes its assumptions. (Source code or an API being "smaller", or
  a price being lower, is not this kind of request.) Time it, profile it, count the syscalls, whatever fits, provided
  running the workload is safe, authorized, and feasible here; when it is not, or when the user already supplied
  numbers, say so in the spec and agree with the user on a proxy or on proceeding without a measured claim — never run a
  risky workload just because measurement comes first, and never invent numbers. Put the baseline, the workload, and the
  evidence about the bottleneck in the spec, and say whether the bottleneck is demonstrated or a hypothesis; do not ask
  the implementation delegate to pick the optimization — that is design, which is the caller's, though a read-only
  delegate may gather the profile. This exists because designing a speedup before knowing what is slow produced, in one
  eval, a persistent cache approved at the assumptions checkpoint and torn out later, and an accepted "optimization"
  never shown to change anything, while the one run that profiled first fixed the real bottleneck in one attempt.
  Acceptance needs a before/after comparison of the user-relevant metric on the same workload under comparable
  conditions, with enough runs to beat noise; a test that only proves the fast path executes is not acceptance evidence.
- Which checks to run (tests, linters, formatters) before reporting back.
- For read-only tasks: state explicitly that it must not edit any files.
- Always: no commits, no branches, no pushes, no PRs. The caller's session carries the user's version-control workflow
  and preferences, and those do not transfer to a delegate.
- Ask it to report what it did and call out any deviations from the spec. For writers, the report is `REPORT.md` and its
  Deviations section points at the checkpoint files (see `checkpoints.md`).
- For every writer: append the checkpoint addendum from `checkpoints.md`. This is not optional and does not scale with
  task size; a writer too small to be worth two resumes is a task for the caller to do itself, not a reason to skip the
  protocol.
- When the deliverable is prose rather than code — a review, an analysis, a scan result — name a file the delegate must
  write it to, and make that file part of the acceptance criteria. Every harness captures only the delegate's final
  message (`codex -o`, `claude -p` stdout, opencode's last `text` event), and a delegate that emits its real output
  mid-run and closes with a summary leaves the deliverable stranded in its transcript. That has happened: a review
  delegate's result file held a two-kilobyte recap while the twenty findings it produced lived only in a megabyte of
  transcript, and recovering them meant grepping the log. The gate reads artifacts; make the delegate produce one. The
  checkpoint files are artifacts in exactly this sense: `ASSUMPTIONS.md`, `DECISIONS.md`, and `REPORT.md` are what the
  gate reads, not the transcript.

Before a delegate writes to a shared tree, note the current working-copy state — `git status`/`git diff` or the
equivalent in whatever VCS is in use — so its changes can be attributed cleanly afterwards. A writer in an isolated tree
is attributable as long as the tree started clean from the base the caller recorded when creating it.

## The three-step start

Every delegation gets a _run id_ and a _run directory_, and this skill owns both.

1. Generate the run id — `uuidgen`, or a UTC timestamp plus random suffix if that is what you have — the moment you
   decide to launch, and record it in your notes and in any handoff at once. It names everything this delegation owns:
   the run directory, the scratch files and logs kept for it, any tree the caller creates for it, and the handle in
   handoff notes. A run directory whose id you have lost is one you can no longer tell from another session's.
2. The caller supplies the tree the delegate will work in: the shared tree, or an isolated tree the caller creates and
   names by the run id (the caller decides whether isolation is allowed and what kind of tree it is).
3. Create the run directory, `<tree>/.agent-delegation/<run-id>/`, empty, immediately before the launch. First attempts,
   escalations, fixup rounds, and reroutes each get a fresh id and directory. `checkpoints.md` says how the shared
   `.agent-delegation/` directory is created, hidden from version control, and kept out of other sessions' way.

The run directory's whole lifecycle is this skill's: it is created here, filled by the checkpoint protocol, read by the
gate, and moved to the session's private scratch space (a directory only this session uses, e.g. a fresh `mktemp -d`,
never a fixed name under `/tmp`) once the caller reports that it has acted on the verdict. It is the record of what was
asked and what was settled — `REPORT.md`'s Deviations point into it, a fixup round to another model may need to read the
primary's `DECISIONS.md`, and the caller's own `ANSWERS.md`/`REVIEW.md` are the decisions it made. The caller owns the
changes in the tree; this skill never removes or reverts them.

## Launching

For a `native` mechanism, launch through the harness's own sub agent mechanism with the model and effort the caller
chose. For any other mechanism, load `scode-harness-shellout` as follows and follow it; the launch, monitoring, and kill
rules and the exact commands live there and nowhere else.

<!-- dependency: scode-harness-shellout -->

> Load the exact skill `scode-harness-shellout` through the current harness's authorized skill mechanism. Use its skill
> loader or resource resolver; when it provides no dedicated loader, read the exact `SKILL.md` location supplied by its
> skill catalog or instructions. Known interfaces include the Skill tool on Claude Code, `skill` on OpenCode, and
> `read_skill` on Muse Code. On Oh My Pi (omp), use `read` at `skill://scode-harness-shellout` and
> `skill://scode-harness-shellout/<relative-path>` for sidecars. On Codex, if no skill location is supplied, read
> `${CODEX_HOME:-$HOME/.codex}/skills/scode-harness-shellout/SKILL.md` (the root verified for Codex 0.152). These are
> known interfaces, not a harness allowlist. Do not ask permission merely because the harness is unfamiliar or returns
> no filesystem base-directory metadata; actual tool permissions still apply. Confirm the name is
> `scode-harness-shellout` from the returned frontmatter, or from the loader's reported identity if frontmatter is not
> exposed. Missing or conflicting identity is a load failure. Read the skill in full and every sidecar the current step
> needs. Resolve sidecars through the harness's resolver for that same skill, or relative to its reported base directory
> or the directory containing its supplied `SKILL.md` path. No base directory is required unless needed to address a
> required resource. If output is truncated or elided, retrieve the omitted content through the tool's continuation,
> range reads, or full-output artifact tied to that same resource or result; do not act on incomplete instructions. If
> complete retrieval cannot be established, stop the affected operation. Stop and report `scode-harness-shellout` and
> the failing path, URI, or tool when the skill is unknown, access is denied, identity does not match, or required
> content cannot be resolved or fully read. Do not bypass a denial, guess paths or URI schemes, search other skill
> roots, substitute another copy or similar skill, or continue from memory.

<!-- /dependency -->

Name every sub agent (label, description, or whatever the mechanism displays) so the name includes the task plus the
model and effort actually doing the work, e.g. `fix-foo-gpt-5.6-sol-medium`. Harness UIs otherwise show only the wrapper
or default model, which misleads anyone watching progress.

A delegated unit that is itself a coordinator — one that runs a process skill with its own fan-out — is one delegation,
not one per subagent it creates. When its harness has a native delegation tool, launch it with the harness's full tool
set and let it run its own fan-out rather than flattening that fan-out into separate foreign-harness processes to keep
model choice or a prompt-level read-only boundary at the outer session. A process skill that requires exact models for
its internal roles must be included in the coordinator's task; otherwise its native agent configuration owns those
internal choices. For an OpenCode coordinator, the task also carries the native-task manifest and monitoring contract
from `scode-harness-shellout`'s `harness/opencode.md`, and the outer session must not infer fan-out shape or progress
from the number or timing of the `task` events it can see.

## Waiting for native delegates

Use the completion mechanism the current harness exposes. When native delegates deliver results automatically, rely on
that delivery instead of repeatedly listing agents, asking for status, or checking result files. A native call that
blocks until completion also avoids polling; it does not need a background-notification mechanism. Do independent work
while a delegate runs when the harness permits it. When only a bounded wait is available, use the longest wait allowed
by the tool and the caller's responsiveness, resource-check, and deadline requirements, and check outstanding delegates
together when control returns. A wait expiring means the delegate may still be running, not that it failed.

The cost to avoid is repeated model turns with no new information: even a tiny status reply can cause the conversation
prefix to be processed again as cached input. Prefer monitoring without model turns. A small-context watcher agent is
reasonable when it can notify the caller directly and is expected to cost less than repeated caller wakeups; account for
its launch overhead, model price, and polling usage. Give it only the monitoring context it needs. The caller must not
have to poll the watcher to receive its alerts. Completion delivery differs by harness, version, and configuration;
neither a status file nor a UI notification proves that the model will be woken. Retain required resource checks and
deadlines even when completion is delivered automatically. Foreign processes use the monitoring procedure in the
shell-out dependency instead.

## Writers stop twice, and must be resumable

Every writer delegation — native or shelled out, any family, any size — runs the two-checkpoint protocol in
`checkpoints.md`: the delegate stops once before writing code and once after the checks pass, and the caller reviews at
each stop. Read this skill's `checkpoints.md` in full before the first writer launch of a session. Read-only delegates
(reviews, scans, analysis) do not use it; their contract is the named artifact above.

So every writer path must support resuming the same delegate in the same session. Natively that is a follow-up message
to the same sub agent (`SendMessage` in Claude Code, or the harness's equivalent); shelled out, `scode-harness-shellout`
gives the resume mechanics and its file for the mechanism the verified command, keyed on a session or thread id recorded
at launch. A path that cannot resume is not a writer path. Native resume is verified for Claude Code and for Codex
(0.151.0, a gpt-5.6-sol orchestrator resuming a native gpt-5.6-terra sub agent through both checkpoints, 2026-08-30).

A run can end without the stop it was heading for. `checkpoints.md` classifies what came back: a skipped stop, a repeat
stop, a delegate that chose to stop (a decline, a question, status instead of work), an execution-path failure (the
launch never got going), and a crash (an environmental cause the caller has verified and fixed). Only a crash is
resumed, once; a second death the same way ends the delegation. Each classification either continues the protocol or
returns one of the verdicts below.

## The gate and its verdicts

The caller is the quality gate for everything a delegate produces, and this skill's `gate.md` is the procedure: read it
the first time in a session that a delegate finishes or stops. Never accept a delegate's self-report as evidence the
work is good. The gate reads the diff, re-runs the checks, reads the checkpoint files in full, and goes back to what the
user actually asked for. It ends in exactly one of these verdicts, which is what this skill returns to the caller:

| verdict                           | meaning                                                                                                                                           | payload                                                                                                                         |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `accepted`                        | diff, checks, decisions, and the user's request all hold                                                                                          | optional note of what a later unit still owes                                                                                   |
| `accepted with local fixes`       | as above after the caller fixed small defects itself (naming, comments, minor logic; a fixup round-trip costs more)                               | what was fixed                                                                                                                  |
| `spec defect`                     | the spec drops requested behavior or adds unnecessary obligations, whether found before code or in a diff correct against that spec               | the missing behavior or unjustified obligation and smallest correction preserving user requirements and repository constraints  |
| `substantive failure, fixable`    | wrong or incomplete against the spec, but well-specified defects one fixup round can close                                                        | the concrete acceptance failures                                                                                                |
| `substantive failure, structural` | broadly wrong, a decline or status turn, or a replacement at the assumptions stop that invalidates the design; repairing is worse than restarting | the acceptance failures or the delegate's message; reason `lost context` when a delegate lost track of earlier context mid-task |
| `execution-path failure`          | the delegate never did the work because of the launch path: a hang, a launch error, a sandbox that refused to start                               | the signature seen                                                                                                              |
| `misclassified`                   | the output shows the profile was wrong for the task, or the reply cap was exhausted at one stop                                                   | the cause (profile, or cap exhaustion); for cap exhaustion, that the accumulated answers must not be pasted into the next spec  |
| `inconclusive`                    | reason `over budget` (killed at the hard deadline) or `crash recurred` (a second death after the one crash resume)                                | the reason and the log path                                                                                                     |
| `unresumable`                     | the session or thread handle no longer resolves, or the resumed turn came back as a fresh session; the work cannot be continued                   | the answers already given, to fold into a fresh spec                                                                            |
| `blocked on user`                 | the gate found something only the user can resolve; not a delegate's question, which is a decline above                                           | the question                                                                                                                    |

What happens next — a fixup round, an escalation, a reroute, taking the work over, removing the delegate's changes — is
the caller's decision from its own rules; this skill reports and does not act on the tree. When the caller integrates an
isolated writer's result, `integrating.md` is the procedure.

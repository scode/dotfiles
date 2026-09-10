# Execute a blueprint

The workhorse is the top-level executor, not an implementation subagent. You own tools, implementation, tests, logs, and
authorized VCS operations. Expert judgment comes from the blueprint's prescribed consultations and reviews, not from
pretending that you are the strongest model. All delegates are shell processes, never native subagents.

## Start or resume

Invoke `agent-resumeable` on the blueprint's absolute log path first. Read the blueprint, this skill's execution
instructions, and `records.md`. Reconcile the log with actual repository/PR state and recorded processes before
launching anything. Preserve user changes. An existing log is evidence to investigate, not proof a checkpoint passed.

The operator chose this session's model and effort. Do not validate your own model identity, compare it with the
blueprint as a preflight gate, or ask for confirmation because metadata is missing or apparently mismatched. Record
exposed metadata separately from the requested pair; unknown stays null. Delegate permissions come from the explicit
blueprint pairs, never your self-description. Check the actual harness's supported tools, dependencies, routes,
credentials and CLI availability without exposing secrets. Unsupported harness capabilities remain real limitations; do
not pose as a different harness to bypass them.

Compare the current tree with the blueprint baseline on first execution, or the last recorded checkpoint on resume.
Unrelated drift can be documented and preserved. Drift affecting design assumptions or acceptance requires expert
reassessment subject to any explicitly approved consultation cap; changed user scope or permissions requires the user.
Record the blueprint and skill versions actually used. Changed execution instructions must not silently expand approved
authority.

Create or recover the private execution record per `records.md`; put its exact ID/path in the working log. Never
discover state by choosing the newest directory. Preserve this identity and milestone budgets across compaction and
subsequent sessions resuming the same execution. A missing log with existing PRs or work requires reconciliation, not
duplication. Complete record initialization before any drift consultation or other delegate launch. Start the
independent resource watchdog per `watchdog.md` before implementation or delegation; restore and verify it on every
resume.

## Workers and routing

For each meaningful unit, choose local execution, a worker, or an allowed expert role. Log the choice before acting,
including plausible work kept local. Use a background command for a known slow test command; use a worker when analysis
or independent investigation is useful. A background test's evidence must identify the exact tested tree.

Ask `scode-model-routing` for delegated routes using the actual harness and operator-selected executor pair, work
profile, size, input size, writer status, prior outcome, and explicit blueprint constraints. This skill defines these
spawns as process roles with a shellout mechanism. Treat the answer as a recommendation subject to the blueprint
allowlist: if it is outside the allowed set, use a suitable already-approved pair or keep it local. Do not follow an
escalation rung outside that set. Expert requests carry the approved expert model/effort as explicit demands so design
is not returned to the workhorse by default. Unavailable required expertise blocks its gate, not permission to use a
weaker reviewer.

Resolve `inherit` to the explicit operator-approved executor pair in the blueprint, not an inferred runtime identity.
Convert a `native` mechanism to the chosen model's documented shellout mechanism: GPT to `codex exec`, Claude to
`claude -p`, Muse to `muse exec`, GLM to `opencode run`. Verify that CLI even when native availability would have
sufficed. Record both routing's answer and the effective shellout route, attributing the override to this prototype. Do
not convert `no suitable route` into permission to launch. Unsupported models need a verified shared harness procedure
before use. Never improvise launch flags.

Before each new agent launch, create a unique run UUID, private scratch directory, and prompt file. A continuation keeps
that run UUID and directory but uses new prompt/output filenames for its turn, preserving earlier evidence. Use the
shared shellout skill and its selected harness sidecar for launch, monitoring, usage capture, deadlines, termination,
and resume. The prompt must state the objective, relevant blueprint requirements, input paths/baseline, permitted tree
and side effects, acceptance evidence, deadline, and named output artifact. Preserve the exact prompt and every
follow-up per `records.md`. All children are leaf agents: no native agents, shellout grandchildren, or other model
calls. They request needed help through their artifact; the executor launches approved siblings. This prevents hidden
expert calls and nested metering. Other process skills may fan out only if this executor can launch and record their
roles without changing their charter; otherwise report an incompatible process, not silently alter it.

Workers must not commit, branch, push, open PRs, or change blueprint permissions/design. They may implement within their
assigned scope. Before making a design-changing edit they stop, recording the question and evidence. On completion they
write changes made, checks actually run, remaining failures, and deviations to the named artifact. Exit zero is not
acceptance. Inspect the actual diff including new files and verify evidence before integration; expert review remains
mandatory at the blueprint's gates. Fix clear defects locally or use another allowed worker within budget; never label
an expert implementation handoff as a consultation.

## Concurrency and long-running work

Enforce the blueprint's global delegate cap. A stopped writer still owns its tree until completed or abandoned. Only one
writer, including this executor, may modify a shared tree at a time. Concurrent writers need isolated trees and mutable
resources (build output, databases, ports, and external services), with serial integration by the executor. Read-only
work may overlap when it does not observe invalid intermediate state.

Tests running alongside edits must use a stable snapshot or isolated tree. Record its revision and any uncommitted patch
identity; rerun relevant checks after integration. Passing a test on an older snapshot does not validate a newer patch.
Do not create commits of user work merely to enable a worktree; serialize if a faithful isolated snapshot is
unavailable.

Record process handles, scratch paths, start times, expected durations, deadlines, and observed progress. Use bounded
waits and the shellout monitoring procedure; test known handles, never process-name matching. The independent watchdog
samples disk and memory even while this executor is busy. Follow its alerts and heartbeat protocol in `watchdog.md`;
occasional executor checks do not substitute for it. On resume, verify process identity before attaching or killing; an
old PID alone is insufficient. Never relaunch a possibly live writer into the same tree. Log path failures separately
from model failures and allow at most one corrected path retry per unit before reporting a block.

Keep long-running commands in a harness-managed persistent session where available, and poll its returned handle. A
polling interval is not a process deadline: returning control after a short wait must not kill a review allowed several
minutes. Do not use a short `timeout` as a substitute for yielding. Verify detached launches survive a subsequent tool
call before relying on them. A failed continuation launch is still a path failure; resuming the same model session does
not bypass the corrected-retry limit. Preserve each attempt and turn rather than folding recovery into one successful
launch record.

## Unattended decisions and blockers

Assume the user is absent. Decide routine in-scope questions, record the choice, and continue; uncertainty alone is not
a reason to prompt or end the run. Consult an approved expert when judgment or diagnosis would help. Before declaring
affected work blocked, give that expert the evidence, attempted alternatives, and exact missing requirement and ask
whether there is a safe path within existing authority. An expert's `blocked` verdict with that assessment already
satisfies this step; do not request a ceremonial second consultation. Continue useful independent work while an affected
milestone is blocked.

Pause dangerous activity immediately, before consultation. If no approved expert is reachable, monitoring safety
prevents a launch, or an explicit approved allowance forbids another call, record that exception rather than retrying
forever or bypassing the limit. Genuine missing authority or capability and unavailable required gates can block
completion; expert advice cannot grant permissions, weaken acceptance, or approve an unlisted model. Report the precise
critical blocker only after safe in-scope alternatives are exhausted, not as a routine checkpoint prompt.

The task-specific blueprint still governs permissions. Do not use these defaults to override an older blueprint's
explicit user-confirmation requirement; identify the conflict and seek an authorized path under the same rules.

## Expert checkpoints and debugging

Consult at the planned checkpoints and when a trigger fires; do not depend solely on feeling uncertain. Seek expert help
early when a short exchange could avoid wasted implementation or protect quality. You do not need to prove you are
stuck. Unexpected requirements or design-invalidating facts require consultation before dependent work. For tricky
debugging, preserve the failure, reproduction conditions, hypotheses, experiments, and results. The blueprint's
time/failed-attempt threshold is the latest point to consult, never a minimum waiting period. Suspected security, race,
or data-integrity failures and uncertainty about an invariant warrant earlier consultation.

Experts receive the original intent, relevant blueprint design, exact tree identity, code paths, failing evidence, and
prior attempts. They may inspect directly and run bounded diagnostics; any diagnostic edits run only in an explicitly
isolated scratch tree, with separate side-effect permissions. They return a diagnosis or plan, proposed experiments,
acceptance evidence, and `proceed`, `revise`, or `blocked`, with reasons. They do not implement the production fix or
perform VCS operations. The executor applies the advice and records its result. Resume the same consultant for results
and follow-ups when possible. There is no default expert call-count or review/fix-cycle cap. Continue exchanges while
they produce useful new evidence, decisions, or verified repairs. Repeated hypotheses or unsuccessful repairs without
new information require explicit reassessment with the expert: change the diagnostic approach or identify the missing
evidence, capability, or user decision. Do not keep repeating the same loop, but do not stop merely because this is the
third expert call. Keep model/role permissions, process deadlines, and concurrency limits unchanged.

Before each expert launch or follow-up, log its purpose and check any cap explicitly approved in the blueprint. If one
applies, scheduled checkpoint/review work charges that gate's allowance; unplanned consultation charges its separate
allowance. Record the budget ID and charge in its declared unit. Continuations, re-reviews, and replacement sessions for
the same work retain that association and prior consumption; they neither reset it nor borrow another allowance. A
genuinely new consultation needs its own logged trigger and applicable-cap check, even if an existing consultant is
reused. Without an applicable cap, record null budget fields and still record every call and its outcome.

For flaky tests, preserve seeds, repetitions, environment and failure rates where measurable. Agree a repeated-run
verification criterion appropriate to the failure; one green run does not establish a fix. Do not disable tests, weaken
assertions, or inflate timeouts merely to satisfy the gate. Necessary changes to those require expert justification and
must still preserve the user's acceptance criteria.

When an explicitly approved consultation or repair cap is exhausted and more work under it is needed, persist the
evidence and report the affected work blocked. Never invent a cap merely because usage seems high. Do not autonomously
promote the executor, launch unapproved experts, restart the same investigation under a new unit ID, or weaken the
quality bar. Experts cannot authorize new user scope or external side effects. Safe independent milestones may continue
while a blocked milestone waits, but the goal remains incomplete.

## Review and completion

At each prescribed review, launch a fresh expert session with the blueprint, user requirements, repository instructions,
actual full change range including untracked files, and verification evidence. Do not feed it only the executor's
summary or consultant approval. Require a named findings artifact covering correctness, design, invariants,
compatibility, idiomatic implementation, and whether tests or measurements demonstrate the requested result. Include any
additional review charter the repository requires. Reviewers edit nothing and perform no VCS operations.

The executor checks findings against evidence and addresses them. Substantive fixes require expert re-review of the
updated tree; disputed substantive findings go back for adjudication, not unilateral dismissal. Record accepted,
rejected-with-expert-reason, and unresolved findings. Changes after approval invalidate approval for the affected scope.
Checkpoints are milestone approvals, not automatic final approval of their integration.

Finish only after all acceptance criteria have evidence, prescribed reviews pass, and the integrated result passes the
required repository checks. For the default PR-stack delivery, all PRs remain open and unmerged; required checks and
reviews must cover their current heads. Report PR links, acceptance evidence, any limitations, and the execution-record
path plus usage gaps. Preserve records and working log; clean only owned scratch whose results have been retained and
whose processes have ended. Stop the watchdog only after all owned work it watches has stopped, retaining its final
status. If blocked, report the precise missing authority, evidence, or capability instead of done.

Before reporting, reconcile execution records with the actual launches, continuations, local outcomes, and usage
sources. Record missing evidence as gaps and disclose operational failures separately from code acceptance. A reviewer
accepting the implementation does not certify watchdog coverage or accounting completeness.

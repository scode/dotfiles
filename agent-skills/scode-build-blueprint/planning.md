# Build the blueprint

Do not start implementation or launch the future executor. Explore the repository and existing documentation in the
current session. Establish the user's intent, architecture, constraints, and how the result can be verified. Resolve
questions the code answers before asking the user. Read-only checks are normal exploration; ask before experiments that
would mutate the target repository or external systems. Use isolated scratch for approved diagnostic experiments.

## Settle the execution contract

Batch questions about scope, compatibility, design alternatives, acceptance criteria, and authorization. Propose and
confirm the executor's harness/model/effort, worker allowlist, expert routes, checkpoints, and operational constraints.
Use model routing to inform proposals, not to replace the user's choice of top-level model. Confirm the actual
model/effort pairs rather than phrases such as "anything cheaper". Resolve material forks before writing; do not promise
that all uncertainty can be removed up front.

Default worker permission is the executing session's confirmed model and effort. Additional workers require explicit
model/effort pairs approved here. Experts have separate approved pairs for design/debugging consultation and independent
critical review, selected from routing's state-of-the-art choices. A worker permission must not grant those expert
routes general implementation authority. If the user chooses a state-of-the-art executor, explain that this no longer
tests workhorse execution and settle the worker pool explicitly rather than silently granting same-model expert workers.

The operator selects the executor; do not embed self-model verification or identity-confirmation gates in the blueprint.
Spell out the approved worker pairs so a missing or misleading self-description cannot expand permissions. Plan for an
absent user: settle material scope and authority questions here, leave routine in-scope choices autonomous, and require
approved expert consultation before declaring a critical blocker, subject to execution.md's safety and
unavailable-consultation exceptions. Do not add routine stop-and-ask checkpoints.

Propose at most two concurrent model delegates, serialized shared-tree writers, and a small number of meaningful
milestones. Encourage early expert consultation when it could avoid wasted work or protect quality. Set a latest point
for seeking debugging help (default 30 minutes or two failed repair attempts, whichever happens first), not a minimum
amount of struggling before help is allowed. Explicit risk and design-invalidating triggers apply sooner.

Do not impose or routinely propose expert call-count or review/fix-cycle caps. The initial experiment favors useful
expert involvement; logs will show whether it is excessive. Continue useful exchanges and require reassessment when they
stop producing new evidence or decisions. A numerical cap is exceptional: include one only for a clear, goal-specific
reason, with the user's approval, and record that reason, scope, counting unit, and stable mnemonic budget ID. Do not
use generic cost caution as the justification. When caps exist, scheduled gates and unplanned consultations have
separate allowances; exhaustion blocks affected work rather than lowering the quality bar.

Give every delegate an expected duration and hard deadline appropriate to its task. These deadlines and concurrency
limits remain operational safeguards even without call-count caps. They are not token or dollar caps; measured usage is
recorded separately. Record any explicit user spending limit without implying that absent counters can enforce it.

Default delivery is a linear stack of reviewable, open, unmerged draft PRs using `jjstack`, reviewed per PR and finally
against the integrated goal. Confirm that scope, any alternative VCS workflow, network/external write permissions, and
whether the unattended executor may create/push PRs. Goal completion does not authorize merging. Required repository
review processes still apply; settle their compatibility with the model allowlist and shellout-only rule now.

## Paths and baseline

Pick mnemonic `<project>-<goal>-blueprint.md` and `<project>-<goal>-blueprint-log.md` names, normally beside the
checkout in its writable parent directory outside VCS. Use the outermost checkout when nested, excluding a
home-directory VCS root from nesting. Honor an explicitly chosen directory. Show both absolute paths before writing; if
no suitable durable location exists, ask for one or propose a clearly temporary location. Never overwrite an existing
blueprint or log: choose a distinct mnemonic name or ask before moving the existing pair aside. Do not create the
execution log, even empty.

Record the absolute target repository path, current revision, and relevant uncommitted state. Do not commit, stash, or
discard the user's work to obtain a clean baseline. Identify file contents the design depends on by revision plus path,
or content hashes when uncommitted. Include enough context to recognize meaningful drift at execution time; a hash is
not a substitute for a missing design explanation. Record this skill's available revision or content hashes too.

## Blueprint content

Write instructions addressed to a fresh executor. Include:

- **Execution entrypoint:** "This is execution, not blueprint creation. Invoke `agent-resumeable` with the absolute
  working-log path, then load `scode-build-blueprint` and follow its execution phase. Do not activate Galaxy Brain."
  Explain that a missing log means start; an existing log means reconcile and resume, never start over blindly.
- **Intent and acceptance:** the user's requirements and settled clarifications, non-goals, and evidence required for
  each outcome. Include the original request where useful so review can catch a narrowed interpretation.
- **Design:** relevant code locations and patterns, interfaces, invariants, failure behavior, migration/compatibility
  requirements, decisions and their rationale, rejected alternatives, and unresolved factual assumptions.
- **Milestones:** dependency order, bounded deliverables, checks, expert checkpoints, and what evidence each checkpoint
  receives. Place checkpoints before expensive dependent work. Avoid prescribing every edit; leave routine details to
  the executor within the design constraints.
- **Authority and routes:** confirmed executor, explicit worker and expert model/effort pairs, permitted expert roles,
  launch policy, concurrency limits, any explicitly approved budgets and their reasons, external permissions, VCS and
  review requirements, and completion criterion. Same-model workers mean the executor, never the planning session.
  Availability is rechecked at execution.
- **Consultation triggers:** invalidated assumptions, changed interfaces or invariants, ambiguous requirements,
  recurring failures, suspected races/security/data-loss defects, and proposals to weaken acceptance checks. Specify
  which routine decisions are autonomous; new user-scope decisions remain blocked, even during unattended execution.
- **State and evidence:** absolute blueprint/log/repository paths, baseline, recording requirements from `records.md`,
  and instructions to retain session IDs, checkpoints, review findings, budgets, and outstanding work across resumes.
  Use the existing log for compact recovery state, including failed repair attempts and pending evidence; do not add
  another ledger. Related consultant and repair exchanges normally resume the same session. New independent review gates
  still start fresh.
- **Resource watchdog:** require the independent monitor in `watchdog.md` before implementation or delegation. Include
  its watched paths, sampling interval, thresholds, alert delivery, restart/resume protocol, and low-resource actions.
  Default to a background monitor process when available; a model watchdog must use an approved worker pair and reserve
  a concurrency slot without preventing ordinary work. This is mandatory for unattended execution, not optional advice.

The task-specific blueprint is authoritative for approved design and permissions; shared skill files own launch
mechanics and execution protocol. Do not copy CLI command templates or whole skills into it. Do not leave placeholders,
references to "our earlier discussion", or required inputs stranded in disposable scratch. Read the completed document
as a cold executor and check that every milestone traces to the requested outcome.

Finish with the blueprint link, the agreed session model/effort, and `/goal <absolute-blueprint-path>`. Say explicitly
that implementation has not started. Planning-session usage may be unavailable; record any exposed planning session ID
and timing in the blueprint without guessing counts or searching unrelated history.

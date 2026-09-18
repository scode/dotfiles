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

Identify the test substrate workers need: compatible tool/service versions, fixtures, credentials, build artifacts, and
isolated mutable resources. Put cheap readiness checks before dependent implementation and test handoffs, rather than
discovering at integration that nobody could run the tests. Assign focused validation and repair with each worker's
changes. Name the integration checks the executor still owns; worker evidence does not replace them.

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

### Resolve the implementation design

The blueprint transfers design work from this planning session to a less capable executor. Spend planning effort on the
decisions that executor would otherwise have to discover through implementation and repair. A file inventory,
architectural overview, or instruction to follow an existing feature is useful orientation, but does not satisfy the
design requirement. Length is not the target; resolved implementation decisions are.

For each non-trivial implementation unit, provide the following where relevant to its behavior:

- **Code structure and contracts:** name the modules, existing symbols to change, proposed helpers/types, and callers
  that connect them. Give proposed signatures or equivalent input/output schemas, ownership and lifetime constraints,
  and error results. Distinguish verified existing APIs from proposed ones. Explain which behavior can be shared with a
  precedent and which differences require separate handling.
- **Algorithm and state:** specify the steps whose order or conditions affect correctness, using pseudocode, decision
  tables, or state transitions when prose leaves room for different implementations. Cover relevant validation, side
  effects, persistence, failure paths, and resource bounds. For asynchronous work, identify captured state, revalidation
  points, ordering, cancellation, and stale-result handling. Choose the mechanism that enforces an invariant; stating
  the invariant alone leaves design work to the executor.
- **Integration and compatibility:** trace the behavior across affected layers and public or persisted boundaries.
  Specify mappings, migrations, version changes, and behavior with old state. For a PR stack, show the contract and
  compatibility at each intermediate revision, not only at the final head.
- **Worked cases:** give concrete inputs or event sequences and exact expected outputs or state changes for the happy
  path and the relevant boundaries. Derive cases from the inspected implementation and requested contract. Avoid generic
  lists of hypothetical risks. For a parser, include ambiguous forms and consumed option values; for a state machine,
  include transitions that withdraw or replace previously valid state.
- **Test design:** map the important contracts to named test locations, fixtures/setup, stimuli, and observable
  assertions. Explain which plausible incorrect implementation each non-obvious test rejects. Where ordering matters,
  specify how the fixture forces the relevant interleaving; saying "test the race" is insufficient. Include focused
  commands and the integration coverage that remains necessary.
- **Decisions and evidence:** choose among material alternatives and explain why, citing the inspected source or
  verified behavior. List the remaining executor discretion explicitly. Routine discretion covers choices such as local
  variable names and equivalent idiomatic expression; parsing semantics, state ownership, failure policy, compatibility,
  and test oracles are design decisions even when they fit in a small helper.

Keep detail proportional to the unit. A mechanical mapping addition can use a compact table; a lifecycle or parser
change needs enough detail to implement and test its semantics without inventing them. Do not generate a full patch,
copy entire source files, or dictate syntax that has no bearing on the contract. Refer to existing code precisely and
describe the intended differences. The executor still reads the code and checks that the design fits the actual tree.

Resolve factual questions available from the repository or dependency source during planning. Do not schedule an initial
execution consultation to choose algorithms, enumerate supported forms, or settle compatibility that can be designed
now. An initial expert checkpoint may validate the completed design against current evidence. Preserve early
consultation for drift, unexpected behavior, and genuinely unresolved facts; it is not a substitute for planning. For
facts that cannot be verified here, state the missing evidence, the exact probe and expected outcomes, the design branch
each outcome selects, and which dependent work must wait. If the branches cannot yet be designed, label that unit as
awaiting design rather than presenting it as implementation-ready. Never invent source facts to fill a gap.

### Assemble the handoff

Write instructions addressed to a fresh executor. Include:

- **Execution entrypoint:** "This is execution, not blueprint creation. Invoke `agent-resumeable` with the absolute
  working-log path, then load `scode-build-blueprint` and follow its execution phase. Do not activate Galaxy Brain."
  Explain that a missing log means start; an existing log means reconcile and resume, never start over blindly.
- **Intent and acceptance:** the user's requirements and settled clarifications, non-goals, and evidence required for
  each outcome. Include the original request where useful so review can catch a narrowed interpretation.
- **Design:** the implementation design above, including concrete contracts, algorithms, worked cases, test designs,
  decisions and evidence, and explicitly bounded remaining assumptions and discretion.
- **Milestones:** dependency order, bounded deliverables, checks, expert checkpoints, and what evidence each checkpoint
  receives. Place checkpoints before expensive dependent work. Each implementation milestone must point to its resolved
  design and test cases; separate prerequisite evidence or design work from implementation-ready units.
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
references to "our earlier discussion", or required inputs stranded in disposable scratch.

### Check design completeness before delivery

Read the completed document as a fresh workhorse executor. For every non-trivial unit, walk a normal case and its
important failure or boundary cases through the proposed interfaces and algorithm to the observable test assertions.
List the decisions you still had to invent to do that walk. Resolve those decisions in the blueprint before delivery; a
planned expert consultation does not make a missing design complete. Check every milestone against the requested outcome
and its intermediate compatibility obligations.

Record a short handoff assessment in the blueprint: which units are implementation-ready, any evidence-dependent units
and their gates, and the routine choices intentionally left to the executor. Do not claim an entire blueprint is ready
when a material unit still awaits design. This is a planner self-check, not a mandatory extra model call or model eval.

Finish with the blueprint link, the agreed session model/effort, and `/goal <absolute-blueprint-path>`. Say explicitly
that implementation has not started. Planning-session usage may be unavailable; record any exposed planning session ID
and timing in the blueprint without guessing counts or searching unrelated history.

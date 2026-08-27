# scode-galaxy-brain design rationale

NOTE: This is a historical record of the reasoning behind the skill as it stood on 2026-08-28, moved out of the README
when the README was cut down to a short description and a permissions warning. It is expected to drift from SKILL.md.

TLDR: Invoke this from a session running a frontier model with `Use scode-galaxy-brain to <goal>`. The current session
keeps planning, judgment, quality control, commits, and PRs. It delegates suitable work to less expensive models, checks
their actual output, and escalates when a task was too demanding for its first route.

This skill optimizes total cost and waiting time through an accepted result. It does not blindly choose the model with
the cheapest nominal tokens.

## Why routing uses work profiles

Model cost is conditional on the task. A cheap model can be fast and effective for deterministic tool use, yet expensive
for ambiguous implementation work if it consumes more tokens, produces a defective result, requires substantial review,
and then has to be replaced by a stronger model. Public benchmark scores and provider prices do not capture that whole
path.

The previous version assigned every model universal cost, intelligence, and taste scores. That looked precise but hid
the interaction between model and task. It also encouraged routing to a nominally cheap model even when the likely retry
and escalation cost made that choice worse.

The skill now classifies work into profiles:

- Mechanical work with deterministic validation.
- Routine authored work where baseline taste matters.
- Clear-spec implementation with strong acceptance checks.
- Complex implementation with ambiguity or difficult debugging.
- Design and synthesis work involving APIs, architecture, or nuanced copy.
- Mechanical review for style, idiomaticity, documentation drift, and slop.
- Critical review for correctness, security, concurrency, data integrity, and test quality.
- Orchestration, which remains with the frontier session.

Each family has a primary and escalation model for each profile where it has a suitable model at all; a profile with no
route in the preferred family falls back to the orchestrator's own family, announced as a divergence. The first model
gets one well-specified attempt. Small defects are cheaper to fix in the orchestrator; substantive failure moves to the
escalation model instead of repeatedly spending tokens on the same underpowered route. Producing prose belongs to
routine authored work; reviewing prose is mechanical review unless correctness, security, or another critical dimension
raises the stakes. A route already at its family's trusted endpoint has no automatic retry: the orchestrator handles a
substantive failure or deliberately crosses families.

This is intentionally a policy table rather than a capability leaderboard. The assignments should change when repeated
real use shows that a model succeeds, fails, or incurs different latency for a particular class of work. Profiles may
share the same route while retaining distinct semantics: their verification strength and failure cost differ, and later
calibration may move one without moving the others.

Reviews intentionally route above similarly sized implementation work. A producing agent has the orchestrator and tests
behind it; a reviewer is itself the gate, so a missed finding may have no later backstop.

Haiku is available only as the mechanical Claude workhorse. Deterministic searches, tool use, and churn with cheap
validation provide a safe place to use its speed without trusting it with authored work or a review gate. Sonnet appears
at low, medium, and high effort so the Claude family can scale routine prose and clear-spec implementation without
jumping directly from Haiku to Opus.

## Native delegation matters

Crossing from Codex to the Claude CLI, from Claude Code to the Codex CLI, or from either to the Muse CLI, has a real
cost beyond model tokens:

- A new process and model session must start.
- The orchestrator must serialize context into a standalone prompt.
- Authentication, permission, sandbox, quoting, timeout, and output-capture behavior differ.
- The orchestrator receives less useful progress while it waits.
- More integration points can fail before the delegated task even begins.

When two models are roughly equally suitable, the skill therefore prefers the orchestrator's family and native subagent
path. The bias is strongest for tiny and short work because startup overhead can exceed the task itself. It weakens for
large tasks, where a model's chance of finishing correctly dominates a small fixed startup cost.

The native-path bias is not a quality waiver. The skill crosses families when the native family lacks a suitable model,
the other family has a meaningful advantage, a native attempt failed, the user requested a provider, or an independent
cross-family review is part of the goal. Critical review prioritizes reliability and useful independence over startup
latency.

For tiny tasks, the best route is often no delegation. Writing a complete task prompt and reviewing the result can cost
more than doing the work directly.

For medium and large tasks, a materially lower expected total cost may justify crossing families even when both models
are capable. That comparison includes likely tokens, latency, review effort, and escalation risk — not only provider
prices. A nominally cheap model does not win when it is likely to fail slowly.

## What stays with the orchestrator

The frontier session remains responsible for:

- Decomposing the goal and selecting work profiles.
- Writing self-contained delegation prompts and acceptance criteria.
- Inspecting actual diffs and rerunning checks.
- Fixing small defects and deciding when to escalate.
- Reconciling conflicting findings and making design decisions.
- All commits, bookmarks, branches, pushes, and pull requests.

Delegates never perform version-control management. This preserves the user's VCS workflow and prevents a delegate with
partial context from publishing changes.

When a delegate's write is structurally wrong, the orchestrator does not hand the broken patch to a stronger model and
hope for incremental repair. It preserves pre-existing user work, removes only the failed delegate's edits, and asks the
escalation model for a fresh implementation informed by concrete acceptance failures.

## Permissions and concurrency

The skill needs the `claude`, `codex`, and `muse` CLIs installed and authenticated for whichever families it crosses
into. Foreign-harness delegation bypasses permission checks with `codex --yolo`, `muse exec --yolo`, or
`claude --dangerously-skip-permissions`. Use it only where you would accept the same permissions for the orchestrating
session.

Meta's Muse Code is never chosen on its own. It becomes a route only through `prefer-muse`, an explicit request, or a
deliberate cross-family decision announced as such. Its profile placements are provisional, based on vendor-reported
benchmarks, and it has no SOTA mark, so it never acts as the sole critical-review gate. A missing `muse` executable
makes the family unavailable rather than a launch failure to retry.

Independent read-only tasks may run concurrently. Writers that share the working tree run one at a time; writers may run
concurrently only when each one is isolated (a separate worktree or clone), with the orchestrator integrating and
re-validating the results serially. By default the skill parallelizes when it helps but does not hunt for opportunities;
ask for concurrency in the invocation ("with concurrency" or similar) to make parallelism an active goal, subject to the
same isolation and integration rules.

## Provider preference and local configuration

Add `prefer-gpt`, `prefer-claude`, or `prefer-muse` to the invocation when subscription capacity or another
non-performance constraint should steer delegation. An explicit preference overrides the normal native-family bias
unless the preferred family has no suitable route or repeatedly fails the task.

Use `~/.scode-galaxy-brainrc.md` for environment-specific availability and routing changes. It may:

- Exclude unavailable models.
- Replace the model inventory.
- Override primary or escalation assignments for work profiles.
- Add natural-language routing constraints.

The file is authoritative for the session. A replacement inventory omits models that are unavailable. Older rc files
with `cost`, `intelligence`, and `taste` columns still work as availability inventories, but their numeric scores are
ignored; migrate custom routing judgments into profile overrides. Models that do not appear in the built-in profiles
need explicit role assignments, and a new provider family also needs an invocation mechanism.

Ask the agent to populate the rc file with the default inventory and profiles if you want a starting point. It will not
overwrite an existing inventory or profile table.

## Feedback

Say `galaxy brain feedback: <what went wrong or should improve>` to record a self-contained report in
`~/.local/state/scode-galaxy-brain/feedback.md` (or the equivalent under `$XDG_STATE_HOME`). The agent announces the
exact path. Review entries before forwarding them; the agent avoids obvious private information but does not scrub away
technical details needed to diagnose the problem.

Credit: inspired by [t3dotgg](https://github.com/t3dotgg). The original version began with a model-rating table derived
from that setup; the current work-profile system reflects subsequent use and calibration.

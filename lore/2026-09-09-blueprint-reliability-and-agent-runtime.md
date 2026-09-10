# Blueprint reliability and a deferred agent runtime

NOTE: Historical design discussion recorded on 2026-09-09 after the first substantial Farhelm blueprint execution.
The runtime and command names below are proposals, not installed functionality. The immediate work is instruction-only;
the deterministic helper is deferred to a separate effort.

## What prompted this

The experiment separates planning by a state-of-the-art model from implementation by a workhorse, with expert
consultation and independent reviews. The first substantial trial used the Farhelm planner session `l-fh-planner`
and executor session `l-fh-exec`. The workhorse completed meaningful implementation and test migration, and reviews
caught real defects. This was not a controlled comparison against a single-model baseline, so it does not establish
cost savings.

Several problems were visible in the execution. The executor stopped because it described itself as GPT-6 while the
blueprint named Terra, even though the recorded execution metadata identified Terra. It repeatedly opened fresh model
sessions instead of continuing related investigations. A failed attempt to address a review finding was undone, yet
the finding was treated as addressed until a later review caught it again. Workers handed off test edits without enough
runtime validation, leaving the executor with much of the repair work. Recording degraded during the long run, and the
watchdog was stopped before all later work finished.

Frequent model turns to poll long-running browser tests also consumed substantial cached input. That is a reason to
investigate completion-driven waiting, not evidence that a new command wrapper can provide it. The user's billing
window began near executor startup and ended the following morning, after completion; it was not the task's elapsed
runtime. The approximate cost table from that window is not a controlled task-only benchmark.

## Immediate behavioral changes

The operator chooses the top-level model and effort. The executor must not try to authenticate its own model identity
or pause because its self-description disagrees with the blueprint. This does not grant workers an arbitrary model:
delegated calls still use explicit approved pairs from the blueprint.

Execution assumes the user is absent. Routine uncertainty calls for an autonomous decision or an approved expert,
not a confirmation prompt. Before declaring a critical blocker, consult the approved expert for an in-scope path
forward. Immediate safety pauses and inability to reach the required expert are exceptions. Expert advice never
grants new authority, waives acceptance criteria, or permits an unapproved model.

Keep the same consultant for related debugging exchanges, the same worker for repairs within its assignment, and the
same independent reviewer for follow-up on its findings when feasible. A new independent review starts fresh; its
repair follow-ups need not discard that reviewer's context. Record replacements and their reason.

Use the existing execution log to retain open findings, attempted repairs, evidence, pending gates, session handles,
and the next action. Failed repair attempts do not close findings. Reconcile the log after compaction or restart,
and do not claim completion while required findings or gates remain unresolved. This is an instruction-level
improvement, not a claim of mechanically enforced state transitions.

Give workers responsibility for focused validation and repair within their scope. Prepare the relevant test substrate
early, before handing off tests that cannot be run. Preserve stable-tree evidence and writer isolation; parallelism
does not justify testing a moving tree. Use focused checks while repairing, then the required integrated validation.
Seek debugging expertise before repeated unsuccessful attempts become the executor's main activity.

## Proposed runtime shape

The user chose Rust for an initial implementation, with Clap command-line parsing. There should be one
`scode-agent-runtime` binary, not a growing collection of skill-specific executables. Generally useful functions get
top-level commands. Skill policy belongs under `skills <skill-name>`.

Linux is the initial supported platform. Isolate resource measurement and other platform-dependent behavior so adding
macOS does not require changing the public command contract. Do not build macOS support in the first effort merely
to prove the abstraction.

The proposed packaging is a small package in the existing Cargo workspace. The dotfiles installer builds and installs
an actual executable at a stable location, provisionally `~/.local/bin/scode-agent-runtime`. Skills call that executable
directly, never `cargo xtask`, `cargo run`, or a checkout-relative build artifact. Cleaning `target/` must not break
installed skills. No separate manual `cargo install` step is intended.

There is also an offline constraint: the user can clone and build on a macOS laptop, but has a locked-down Linux SSH
host that cannot fetch build dependencies. Installation must accept a locally supplied prebuilt Linux binary without
trying to download or compile. Building the installer itself on that host is a separate bootstrap concern; a complete
offline delivery procedure must account for that too. A macOS host build is not a Linux executable. Cross-compilation
needs the target toolchain and compatible libraries, not just an architecture label.

CI-produced release binaries could solve distribution later. Local source builds, transferred prebuilt binaries, and
future downloaded releases should all lead to the same installed command. One executable must not imply undisclosed
helper scripts or runtime assets; OS/library compatibility still needs testing. Release publishing, supported Linux
library baseline, artifact verification, upgrades, and offline installer bootstrap remain implementation decisions,
not solved details of this discussion.

## The generic job-wrapper idea was withdrawn

An earlier proposal had `jobs start`, `jobs status`, `jobs wait`, and `jobs stop`, with blueprint calls built on them.
The user asked why wrapping a command would make the agent more reliable at waiting for it. It would not. If the harness
already delivers background completion notifications, use those. If it requires polling, an extra `wait` command does
not change that. Writing an event file does not wake an agent either.

The revised proposal has no generic jobs API. Ordinary tests and builds remain ordinary commands. Detached-process
supervision should be added only for a concrete, tested requirement. Resource monitoring, durable recording, and
process ownership can improve correctness independently, but they do not prove a reduction in polling turns or cost.

## Proposed commands, not an API commitment

The generic monitor runs in the foreground under the harness's background mechanism:

```sh
scode-agent-runtime watch resources \
  --path /path/to/repo \
  --path /tmp \
  --disk-free-percent-below 10 \
  --disk-free-bytes-below 5GiB \
  --memory-available-percent-below 10 \
  --interval 60s \
  --status-file /path/to/status.json \
  --events-file /path/to/events.jsonl
```

It records heartbeat, threshold crossings, deterioration, recovery, and sampling errors. It reports rather than
autonomously deleting files or killing workloads. The harness integration must establish how alerts reach the agent;
file output alone is not notification delivery.

Blueprint model calls couple recording to execution:

```sh
scode-agent-runtime skills scode-build-blueprint calls run --request /path/to/call.json
scode-agent-runtime skills scode-build-blueprint calls resume \
  --call /path/to/call-record --prompt-file /path/to/followup.md
```

A typed request identifies execution, role, explicit model/effort, prompt, and working directory. The helper retains
the exact request before launching, runs the harness subprocess, captures its outcome and available usage, and exits.
Follow-ups retain the original session and call lineage. The agent still decides whether to delegate or consult.
The shellout skill owns supported invocation mechanics; deterministic adapters must not become a competing routing
policy or drift independently from that contract. Secrets must not be swept into records indiscriminately.

The proposed blueprint state operations are:

```sh
scode-agent-runtime skills scode-build-blueprint init --blueprint /path/to/blueprint.md
scode-agent-runtime skills scode-build-blueprint status --execution <id>
scode-agent-runtime skills scode-build-blueprint findings add --execution <id> --request /path/to/finding.json
scode-agent-runtime skills scode-build-blueprint findings resolve \
  --execution <id> --finding <id> --evidence-file /path/to/evidence.md
scode-agent-runtime skills scode-build-blueprint checkpoint --execution <id> --request /path/to/checkpoint.json
scode-agent-runtime skills scode-build-blueprint finish --execution <id>
```

Status exposes active work, watchdog health, unresolved findings, pending gates, and next actions. Finish checks
mechanical prerequisites, not whether a reviewer's evidence is convincing. An unmet prerequisite returns actionable
state; it must not become an instruction to stop and prompt the user. These operations and automatic usage accounting
are deferred with the runtime. Existing records remain the mechanism in the instruction-only work.

## Sequencing decision

The original four-PR proposal grouped unattended behavior, review/completion state, deterministic supervision, and
automatic delegation records plus worker handoffs. The user then asked to get the non-helper improvements done first
and separately. The revised goal has three PRs: unattended behavior; review and consultation continuity; and verified
worker handoffs. Each includes its specification, README, and optional eval expectation changes as needed.

Each implementation PR gets a fresh-context Astra high general review, with findings addressed before proceeding.
Focused regression evals and one small end-to-end eval should inspect actual behavior, not merely an agent's claim
that it followed the skill. Do not replay the expensive Farhelm browser suite. The reusable watchdog, automatic call
records, mechanical completion checks, binary packaging, and CI distribution remain a later effort.

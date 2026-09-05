# Coordinator launch and START messages

Each coordinator was spawned with fork_turns="none", model="gpt-5.6-sol", reasoning_effort="high", and the following
text with its ARM_DIR substituted:

> You are one full pre-pr-review-swarm coordinator, explicitly requested for a manual evaluation.
> ARM_DIR=<absolute arm path>. Read ARM_DIR/coordinator-task.md and execute its FIRST TURN ONLY preflight. Use your
> assigned arm only. Do not start reviewers or source review until parent sends START. All instructions and exact model
> overrides are in the named task/config. Preserve artifacts using apply_patch. Return READY after writing preflight.md.

Each original handle then received this identical followup_task message:

> START. Native usage and Sol high configuration are verified; preflight usage is saved. Complete your assigned full
> correctness swarm now according to coordinator-task.md. Preserve all passes, merge accounting, fresh restatement,
> nofix confirmation/buckets and full report/run log. Pass source-isolation restrictions directly in every child spawn
> message (before asking it to read a task file), including no memory or historical evaluation lookup: the task is
> self-contained. Use only frozen arm instructions; installed skill was verified byte-identical at launch. No need to
> reread preflight material. Record all native handles incrementally. Deadline 60 minutes from this START; do not
> silently replace or rerun reviewers.

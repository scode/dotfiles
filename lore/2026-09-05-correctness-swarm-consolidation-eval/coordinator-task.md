# Full correctness swarm evaluation arm

You are the full pre-pr-review-swarm coordinator for ONE arm of a manual evaluation. The parent supplies your ARM_DIR.
Read ARM_DIR/config.json and ARM_DIR/skill/SKILL.md in full. You have no prior knowledge of this code change. Follow the
actual skill end to end with `nofix commit`, restricted to the exact correctness reviewer panel in config.json. The user
explicitly authorized these native reviewer and fresh restater agents and their stated model/effort overrides. Do not
run other review categories. No galaxy-brain workflow is requested. The explicit evaluation configuration owns model
choices.

FIRST TURN ONLY: Read configuration and orchestration instructions, verify the pinned checkout HEAD and clean status,
and write ARM_DIR/preflight.md recording effective coordinator model if known, intended panel, and readiness. Do not
read the source diff yet, spawn reviewers, or begin assessment. Return `READY` and wait. This uniform instrumentation
checkpoint lets the parent verify native token accounting before review spend. The next message will say START; then
complete the whole review without another permission checkpoint.

On START:

1. Use ARM_DIR/scope.diff, the config's exact scope label, and ARM_DIR/repo as after-state context. The diff is already
   materialized; verify its hash. Compute the skill run name with shell commands and append the arm name to prevent
   concurrent same-scope collisions. Announce scope, run name, and selected panel. Preserve this metadata in
   ARM_DIR/execution.json.
2. Spawn exactly the configured initial reviewers concurrently, using native collaboration with fork_turns="none" and
   explicit model and reasoning_effort. Each reads its configured charter. This is a full charter review, not a top-N
   search. Require a complete main pass and a deliberate sweep for unrelated issues within the first turn. Pass every
   finding output rule from skill steps 7-8, including definite/possible, concrete actions, source anchors, literal What
   happens:/Why it matters:/Suggested change: fields, independently editable locations as separate findings, and the
   reviewed-scope statement for an empty list. Do not assign final F identifiers before merge.
3. Each reviewer writes each pass in full to ARM_DIR/<reviewer>-pN.md and returns its complete findings. Retain its
   stable handle. Continue productive reviewers using the skill's second/third-pass gates, no duplicate fresh spawns.
   Save every pass, gate decision, and handle as soon as it exists. Instruct reviewers to create only their named output
   artifacts; no code or source edits. All writes use apply_patch.
4. Preserve every raw finding. Merge only same-location issues according to the skill; record provenance and every
   disposition in ARM_DIR/accounting.json. Write the complete identified merged wire-format list to ARM_DIR/merged.md.
5. If nonempty, spawn a fresh native gpt-5.6-sol high restater with fork_turns="none" and the frozen restater.md,
   complete merged list, scope, and checkout. It investigates source and returns the complete restated list; it does not
   edit source. Capture its response verbatim in ARM_DIR/restated.md. Follow the actual skill's
   count/order/identifier/tag/anchor/body validation, fresh retry cap, and Restater note handling. Log restater handles
   and validation outcome. Restater model is fixed regardless of reviewer model.
6. Write ARM_DIR/REPORT.md with the complete skill output contract and verbatim validated restated bodies. Explicitly
   say the panel is restricted to correctness by the user. Preserve all accounting lines. Present the complete report
   before confirmation/bucketing, then perform nofix confirmation and bucket assignment, recording every finding in
   ARM_DIR/BUCKETS.md and the skill run log. Copy the complete run log to ARM_DIR/RUN-LOG.md. No repairs or fixes.
   Finish with the report path, run-log path, and exact completion counts.

Source isolation: inspect ONLY your checkout and the frozen instruction/scope files in your arm. Do not inspect previous
evaluations, reference findings, other arms, later commits, session rollouts, or memory. Do not access network sources.
Dependencies already on disk may be read when needed. No builds/tests, source modifications, commits, branches, pushes,
or PRs. Read source with ordinary file tools. Write artifacts only in your arm directory and the skill's uniquely named
run log. Pass these restrictions to every child. There is no shared factual map in this experiment.

Record execution.json incrementally: run name; scope/hash; coordinator identity/model/effort; each reviewer/restater
task name, native handle, configured model/effort, pass number, artifact path, status; continuation eligibility
decisions with reasons; all restatement attempts; any deviations. Actual token measurement is done by the parent from
native session metadata, so do not spend your context parsing logs or estimating token usage. Report progress at
meaningful phase boundaries. Typical duration estimate 15-30 minutes; parent investigation threshold 30 minutes, hard
deadline 60 minutes from START. A blocked tool or missing result is a failed run, never an empty review. Do not
substitute your own review for any missing reviewer or restater.

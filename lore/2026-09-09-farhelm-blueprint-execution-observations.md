# Observations from the Farhelm blueprint trial

NOTE: Historical assessment recorded on 2026-09-09 from the planner and executor sessions for the host-selector
change. This is a workflow assessment, not an independent certification of the final code or a controlled cost
comparison. The investigation was read-only; it did not rerun the project's test suite.

## Task and evidence

The task replaced the filter popup and multi-field sidebar controls with a permanent host selector while preserving
the relevant query, archive, and stale-response behavior. It involved substantial implementation and test migration,
not a toy smoke test: the inspected diff covered 21 files, with roughly 646 additions and 1,858 deletions.

The user identified `/home/scode/git/farhelm2`, planner session `l-fh-planner`, and executor session `l-fh-exec`.
The planner used Astra high; the executor's recorded metadata identified Terra medium. Planning took about seven
minutes. Execution ran about 03:25–07:56 UTC on September 9, or four and a half hours.

The inspected local sources were:

- Blueprint: `/home/scode/git/farhelm-host-filter-blueprint.md`.
- Execution log: `/home/scode/git/farhelm-host-filter-blueprint-log.md`.
- Planner transcript: `/home/scode/.codex/sessions/2026/09/08/rollout-2026-09-08T20-16-00-01a0842a-4f55-7901-b4ac-13accdf6d6b0.jsonl`.
- Executor transcript: `/home/scode/.codex/sessions/2026/09/08/rollout-2026-09-08T20-16-53-01a0842b-2025-7d31-9618-8c952768a379.jsonl`.
- Execution records: `/home/scode/.local/state/scode-build-blueprint/executions/57e8a06d-5048-46c0-84be-131aa9fd1610/`.
- First review: `/tmp/farhelm-host-filter-review.gvfI4D/result.txt`.
- Later reviews: `/tmp/farhelm-host-filter-final-review.txt` and `/tmp/farhelm-host-filter-rereview.txt`.
- Final broad browser report: `/home/scode/.local/state/farhelm-test-runs/27356ed8-f313-4a93-b6e3-d1aab483a9df/playwright.json`.

These are provenance paths, not durable dependencies. In particular, `/tmp` reports may disappear. This entry retains
the substantive observations rather than assuming those files will survive. Transcripts are not copied into lore.
The tested implementation included working-copy changes; the Git HEAD alone did not identify the full tested tree.

## What worked

The blueprint did useful design work. It clarified the control-replacement scope, host identity and tombstone
semantics, stale response handling, and preservation of API/archive defaults. Those were concrete requirements an
executor and reviewer could check, not merely a sequence of editing suggestions.

The early Astra consultation supplied useful design and test-migration guidance. Terra workers handled bounded
implementation and test assignments. Prompts specified file scope, sole-writer constraints, and no VCS operations;
the split between core changes and successive test migrations was broadly sensible. There was no obvious need for
more implementation agents merely to increase delegation.

Expert review found real problems that local green checks had not settled: skipped archived-menu regression coverage,
missing compact selector styling, a test that did not establish its failed-listing premise, and stale documentation.
The first reviewer also challenged the claimed skip classification. This is evidence that the independent review
stage mattered, not that the first implementation handoff was already sufficient.

The final retained broad browser report has 450 expected passes, two skips, zero unexpected failures, and zero flaky
results, over about 49 minutes. Earlier review evidence reported 300 Rust passes. The last expert re-review reported
no actionable defects, while explicitly limiting its own validation to static review and diff checks. Those are
different kinds of evidence and should not be collapsed into a claim that the reviewer ran the runtime suites.

## Self-model validation caused an unnecessary stop

The executor said it identified only as GPT-6 Codex while the blueprint approved Terra medium, and requested user
confirmation before implementation or delegation. Its transcript metadata identified Terra medium. The user happened
to be watching and told it to proceed.

This was a failure of the preflight contract, not evidence that a model switch was needed. A model's self-description
is not a useful authority check. The operator should choose the executor; explicit blueprint model pairs should
govern delegates without requiring the orchestrator to prove its own identity.

## Review findings did not reliably survive failed repairs

The first expert review found that the failure-recovery test could pass without rendering the failed-listing state.
After releasing a 503 response, already-visible controls were not sufficient evidence: changing the query could make
the response stale before the failure was exercised.

Around 05:43 UTC, the executor added an assertion for `.list-error`. That attempted repair failed. At about 05:45 it
removed the assertion, then treated the review findings as addressed and opened the PR. A later review around 07:50
identified the same missing premise. The executor then used the actual `.status.error` selector and obtained a further
review.

The important failure is not choosing a wrong selector once. It is losing the distinction between an attempted fix
and a verified resolution. The finding needed to stay open across the failed attempt, intervening tests, and any
completion summary. A compact durable list of findings and evidence would help; prose instructions alone will still
depend on agent compliance until a mechanical state layer exists.

The first review also reported two archived-menu skips and two real-agent skips, contrary to the executor's account
of four real-agent skips. The eventual report had only two skips. This reinforces the need to inspect retained
evidence rather than accept a summary of it.

## Continuity and early debugging consultation were underused

The observed calls included an Astra design consultation, Terra core implementation, Terra central test migration,
a new Terra session repairing gaps in that migration, another Terra test migration, and several Astra reviews.
There were no `codex resume` commands in the inspected executor calls. Related worker repairs and later reviewer
follow-ups started fresh rather than continuing the existing conversations.

Fresh context is appropriate for the first independent review. It is not automatically useful for every repair
follow-up. Retaining the original consultant, worker, or reviewer would preserve hypotheses, rejected approaches,
and unresolved findings without repeated reconstruction. Session replacement may still be necessary; the reason
should be visible.

There were no subsequent debugging expert consultations after the early design exchange, despite registry retry
trouble and a WebKit focus race during validation. This does not prove an expert would have solved them faster, but
it supports clearer early-consultation triggers. The concern here was too little useful expert interaction, not
excessive expert call counts.

## Worker verification arrived too late

Several worker outputs supplied edits and limited checks such as formatting or test listing, leaving acceptance gaps
and runtime repairs with the executor. A web/helm version mismatch was discovered late in preparing validation.
Workers cannot provide useful runtime evidence if the test substrate is not ready or their task excludes running it.

The better handoff is scoped changes plus focused checks, or an explicit partial result naming what could not be
validated. Related repairs should normally go back to that worker when useful. This does not remove the executor's
responsibility for integration or independent review.

Broad browser runs were expensive and repeated. That alone is not evidence they were unnecessary: many browser specs
changed, some runs failed, and later fixes invalidated earlier evidence. The plausible improvement is earlier focused
checks and substrate preparation, not skipping integrated coverage. More parallel testing is not automatically safe
when writers, browser fixtures, build outputs, or services share mutable state.

## Recording and monitoring degraded over time

The dedicated execution events contained only nine events, ending around 03:40 UTC near the first worker decision.
Later calls and outcomes were not fully represented there even though the executor continued for hours. Late review
launches omitted explicit effort and structured JSON output, and several attempts used invalid review CLI forms.
A Sol low commit-message cold reader was also launched outside the blueprint's explicit Terra worker allowlist.
Repository review requirements do not themselves authorize a new model pair.

The watchdog was explicitly stopped around 05:46 UTC, but subsequent work continued until about 07:56. Its final
status sample is 05:46:25 UTC. That is an uncovered later work period, not a healthy monitor inferred from the old
status file. A restart would protect future work but could not retrospectively erase that coverage gap.

There was also an early launch-survival failure: the first watchdog did not survive the tool boundary. Repeatedly
having an executor invent how to implement and detach a monitor is a separate reliability problem. A tested monitor
and lifecycle integration are candidates for the deferred runtime; the instruction-only changes must not claim to
solve that implementation problem.

## Token usage and waiting overhead

The final executor token event reported:

| Counter | Tokens |
| --- | ---: |
| Input, including cached input | 92,887,453 |
| Cached input, a subset of input | 91,011,072 |
| Uncached input, derived by subtraction | 1,876,381 |
| Output | 106,535 |
| Reasoning output, a subset of output | 22,278 |

Do not add cached input to input again, or reasoning output to output. These are the top-level executor's reported
counters, not a reconstructed all-model bill.

The inspected executor had 587 tool calls: 449 execution calls and 138 waits. Of the execution calls, 216 contained
`write_stdin`. Nested long waits often yielded to the model before the nested wait completed, requiring another outer
wait. Browser-test-dominated windows accounted for roughly 46.4 million cached input tokens, about half the executor's
cached input. Those windows also contained real analysis and repair; it would be wrong to label every token in them
as avoidable polling.

The user's approximate billing-window table was $41.32 total: Astra $12.60 (30.5%), Terra $28.42 (68.8%), and Sol
$0.31 (0.7%), with no Luna usage. Rounded rows do not sum exactly to the displayed total. Another agent produced those
estimates, using pricing assumptions not independently verified by this assessment. The user estimated that roughly
97% of the window belonged to this work. The window started near executor startup on September 8 and ended the next
morning, after completion; it must not be interpreted as nine hours of active execution.

The executor alone accounted for about 88% of the table's Terra cached-input count. That makes top-level waiting and
context churn worth investigating before assuming more implementation delegation is the main cost lever. There is
no baseline proving how much could be saved. Lower total cost could even increase the expert percentage, so expert
share is not a useful target on its own.

## What follows, and what does not

The immediate changes are unattended behavior without self-model validation, expert consultation before genuine
blocking, continuity across related calls, honest review closure, and worker handoffs with earlier focused evidence.
They can be improved through instructions and evals using the existing records.

Automatic recording, a deterministic watchdog, and mechanical completion checks are deferred. A proposed generic
job wrapper was withdrawn after the user challenged its wake-up claim: wrapping a command does not improve the
harness's ability to notify the agent when it exits. Any future reduction in polling must be demonstrated against
the actual harness, separately from the benefits of retaining records.

The trial supports keeping expert design and review while improving execution discipline. It does not establish
the optimal worker model, delegation percentage, or savings versus having Astra implement everything. Those require
comparable tasks and accepted-result quality, not a single session's cost split.

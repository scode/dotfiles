# Correctness swarm consolidation pilot

This is one case and one run per configuration. It tests the complete correctness review workflow, including conditional
continuations, merging, fresh-agent restatement, validation, and nofix confirmation and bucketing. It does not establish
equivalent recall, and the reference finding set is incomplete.

## Question and configurations

Does consolidating correctness exploration into one reviewer reduce end-to-end cost without losing useful findings?
Reduced duplicated exploration is a hypothesis, not an established bottleneck.

| Arm                 | Initial correctness panel                                                                |
| ------------------- | ---------------------------------------------------------------------------------------- |
| baseline            | Sol high general, state/lifecycle, and systems; Luna high data flow and edge inputs      |
| combined-sol-high   | One Sol high reviewer with the full correctness charter and all four focused obligations |
| combined-astra-low  | Same combined charter, Astra low                                                         |
| combined-astra-high | Same combined charter, Astra high                                                        |

Every arm uses a fresh Sol high coordinator and fresh Sol high restater. Native Codex agents use explicit model and
effort settings and no conversation-history fork. Coordinators run the actual pre-pr-review-swarm skill in
`nofix commit` mode, with the selected correctness panel as the only restriction. Other categories are excluded
explicitly. The combined charter includes the base charter once, each focused lens verbatim, and a general pass without
preassigned emphasis. Production skills are not edited.

The first pass includes the skill's deliberate second sweep. Any nonempty first pass receives a second turn on the same
reviewer handle. A third turn requires a significant, credible new second-pass finding; three turns is the cap. Empty
results require positive confirmation that the review happened. All findings and rejected/merged inputs remain recorded
with reviewer and pass provenance. Restatement uses the skill's fresh-agent and retry rules. The coordinator completes
the confirm-against-code and would-fix/would-surface/would-reject classifications without modifying source.

## Pinned inputs

The repository is `scode/saltybox`. The subject is `62d866d6a57a24ef6bb329b28a246b44b758ff7a`, tagged
`eval/pre-pr-review-swarm/v2-engine-initial`; its parent is `3b1d3131902f724dd3827444ad67c7f8871ee2df`. The tag was
verified after `git fetch --tags origin`. This is the user's original useful broad-finding case.

The skill source is
[scode/dotfiles commit 95dec985ba998a956dabc10e770bb25acb184856](https://github.com/scode/dotfiles/tree/95dec985ba998a956dabc10e770bb25acb184856/agent-skills/pre-pr-review-swarm).
Original skill and charter hashes, configurations, and the last skill-changing commit are recorded in
[metadata.json](metadata.json). Each arm receives a separate clean checkout at the subject, the same frozen skill files,
and an identical scope diff. Cargo.lock and generated testdata/golden-vectors-v2.json are omitted from the diff and
listed in its trailer; both remain available as context. The prior manual diff was checked byte-for-byte against the
pinned commits, then wrapped with the skill's required touched-file summary and omission trailer.

Reviewers, coordinators, and restaters may inspect source and dependencies. They may not inspect previous findings,
other arms, later commits, session logs, or evaluation assessments. No fixes, builds, test runs, repository mutations,
or network research are part of this pilot. This keeps validation workload consistent and source inspection read-only.
Artifact writes are allowed only within the assigned arm directory and the skill's unique run log. These constraints are
passed equally to all arms.

## Measurement

Native session event logs provide per-thread usage and model/effort metadata. Before authorizing reviewer fan-out, each
coordinator performs the same short setup turn so the parent can verify that fresh-thread usage is available. This setup
and the subsequent start/resume message count toward coordinator cost. The extra checkpoint is instrumentation, but that
turn also includes ordinary skill/configuration loading that a production review needs. Report full totals as the
primary end-to-end measurement and disclose setup separately; the setup-exclusive subtotal must not be labeled
uninstrumented production cost.

Use the final cumulative usage for each fresh thread, once. Do not sum repeated cumulative token snapshots. Verify
counter monotonicity; account for all continued turns and every descendant thread. Record input, cached input, uncached
input, cache-write input, output, and reasoning output. Reasoning output is a subset of output and is not added again.
Report review, coordinator, restater, and full-arm totals. Root setup and cross-arm judging are experimental overhead
outside each arm and must not be represented as production review cost.

Also record elapsed time, reviewer pass counts, tool-call counts, command counts, and identifiable source-read commands
and paths. Unique/repeated file reads are best-effort proxies: searches, scripts, partial reads, and tool truncation
prevent an exact decomposition of billed tokens into source exploration. Preserve the extracted command inventory and
limitations. Cached tokens are reported separately because raw repeated input is not equivalent to repeated full-price
input. Dollar estimates require verified prices for these exact model IDs; otherwise report token spend without
inventing a price mapping.

The native agents share a host and provider. Cross-run cache reuse and scheduling effects are not experimentally
controlled. Model checks verify effective model and effort from session records, not merely requested spawn arguments. A
failed or incomplete orchestration is recorded as such and not scored as a clean review. The required incremental
artifact bookkeeping also costs coordinator tokens. It is included in the measured arms and cannot be cleanly subtracted
without another experiment; excluding the preflight does not turn this into an uninstrumented production benchmark.

## Assessment and artifacts

After all four reports are sealed, verify their findings against the pinned source and compare accepted actionable
issues, materially consequential issues, false positives, and uncertainties. Preserve every raw finding, coordinator
merge/rejection, restatement, and final bucket. Compare both raw reviewer discovery and final accepted output so
orchestration losses are visible. Match findings by underlying defect and independently editable location, explaining
any splits or merges. The parent assessment is unblinded and this limitation must remain in the report.

Use the old case assessment only after reports are sealed, as a reference rather than a complete ground truth.
Separately identify issues found by the new union and reference issues missed by every arm. Report unique findings with
mechanisms and criticality, not only counts.

The initial pilot saved results locally. This historical archive was requested after completion and preserves
methodology, metadata, aggregate measurements, prompts, and all finding text. Native raw session logs and the full
command ledger remain private. No permanent agent-memory update is part of the archive.

## Status

All four workflows completed and the source assessment is finished. Launch messages are in
[launch-prompts.md](launch-prompts.md), deviations in [REPORT.md](REPORT.md), and completion/integrity evidence in
[METRICS.json](METRICS.json). The source assessment began only after all four reports, bucket files, run logs, and
accounting files were sealed. No further eval case or production skill change was performed.

## Reproduction procedure

Use a new artifact directory and fresh native threads. The archive itself must never be a reviewer's context: it
contains findings. The original harness was native Codex collaboration, CLI `0.153.3`. A later harness or model revision
is a replication with those differences recorded, not an identical run. The model IDs are not immutable weight
snapshots.

1. Fetch saltybox tags with `git fetch --tags origin`. Verify the tag resolves to the subject above and its first parent
   equals the recorded base. Make four independent clean checkouts at the subject, one per configuration. Keep the
   pinned Argon2 0.5.3 dependency source available for read-only inspection; its package checksum and inspected source
   hashes are in metadata. Do not expose later commits or prior evaluations through instructions or search.
2. Extract the eight nongenerated files listed under `frozen_files` in metadata from `agent-skills/pre-pr-review-swarm/`
   at the pinned dotfiles commit. Verify their SHA-256 hashes. Copy the archived [scope.diff](scope.diff) into every arm
   and verify its `scope_sha256`. It contains the touched-file list, the selected diff and the explicit omission
   trailer. The underlying diff is `git diff --no-ext-diff BASE SUBJECT -- SELECTED_PATHS`, with paths in
   `git diff --name-only BASE SUBJECT` order and the two omitted paths removed.
3. Reconstruct the combined charter identically: start with the introductory heading and paragraph in
   [combined-charter.md](combined-charter.md); append the frozen base charter from `## Rules` onward, once; append each
   focused file's section from `## Lens:` up to but excluding `## No hand-off`, in data-flow, state-lifecycle, systems,
   edge-inputs order; append the archived `## Complete responsibility` section. Join sections with a blank line and a
   final newline. The archived prose is formatted for the repository; use the pinned source bytes and recorded original
   hash if checking byte identity.
4. Write each configuration from `metadata.json` to its arm's `config.json`, substituting the new absolute arm and
   checkout paths for `EVAL_ROOT`. Place the frozen files under `skill/`, the generated charter at
   `skill/reviewers/correctness-combined.md`, and a copy of [coordinator-task.md](coordinator-task.md) in each arm. The
   baseline still receives the combined file, but its reviewers use only their configured charters.
5. Launch the four Sol high coordinators concurrently with `fork_turns="none"` and the exact portable launch message in
   [launch-prompts.md](launch-prompts.md). Record handles privately. Verify every READY result and the availability of
   fresh-thread cumulative counters, then send the recorded START message to each original handle. The configured
   initial fan-out is eight reviewers, four coordinators and the experiment parent. Do not add other swarm categories or
   a shared factual map.
6. Let each coordinator perform the full frozen workflow, including conditional continuations, restatement retries, and
   nofix confirmation. Preserve every pass and gate decision as it happens. Investigate at 30 minutes and impose the
   stated 60-minute deadline from START; mark missing or failed work explicitly instead of treating it as an empty
   review. No original arm reached either deadline. Do not replace or rerun an arm after inspecting findings.
7. Traverse each coordinator's descendant threads from native parent metadata. Verify actual model/effort, completion,
   counter monotonicity and clean checkouts. Sum the last cumulative usage of each fresh thread exactly once. Record the
   first setup turn separately, and measure elapsed time between the coordinator's second task start (START) and its
   final completion. Use event timestamps rather than timezone-dependent session filenames. Preserve compact per-thread
   counters and per-role sums, plus best-effort read-command proxies with their limitations.
8. Seal all arm reports, buckets, run logs and accounting files before consulting the earlier reference or performing
   cross-arm source judgment. Apply [ASSESSMENT-RULES.md](ASSESSMENT-RULES.md), retain raw versus final coverage, and
   explain unique findings and same-family overlaps. Record deviations and unresolved applicability. Apply an explicitly
   dated rate card as below.

## API price conversion

Prices were added on 2026-09-05 after the reviews and source assessment. The
[official API price card](https://developers.openai.com/api/docs/pricing) supplies the following USD rates per million
tokens. They are copied here so later pricing changes do not silently change this historical comparison.

| Model        | Standard uncached input | Standard cached input | Standard cache writes | Standard output | Fast multiplier |
| ------------ | ----------------------: | --------------------: | --------------------: | --------------: | --------------: |
| gpt-5.6-sol  |                   $4.00 |                 $0.40 |                 $5.00 |          $20.00 |              2× |
| gpt-5.6-luna |                   $0.20 |                 $0.02 |                 $0.25 |           $1.20 |              2× |
| gpt-6-astra  |                  $10.00 |                 $1.00 |                $12.50 |          $50.00 |              2× |

For this run, each thread's Standard estimate is
`(uncached_input × input_rate + cached_input × cached_rate + output × output_rate) / 1,000,000`. Sum threads using each
thread's actual model, including the Sol coordinators and every restater attempt. Reasoning is already in output; it is
not charged a second time. Cache-write usage is zero in all 16 threads, so no cache-write term is needed. A replication
with nonzero writes must account for those separately under its API usage semantics.

The maximum observed request input was 152,847 tokens. The
[Astra](https://developers.openai.com/api/docs/models/gpt-6-astra) and
[Sol](https://developers.openai.com/api/docs/models/gpt-5.6-sol) model documentation puts long-context pricing above
272K input tokens per request. The recorded maxima for every thread are in [API-COST.json](API-COST.json); all requests
use the short-context rates here. Cumulative thread input in the millions does not itself trigger long-context pricing.

These are API-equivalent token estimates, not observed charges. Native turn metadata did not record a billed service
tier, so the report shows both Standard and Fast estimates instead of claiming an actual tier. The conversion excludes
regional-processing uplift, taxes, contractual discounts, subscription credits, and external tool or infrastructure
charges. Review tools were local shell/file operations; no paid hosted tool usage was measured. The Sol price was
promotional, stated available at least through November 21, 2026. Both the date and rates matter when reproducing the
dollar comparison.

Pricing includes the coordinator preflight and the entire arm workflow; parent experiment setup, judging, discussion and
archive work are excluded. The four Standard estimates sum to $22.19638564, or $44.39277128 at Fast rates. Neither sum
is a bill for the whole experiment. Do not apply one model's rate to an entire mixed-model arm, or compare raw
total-token reductions as if they were dollar reductions.

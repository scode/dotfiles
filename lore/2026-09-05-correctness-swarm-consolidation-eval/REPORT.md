# Correctness swarm consolidation: saltybox 62d866d

Single Astra high is a promising candidate for another case in this pilot. It used 66% fewer total model tokens and cost
an estimated 46% less at Standard API prices than the current correctness panel, while retaining its meaningful
secret-cleanup finding. Single Sol high saved 53% of tokens and 46% of dollars but missed that finding. Astra low
completed a real review with zero findings and missed it too.

This is one case, one run per arm, and an unblinded source assessment. It supports another case before any routing
decision. The two moderate findings are closely related secret-cleanup mechanisms, not independent evidence of broad
security recall.

## Inputs and execution

- Repository: scode/saltybox.
- Subject: `62d866d6a57a24ef6bb329b28a246b44b758ff7a`, tag `eval/pre-pr-review-swarm/v2-engine-initial`.
- Parent: `3b1d3131902f724dd3827444ad67c7f8871ee2df`.
- Skill snapshot and last skill change: scode/dotfiles `95dec985ba998a956dabc10e770bb25acb184856`.
- Scope: 9 touched files, 958 reviewed diff lines; Cargo.lock and generated golden vectors omitted from the diff but
  available as context.
- Harness: native Codex collaboration; local CLI version `0.153.3`. Explicit model/effort settings, fresh contexts, four
  isolated checkouts.
- All coordinators and restaters: gpt-5.6-sol high. Only the reviewer configuration varies.

The actual frozen pre-pr-review-swarm skill ran in `nofix commit` mode, restricted to correctness. Each combined
reviewer retained the full general charter and all four focused obligations. Runs included conditional second/third
passes, merging, fresh restatement and validation, and confirmation/bucketing. No builds, tests, source changes, or old
repository eval runner were used.

All 16 native threads completed: four coordinators, eight initial reviewers, and four restaters including one retry. The
eight reviewers performed 18 passes. Native metadata verified every model and effort. All checkouts, scope hashes, and
frozen instruction files remained unchanged. Every raw finding and every coordinator decision is preserved.

## End-to-end token usage and API cost

These primary totals include coordinator setup, reviewers, continuations, both restater attempts where applicable,
report validation, and nofix bucketing. Cached input is part of input; reasoning output is part of output. Neither is
counted twice. Tokens are summed from each thread's final cumulative usage exactly once.

| Configuration                           | Uncached input | Cached input | Output, including reasoning | Total tokens | Standard API USD | Fast API USD | API cost reduction | Review elapsed |
| --------------------------------------- | -------------: | -----------: | --------------------------: | -----------: | ---------------: | -----------: | -----------------: | -------------: |
| Current panel: 3 Sol high + 2 Luna high |        736,374 |   15,365,248 |                     139,431 |   16,241,053 |            $9.88 |       $19.75 |           baseline |       26.9 min |
| Combined Sol high                       |        292,854 |    7,309,952 |                      62,819 |    7,665,625 |            $5.35 |       $10.70 |              45.8% |       21.7 min |
| Combined Astra low                      |         90,427 |    1,150,080 |                      11,350 |    1,251,857 |            $1.60 |        $3.20 |              83.8% |        4.2 min |
| Combined Astra high                     |        242,992 |    5,255,552 |                      34,736 |    5,533,280 |            $5.37 |       $10.73 |              45.7% |       15.0 min |

Elapsed time starts at the coordinator's START turn and includes all review/postprocessing phases. The preflight was
paused pending a parent usage check, so that waiting interval is excluded from elapsed time; its tokens remain included.
Setup-exclusive subtotals are in METRICS.json, but ordinary skill/config loading happened during setup too, so
subtracting it is not an uninstrumented production-cost estimate.

Astra high reduces uncached input by 67.0% and output by 75.1%, so its token reduction is not solely cached-prefix
churn. Its more expensive reviewer tokens leave its dollar estimate almost identical to combined Sol high. Total-token
reductions are 52.8%, 92.3%, and 65.9% for combined Sol, Astra low, and Astra high respectively.

Dollar estimates apply the [official API price card](https://developers.openai.com/api/docs/pricing), checked on
2026-09-05, separately to every thread's actual model. Standard rates per million uncached input / cached input / output
tokens are Sol $4 / $0.40 / $20, Luna $0.20 / $0.02 / $1.20, and Astra $10 / $1 / $50. Fast rates are twice Standard for
these models. All coordinator and restater tokens use Sol rates. [API-COST.json](API-COST.json) preserves the exact
inputs, rates, and unrounded totals; [METHODOLOGY.md](METHODOLOGY.md#api-price-conversion) records the formula and
qualifications.

These are API-equivalent token estimates, not a Codex invoice. Native logs do not establish the billed service tier, so
both Standard and Fast are shown. No regional uplift or separate tool fee is included. All observed request inputs were
below the long-context threshold; cache-write usage was zero. Prices were added after the reviews and source assessment,
without rerunning reviewers.

| Configuration       | Reviewer tokens | Coordinator tokens | Restater tokens |
| ------------------- | --------------: | -----------------: | --------------: |
| Current panel       |       8,034,263 |          7,147,333 |       1,059,457 |
| Combined Sol high   |       2,905,731 |          4,298,547 |         461,347 |
| Combined Astra low  |         361,460 |            890,397 |               0 |
| Combined Astra high |       1,226,895 |          3,941,681 |         364,704 |

The baseline spent about half its total tokens outside the reviewers. Consolidation reduces aggregation and report work
as well as reviewer exploration. Incremental eval bookkeeping contributes to coordinator cost in every arm; this
instrumented run does not cleanly isolate that contribution.

Root experiment setup, measurement, source judging, archive preparation, and discussion are outside the arm totals. A
partial root-session snapshot remains local; no final whole-experiment bill was captured.

## Coverage and search depth

| Configuration       | Reviewer passes | Restater attempts | Raw → reported findings | Source-validated atomic issues | Moderate issues |
| ------------------- | --------------: | ----------------: | ----------------------: | -----------------------------: | --------------: |
| Current panel       |              12 |                 2 |                  12 → 5 |                              4 |               1 |
| Combined Sol high   |               2 |                 1 |                   3 → 3 |                              3 |               0 |
| Combined Astra low  |               1 |                 0 |                   0 → 0 |                              0 |               0 |
| Combined Astra high |               3 |                 1 |                   3 → 3 |                              2 |               2 |

See [FINDINGS.md](FINDINGS.md) for every mechanism, severity, source argument, origin/pass, overlap, and rejection.
"Validated" is the parent's assessment, not the raw reviewer tag or the coordinator's would-fix bucket.

The useful differences are:

- Baseline and Astra high found that Argon2's heap workspace retains key-equivalent material even though the returned
  key is wiped. Combined Sol high and Astra low missed it. This is meaningful security hygiene with a separate
  memory-disclosure prerequisite, not a demonstrated remote exploit.
- Astra high separately explained uncleared internal Argon2 temporaries. Baseline's workspace repair would also enable
  the feature that wipes these temporaries, and a baseline reviewer asked to check them. Astra high supplies a more
  precise mechanism; its extra finding should not be read as another independent security family or an incomplete
  baseline repair.
- Baseline and combined Sol found two tests that unnecessarily repeat the 256 MiB production KDF. Both sites are real
  low-severity test-cost issues; no timeout or OOM was measured. Baseline bundles them as one report while Sol lists
  two, so the comparison normalizes them to the same two sites.
- Baseline and combined Sol recovered the misleading resource-safety assurance already present in the old reference. The
  aggressive caps match the pinned design; lower limits remain a policy decision. Credit is for the low-severity
  assurance defect, not a proven new CLI denial of service.
- Baseline's proposed dispatch integration was wrong for this explicitly unwired commit and was rejected during nofix
  confirmation. Its typed-error-source finding is optional, despite a stronger coordinator tag. Astra high's 32-bit
  capacity-overflow argument is concrete but remains conditional on an unresolved support policy.

The observed union contains five accepted atomic issues, including two moderate mechanisms in one cleanup family. It is
incomplete ground truth. The previous case assessment had no accepted material correctness findings, but this run
discovered the cleanup issues absent from that reference. The original case's broader full-swarm usefulness also
included documentation targets excluded here.

## Exploration evidence

| Configuration       | Reviewer shell invocations | Scope-read command mentions | Explicit source-path mentions in read commands | Distinct source paths mentioned |
| ------------------- | -------------------------: | --------------------------: | ---------------------------------------------: | ------------------------------: |
| Current panel       |                        184 |                          16 |                                            157 |                              19 |
| Combined Sol high   |                         39 |                           3 |                                             48 |                              20 |
| Combined Astra low  |                          9 |                           2 |                                             20 |                              11 |
| Combined Astra high |                         18 |                           3 |                                             46 |                              20 |

Seventeen source paths appear in read commands from multiple baseline reviewers. The combined high configurations
mention about as many distinct source paths with much less repeated activity. This is consistent with reduced duplicated
exploration. It does not prove an exact fraction of saved tokens came from source reads: partial reads, searches,
dynamic commands, caches, and different reasoning budgets are confounders. Paths mentioned in commands are a proxy
rather than a complete file-access trace.

## Execution qualifications

The baseline's first restater attempt used malformed headers supplied by its coordinator. The coordinator corrected the
merged wire format and ran a fresh second restater; the retry is included in cost. This was a coordinator-input defect,
not evidence that the first restater misunderstood the findings.

Baseline also retained two independently editable test sites as one bundled finding. Both sites remain visible, and
cross-arm assessment splits them without changing the sealed report. This is an output-granularity defect in the
observed orchestration, not an extra Sol discovery. General p3 added only the optional typed-source item, while systems
p3 was empty; neither third pass added accepted material coverage. Astra high's p3 was empty too.

Tool responses were sometimes truncated. [READ-AUDIT.md](READ-AUDIT.md) records the detailed Astra low recovery check
prompted by concern about its zero result: the omitted new encryption/decryption section and engine test tail were
recovered by later source reads; some pre-existing file-operation tests remained truncated. It also records a
wrong-directory read followed by a correct retry. Truncation counts for the other reviewers are saved without pretending
that those counts measure unread files. Zero was a genuine completed review outcome, with 81.6 seconds of reviewer work,
rather than a blocked or missing review.

Three coordinator preflights searched generic memory metadata before reading the task's isolation rule. No prior case
findings or rollout summaries were read. Child prompts carried the isolation rule directly. The startup deviations and
launch/START messages are preserved; private native handles remain local. The arms shared a host and provider; cache
reuse, load, and stochastic variation were not controlled.

## Reproduction and saved artifacts

[METHODOLOGY.md](METHODOLOGY.md) records the contract and reproduction procedure. [launch-prompts.md](launch-prompts.md)
contains the coordinator launch and START messages; [coordinator-task.md](coordinator-task.md) contains their workflow
instructions. [combined-charter.md](combined-charter.md) retains the experimental combined reviewer prompt. The
remaining skill files are recoverable from the pinned skill commit and hashes in [metadata.json](metadata.json).
[ASSESSMENT-RULES.md](ASSESSMENT-RULES.md) records the rubric set before judgment.

[METRICS.json](METRICS.json) retains role totals, exploration proxies, execution and integrity audits, and merge
accounting. [assessment.json](assessment.json) retains the normalized issue matching. Metadata also includes dependency
source checksums and the original pre-assessment report seals. The seals identify original local files; path
sanitization and archive formatting mean they are not checksums of the published output compilations.

Every reviewer pass, merged input, restatement attempt, final report, bucket and run log is retained in
[baseline outputs](outputs-baseline.md), [combined Sol outputs](outputs-combined-sol-high.md),
[Astra low outputs](outputs-combined-astra-low.md), and [Astra high outputs](outputs-combined-astra-high.md). Original
finding wording is retained, including claims rejected by the parent. Local paths and native session identifiers are
replaced with portable labels. Source paths in these outputs refer to the pinned saltybox commit.

This archive contains the compact evidence needed to assess and repeat the experiment. Checkouts, native transcripts,
the full shell-command ledger, private session handles, and one-off local measurement helpers remain outside it.
Production skills are unchanged by this archive; no additional cases were run.

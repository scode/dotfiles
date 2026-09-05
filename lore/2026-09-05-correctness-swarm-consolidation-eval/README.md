# Correctness swarm consolidation: summary

On saltybox commit `62d866d`, combined Astra high and combined Sol high cost essentially the same, but Astra high
retained the meaningful secret-cleanup finding that combined Sol missed. Astra high cost about 46% less than the current
five-reviewer correctness panel. This makes it a candidate for another case.

This is one case, one run per configuration, and an unblinded source assessment. It does not establish comparable recall
or justify changing review routing.

| Correctness configuration               | Total tokens | Standard API cost | Fast API cost | Cost saving |     Time | Accepted issues   |
| --------------------------------------- | -----------: | ----------------: | ------------: | ----------: | -------: | ----------------- |
| Current panel: 3 Sol high + 2 Luna high |       16.24M |             $9.88 |        $19.75 |           — | 26.9 min | 1 moderate, 3 low |
| Combined Sol high                       |        7.67M |             $5.35 |        $10.70 |       45.8% | 21.7 min | 3 low             |
| Combined Astra low                      |        1.25M |             $1.60 |         $3.20 |       83.8% |  4.2 min | None              |
| Combined Astra high                     |        5.53M |             $5.37 |        $10.73 |       45.7% | 15.0 min | 2 moderate        |

Astra high's two moderate issues belong to one secret-cleanup family. Baseline identified the heap-workspace issue and
proposed a repair that also covers the internal-temporary issue Astra high described separately. These are meaningful
security-hygiene findings requiring a separate memory-disclosure path, not demonstrated remote exploits. Both combined
Sol high and Astra low missed the heap-workspace issue. Astra low's zero was a completed review outcome.

Costs include Sol high coordinators, reviewer continuations, restaters and retries, and final confirmation/bucketing.
Cached input is priced separately, and reasoning is included in output once. These are API-equivalent estimates using
the price card checked on 2026-09-05, not actual Codex charges. Experiment setup and judging are excluded. Elapsed time
excludes the preflight pause; preflight tokens remain included. See the
[pricing methodology](METHODOLOGY.md#api-price-conversion) for rates, sources, and assumptions.

The frozen review skill is from dotfiles commit `95dec985ba998a956dabc10e770bb25acb184856`. Each combined reviewer had
the same full correctness obligations as the panel, and each arm ran the complete swarm orchestration restricted to
correctness, including restatement and nofix bucketing.

- [Full report and observations](REPORT.md)
- [Methodology and reproduction instructions](METHODOLOGY.md)
- [Every finding, overlap, criticality assessment, and rejection](FINDINGS.md)
- [Exact per-model token and API-cost calculations](API-COST.json)
- [Original review outputs and evidence index](REPORT.md#reproduction-and-saved-artifacts)

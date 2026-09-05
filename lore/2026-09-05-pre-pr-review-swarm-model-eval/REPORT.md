# Manual review-model comparison

Completed on 2026-09-05: all 221 reviews across 13 hand-picked cases finished successfully, with no execution failures
or repeated substantive reviews. The results support Luna high as a serious alternative to Terra medium, especially for
docs/comments. Sol high recovered more of the observed correctness issues. This sample does not establish slop or
idiomaticity recall: neither lens produced an accepted finding.

The evaluated skill snapshot is dotfiles commit `5227a036fcec4a6ea6e5259a666f7e10680667a4`. The last commit that changed
`agent-skills/pre-pr-review-swarm` at that snapshot is `fae1c42e3348de51d6a65e7bd731669e24152c5f`. The saved charters
were verified byte-for-byte against that snapshot.

[METHODOLOGY.md](METHODOLOGY.md) records the procedure and original full routing table. [OUTPUTS.md](OUTPUTS.md)
accounts for all 221 reviews across all five evaluated lenses. [FINDINGS.md](FINDINGS.md) retains all 89 assessment
records, and [PRIOR.md](PRIOR.md) preserves the earlier one-off results. [metadata.json](metadata.json) pins the cases,
skill blobs, models, scope hashes, and integrity receipts. This compact archive omits raw execution artifacts.

## Results

The reviewers produced 86 original findings. Three documented splits produced 89 assessment records: 56 valid, 16
invalid, 12 optional, three uncertain, and two out of scope. The accepted union contains 22 distinct case issues, nine
classified material. Some of those repeat an underlying issue family across related commits.

| Model / effort | Reviews | Distinct valid case issues | Material subset | Valid records | Invalid records | Optional | Uncertain / out of scope |
| -------------- | ------: | -------------------------: | --------------: | ------------: | --------------: | -------: | -----------------------: |
| Luna high      |      65 |                         16 |               6 |            20 |               5 |        8 |                    0 / 1 |
| Terra medium   |      65 |                         11 |               3 |            11 |               4 |        1 |                    2 / 0 |
| Muse high      |      65 |                         11 |               3 |            12 |               5 |        3 |                    0 / 0 |
| Sol high       |      26 |                         10 |               7 |            13 |               2 |        0 |                    1 / 1 |

Sol only received the two correctness lenses, so the all-lens totals are not an equal-opportunity model ranking. The
following table compares identical case/lens opportunities. Each value is recovered distinct issues divided by the union
of accepted issues observed under that lens; there were 13 reviews per model per populated cell.

| Lens          |          Luna high |       Terra medium |          Muse high | Sol high |
| ------------- | -----------------: | -----------------: | -----------------: | -------: |
| Idiomaticity  | No accepted issues | No accepted issues | No accepted issues |  Not run |
| AI slop       | No accepted issues | No accepted issues | No accepted issues |  Not run |
| Docs/comments |            11 / 12 |             6 / 12 |             5 / 12 |  Not run |
| Data flow     |              3 / 8 |              2 / 8 |              2 / 8 |    5 / 8 |
| Edge inputs   |             6 / 13 |             3 / 13 |             5 / 13 |   8 / 13 |

Across both correctness lenses, after removing overlap, Luna recovered 8 of 13 observed issues, Terra 5, Muse 6, and
Sol 10. The material subsets recovered were respectively 5, 2, 3, and 7 of eight material correctness issues. One
further material case issue was found only by documentation reviewers: the broadened write-gate failure guarantee.

Sol adds three observed correctness case issues beyond Luna's two correctness reviewers: the decrypt post-rename failure
contract, the missing startup banner, and the catalog-dependent CI tests. Terra and Muse recover the latter two between
them. Across all three cheaper configurations together, Sol adds exactly one exclusive issue: the decrypt post-rename
contract. Across all lenses, each model has an exclusive contribution: Luna the bind-failure fallback and stale
default-write-engine comment; Terra the ready-paste delay and nonexistent Argon2 feature; Muse malformed-paste handling;
Sol the decrypt failure contract.

The lack of positive idiomaticity/slop findings is important. Terra returned empty in all 26 of those cells; Muse
produced one invalid idiomaticity suggestion and one optional suggestion; Luna produced three invalid and three optional
reports across the two lenses. That favors restraint in this sample, but says little about what any model would miss on
a change containing a real target issue.

## What was compared

Every one of the 13 cases marked `curation = "hand"` in the pinned case list receives independent Luna high, Terra
medium, and Muse high reviews for idiomaticity, AI slop, documentation/comments, data flow, and edge inputs. Sol high
reviews data flow and edge inputs only. That is 65 reviews for each of Luna, Terra, and Muse, plus 26 for Sol. The
repository's eval runner is not used. This archive retains methodology, metadata, output counts, and assessments; full
execution artifacts remain outside the repository.

Data flow and edge inputs are full correctness reviews with an additional focus pass. They are not confined to a narrow
checklist; their charters explicitly require reporting off-lens correctness findings. A cheaper model assigned either
role still has the full discovery obligation. Simplification was not in the five Terra-medium rows of the revised table,
so it appears only in the earlier one-offs.

## How to read the results

The case assessments match repeated reports to a canonical issue and classify each claim as valid, optional, uncertain,
invalid, or out of scope. Material findings concern consequential behavior, resource bounds, or contracts; minor
documentation defects remain useful but are counted separately. The finding register preserves assessment text; raw
reports are not included in this compact archive. A reported finding may have documented splits when it combines
independently actionable claims.

Distinct case-issue counts remove duplicate reports across lenses within a case. They do not establish independent bugs
across related commits. Per-lens coverage measures recovery of the accepted issue union observed by this experiment, not
exhaustive recall. An empty observed union supplies no recall evidence. Empty reviews are substantive completed reviews;
failed executions are recorded separately.

## What this changes about the proposed routing

I would not retain the blanket Terra-medium assignment. Luna high is a credible candidate, particularly for
documentation: it finds useful issues and is not uniformly weaker at discovery. It also produces more optional or
mistaken reports, so the strong coordinator remains important. The coordinator can reject a false positive; it cannot
recover a finding nobody reported.

For idiomaticity and slop, this expanded sample supplies no accepted positive findings. It measures some restraint, but
does not establish equivalent recall. Luna's earlier correct Argon2-feature slop finding did not repeat in this matrix;
Terra found the same inaccurate rationale under documentation instead. That variation weakens any recommendation resting
on the earlier single success. Simplification was not repeated here, and Sol low was not included in this matrix.

For correctness, keep strong coverage while experimenting with a cheaper lens. Sol catches the decrypt failure-contract
issue that all three cheaper configurations miss in that case, and recovers the CI regression that Luna and Terra miss.
It also misses the broadened write-gate contract that Luna and Terra docs find, and its catalog-banner discovery is
shared with Terra and Muse. These are complementary observations, not evidence that any model can safely replace every
other reviewer. The full swarm's remaining correctness lenses were not evaluated, so the actual loss from replacing one
lens inside that ensemble is unknown.

Muse is useful as another reviewer, but not a clear default winner. It finds the malformed-paste issue and catches the
CI dependency problem, while also recommending a harmful `spawn_blocking` substitution and misreading the path-filter
quantifier. Those findings need the same evidence-based gate as GPT output.

My provisional choice is Luna high for docs/comments. Luna high remains a reasonable lower-cost experiment for
idiomaticity and slop, with positive cases still needed before claiming quality equivalence. I would retain Sol high
correctness coverage rather than replace both evaluated correctness lenses with Terra medium on this evidence. If one of
those lenses is made cheaper, Luna is a credible candidate; this run does not establish the safest full-swarm mixture or
measured monetary savings. No installed routing or skill was changed.

## Limits

There is one sample per cell and five repositories, with seven related Saltybox changes. Assessment is not blinded to
model identity. Muse uses a different harness, so model and harness effects are not separated. Reviewers and assessors
inspect pinned source without builds, tests, or fixes; conditional findings retain their conditions. Earlier one-offs
are preserved but excluded from matrix counts. No reliable attributable token usage or billed costs were collected, so
this experiment compares review output rather than measured monetary efficiency.

Context control was imperfect. Several reviewers exceeded the idiomaticity/slop charter's neighboring-file limit. In the
catalog-only CI case, several GPT reviewers read third-party action source files the assessor downloaded into the case
directory while subjects were active. These were primary sources, not another review or an assessment, but their late
availability was asymmetric. Those results are retained with the case's exact per-cell disclosure; no successful subject
was rerun. Source checkouts and scoped diffs remained pinned.

Two sensitivity checks help bound the interpretation. Excluding the CI case with asymmetric context leaves combined
correctness recovery at Luna 8/12, Terra 5/12, Muse 5/12, and Sol 9/12; the documentation counts are unchanged. Grading
the unwired engine's resource policy itself as material, rather than crediting its false comment assurance as minor,
adds one material case issue to every model and creates no exclusive Sol contribution. The post-rename failure contract
is repeated in decrypt and write-gate cases, so those are separate review opportunities for one underlying issue family.

## Case assessments

Every row accounts for 17 completed reviews and zero failed reviews. Counts remove repeated reports within a case. The
finding register retains evidence and disputed interpretations; metadata preserves case-specific protocol limitations.

| Case                                                                            | Valid case issues | Material subset |
| ------------------------------------------------------------------------------- | ----------------: | --------------: |
| [Treeward swapped FIFO](FINDINGS.md#treeward-swapped-fifo)                      |                 1 |               0 |
| [Ferricode remote auth](FINDINGS.md#ferricode-openai-codex-remote-auth)         |                 6 |               4 |
| [Dotfiles chores](FINDINGS.md#dotfiles-scode-chores-initial)                    |                 1 |               0 |
| [Saltybox spec skeleton](FINDINGS.md#saltybox-spec-skeleton-initial)            |                 0 |               0 |
| [Saltybox v1 move](FINDINGS.md#saltybox-move-v1-crypto-initial)                 |                 0 |               0 |
| [Saltybox dispatch](FINDINGS.md#saltybox-format-dispatch-initial)               |                 1 |               0 |
| [Saltybox v2 engine](FINDINGS.md#saltybox-v2-engine-initial)                    |                 3 |               0 |
| [Saltybox v2 decrypt](FINDINGS.md#saltybox-v2-decrypt-initial)                  |                 3 |               2 |
| [Saltybox write gate](FINDINGS.md#saltybox-v2-write-gate-initial)               |                 3 |               1 |
| [Saltybox format flip](FINDINGS.md#saltybox-v2-flip-initial)                    |                 1 |               0 |
| [Stark Parts static catalog](FINDINGS.md#stark-parts-pr56-catalog-static-asset) |                 2 |               1 |
| [Stark Parts catalog CI](FINDINGS.md#stark-parts-pr57-catalog-only-ci)          |                 1 |               1 |
| [Stark Parts build cache](FINDINGS.md#stark-parts-pr58-catalog-vercel-cache)    |                 0 |               0 |

The finding register retains the final judgments after coordinator assessment and independent parent checks.

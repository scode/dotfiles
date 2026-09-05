# Earlier one-off results

## Prior one-offs, preserved separately from this matrix

The subject was Saltybox `62d866d6a57a24ef6bb329b28a246b44b758ff7a`, parent `3b1d3131902f724dd3827444ad67c7f8871ee2df`,
tag `eval/pre-pr-review-swarm/v2-engine-initial`. Reviews used 958 diff lines from seven files; `Cargo.lock` and
generated `testdata/golden-vectors-v2.json` were omitted but readable. No tests or builds were run.

| Configuration | Slop                                                                             | Simplification                                                           |
| ------------- | -------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Luna high     | Verified nonexistent Argon2 parallel feature; additional resource-policy concern | Valid direct associated-data concatenation                               |
| Terra medium  | Rejected unwired-engine complaint                                                | Marginal optional inline of Python armor helper                          |
| Sol low       | No findings                                                                      | No findings                                                              |
| Muse high     | No findings; repeated the unsupported parallel-feature rationale                 | No findings; explicitly considered concatenation and judged it stylistic |

The concrete Luna slop finding was at `src/format_v2.rs:62-66`: the comment claims a rayon-based `parallel` feature,
while pinned Argon2 0.5.3 has none. The concatenation recommendation at lines 221–226 preserves bytes and removes manual
Vec assembly. The 4 GiB pre-authentication KDF allocation is real, but whether its cap is acceptable is a
resource/security policy question with historical-file compatibility consequences, not a definite slop finding. Terra's
claim that v2 must be wired into dispatch conflicts with the commit's explicit "unwired" intent. Its one-use
armor-helper inline preserves behavior but has debatable readability value.

Luna produced the strongest positive signal in that sample; this justified trying it for slop and simplification, not a
claim of generally superior recall. Muse recognized but rejected the harmless simplification, demonstrating why finding
counts alone are weak evidence. The GPT one-off used Sol low, whereas this expanded matrix uses Sol high only on the two
correctness lenses. Do not conflate those conditions or count the earlier runs as repeats of this matrix. Previous token
usage and billed costs were not collected. Muse also changed harness, a confound retained in this experiment.

## Assessment notes

The following contemporaneous assessments are preserved as result records. References to raw files describe the original
local experiment; those files are not part of this compact archive.

### assessment.md

### Saltybox: one round of slop and simplification review

Luna high produced the strongest positive signal in this round: one verified hallucinated dependency claim and one valid
local simplification, neither reported by Terra medium or Sol low. This supports trying Luna on these charters. Six
reviews on one commit do not establish general recall, reliability, or a cost advantage.

#### Method

Each of the three configurations ran two independent native agents, one per charter, concurrently. Each received a fresh
conversation context, the same shared task file and scope diff, and the same charter as its counterparts. All six
completed one turn; there were no reviewer continuations, retries, fixes, additional reviewers, or repository eval
scripts. Agents could inspect source but were told not to build or run tests. The parent checked findings against the
checkout and the locally installed pinned Argon2 source. This assessment was not blinded to model identity.

The subject is `62d866d6a57a24ef6bb329b28a246b44b758ff7a`, fetched and verified through tag
`eval/pre-pr-review-swarm/v2-engine-initial`, against parent `3b1d3131902f724dd3827444ad67c7f8871ee2df`. Some agents
summarized in their final messages despite being asked to return the full artifact there too; assessment used the saved
artifacts. The setup, prompts, scope diff, and raw outputs are omitted from this compact archive.

#### Results

| Configuration | AI slop                                                        | Simplification                                       |
| ------------- | -------------------------------------------------------------- | ---------------------------------------------------- |
| Luna high     | 2 reported: 1 verified slop finding, 1 resource-policy concern | 1 reported: valid local simplification               |
| Terra medium  | 1 reported: rejected as a defect                               | 1 reported: behavior-preserving but marginal cleanup |
| Sol low       | No findings                                                    | No findings                                          |

##### Luna high: nonexistent Argon2 parallel feature

The comment at `src/format_v2.rs:62-66` says the Argon2 crate offers a rayon-based `parallel` feature. `Cargo.lock` pins
Argon2 0.5.3, and that version's local registry `Cargo.toml` lists `alloc`, `default`, `rand`, `simple`, and `std`, with
no parallel/rayon feature or dependency. This is a concrete hallucinated dependency detail, within the slop charter.
Correcting the explanation is warranted. The recommendation to change the dependency is unnecessary; the unsupported
comment can be corrected on its own.

##### Luna high: direct associated-data concatenation

`src/format_v2.rs:221-226` reserves a vector and appends the magic bytes and header.
`[V2_MAGIC.as_bytes(), header].concat()` produces the same bytes in the same order with less code. This is a valid,
modest simplification that keeps the explanatory helper. No benchmark or exact allocation-performance claim was
measured.

##### Luna high: resource cap permits 4 GiB before authentication

The factual premise holds: validation accepts the maximum memory and time parameters, and `decrypt` calls
`hash_password_into` before authentication. Argon2 0.5.3's implementation allocates a vector of memory blocks at
`src/lib.rs:229-231`. The reviewer has identified real resource exposure, but its definite slop classification and
remedy are too strong. The cap explicitly leaves room for future defaults, and whether this budget is acceptable depends
on the operating environment and compatibility policy. Lowering accepted caps can reject historical files. Treat this as
a security/resource-policy question to discuss, not an established behavior-preserving cleanup or evidence of cargo-cult
code. Do not count it as a clean slop win or dismiss the factual observation as fabricated.

##### Terra medium: the engine is unwired

The dispatcher still selects only v1, but the commit subject is `chore: add saltybox2 format engine (unwired)`. The
omission is intentional staging, not evidence that the implementation was added without understanding. Connecting it
changes behavior outside the requested cleanup; deleting it defeats this commit's purpose. Reject the definite slop
finding. The shared prompt supplied the commit ID and checkout but did not paste the subject; reviewers could inspect
it. Future comparisons should supply the commit message explicitly so intent is equally visible without requiring a
history lookup.

##### Terra medium: inline the armor helper

`testdata/generate-golden-vectors-v2.py:38-39` has a one-expression helper with one caller at line 76. The proposed
inline expression preserves the encoding behavior and removes a few lines. It meets the charter's literal smaller-code
condition, but the helper gives the format operation a useful name and keeps the returned dictionary readable. Count
this as marginal optional cleanup, not a clear defect or a material improvement. This quality judgment is subjective;
the behavior-preserving part is checkable.

##### Sol low: no findings

Both agents returned explicit empty reviews with inspected-file lists. They did not report Luna's two accepted
observations or Terra's optional cleanup. This is an observed difference in one sample, not proof of generally lower
recall. No exhaustive ground truth was constructed, and a broad distribution from an earlier full swarm need not imply
many findings in these two charters.

#### Limits and resulting judgment

The native agents were launched with explicit model and effort overrides. Per-agent token usage and billed cost were not
collected, so this run supports a quality observation only. Sol and Terra completed before the Luna reviews, but the
experiment did not measure comparable per-agent runtimes. All six finished before the assessment was written; the
reviewed checkout, the original Saltybox checkout, and dotfiles were clean afterwards.

The result weakens the earlier reason for excluding Luna from complete slop and simplification charters. Luna found
useful issues while showing one instance of overreaching on classification. Terra also overreached, and Sol's restraint
came with two unreported opportunities. My next routing hypothesis would allow Luna high for these two lenses while
retaining coordinator verification. More samples would be needed before calling any configuration reliably best; none
were run here.

### muse-assessment.md

### Muse high: follow-up to the Saltybox manual comparison

Muse returned no findings for either charter. Both processes completed successfully and saved substantive review
artifacts. The simplification agent explicitly considered Luna's concatenation suggestion and rejected it as stylistic.
The slop agent missed the nonexistent Argon2 parallel feature and repeated that claim in its justification for accepting
the comments.

#### Method and completion

Model: `muse-spark-1.3-contributor`, high effort, through Muse Code 1.0.3. Two concurrent fresh sessions, one per
charter, with the identical saved scope, charters, and shared task used in the GPT round. No earlier findings were
supplied. There were no continuation turns, retries, fixes, full swarms, or repository eval scripts. Both processes
exited 0 and emitted successful terminal completion events; their artifacts contain actual reviews, not status or
refusal messages. Both were confirmed complete by 2026-09-05 02:11:01 UTC, after launch at 02:09:41 UTC. This is an
observed upper bound of about 80 seconds for the pair, not a precise model-latency benchmark. The reviewed checkout and
dotfiles were clean afterwards.

Session IDs and process handles are omitted from this archive. The harness differs from the native Codex agents used
previously, so this is not a controlled model-only comparison. Token usage and billed cost were not established.

#### Findings and interpretation

| Configuration | Slop findings                                                                | Simplification findings                                |
| ------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------ |
| Luna high     | Verified hallucinated dependency feature; additional resource-policy concern | Valid local concatenation cleanup                      |
| Terra medium  | Rejected unwired-engine complaint                                            | Marginal optional armor-helper inline                  |
| Sol low       | None                                                                         | None                                                   |
| Muse high     | None                                                                         | None; explicitly considered and rejected concatenation |

Muse's simplification agent correctly preserved the distinct truncation and parameter-validation diagnostics. It also
rejected helper extraction and broadening Cargo profile overrides. These are useful signs of restraint. Its decision
that concatenation is a stylistic wash is reasonable as a value judgment, even though the proposed change meets the
charter's literal smaller-code and identical-behavior conditions. This cannot be counted as an inability to discover the
opportunity. The same distinction cannot be recovered for Sol, whose empty report did not record that candidate.

Muse's slop review says the comments are substantive, including the reason for setting `p` to 1 without a rayon feature.
That explanation accepts the exact unsupported dependency detail Luna challenged. The earlier source verification
stands: `Cargo.lock` pins Argon2 0.5.3, whose feature table contains no rayon or parallel feature. The Muse report
additionally treats similarity to sibling AEAD code as proving the new imports against the dependency set; matching
usage patterns is weaker evidence than checking the new dependency's API. This supports identifying a verification gap
in this sample, not concluding that all of Muse's API reasoning is unreliable.

The simplification report separately notes missing SPEC coverage as an out-of-charter observation. It did not present
this as a simplification finding, so it is not included in the table. The report's `validate_params` line range is also
inaccurate; the checks are at source lines 131–179, not 194–243. Neither issue changes the zero-finding result.

#### Updated judgment

Luna remains the only configuration in these samples to report the verified hallucinated dependency feature. The Muse
result adds no reason to exclude Luna from these two charters. It also shows why raw finding counts are a weak
comparison: one model may notice a harmless cleanup and decide it is not worth reporting. One sample does not establish
general recall or a reliable ranking, and no further rounds were run.

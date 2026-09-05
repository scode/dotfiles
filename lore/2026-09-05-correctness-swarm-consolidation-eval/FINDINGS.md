# Findings and source assessment

These are the parent's unblinded, source-verified judgments after all four arm reports were sealed. They are separate
from the coordinators' own nofix buckets. Every original finding is retained in the four `outputs-*.md` compilations,
with local paths and native identifiers replaced. This inventory explains cross-arm matches without replacing those
outputs.

Subject: `62d866d6a57a24ef6bb329b28a246b44b758ff7a`, parent `3b1d3131902f724dd3827444ad67c7f8871ee2df`. Source
references below refer to
[the pinned checkout](https://github.com/scode/saltybox/tree/62d866d6a57a24ef6bb329b28a246b44b758ff7a). Dependency
evidence is the installed, pinned Argon2 0.5.3 source, with checksums recorded under `dependency_evidence` in
[metadata.json](metadata.json).

## Comparison

"Found" means the mechanism was identified, not merely that a proposed patch would incidentally fix it. Baseline's
test-cost item covers two independently editable tests; it is split into E04/E05 for comparison. Its workspace
recommendation also enables the feature that clears E03's temporaries, but it does not establish that separate
mechanism. Those distinctions prevent report formatting from manufacturing a coverage advantage.

| ID  | Mechanism                                                                  | Parent verdict                                             | Criticality                             | Baseline                                     | Combined Sol high            | Astra low | Astra high |
| --- | -------------------------------------------------------------------------- | ---------------------------------------------------------- | --------------------------------------- | -------------------------------------------- | ---------------------------- | --------- | ---------- |
| E01 | Resource-safety assurance overstates what the specified KDF caps guarantee | Accepted wording defect; lower-cap policy remains optional | Low                                     | F2                                           | F1                           | —         | —          |
| E02 | Argon2 heap workspace retains key-equivalent state                         | Accepted                                                   | Moderate                                | F3                                           | —                            | —         | F1         |
| E03 | Argon2's internal secret temporaries are not wiped                         | Accepted; same security family as E02                      | Moderate                                | Partial lead; recommended fix also covers it | —                            | —         | F2         |
| E04 | Armor/engine test redundantly runs production-cost KDF                     | Accepted                                                   | Low                                     | Part of F5                                   | F2                           | —         | —          |
| E05 | Randomness test redundantly runs production-cost KDF                       | Accepted                                                   | Low                                     | Part of F5                                   | F3                           | —         | —          |
| E06 | Argon2 errors preserve text but lose typed source                          | Optional enhancement                                       | Low                                     | F4                                           | —                            | —         | —          |
| E07 | V2 is absent from normal dispatch                                          | Rejected; explicitly intentional staging                   | None as a defect in this commit         | F1, later rejected in buckets                | —                            | —         | —          |
| E08 | Accepted 4 GiB allocation exceeds 32-bit vector capacity                   | Uncertain applicability; arithmetic is concrete            | Moderate if 32-bit builds are supported | General resource family only                 | General resource family only | —         | F3         |

There are five accepted atomic issues in this run's observed union, two moderate and three low. Baseline identifies
four, combined Sol high three, Astra high two, Astra low zero. The two moderate issues are one secret-cleanup family;
baseline and Astra high both cover that family, and baseline's recommended complete repair would address both
mechanisms. These counts are not five independent bug families or an exhaustive recall denominator.

## E01: Resource-safety wording versus the chosen policy

The new comments at
[src/format_v2.rs:69](https://github.com/scode/saltybox/blob/62d866d6a57a24ef6bb329b28a246b44b758ff7a/src/format_v2.rs#L69)
and line 125 describe the caps as preventing hostile memory/CPU bombs. The accepted maxima are 4,194,304 KiB, 64 passes,
and eight lanes. Decryption accepts a 52-byte header and dummy 16-byte tag, validates those numbers, and calls the KDF
before authenticating. Argon2's `hash_password_into` allocates the workspace with an ordinary infallible `vec!`.

The allocation and work claim is concrete. The implementation also follows the exact maxima chosen in
[the pinned plan](https://github.com/scode/saltybox/blob/62d866d6a57a24ef6bb329b28a246b44b758ff7a/lore/2026-07-01-saltybox2.md#L110),
with no lower supported-host budget. Step 4 deliberately leaves normal CLI dispatch unchanged. Public library callers
can reach the engine, but this commit does not newly expose v2 ciphertext through the CLI.

Accept the overstated safety guarantee as a low-severity finding, consistent with the prior assessment's SVE-003. The
minimum justified action is to describe finite caps and residual resource risk accurately. Lowering format caps, adding
a combined-work budget, or adding an opt-in policy deserves a design decision rather than being scored as an established
mandatory runtime fix. A Result return type does not guarantee recovery from every allocation failure.

Baseline general/data-flow explicitly identify the false assurance; the merged report emphasizes the resource concern.
Combined Sol high explicitly says this defeats the stated validation purpose. Their preferred runtime remedies are
broader than the demonstrated wording defect. Neither gets a high/material credit for an unverified host-specific OOM
scenario.

Origins: baseline all five reviewers p1; combined Sol p1. Astra high's E08 is a narrower, different precondition and is
recorded separately rather than credited with diagnosing the misleading safety wording.

## E02: Key material in the released heap workspace

[derive_key](https://github.com/scode/saltybox/blob/62d866d6a57a24ef6bb329b28a246b44b758ff7a/src/format_v2.rs#L182)
zeroizes the returned 32-byte key but calls Argon2 0.5.3 `hash_password_into`. Dependency src/lib.rs:229-232 allocates
an ordinary `Vec<Block>`; Block is Copy and has no drop-time wipe. Finalization at lines 481-505 combines the last block
of each lane and hashes it to produce the key. The workspace therefore retains enough information to reconstruct that
key after the function releases it. At default parameters this is a 256 MiB allocation.

This is a concrete secret-lifecycle gap, assessed moderate. Exploitation requires a separate way to recover process
memory or a memory capture; the review did not demonstrate a remote exploit, passphrase bypass, or current CLI exposure.
The large heap allocation and direct key reconstruction make it useful security feedback despite that prerequisite.

The complete repair is an explicitly owned zeroizing workspace passed to `hash_password_into_with_memory`, with the
dependency's zeroize feature enabled so Block implements the trait. Enabling the feature alone does not wipe the
internally allocated vector.

Baseline systems p1 and lifecycle p1 gave the complete repair. General p2 rediscovered the issue but incorrectly
recommended the feature alone; the coordinator's merged repair retained the complete version. Astra high found it in p1.
Combined Sol high and Astra low missed it through their completed searches.

## E03: Internal Argon2 stack temporaries

[Cargo.toml:18](https://github.com/scode/saltybox/blob/62d866d6a57a24ef6bb329b28a246b44b758ff7a/Cargo.toml#L18) selects
Argon2's defaults, which omit zeroize. Dependency src/lib.rs:322-323 conditionally clears `initial_hash`; lines 501-505
conditionally clear `blockhash` and `blockhash_bytes`. The latter buffers directly feed the final key-producing hash.
With the feature disabled, these copies remain uncleared independently of the returned key or heap workspace.

Accept this as a second moderate cleanup mechanism. It has the same separate-memory-disclosure prerequisite as E02 and
belongs to the same security family. Enabling the dependency feature activates these cleanup paths; it complements
rather than replaces owning and wiping the heap workspace.

Astra high p1 explicitly identified these copies and their feature-gated cleanup. Baseline lifecycle p1 asked to verify
the dependency's other temporaries and recommended enabling the same feature as part of E02, so its repair would cover
this too. It did not identify the exact uncleared stack values. Count Astra high's more precise discovery, but do not
describe it as a wholly new security family or claim baseline's proposed fix would leave the problem open.

## E04 and E05: Redundant production-cost unit tests

The module comment at
[src/format_v2.rs:379](https://github.com/scode/saltybox/blob/62d866d6a57a24ef6bb329b28a246b44b758ff7a/src/format_v2.rs#L379)
says defaults are exercised once, and the pinned step-4 plan asks for small parameters in most tests plus one default
round trip. In addition to the explicit default test, the armor/engine test at line 436 performs a production-cost
encrypt and decrypt, while the randomness test at line 468 performs two production-cost encryptions. That adds four 256
MiB KDF derivations to the intended two. Three workspaces can overlap if the test harness schedules these tests
concurrently.

Both are accepted low-severity test-cost defects against the stated testing intent. The needless work is
source-verifiable; actual timeout, OOM, wall time, and flakiness were not measured and are not promoted to established
failures. Preserve coverage while consolidating default/trait coverage or using a test-only small-parameter
random-header path.

These are independently editable test sites. Baseline lifecycle p1 and systems p2 bundle them as one item, retained as
F5. Combined Sol p1 reports them separately as F2/F3. Both configurations receive coverage of both sites, so the extra
Sol report is not a unique finding. The two Astra arms miss both sites.

## E06: Structured error-source preservation

Both error mappings at
[src/format_v2.rs:193](https://github.com/scode/saltybox/blob/62d866d6a57a24ef6bb329b28a246b44b758ff7a/src/format_v2.rs#L193)
include the underlying error's display text in the message and retain an Argon2Failure kind. They do not retain a typed
source. That would require enabling the dependency's nondefault std feature and using with_kind_and_source.

This is optional, low-severity diagnostic improvement. No caller requiring typed Argon2 source inspection was
established, and the causal message is not swallowed. This matches the prior assessment's SVE-004. Baseline general p3
found it; the coordinator would fix it as definite, which is stronger than the source establishes. It is the baseline's
only third-pass discovery and does not add material accepted coverage.

## E07: Unwired format engine

The absence of V2Engine from
[format dispatch](https://github.com/scode/saltybox/blob/62d866d6a57a24ef6bb329b28a246b44b758ff7a/src/format.rs#L72) is
real and intentional. The commit title says unwired; step 4 explicitly says not to register the engine or touch the CLI.
Reject the proposed integration fix in this scope.

Baseline Luna edge p1 reported it as definite. It survived merge/restatement into the pre-bucketing report; the
coordinator subsequently rejected it in the required nofix confirmation. This is a reviewer false positive that the
complete pipeline caught. Do not count it as accepted baseline output merely because it appears in REPORT.md.

## E08: 32-bit address-space overflow

With m=4,194,304 KiB, t=1, p=1, the header reaches a request for 4,194,304 1,024-byte blocks. Four GiB cannot fit a
32-bit vector's permitted allocation size. The older v1 engine has an explicit platform-size guard, and v2 lacks one.

The numerical mechanism is concrete. Applicability remains uncertain:
[dist-workspace.toml](https://github.com/scode/saltybox/blob/62d866d6a57a24ef6bb329b28a246b44b758ff7a/dist-workspace.toml)
names 64-bit release targets, and no contract establishing 32-bit source-build support was found. No 32-bit build was
run. A checked workspace-size guard is reasonable if those targets are supported; otherwise the platform requirement
should be explicit. Keep this possible/uncertain and outside accepted/material counts.

Astra high p2 identified it; p3 was empty. Baseline and combined Sol describe broader allocation risks without this
platform-specific argument. It is a sharper conditional analysis of the same allocation path, not another proven shipped
critical bug.

## Reference comparison and limitations

The old case assessment contained three accepted minor issues: a broken SPEC reference, a nonexistent Argon2 parallel
feature in a comment, and the resource-safety overstatement. The first two are documentation-lens targets excluded from
this correctness-only experiment. No new arm reported them; that is recorded without counting omitted review categories
as failures. E01 matches the third. Baseline and combined Sol recover it; neither Astra arm explicitly reports that
wording defect.

E02/E03 and the two test-cost sites are new relative to the old case assessment. The old reference is consequently
incomplete, and a zero result cannot be vindicated merely by its lack of previously accepted material bugs. Conversely,
broad usefulness in the original full swarm does not make this case a dense sample of runtime correctness failures.

No source code or dependencies were changed and no exploit, OOM, build, or timing reproduction was run. Exact dependency
source confirms the memory, cleanup, allocation, and feature claims. Severity and materiality remain the parent's
judgments under ASSESSMENT-RULES.md, with one case and one run per arm.

# Manual review-model evaluation, 2026-09-05

The question was whether cheaper models could replace some pre-PR review lenses without losing useful findings. The
initial exclusion of Luna was a hypothesis about discovery quality. This experiment tests that hypothesis through one
manual sample per case, lens, and model configuration. It does not measure billed cost or exhaustive recall.

This directory preserves the results of the completed experiment. It contains the report, methodology, metadata, output
tables, and assessed finding list. Raw transcripts, prompts, scoped diffs, copied charters, process logs, and repository
checkouts are omitted. Historical references to those artifacts in assessment prose describe the original experiment.

## Skill and case provenance

The skill used was `agent-skills/pre-pr-review-swarm` in `scode/dotfiles` at commit
**`5227a036fcec4a6ea6e5259a666f7e10680667a4`**. At that snapshot, the last commit changing the skill directory was
**`fae1c42e3348de51d6a65e7bd731669e24152c5f`**. The skill entrypoint's Git blob is
`b8283a47e3a8350b96b582d93ff259c4ce7f6a39`.

The
[skill snapshot](https://github.com/scode/dotfiles/tree/5227a036fcec4a6ea6e5259a666f7e10680667a4/agent-skills/pre-pr-review-swarm)
is the source of the reviewer charters. The full swarm workflow was not executed. All six saved matrix charter files
(five lenses plus the shared correctness contract) were verified byte-for-byte against that commit when preparing this
archive. The earlier simplification and AI-slop charters were also verified. [metadata.json](metadata.json) records Git
blob IDs and SHA-256 hashes so the exact text can be recovered without copying installed skills into lore.

Case selection used every entry marked `curation = "hand"` in
[`evals/pre-pr-review-swarm/cases.toml`](https://github.com/scode/dotfiles/blob/5227a036fcec4a6ea6e5259a666f7e10680667a4/evals/pre-pr-review-swarm/cases.toml).
The saved list was verified byte-for-byte against the same commit. There were 13 selected cases, including the three
Stark Parts cases after the mined entries. The 20 mined cases were excluded. Five public repositories are represented:
Treeward, Ferricode, Dotfiles, Saltybox, and Stark Parts; seven cases are related Saltybox commits.

Every subject ref was resolved after fetching tags into isolated repositories. Each review compared a pinned subject
against its explicit base or first parent. Metadata retains full subject/base hashes, original refs, commit messages,
touched and omitted paths, diff sizes, and SHA-256 hashes of the exact scoped diffs. Omitted lockfiles and generated
vectors remained readable as context. All 13 after-state checkouts were clean and their heads and scope hashes matched
at completion; the saved integrity receipts record that check.

## Original proposed routing

The first conservative proposal used Sol medium for idiomaticity, AI slop, docs/comments, and simplification; Sol high
for the five correctness reviewers, three security reviewers, test quality, and SPEC compliance; Sol medium for the
restater; and the current strong model for the coordinator. Terra was proposed as a subsequent experiment for the four
non-critical charters and then data flow and edge inputs.

The user asked for a first-principles proposal independent of the skill's reviewer exemptions. The revised table below
motivated the expanded experiment. It covers the full proposed routing; the experiment itself covered only the five
Terra-medium rows. Security's originally grouped row is expanded here to show every reviewer.

| Role                          | Original revised proposal | Expanded experiment                          |
| ----------------------------- | ------------------------- | -------------------------------------------- |
| Idiomaticity                  | Terra medium              | Luna high, Terra medium, Muse high           |
| AI slop                       | Terra medium              | Luna high, Terra medium, Muse high           |
| Docs/comments                 | Terra medium              | Luna high, Terra medium, Muse high           |
| Simplification                | Sol medium                | Earlier one-off only                         |
| Correctness: data flow        | Terra medium              | Luna high, Terra medium, Muse high, Sol high |
| Correctness: edge inputs      | Terra medium              | Luna high, Terra medium, Muse high, Sol high |
| Correctness: general          | Sol high                  | Not evaluated                                |
| Correctness: state/lifecycle  | Sol high                  | Not evaluated                                |
| Correctness: systems          | Sol high                  | Not evaluated                                |
| Security: general             | Sol high                  | Not evaluated                                |
| Security: input/trust         | Sol high                  | Not evaluated                                |
| Security: secrets/environment | Sol high                  | Not evaluated                                |
| Test quality                  | Sol high                  | Not evaluated                                |
| SPEC compliance               | Sol high                  | Not evaluated                                |
| Restater                      | Sol medium                | Not evaluated                                |
| Coordinator                   | Current strong model      | Assessment role, not an experimental subject |

The rationale for Terra was that local conventions, claim-to-code comparisons, explicit data tracing, and boundary
enumeration give the reviewer structure. Stronger models were retained for open-ended discovery, temporal and systems
reasoning, exploitability, and test-oracle judgment. A strong coordinator can reject a bad finding but cannot validate
an unreported bug. The report revises the Terra hypothesis using the observed results; no installed routing changed.

## Execution

Each case received 17 independent reviews: Luna high, Terra medium, and Muse high for all five selected lenses, plus Sol
high for the two correctness lenses. That is 65 reviews each for Luna, Terra, and Muse, and 26 for Sol: 221 total. The
exact model IDs were `gpt-5.6-luna`, `gpt-5.6-terra`, `muse-spark-1.3-contributor`, and `gpt-5.6-sol`. GPT subjects used
native Codex subagents with explicit model and effort overrides and `fork_turns="none"`. Muse subjects used fresh
`muse exec` contributor sessions with explicit high effort; the verified Muse Code version was 1.0.3. There was no model
fallback. Coordinators inherited the parent model and were assessors, not experimental subjects; their exact model ID
was not recorded in the aggregate.

Reviewers received the same case metadata, commit message, immutable diff, pinned after-state checkout, common task, and
selected charter. Correctness reviewers also received the shared correctness contract. Both correctness lenses require a
full correctness review followed by their additional focus pass, including off-lens correctness discoveries. All
selected lenses ran even for prose-only cases. The swarm's prose fast path was not applied.

Each cell ran one complete review turn, including a deliberate second sweep for unrelated issues within that turn. There
were no continuation turns or repeated successful reviews. Empty findings were valid completed results, not a reason to
retry. Subjects could inspect applicable repository instructions, SPEC context, callers, and dependencies as allowed by
their charter. They were instructed to stay read-only, avoid builds and tests, avoid later revisions and sibling
outputs, and report source anchors, confidence, behavior, consequence, and suggested changes. No repository eval runner
was used.

Case coordinators scheduled concurrent reviewers and assessed their outputs; the parent aggregated results and checked
disputed or consequential judgments. The normal schedule used three coordinators, each with up to four native subjects
and two Muse processes. Overlap was adjusted as cases finished, within 16 native slots including the parent and
coordinators. Concurrency and runtime were not controlled benchmarking variables. Completion required substantive saved
reports and agent/process completion, not an exit code alone. All 221 reviews completed successfully with zero failures.

## Assessment and counting

Assessors checked reported claims against pinned source and relevant contracts, without builds, tests, or fixes.
Assessment was not blinded to model identity. Each claim received a verdict (`valid`, `optional`, `uncertain`,
`invalid`, or `out_of_scope`), impact (`material` or `minor`), evidence, rationale, and canonical issue ID. Neither an
empty result nor another model's disagreement was treated as ground truth. Conditional findings retain their conditions.

There were 86 original findings. Three combined findings were split into independently assessable claims, producing 89
records. [FINDINGS.md](FINDINGS.md) retains all of them, including rejected claims, original local IDs, model/lens
attribution, source evidence, and final rationale. [OUTPUTS.md](OUTPUTS.md) lists every cell, its original finding
count, and the assessment verdict counts. [metadata.json](metadata.json) also retains each original output's SHA-256
hash, but the original output itself is intentionally outside this archive.

The accepted union has 22 distinct case issues, nine material. Deduplication joins reports across lenses within a case;
it does not collapse related commits into independent bug families. Material findings concern consequential behavior,
resource bounds, or contracts. Minor documentation defects are counted separately. Per-lens recovery uses the union of
accepted issues observed in this experiment as its denominator. It is not exhaustive recall; a lens with no accepted
issues has no positive recall evidence. All five evaluated lenses appear in the report, including idiomaticity and AI
slop, which had no accepted findings.

## Limits and context deviations

There is one sample per cell, and the selected cases are neither random nor independent. Muse's different harness
confounds model comparisons. No reliable attributable token usage or billed costs were collected. The result measures
review output, not monetary savings. The full swarm's unevaluated lenses may provide complementary coverage that this
experiment cannot quantify. The [earlier one-offs](PRIOR.md) used Sol low and included simplification; they are excluded
from the expanded matrix counts.

Several subjects exceeded the idiomaticity/slop charter's neighboring-file limit. In the Stark Parts catalog-only CI
case, several GPT subjects read third-party action sources downloaded by the assessor into the case directory while
reviews were active. These were primary sources rather than sibling findings, but availability was asymmetric. No
successful review was rerun to compensate. Metadata retains the case-level disclosures, including the affected cells.

The report includes two sensitivity checks: excluding that CI case, and treating the unwired engine's resource policy as
material instead of crediting its false comment assurance as minor. Related decrypt and write-gate cases expose one
post-rename contract issue family. These qualifications limit generalization; they do not invalidate the recorded
observations.

## Archive privacy and scope

The reviewed repositories were verified public when preparing the archive. Host-local checkout and dependency-cache
paths were normalized or replaced with pinned public source URLs. Session IDs, agent handles, process handles, and
process logs are omitted. Public repository identifiers, commit hashes, dependency versions, and localhost OAuth
examples are retained because they explain the findings. The archive contains no execution workspace or copied source
tree. Markdown and JSON whitespace is formatted to the repository's conventions.

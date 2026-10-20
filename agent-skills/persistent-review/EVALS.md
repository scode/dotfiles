# Evaluating persistent-review

Use fresh contexts and an isolated copy of the installed skill and its dependency. Supply the skill catalog entry and an
ordinary user task; enter through `SKILL.md` rather than supplying the harness sidecar or the expected answer. Keep all
generated inputs, prompts, results and transcripts in private scratch space. Product files stay read-only.

## Real reviewer exercises

Exercise both Codex and Claude, with concurrent independent functional groups. Name the reviewer model and effort,
charter, mandatory scope and result destination. Use stable text whose required ranges are easy to identify. A large
numbered sequence with a consistency charter works without needing planted defects or a finding quota.

Constrain one first-turn helper packet with a deliberately small output allowance to trigger clipping. Let the reviewer
return before it has consumed the whole group. Subsequent turns can repair through smaller packets. Preserve every
turn's evidence. Check that the orchestrator continues while making progress without inventing a deadline or asking for
a time budget, including when loading the shellout dependency.

Inspect the outcome independently:

- Which skill did the orchestrator load, and which mechanism did it choose from the actual observation capabilities?
- Do per-group identities and actual initial/resume events establish at least two turns in each session?
- Does the helper match actual delivered packets against immutable source text, keeping code out of the parent's compact
  report? Check raw events independently in the test, not by making the production parent reread every payload.
- Did missing ranges cause concrete continuations even when the first result had no findings? Does the union of
  inspected delivered ranges cover the mandatory scope, with the right version and any diff sides?
- Did a later turn synthesize interactions after the missing material arrived, and did the parent audit it afterwards?
- Does the completion record match the evidence, preserving findings and naming unresolved material on a stop?
- Give an early planted defect and later source reads: was its finding appended before those later reads, rather than
  buffered to the turn boundary? Can the parent see the finding/count from artifacts while reviews are still running?
- Do semantic groups run concurrently, with disjoint logs, genuine capacity constraints recorded, and observed counts
  kept distinct from deduplicated findings? Do cross-group interface questions receive synthesis?

The shellout harness files describe version-specific evidence sources. Probe current local representations before
crediting them: a Codex raw command event can contain more than its delivered result, and Claude Read can declare
clipping in structured metadata without a literal marker in its text. Native Read is not automatically credited by the
narrow helper. Success on one backend does not establish the other's semantics. Keep verified observations separate from
a child's claims.

## Discovery and failure cases

In independent cold scenarios, check explicit invocation, the phrase "persistent adversarial review," ordinary review,
and discussion about changing the skill. The latter two must not silently launch this protocol. Exercise an unsupported
reviewer backend and both observable and unobservable native delegation.

Exercise a large multi-subsystem scope: groups must follow behavior boundaries, include callers/contracts/tests, cover
every required range, and fit the stated ceilings. A caller's explicit serial restriction must override the default;
otherwise ready groups launch concurrently. Test overlapping findings, distinct defects at the same location,
conflicting claims, one incomplete group, and resumptions after compaction using the existing logs and group map.

Use controlled transcript/result fixtures for absent evidence, an empty claimed-complete result backed only by partial
reads, a caller-supplied time limit with retained findings, and two repair turns with new findings but no new delivered
content. Also check that elapsed time alone does not stop a progressing review when the caller supplied no time limit.
Require concrete next actions and honest completion status. A completed second-turn repair followed by synthesis must
not require an otherwise unnecessary third turn.

Run `deno check` and `deno test -A` on `evidence.ts`. Cover both delivered-result schemas, false raw-stdout coverage,
unknown records/blocks, wrong session/cwd, unpaired calls, malformed/trailing records, source drift, corrupt and clipped
packets, overlapping ranges, resumed streams, failed tools, empty findings, finding revisions/withdrawals, and log-owner
identity. Unknown representations must visibly fail without a partial success report. A cold agent encountering that
failure must ask before direct inspection, explicitly explain the token cost, and retain findings. Check that it neither
silently skips records nor treats missing data as zero findings or complete coverage.

Include Codex top-level `compacted` records between original calls/results and after completed reads. Coverage must
survive when original delivery exists; packets found only in compaction summaries or replacement history must not fill
gaps. Mutated compaction envelopes must still fail visibly, including after full coverage was established.

For both backends, mix valid required-source packets with context packets from another manifest. The audit must report
the latter without certifying them, credit only matched mandatory text, preserve gaps, and avoid loading foreign source
paths. Fully covered mandatory scope may complete despite extra context. Malformed foreign packets must still fail.

For both backends, vary JSON key order, additive packet metadata, spacing and digest case while preserving essential
values. Coverage must remain identical. Fully delivered matching packets above producer line/byte budgets remain valid
evidence, while `read` still enforces its own budgets. Blank JSONL separators preserve physical references; blank-only
required logs remain errors. Corrupt text, version mismatches, invalid source bounds and unknown evidence shapes must
continue to fail, including after earlier successful coverage.

Scenarios may be combined; do not spend a separate live run on each assertion. The real exercises prove the operational
path, while fixtures cover stopping cases without waiting for a stalled run. Validate helper code behavior if any is
added, and run the repository checks. Matching instruction text is not a behavioral test.

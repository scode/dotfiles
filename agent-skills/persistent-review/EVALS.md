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
- Do per-group identities and actual initial/resume events establish initial review, later synthesis, and a subsequent
  discovery search in the same session, with further passes while new verified or investigated-unverified findings
  appear?
- Does the helper match actual delivered packets against immutable source text, keeping code out of the parent's compact
  report? Check raw events independently in the test, not by making the production parent reread every payload.
- Did missing ranges cause concrete continuations even when the first result had no findings? Does the union of
  inspected delivered ranges cover the mandatory scope, with the right version and any diff sides?
- Did a later turn synthesize interactions after the missing material arrived, and did the parent audit it afterwards?
- After coverage/synthesis, did an explicit new search occur even when earlier findings were empty? Did a verified new
  finding trigger another same-session search, with immediate persistence and explicit permission to return empty?
- Did a concrete concern retained as unverified after investigation also trigger a search beyond known findings, without
  repeatedly reopening the same uncertainty or counting it as a verified defect?
- Does the completion record match the evidence, preserving findings and naming unresolved material on a stop?
- Did every reviewer prompt tell the reviewer to read source itself rather than through subagents, while still allowing
  subagents for tests or builds? If a reviewer used a subagent to read anyway, did the parent treat those ranges as
  missing rather than delivered?
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
not require another turn solely for coverage approval; the separate subsequent discovery search is still mandatory.

Use controlled search results to exercise productive passes beyond three turns followed by an explicit empty search.
Require each new verified or investigated-unverified finding to trigger another pass without a default cap. Also
exercise duplicate-only outputs, new IDs for old defects, unsupported or out-of-charter claims, empty replies without
performed work, unresolved leads, unavailable continuation, and a caller limit reached before an empty pass. The
orchestrator must verify novelty and actionability, retain original logs, and distinguish an actual final empty search
from an incomplete stop. Reopen discovery after new cross-group evidence is synthesized. Check that a valid
charter-relevant finding is not discarded solely for failing another workflow's severity threshold, and that finding
progress cannot excuse missing delivery.

Exercise a concrete suspected crash-loss sequence whose source is available but whose filesystem guarantees cannot be
established from available evidence. The reviewer must make focused verification attempts, record what those checks
showed and the precise remaining premise, and surface the concern as **unverified after investigation** in both the
findings log and caller-visible result. After the required synthesis and a proper search with no new findings of either
kind, the group and overall review must complete despite that uncertainty. Include another verified finding and check
that counts distinguish the two. An old uncertainty alone must not force failure or prevent a caller's checkpoint from
advancing when its other requirements are met.

Contrast that case with a first-impression suspicion labeled unverified without any checks: the orchestrator must send
it for investigation rather than accept the label as a completed disposition. Also exercise a refuted suspicion, a
productive investigation that resolves its missing premise, and a repeated uncertainty with no new evidence. These
should respectively be rejected, resolved according to the evidence, and kept without renewed investigation. A lead left
uninvestigated, missing mandatory source delivery, lost continuation, or an unperformed discovery pass still keeps the
review incomplete; investigated uncertainty must not hide those failures. This distinction must hold across groups: one
group with a properly recorded investigated uncertainty does not invalidate other completed groups.

Give a padded investigation record that lists checks without useful results and calls a premise unavailable even though
readable context or a permitted test can settle it. The orchestrator must request the concrete check, not accept the
label. Give a reviewer-open finding that the orchestrator downgrades after focused checks: only the owning reviewer may
append the revision, under the original ID, and the parent must confirm it. Mix rejected, duplicate, and unassessed open
records with retained findings; per-kind counts must come from reconciled dispositions after deduplication, never raw
active counts or label absence. Conflicting verification claims about the same defect require synthesis before that
count. Repeated or drifting replies that never perform the search still stop incomplete; a known investigated
uncertainty does not. A caller's explicit stricter acceptance condition remains binding, while a generic requirement for
a complete review uses this protocol's completion definition.

Run `cargo test --test helper_launcher` for launcher selection and `cargo test --test helper_launcher -- --ignored` for
a real native compile and startup freshness checks. The latter requires Deno on the build host and may download its
runtime. Cover directory symlinks, spaces in paths, an unrelated working directory, missing Deno, broken/nonexecutable
artifacts, and stale or missing neighboring source. Builds must preserve the old executable on compiler failure or a
concurrent source edit. These checks do not run a real reviewer or establish CentOS compatibility from a successful
native or Ubuntu test.

Run `deno check` and `deno test -A` on `evidence.ts`. Cover both delivered-result schemas, every Codex tool family seen
in real rollouts, false raw-stdout coverage, unknown records and blocks, wrong session/cwd, orphan and repeated results,
unfinished calls, malformed/trailing records, source drift, corrupt and clipped packets, oversized Codex results,
overlapping ranges, resumed streams, failed tools, empty findings, finding revisions/withdrawals, and log-owner
identity. Unknown records, blocks and result shapes must never abort the audit or earn coverage, even when they quote
packets; they must appear in `unrecognized`. Identity and packet-integrity failures must visibly fail without a partial
success report. Give a cold orchestrator a report whose missing ranges match reads the reviewer claims and whose
`unrecognized` holds a plausible tool-output kind: it must treat that as a format failure, ask before direct inspection
with the token-cost warning, and retain findings, rather than requesting the same reads again or calling coverage
complete.

Before trusting a parser change, run the audit with a throwaway manifest over the local corpus of real Codex rollouts
and fresh Claude `-p --verbose` streams that use subagents, ToolSearch and background tasks. Every completed top-level
transcript should parse, and `unrecognized` should be empty for the shapes already checked; failures should be limited
to unfinished writes and unverified identity cases.

Include Codex top-level `compacted` records between original calls/results and after completed reads. Coverage must
survive when original delivery exists; packets found only in compaction summaries or replacement history must not fill
gaps. Mutated compaction envelopes are skipped like other non-tool records and must not change coverage.

For both backends, mix valid required-source packets with context packets from another manifest. The audit must report
the latter without certifying them, credit only matched mandatory text, preserve gaps, and avoid loading foreign source
paths. Fully covered mandatory scope may complete despite extra context. Malformed foreign packets must still fail.

For both backends, vary JSON key order, additive packet metadata, spacing and digest case while preserving essential
values. Coverage must remain identical. Fully delivered matching packets above producer line/byte budgets remain valid
evidence, while `read` still enforces its own budgets. Blank JSONL separators preserve physical references; blank-only
required logs remain errors. Corrupt text, version mismatches, invalid source bounds and undecodable tool results must
continue to fail, including after earlier successful coverage.

Scenarios may be combined; do not spend a separate live run on each assertion. The real exercises prove the operational
path, while fixtures cover stopping cases without waiting for a stalled run. Validate helper code behavior if any is
added, and run the repository checks. Matching instruction text is not a behavioral test.

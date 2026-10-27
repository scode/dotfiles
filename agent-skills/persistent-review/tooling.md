# Compact evidence and findings

Always invoke the `evidence` launcher relative to this skill's actual base directory; `/skill/evidence` below is a
placeholder. It selects a matching compiled executable when present, otherwise runs the neighboring `evidence.ts` with
Deno. Make that selection through the launcher, not separate agent probes. Source snapshots and logs remain private run
artifacts. The tool never launches a reviewer or inspects reasoning.

A stale or broken binary is an error, even if Deno is installed. Compiled helpers check their neighboring source before
executing any command and refuse changed or missing source; do not bypass this by invoking `evidence.ts` directly. Ask
for the helper to be rebuilt and copied again. A missing matching binary permits source execution only when Deno exists.
Checks detect drift at startup; they do not lock files against concurrent edits.

## Pin a functional group

Create a manifest from absolute paths to immutable UTF-8 source snapshots. Keep a mapping to original paths/revisions
and diff sides in the group notes. Separate manifests and log sets belong to separate reviewers.

```text
/skill/evidence manifest /run/group/manifest.json /run/snapshots/module.rs /run/snapshots/module_test.rs
```

This creates a version-1 manifest exclusively, with each source's absolute path, SHA-256, and required inclusive line
ranges. Full files are the default; narrow `ranges` in the manifest when the agreed scope requires only part of a file.
Do not change it while a reviewer is running. Empty files have no required text ranges. Source content is not printed.
Line endings are preserved; binary/non-UTF-8 sources are unsupported.

## Deliver small reads

Tell the reviewer to read mandatory text with:

```text
/skill/evidence read /run/group/manifest.json /run/snapshots/module.rs 1 120
```

Reads contain at most 200 lines and 24,000 encoded bytes. Use fewer lines if the helper rejects a range or the harness
clips the result. Each output packet includes source identity, version, exact text and range between versioned markers.
The audit compares the delivered text with the retained source; it does not trust the packet's claimed hash or range.
Surface stdout directly in the model's tool result. Do not JSON-encode it again, summarize it, or hide it in an
artifact. Native Read, arbitrary shell output, direct prompts, images, and raw command events are not automatically
credited. They may help investigation, but mandatory text still needs supported packets or an explicitly approved manual
audit.

## Audit completed turns

```text
/skill/evidence audit codex THREAD_ID /reviewer/cwd /run/group/manifest.json /run/group/rollout.jsonl
/skill/evidence audit claude SESSION_ID /reviewer/cwd /run/group/manifest.json /run/group/turn1.jsonl /run/group/turn2.jsonl
```

Supply the expected reviewer ID and working directory, not values inferred from an arbitrary log. Codex needs the exact
persisted rollout, including resumes, rather than `--json` command events. Claude needs every completed turn's verbose
stream in order. Use the dependency's capture and identity mechanics. Do not concatenate overlapping exports or mix
reviewers. The helper joins calls/results and validates each log's session and cwd before crediting packets.

NOTE: The audit is built not to break when a harness adds a command, tool, record type or content shape, and both Codex
and Claude Code add them often. It aborts only when continuing could misattribute identity or certify text that does not
match the pinned source. Everything else becomes "no credit" plus a report entry, and you decide what it means.

Only tool results can earn coverage. On Codex a tool call or result is any `response_item` payload with a string
`call_id`, whatever its tool family; a result's type ends in `_output`. On Claude they are assistant `tool_use` blocks
and user `tool_result` blocks. A result earns coverage only when a call with its ID appears earlier in the supplied
logs. Its text is either a string or the `text` of each block in an array. Other blocks (images, ToolSearch references,
a renamed text block) and non-array content contribute nothing. One Codex call may have several results, because a
code-mode `exec` cell that outlives its yield delivers its later output under the original call ID. These
representations were checked against Codex 0.153.4 through 0.163.0 rollouts and a Claude Code 2.1.308 verbose stream.
That is not a promise about every version.

Codex persists each tool result in full but sends the model only about 12,000 tokens of it, and when it cuts, the
rollout text carries no marker. A Codex result over 40,000 bytes therefore earns no coverage. That bound comes from
local measurement, not documentation: oversized results added only 11-14k input tokens, and packet-bearing results
stayed under 40k characters. Keep each `evidence read` in its own tool call so its result stays small.

The report separates what it could not use, so silence never stands in for a claim:

- `rejected` lists packet-bearing results that earned nothing, with a reason: no matching call, client-authored, larger
  than Codex reliably delivers, reported failure (including an error flag the helper cannot read), or clipped.
- `unfinished_calls` counts calls that have no result yet, usually because the turn is still running. Re-audit after the
  turn boundary.
- `ignored` counts skipped records of kinds known to carry no delivery: Codex lifecycle records, messages, reasoning,
  agent messages and server-side searches; Claude hooks, task notices, turn results, thinking, synthetic user text, and
  subagent records; image and ToolSearch result blocks. These need no attention.
- `unrecognized` counts skipped records and result blocks of any other kind. They earn nothing, even when they quote a
  complete packet. A non-empty `unrecognized` means the harness wrote something this helper has not seen. Judge whether
  it could hold reads the audit missed: if the missing ranges match reads the reviewer reports making and an
  `unrecognized` kind plausibly carries tool output, treat it as a format change and take the manual-inspection path
  below rather than asking for the same reads again. Otherwise trust the report as usual.

Identity checks still apply around everything that is skipped. Each Codex log needs exactly one `session_meta` with the
expected thread ID and cwd before any other record. Rollouts of native Codex subagents cannot be audited, because what
their model received has not been verified: a spawned child names its root thread in `session_id`, and a forked child
also carries a second `session_meta` followed by a copy of its parent's history. Both fail with a diagnostic saying so;
run Codex reviewers through `codex exec`. Every Claude record that names a session must name the expected one, including
skipped records. Each Claude log needs a `system/init` with the expected session and cwd before its first assistant or
user message. Later inits in the same log, which appear when a finished background task starts another turn, must match
too. Claude records with a non-null `parent_tool_use_id` belong to a subagent and were delivered to it, not the
reviewer; the subagent's answer reaches the reviewer as the result of its `Agent` call and is audited like any other
result.

Codex's top-level `compacted` record is context management. Original call/result evidence before compaction remains
valid, and neither the summary nor the replacement history can prove delivery of missing text. If original results are
absent from the supplied rollout, those ranges remain unverified.

Successful parsing exits 0 with a compact JSON report. Its `status` means delivery coverage only: `complete` or
`incomplete`, with source versions, covered/missing ranges, raw log line references, and the `rejected`,
`unfinished_calls`, `ignored` and `unrecognized` entries described above. Repeated reads do not inflate covered ranges.
Keep reports on disk and read them per group rather than aggregating every event into the orchestrator's context. A
missing packet end is a clipped read and earns no coverage; reread a smaller range.

Reviewers may read additional context with another manifest. Structurally valid packets for paths absent from the
audited manifest appear in `outside_manifest`, with reported ranges and transcript references. Their source versions and
contents are unverified and earn no coverage credit; the helper does not load those sources or expand mandatory scope.
Inspect the list for mistaken group/manifest selection. Required ranges remain missing until independently matched.
Malformed required fields in context packets still fail visibly rather than being silently skipped.

The audit compares essential values, not JSON serialization. Reordered object keys, additive metadata, JSON spacing and
hexadecimal digest case are harmless. Blank JSONL separator lines are skipped with physical evidence line numbers
preserved; whitespace-only required logs still fail as empty.

The 200-line/24,000-byte budgets constrain `read`, not existing delivery evidence. A larger fully delivered packet can
be audited when its source identity, range and exact decoded text match. The audit streams JSONL records, but each
record and retained source snapshots still need to fit in memory; producer defaults are not an audit memory limit.

Malformed JSON lines, mixed sessions or working directories, missing initialization, snapshot drift, malformed packets
and packet content that does not match the pinned source fail with a nonzero exit. Expected errors print
`PERSISTENT_REVIEW_TOOL_FAILED` to stderr and no coverage report to stdout; unexpected runtime errors also exit nonzero.
Preserve the diagnostic. A malformed last line is usually an unfinished write; retry at the turn boundary. For other
failures, or when you judge that `unrecognized` records hide reads, explain the problem and ask whether to use manual
inspection, warning that actual tool outputs duplicate code in the orchestrator's context and use more tokens. Until
approved, credit no new coverage from that invocation. A previous valid audit remains historical evidence, not a
successful current audit.

## Record and count findings

Each reviewer owns one append-only JSONL file, initialized empty before reading source. Once a finding is ready, write
its event to a private JSON file and append it immediately with the command below, before further analysis or reads:

```json
{
  "version": 1,
  "kind": "finding",
  "id": "f1",
  "state": "open",
  "summary": "short finding title",
  "location": "original/path.rs:42",
  "detail": "The complete finding in the caller's format, with evidence and impact."
}
```

```text
/skill/evidence record /run/group/findings.jsonl /run/group/event.json
/skill/evidence findings /run/group/findings.jsonl /run/other-group/findings.jsonl
```

Use the same local ID for revisions and `state: "withdrawn"` for a retraction; include the reason in `detail`. Never
erase the original event. This format supplies persistence and identity, not a review lens or severity taxonomy. Save
unresolved leads and current progress separately; do not count leads as completed findings.

For a concern retained as **unverified after investigation**, put that label in `summary` and its investigation record
in `detail`: source basis, suspected consequence, checks actually performed and results, missing premise, why available
checks cannot settle it, and what evidence would. Keep `state: "open"`; open/withdrawn tracks retention, not confidence.
Revise an existing finding under its original ID when its verification status changes. This uses the existing event
schema. The orchestrator requests these writes from the owning reviewer and confirms the append; it never appends to a
reviewer's log itself. Cross-group duplicates retain each original record's owner and ID. A lead that has not received
actual verification work stays in progress notes until investigated or rejected.

The counter reports active and withdrawn counts and active IDs, summaries, locations, and log paths. It counts every
reviewer-open record, regardless of the orchestrator's disposition: verified findings, concerns unverified after
investigation, duplicates, rejected claims, and candidates still under investigation may all be present. Per-kind counts
come from the orchestrator's reconciled dispositions after semantic deduplication, not label absence or subtraction from
this raw count. It counts the latest state of each ID per log. These are observed counts before semantic deduplication:
the orchestrator reads finding details to determine whether two reviewers found the same defect. Empty initialized logs
count as zero; missing logs or malformed events fail. If polling catches an unfinished append, retry after the write
completes and report counts as temporarily unavailable rather than silently skipping the record. Every log has exactly
one writer.

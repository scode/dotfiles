# Compact evidence and findings

Run `evidence.ts` with Deno. It uses built-in APIs only. Resolve it relative to this skill's actual base directory; the
commands below use `/skill/evidence.ts` as a placeholder. Source snapshots and logs remain private run artifacts. The
tool never launches a reviewer or inspects reasoning.

## Pin a functional group

Create a manifest from absolute paths to immutable UTF-8 source snapshots. Keep a mapping to original paths/revisions
and diff sides in the group notes. Separate manifests and log sets belong to separate reviewers.

```text
deno run -A /skill/evidence.ts manifest /run/group/manifest.json /run/snapshots/module.rs /run/snapshots/module_test.rs
```

This creates a version-1 manifest exclusively, with each source's absolute path, SHA-256, and required inclusive line
ranges. Full files are the default; narrow `ranges` in the manifest when the agreed scope requires only part of a file.
Do not change it while a reviewer is running. Empty files have no required text ranges. Source content is not printed.
Line endings are preserved; binary/non-UTF-8 sources are unsupported.

## Deliver small reads

Tell the reviewer to read mandatory text with:

```text
deno run -A /skill/evidence.ts read /run/group/manifest.json /run/snapshots/module.rs 1 120
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
deno run -A /skill/evidence.ts audit codex THREAD_ID /reviewer/cwd /run/group/manifest.json /run/group/rollout.jsonl
deno run -A /skill/evidence.ts audit claude SESSION_ID /reviewer/cwd /run/group/manifest.json /run/group/turn1.jsonl /run/group/turn2.jsonl
```

Supply the expected reviewer ID and working directory, not values inferred from an arbitrary log. Codex needs the exact
persisted rollout, including resumes, rather than `--json` command events. Claude needs every completed turn's verbose
stream in order. Use the dependency's capture and identity mechanics. Do not concatenate overlapping exports or mix
reviewers. The helper joins calls/results and validates each log's session and cwd before crediting packets.

Supported delivered shapes are Codex 0.163.0 `response_item` / `custom_tool_call_output` arrays of `input_text` blocks
and Claude Code 2.1.302 verbose `user.message.content` tool results, paired with assistant `tool_use` blocks. Claude
text results may be strings or arrays of text blocks. These are verified representations, not a promise about every
version. Other tool-result records or content blocks fail explicitly. The parser ignores known non-tool records and
reasoning without emitting them. Valid logs with no matching packets remain incomplete; full raw stdout cannot fill that
gap.

Successful parsing exits 0 with a compact JSON report. Its `status` means delivery coverage only: `complete` or
`incomplete`, with source versions, covered/missing ranges, rejected packets, and raw log line references. Repeated
reads do not inflate covered ranges. Keep reports on disk and read them per group rather than aggregating every event
into the orchestrator's context. A missing packet end is a clipped read and earns no coverage; reread a smaller range.

Format errors, malformed JSON, orphan results, mixed sessions, snapshot drift, and unmatched packet content fail with a
nonzero exit. Expected format errors print `PERSISTENT_REVIEW_TOOL_FAILED` to stderr and no coverage report to stdout;
unexpected runtime errors also exit nonzero. Preserve the diagnostic. An unfinished writer can be retried at its turn
boundary. For other format failures, explain the failure and ask whether to use manual inspection, warning that actual
tool outputs duplicate code in the orchestrator's context and use more tokens. Until approved, credit no new coverage
from the failed invocation. A previous valid audit remains historical evidence, not a successful current audit.

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
deno run -A /skill/evidence.ts record /run/group/findings.jsonl /run/group/event.json
deno run -A /skill/evidence.ts findings /run/group/findings.jsonl /run/other-group/findings.jsonl
```

Use the same local ID for revisions and `state: "withdrawn"` for a retraction; include the reason in `detail`. Never
erase the original event. This format supplies persistence and identity, not a review lens or severity taxonomy. Save
unresolved leads and current progress separately; do not count leads as completed findings.

The counter reports active and withdrawn counts and active IDs, summaries, locations, and log paths. It counts the
latest state of each ID per log. These are observed counts before semantic deduplication: the orchestrator reads finding
details to determine whether two reviewers found the same defect. Empty initialized logs count as zero; missing logs or
malformed events fail. If polling catches an unfinished append, retry after the write completes and report counts as
temporarily unavailable rather than silently skipping the record. Every log has exactly one writer.

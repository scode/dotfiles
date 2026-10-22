#!/usr/bin/env -S deno run -A
/**
 * Keep delivered source out of the orchestrator's context without trusting read receipts.
 * Only complete, source-matched packets in paired tool results earn coverage. Transcript shapes the helper
 * does not understand earn nothing and are counted rather than fatal, because harnesses add record types,
 * tools and block shapes often; raw logs remain the evidence, not this report's counts.
 */
import { createReadStream } from "node:fs";
import { createInterface } from "node:readline";

type ObjectValue = Record<string, unknown>;
type Range = [number, number];
type Source = { path: string; sha256: string; ranges: Range[]; lines: string[] };
/** A delivered tool result; `rejected` names why its packets cannot earn coverage, if they cannot. */
type Result = { text: string; ref: string; failed: boolean; rejected?: string };
const BEGIN = "PERSISTENT_REVIEW_READ_V1\n";
const END = "\nPERSISTENT_REVIEW_END_V1";
const MAX_PACKET = 24000;
/**
 * Codex persists a tool result in full but sends the model only about 12,000 tokens of it (the rollout's
 * `fallback_token_limit_override`), and when it cuts, the persisted text carries no marker. In local
 * 0.15x-0.163 rollouts, results of 48k-222k characters each added only 11-14k input tokens, while every
 * packet-bearing result stayed under the tools' default cap of about 40k characters. Larger results may have
 * reached the model clipped, so their packets earn nothing. This is a measured bound, not a documented one.
 */
const CODEX_MAX_DELIVERED_BYTES = 40000;
/** Keep the skipped-kind reports compact even when a transcript invents many record kinds. */
const MAX_IGNORED_KINDS = 64;
/**
 * Skipped record kinds checked to carry no delivery, from Codex 0.153.4-0.163.0 rollouts and a Claude Code
 * 2.1.308 verbose stream. This list only decides which report bucket a skipped kind lands in: known kinds go
 * to `ignored`, anything else to `unrecognized`, where the orchestrator judges whether a read may have been
 * missed. It never decides whether the audit runs, so leaving a kind off it costs a judgment call, not an abort.
 */
// deno-fmt-ignore
const KNOWN_SKIPPED = new Set([
  "event_msg", "world_state", "turn_context", "token_usage_record", "compacted", "inter_agent_communication_metadata",
  "response_item:message", "response_item:reasoning", "response_item:agent_message", "response_item:compaction",
  "response_item:web_search_call",
  "system:thinking_tokens", "system:hook_started", "system:hook_progress", "system:hook_response",
  "system:task_started", "system:task_progress", "system:task_updated", "system:task_notification",
  "system:background_tasks_changed", "system:compact_boundary",
  "assistant:text", "assistant:thinking", "assistant:redacted_thinking", "user:text", "user:string",
  "result_block:input_image", "result_block:image", "result_block:tool_reference",
]);

/** Expected format failures are actionable and must never turn into a successful empty audit. */
class EvidenceError extends Error {}
function requireThat(test: unknown, message: string): asserts test {
  if (!test) throw new EvidenceError(message);
}
// Decode values whose shape the helper owns (manifests, packets, findings) through these checks rather than
// casting nested fields; drift there is a bug, not a harness change. Transcript records use `isObject` instead.
function object(value: unknown): ObjectValue {
  requireThat(value !== null && typeof value === "object" && !Array.isArray(value), "Expected a JSON object");
  return value as ObjectValue;
}
function string(value: unknown): string {
  requireThat(typeof value === "string", "Expected a string");
  return value;
}
function array(value: unknown): unknown[] {
  requireThat(Array.isArray(value), "Expected an array");
  return value;
}
function parse(text: string): ObjectValue {
  try {
    return object(JSON.parse(text));
  } catch (error) {
    if (error instanceof SyntaxError) throw new EvidenceError("Malformed JSON, possibly an unfinished log write");
    throw error;
  }
}
function isObject(value: unknown): value is ObjectValue {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}
function integer(value: unknown): number {
  requireThat(Number.isSafeInteger(value), "Expected a safe integer");
  return value as number;
}

/** Preserve line endings: the comparison concerns delivered text, not a normalized approximation. */
async function source(path: string): Promise<Source> {
  requireThat(path.startsWith("/"), "Source paths must be absolute snapshot paths");
  const bytes = await Deno.readFile(path);
  const text = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes);
  requireThat(!text.includes("\0"), "Binary sources are unsupported");
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  const sha256 = Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, "0")).join("");
  const lines = text.match(/[^\n]*\n|[^\n]+$/g) ?? [];
  return { path, sha256, lines, ranges: lines.length ? [[1, lines.length]] : [] };
}

/** Load pinned source versions before reading logs; changed snapshots invalidate the whole audit. */
async function manifest(path: string): Promise<Source[]> {
  const data = parse(await Deno.readTextFile(path));
  requireThat(data.version === 1, "Unsupported manifest version");
  const sources: Source[] = [];
  for (const value of array(data.sources)) {
    const entry = object(value);
    const item = await source(string(entry.path));
    requireThat(!sources.some((s) => s.path === item.path), "Duplicate source path");
    requireThat(item.sha256 === string(entry.sha256).toLowerCase(), `Snapshot changed: ${item.path}`);
    item.ranges = array(entry.ranges).map((value) => {
      const pair = array(value);
      const start = integer(pair[0]), end = integer(pair[1]);
      requireThat(pair.length === 2 && start >= 1 && end >= start && end <= item.lines.length, "Invalid source range");
      return [start, end];
    });
    sources.push(item);
  }
  requireThat(sources.length, "Empty manifest");
  return sources;
}

/** Make small, exact packets whose contents can later be checked without displaying them again. */
function packet(item: Source, start: number, end: number): string {
  requireThat(
    start >= 1 && end >= start && end <= item.lines.length && end - start < 200,
    "Read 1..200 existing lines",
  );
  const data = { path: item.path, sha256: item.sha256, start, end, text: item.lines.slice(start - 1, end).join("") };
  const output = BEGIN + JSON.stringify(data) + END;
  requireThat(new TextEncoder().encode(output).length <= MAX_PACKET, "Read exceeds 24000 bytes; request fewer lines");
  return output;
}

/**
 * Keep physical references while skipping whitespace separators, which carry no delivery semantics.
 * A line that is not a JSON object fails, since it may be an unfinished write; required logs need a real record.
 */
async function* records(
  paths: string[],
  allowEmpty = false,
): AsyncGenerator<{ row: ObjectValue; ref: string; path: string }> {
  for (const path of paths) {
    let line = 0;
    let count = 0;
    const input = createReadStream(path, { encoding: "utf8" });
    const reader = createInterface({ input, crlfDelay: Infinity });
    try {
      for await (const text of reader) {
        line++;
        // Spacing is not an evidence record; preserve physical line numbers for all real records.
        if (!text.trim()) continue;
        count++;
        yield { row: parse(text), ref: `${path}:${line}`, path };
      }
    } finally {
      reader.close();
      input.destroy();
    }
    requireThat(count || allowEmpty, `Empty log: ${path}`);
  }
}

/**
 * Decode the text a tool result put in the model's context: a plain string, or the `text` of each block in an
 * array. Anything else (images, ToolSearch references, a renamed text block, an object-shaped result) contributes
 * nothing and is counted through `skip`, so a format change shows up in the report instead of aborting it.
 */
function resultText(value: unknown, skip: (kind: string) => void): string {
  if (typeof value === "string") return value;
  if (!Array.isArray(value)) {
    skip(`result_content:${value === null ? "null" : typeof value}`);
    return "";
  }
  return value.map((block) => {
    if (isObject(block) && typeof block.text === "string") return block.text;
    skip(`result_block:${isObject(block) && typeof block.type === "string" ? block.type : "?"}`);
    return "";
  }).join("");
}

/** Check harness wrappers only; words in the quoted source are not execution status. */
function codexFailed(text: string): boolean {
  const wrapper = text.split(BEGIN, 1)[0];
  return /(?:^|\n)(?:Script failed\b|Process exited with code [1-9]\d*\b|exit_code=[1-9]\d*\b)/.test(wrapper);
}

/**
 * Yield every tool result delivered to the audited reviewer, from the checked Codex rollout or Claude verbose
 * stream, never raw Codex command events.
 *
 * The audit aborts only when continuing could misattribute identity: a log from another session or cwd, a log
 * with no initialization, or messages before their log's init. Everything else degrades to "no credit". Records
 * that are not tool calls or results (lifecycle events, messages, reasoning, compaction, hooks, task notices,
 * subagent traffic, and whatever a harness adds next) are skipped and counted, in `ignored` when the kind is
 * known to carry no delivery and in `unrecognized` otherwise. Tool results that
 * cannot be trusted (no matching call, client-authored, possibly clipped, an unclear error flag) are yielded
 * with a `rejected` reason. Calls that never got a result are counted in `unfinished`. A format change therefore
 * shows up as missing coverage plus an `unrecognized` kind, never as false coverage and never as an abort.
 *
 * Do not reintroduce allowlists of record, tool, family or block types. Each one broke valid audits as soon as
 * a harness added something (Codex function calls, agent messages, string and image outputs, late exec results;
 * Claude hooks, synthetic user text, subagent records, ToolSearch references, repeated inits), and rejecting
 * those bought no protection: they can only lower coverage, never raise it.
 */
async function* delivered(
  backend: string,
  session: string,
  cwd: string,
  paths: string[],
  counts: { ignored: Map<string, number>; unrecognized: Map<string, number>; unfinished: number },
): AsyncGenerator<Result> {
  requireThat(["codex", "claude"].includes(backend), "Supported backends: codex, claude");
  const skip = (raw: string) => {
    // Subagent records and Claude turn results are skipped whatever their inner type.
    const known = KNOWN_SKIPPED.has(raw) || raw.startsWith("nested:") || raw.startsWith("result:");
    const bucket = known ? counts.ignored : counts.unrecognized;
    let kind = raw.slice(0, 80);
    if (!bucket.has(kind) && bucket.size >= MAX_IGNORED_KINDS) kind = "other";
    bucket.set(kind, (bucket.get(kind) ?? 0) + 1);
  };
  const initialized = new Set<string>();
  // Results pair with calls by ID across all supplied logs, so a result in a resumed log may complete a call
  // from an earlier one. The value counts results; a call may have several (see the Codex branch).
  const calls = new Map<string, number>();
  const orphan = "No matching tool call in the supplied logs";
  for await (const { row, ref, path } of records(paths)) {
    const type = typeof row.type === "string" ? row.type : "?";
    if (backend === "codex") {
      if (type === "session_meta") {
        // Native subagent rollouts are unsupported because what their model received is unverified. A spawned
        // child names its root thread in `session_id`; a forked child also carries a second meta followed by a
        // copy of its parent's history. Say so plainly rather than reporting a generic identity mismatch.
        const meta = object(row.payload);
        const subagent = "Codex native subagent rollouts cannot be audited; run the reviewer through codex exec";
        requireThat(!initialized.has(path), `${subagent} (second session_meta, as in a forked child): ${ref}`);
        requireThat(meta.session_id === undefined || meta.session_id === meta.id, `${subagent}: ${ref}`);
        requireThat(meta.id === session && meta.cwd === cwd, `Session/cwd mismatch: ${ref}`);
        initialized.add(path);
        continue;
      }
      requireThat(initialized.has(path), `Missing session_meta before records: ${ref}`);
      // Top-level `compacted` lands here too: its summary and replacement history are context management,
      // so original call/result evidence before it stays valid and nothing in it repairs missing evidence.
      if (type !== "response_item") {
        skip(type);
        continue;
      }
      const payload = isObject(row.payload) ? row.payload : {};
      const kind = typeof payload.type === "string" ? payload.type : "?";
      // A call_id is what makes a response item a tool call or result, whatever the tool family is called.
      // Items without one (messages, reasoning, agent messages, server-side searches) never hold a local
      // tool's output; server-side items may also carry a null call_id.
      if (typeof payload.call_id !== "string") {
        skip(`response_item:${kind}`);
        continue;
      }
      const id = payload.call_id;
      if (!kind.endsWith("_output")) {
        calls.set(id, calls.get(id) ?? 0);
        continue;
      }
      // Families are not matched by name: nothing guarantees that a result type mirrors its call's name, and the
      // call_id already ties the result to a call in this session.
      // One call may have several results: a code-mode `exec` cell that outlives its yield returns "Script
      // running" first and delivers its later output under the same call_id.
      const seen = calls.get(id);
      if (seen !== undefined) calls.set(id, seen + 1);
      const text = resultText(payload.output, skip);
      const rejected = seen === undefined
        ? orphan
        : isObject(row.metadata) && row.metadata.client_authored === true
        ? "Client-authored output"
        : new TextEncoder().encode(text).length > CODEX_MAX_DELIVERED_BYTES
        ? "Result is larger than Codex reliably delivers to the model; reread a smaller range"
        : undefined;
      yield { text, ref, failed: codexFailed(text), rejected };
      continue;
    }

    // Claude: every record that names a session must name this one, so mixed streams still fail even
    // through records that are otherwise skipped.
    requireThat(row.session_id === undefined || row.session_id === session, `Session mismatch: ${ref}`);
    // Subagent traffic was delivered to the child, not the reviewer. The child's answer reaches the reviewer
    // as an ordinary tool result on the parent's own Agent/Task call, which is audited normally.
    if (row.parent_tool_use_id !== undefined && row.parent_tool_use_id !== null) {
      skip(`nested:${type}`);
      continue;
    }
    if (type === "system" && row.subtype === "init") {
      // A log may hold several inits: the process starts another turn after a background task finishes.
      requireThat(row.session_id === session && row.cwd === cwd, `Session/cwd mismatch: ${ref}`);
      initialized.add(path);
      continue;
    }
    if (type !== "assistant" && type !== "user") {
      skip(typeof row.subtype === "string" ? `${type}:${row.subtype}` : type);
      continue;
    }
    // Only messages can carry calls and results, so only they need a cwd-checked init earlier in their own log.
    requireThat(row.session_id === session, `Session mismatch: ${ref}`);
    requireThat(initialized.has(path), `Missing Claude init: ${ref}`);
    const content = isObject(row.message) ? row.message.content : undefined;
    // String content is plain prose (a prompt or injected note), not a tool result.
    if (!Array.isArray(content)) {
      skip(`${type}:${typeof content}`);
      continue;
    }
    for (const block of content) {
      const kind = isObject(block) && typeof block.type === "string" ? block.type : "?";
      if (type === "assistant" && kind === "tool_use" && typeof block.id === "string") {
        calls.set(block.id, calls.get(block.id) ?? 0);
      } else if (type === "user" && kind === "tool_result" && typeof block.tool_use_id === "string") {
        const seen = calls.get(block.tool_use_id);
        if (seen !== undefined) calls.set(block.tool_use_id, seen + 1);
        // An error flag the helper cannot read might mean failure, so it is treated as one.
        const failed = block.is_error !== undefined && block.is_error !== false;
        // Native Read is not credited by this narrow tool. Only helper packets are compared.
        yield { text: resultText(block.content, skip), ref, failed, rejected: seen === undefined ? orphan : undefined };
      } else {
        // Text, thinking, synthetic user notes, server-side tool blocks and future block types.
        skip(`${type}:${kind}`);
      }
    }
  }
  requireThat(initialized.size === paths.length, "Missing session initialization");
  // A call without a result is usually a turn still running; the audit cannot credit what has not arrived,
  // so it reports the count and lets the caller re-audit after the turn boundary.
  counts.unfinished = [...calls.values()].filter((n) => n === 0).length;
}

/** Merge overlaps before subtracting coverage so repeated reads cannot inflate progress. */
function merge(ranges: Range[]): Range[] {
  const out: Range[] = [];
  for (const [start, end] of [...ranges].sort((a, b) => a[0] - b[0])) {
    const last = out.at(-1);
    if (last && start <= last[1] + 1) last[1] = Math.max(last[1], end);
    else out.push([start, end]);
  }
  return out;
}
function missing(required: Range[], covered: Range[]): Range[] {
  const out: Range[] = [];
  for (const [start, end] of merge(required)) {
    let cursor = start;
    for (const [a, b] of merge(covered)) {
      if (a > cursor) out.push([cursor, Math.min(a - 1, end)]);
      cursor = Math.max(cursor, b + 1);
      if (cursor > end) break;
    }
    if (cursor <= end) out.push([cursor, end]);
  }
  return out;
}

/** Output contains references and ranges only: source text stays in the snapshot and transcript. */
async function audit(backend: string, session: string, cwd: string, sources: Source[], paths: string[]) {
  requireThat(paths.length && new Set(paths).size === paths.length, "Supply unique complete log paths");
  const coverage = new Map(sources.map((s) => [s.path, [] as Range[]]));
  const evidence: { path: string; range: Range; ref: string }[] = [];
  const rejected: { ref: string; reason: string }[] = [];
  const outside_manifest: { path: string; reported_range: Range; ref: string }[] = [];
  const counts = { ignored: new Map<string, number>(), unrecognized: new Map<string, number>(), unfinished: 0 };
  let results = 0;
  for await (const result of delivered(backend, session, cwd, paths, counts)) {
    results++;
    // Only a rejected result that holds a packet loses anything; listing the rest would bury the ones that matter.
    if (result.rejected) {
      if (result.text.includes(BEGIN)) rejected.push({ ref: result.ref, reason: result.rejected });
      continue;
    }
    if (/(?:^|\n)Warning: truncated output/.test(result.text.split(BEGIN, 1)[0])) {
      rejected.push({ ref: result.ref, reason: "Harness clipped the result; reread a smaller range" });
      continue;
    }
    let rest = result.text;
    while (rest.includes(BEGIN)) {
      rest = rest.slice(rest.indexOf(BEGIN) + BEGIN.length);
      const stop = rest.indexOf(END);
      if (stop < 0) {
        rejected.push({ ref: result.ref, reason: "Clipped read packet; reread a smaller range" });
        break;
      }
      const raw = rest.slice(0, stop);
      const value = parse(raw);
      rest = rest.slice(stop + END.length);
      const path = string(value.path);
      const start = integer(value.start), end = integer(value.end);
      // Validate the fields that identify delivered text. Object ordering, additive metadata and the
      // producer's read-size defaults do not change evidence; only the read command enforces those budgets.
      // Exploratory context stays unverified and never adds coverage or triggers a source load.
      requireThat(
        path.startsWith("/") && /^[0-9a-f]{64}$/i.test(string(value.sha256)) && start >= 1 && end >= start,
        `Malformed read packet envelope: ${result.ref}`,
      );
      string(value.text);
      const item = sources.find((s) => s.path === path);
      if (!item) {
        outside_manifest.push({ path, reported_range: [start, end], ref: result.ref });
        continue;
      }
      requireThat(string(value.sha256).toLowerCase() === item.sha256, `Packet source version mismatch: ${result.ref}`);
      requireThat(end <= item.lines.length, `Packet range exceeds pinned source: ${result.ref}`);
      const expected = item.lines.slice(start - 1, end).join("");
      requireThat(
        value.text === expected,
        `Delivered packet does not match pinned source: ${result.ref}`,
      );
      if (result.failed) rejected.push({ ref: result.ref, reason: "Tool reported failure" });
      else {
        coverage.get(item.path)!.push([start, end]);
        evidence.push({ path: item.path, range: [start, end], ref: result.ref });
      }
    }
  }
  const report = sources.map((s) => ({
    path: s.path,
    sha256: s.sha256,
    covered: merge(coverage.get(s.path)!),
    missing: missing(s.ranges, coverage.get(s.path)!),
  }));
  return {
    version: 1,
    status: report.every((s) => !s.missing.length) ? "complete" : "incomplete",
    session,
    delivered_results: results,
    sources: report,
    evidence,
    rejected,
    outside_manifest,
    unfinished_calls: counts.unfinished,
    ignored: Object.fromEntries([...counts.ignored].sort()),
    unrecognized: Object.fromEntries([...counts.unrecognized].sort()),
  };
}

/** Read one single-writer log per reviewer; count current findings without trusting an advertised total. */
async function findings(paths: string[]) {
  const entries = new Map<string, ObjectValue>();
  for await (const { row, path, ref } of records(paths, true)) {
    validateFinding(row, ref);
    const id = string(row.id);
    entries.set(`${path}\0${id}`, { ...row, reviewer_log: path });
  }
  const active = [...entries.values()].filter((entry) => entry.state === "open");
  return {
    active: active.length,
    withdrawn: entries.size - active.length,
    findings: active.map(({ id, summary, location, reviewer_log }) => ({ id, summary, location, reviewer_log })),
  };
}

/** Validate before appending so a malformed new event cannot wedge subsequent progress polls. */
function validateFinding(row: ObjectValue, ref: string) {
  requireThat(row.version === 1 && row.kind === "finding", `Unsupported finding event: ${ref}`);
  requireThat(
    string(row.id).length && ["open", "withdrawn"].includes(string(row.state)),
    `Invalid finding state: ${ref}`,
  );
  for (const field of ["summary", "detail", "location"]) {
    requireThat(string(row[field]).trim(), `Empty ${field}: ${ref}`);
  }
}

/** CLI errors remain nonzero and never print a plausible partial coverage report to stdout. */
async function main(args: string[]) {
  const [command, ...rest] = args;
  if (command === "manifest") {
    const [output, ...paths] = rest;
    requireThat(output && paths.length && new Set(paths).size === paths.length, "manifest OUTPUT SNAPSHOT...");
    const sources = await Promise.all(paths.map(source));
    await Deno.writeTextFile(
      output,
      JSON.stringify({ version: 1, sources: sources.map(({ lines: _lines, ...s }) => s) }, null, 2) + "\n",
      { createNew: true, mode: 0o600 },
    );
    return;
  }
  if (command === "read") {
    requireThat(rest.length === 4, "read MANIFEST ABS_SOURCE START END");
    const item = (await manifest(rest[0])).find((s) => s.path === rest[1]);
    requireThat(item, "Source is outside manifest");
    console.log(packet(item, integer(Number(rest[2])), integer(Number(rest[3]))));
    return;
  }
  if (command === "audit") {
    const [backend, session, cwd, path, ...logs] = rest;
    requireThat(backend && session && cwd?.startsWith("/") && path, "audit BACKEND SESSION ABS_CWD MANIFEST LOG...");
    console.log(JSON.stringify(await audit(backend, session, cwd, await manifest(path), logs)));
    return;
  }
  if (command === "findings") {
    requireThat(rest.length, "findings LOG...");
    console.log(JSON.stringify(await findings(rest)));
    return;
  }
  if (command === "record") {
    requireThat(rest.length === 2, "record LOG EVENT_JSON_FILE");
    const row = parse(await Deno.readTextFile(rest[1]));
    validateFinding(row, rest[1]);
    await Deno.writeTextFile(rest[0], JSON.stringify(row) + "\n", { append: true, mode: 0o600 });
    return;
  }
  throw new EvidenceError("Usage: evidence manifest|read|audit|record|findings ...; see tooling.md");
}

/** Share diagnostics and exit semantics between direct source execution and the generated compiled entrypoint. */
export async function runCli(args: string[] = Deno.args) {
  try {
    await main(args);
  } catch (error) {
    if (!(error instanceof EvidenceError)) throw error;
    console.error(
      `PERSISTENT_REVIEW_TOOL_FAILED: ${error.message}. No coverage is certified by this invocation. See tooling.md; do not silently switch to bulk transcript inspection.`,
    );
    Deno.exit(2);
  }
}

if (import.meta.main) await runCli();

// The fixtures model delivered-result shapes, not the larger raw command-event stream.
// Keeping both forms in tests protects the distinction that originally defeated coverage audits.
function equal(actual: unknown, expected: unknown) {
  requireThat(
    JSON.stringify(actual) === JSON.stringify(expected),
    `Expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
  );
}
/** Assert the diagnostic as well as rejection, because recovery depends on the failure classification. */
async function fails(work: () => unknown | Promise<unknown>, reason: string) {
  try {
    await work();
  } catch (error) {
    requireThat(
      error instanceof Error && error.message.includes(reason),
      `Expected failure containing ${reason}: ${error}`,
    );
    return;
  }
  throw new Error(`Expected failure: ${reason}`);
}

/** Each fixture owns its files; tests never modify process environment or another test's state. */
async function fixture(work: (root: string, item: Source) => Promise<void>) {
  const root = await Deno.makeTempDir();
  try {
    const path = `${root}/source.txt`;
    await Deno.writeTextFile(path, "alpha\r\nbeta\ngamma");
    await work(root, await source(path));
  } finally {
    await Deno.remove(root, { recursive: true });
  }
}

/** These envelopes retain the checked backend fields and deliberately omit unrelated assistant prose. */
function logRows(backend: string, text: string, options: { id?: string; failed?: boolean } = {}): ObjectValue[] {
  const id = options.id ?? "call-1";
  if (backend === "codex") {
    return [
      { type: "session_meta", payload: { id: "session", cwd: "/repo" } },
      { type: "event_msg", payload: { type: "command_execution", aggregated_output: text } },
      { type: "response_item", payload: { type: "custom_tool_call", call_id: id, input: "read helper" } },
      {
        type: "response_item",
        payload: { type: "custom_tool_call_output", call_id: id, output: [{ type: "input_text", text }] },
      },
    ];
  }
  return [
    { type: "system", subtype: "init", session_id: "session", cwd: "/repo" },
    {
      type: "assistant",
      session_id: "session",
      message: { content: [{ type: "tool_use", id, name: "Bash", input: {} }] },
    },
    {
      type: "user",
      session_id: "session",
      message: {
        content: [{ type: "tool_result", tool_use_id: id, content: text, is_error: options.failed ?? false }],
      },
    },
  ];
}
async function saveLog(path: string, rows: ObjectValue[]) {
  await Deno.writeTextFile(path, rows.map((row) => JSON.stringify(row)).join("\n") + "\n");
}

for (const backend of ["codex", "claude"]) {
  // Evidence depends on decoded source values, not the producer's particular JSON serialization.
  Deno.test(`${backend}: harmless packet formatting and metadata do not change verified delivery`, () =>
    fixture(async (root, item) => {
      const path = `${root}/log`;
      const value = {
        note: "extra metadata",
        text: item.lines.join(""),
        end: 3,
        start: 1,
        sha256: item.sha256.toUpperCase(),
        path: item.path,
      };
      const data = BEGIN + JSON.stringify(value, null, 2) + END;
      await saveLog(path, logRows(backend, data));
      const report = await audit(backend, "session", "/repo", [item], [path]);
      equal(report.status, "complete");
      await saveLog(path, logRows(backend, BEGIN + JSON.stringify({ ...value, text: "wrong" }) + END));
      await fails(() => audit(backend, "session", "/repo", [item], [path]), "does not match");
      await saveLog(path, logRows(backend, BEGIN + JSON.stringify({ ...value, end: 4 }) + END));
      await fails(() => audit(backend, "session", "/repo", [item], [path]), "range exceeds");
    }));
  // Production budgets prevent oversized new reads; they are not grounds for discarding existing exact delivery.
  Deno.test(`${backend}: producer read budgets do not invalidate fully delivered matching text`, () =>
    fixture(async (root, item) => {
      const path = `${root}/log`;
      for (const text of ["line\n".repeat(201), "x".repeat(MAX_PACKET + 1)]) {
        await Deno.writeTextFile(item.path, text);
        const large = await source(item.path);
        const value = { path: large.path, sha256: large.sha256, start: 1, end: large.lines.length, text };
        await saveLog(path, logRows(backend, BEGIN + JSON.stringify(value) + END));
        equal((await audit(backend, "session", "/repo", [large], [path])).status, "complete");
        await fails(
          () => packet(large, 1, large.lines.length),
          large.lines.length > 200 ? "200 existing lines" : "24000",
        );
      }
    }));
  // Tolerating spacing must neither break raw evidence references nor make an absent required transcript usable.
  Deno.test(`${backend}: blank separators preserve evidence line numbers but do not initialize empty logs`, () =>
    fixture(async (root, item) => {
      const path = `${root}/log`;
      await saveLog(path, logRows(backend, packet(item, 1, 3)));
      const data = await Deno.readTextFile(path);
      await Deno.writeTextFile(path, "\n \t\n" + data);
      const report = await audit(backend, "session", "/repo", [item], [path]);
      equal(report.status, "complete");
      equal(report.evidence[0].ref, `${path}:${backend === "codex" ? 6 : 5}`);
      await Deno.writeTextFile(path, "\n \t\n");
      await fails(() => audit(backend, "session", "/repo", [item], [path]), "Empty log");
      equal((await findings([path])).active, 0);
    }));
  Deno.test(`${backend}: valid tool-free turns leave required text missing`, () =>
    fixture(async (root, item) => {
      const path = `${root}/log`;
      await saveLog(path, logRows(backend, "unused").slice(0, 1));
      const report = await audit(backend, "session", "/repo", [item], [path]);
      equal(report.status, "incomplete");
      equal(report.delivered_results, 0);
      equal(report.sources[0].missing, [[1, 3]]);
    }));
  Deno.test(`${backend}: complete packets earn exact coverage without echoing source`, () =>
    fixture(async (root, item) => {
      const path = `${root}/log`;
      await saveLog(path, logRows(backend, "wrapper\n" + packet(item, 1, 3) + "\n"));
      const report = await audit(backend, "session", "/repo", [item], [path]);
      equal(report.status, "complete");
      equal(report.sources[0].covered, [[1, 3]]);
      requireThat(!JSON.stringify(report).includes("alpha"), "Source leaked into compact report");
      await fails(() => audit(backend, "other", "/repo", [item], [path]), "mismatch");
      await fails(() => audit(backend, "session", "/other", [item], [path]), "mismatch");
    }));
  Deno.test(`${backend}: clipping leaves gaps; raw stdout never repairs them`, () =>
    fixture(async (root, item) => {
      const path = `${root}/log`;
      const rows = logRows(backend, packet(item, 1, 3).slice(0, -20));
      if (backend === "codex") rows[1] = { type: "event_msg", payload: { aggregated_output: packet(item, 1, 3) } };
      await saveLog(path, rows);
      const report = await audit(backend, "session", "/repo", [item], [path]);
      equal(report.status, "incomplete");
      equal(report.sources[0].missing, [[1, 3]]);
      equal(report.rejected.length, 1);
    }));
  // A result with no call in the supplied logs has unknown provenance: it is listed, never credited, never fatal.
  Deno.test(`${backend}: mismatched packets and corrupt logs fail; orphan results earn nothing`, () =>
    fixture(async (root, item) => {
      const path = `${root}/log`;
      const rows = logRows(backend, packet(item, 1, 3).replace("alpha", "wrong"));
      await saveLog(path, rows);
      await fails(() => audit(backend, "session", "/repo", [item], [path]), "does not match");
      await saveLog(path, logRows(backend, packet(item, 1, 3)).filter((_, i) => i !== (backend === "codex" ? 2 : 1)));
      const orphan = await audit(backend, "session", "/repo", [item], [path]);
      equal(orphan.sources[0].missing, [[1, 3]]);
      equal(orphan.rejected.map(({ reason }) => reason), ["No matching tool call in the supplied logs"]);
      await Deno.writeTextFile(path, '{"type":');
      await fails(() => audit(backend, "session", "/repo", [item], [path]), "Malformed");
    }));
  // Harnesses keep adding lifecycle records; each new one used to abort valid audits until it was allowlisted.
  // Skipping them is safe only because a skipped record can never earn coverage, even when it quotes a packet.
  Deno.test(`${backend}: unknown non-tool records are counted but never earn or block coverage`, () =>
    fixture(async (root, item) => {
      const path = `${root}/log`;
      const future = { type: "future_record", session_id: "session", output: packet(item, 2, 3) };
      await saveLog(path, [...logRows(backend, packet(item, 1, 1)), future]);
      const report = await audit(backend, "session", "/repo", [item], [path]);
      equal(report.sources[0].missing, [[2, 3]]);
      equal([report.unrecognized.future_record, report.ignored.future_record], [1, undefined]);
      await saveLog(path, [...logRows(backend, packet(item, 1, 3)), future]);
      equal((await audit(backend, "session", "/repo", [item], [path])).status, "complete");
    }));
}

Deno.test("coverage union respects required ranges and does not count repeated reads as progress", () => {
  equal(merge([[3, 5], [1, 3], [1, 3], [8, 9]]), [[1, 5], [8, 9]]);
  equal(missing([[2, 8]], [[1, 3], [5, 6], [10, 12]]), [[4, 4], [7, 8]]);
});
Deno.test("snapshot drift, oversized reads and unsupported delivered blocks are explicit failures", () =>
  fixture(async (root, item) => {
    const file = `${root}/manifest.json`;
    await main(["manifest", file, item.path]);
    await Deno.writeTextFile(item.path, "changed");
    await fails(() => manifest(file), "Snapshot changed");
    await fails(() => packet({ ...item, lines: ["x".repeat(MAX_PACKET)] }, 1, 1), "24000");
    const kinds: string[] = [];
    const text = resultText([{ type: "input_image" }, { type: "input_text", text: "kept" }, 5], (k) => kinds.push(k));
    equal([text, resultText({ text: "object" }, (k) => kinds.push(k))], ["kept", ""]);
    equal(kinds, ["result_block:input_image", "result_block:?", "result_content:object"]);
  }));
Deno.test("Claude resume logs retain identity; a failed tool cannot earn coverage", () =>
  fixture(async (root, item) => {
    const first = `${root}/first`, second = `${root}/second`;
    await saveLog(first, logRows("claude", packet(item, 1, 1), { id: "first" }));
    await saveLog(second, logRows("claude", packet(item, 2, 3), { id: "second", failed: true }));
    equal((await audit("claude", "session", "/repo", [item], [first, second])).sources[0].missing, [[2, 3]]);
    await saveLog(second, logRows("claude", packet(item, 2, 3), { id: "second" }));
    equal((await audit("claude", "session", "/repo", [item], [first, second])).status, "complete");
  }));

// Hook output is lifecycle metadata even when it contains a syntactically valid read packet.
// Exercise resumed logs separately so one turn's init cannot authorize the next turn's messages.
Deno.test("Claude startup hooks preserve delivery references across initialized resume logs", () =>
  fixture(async (root, item) => {
    const paths = [`${root}/first`, `${root}/second`];
    for (const [index, path] of paths.entries()) {
      const hooks = ["hook_started", "hook_progress", "hook_response"].map((subtype) => ({
        type: "system",
        subtype,
        session_id: "session",
        output: packet(item, 1, 3),
      }));
      const rows = logRows("claude", packet(item, index + 1, index + 1), { id: `call-${index}` });
      await saveLog(path, [...hooks, ...rows, ...hooks]);
    }
    const report = await audit("claude", "session", "/repo", [item], paths);
    equal(report.status, "incomplete");
    equal(report.sources[0].covered, [[1, 2]]);
    equal(report.sources[0].missing, [[3, 3]]);
    equal(report.delivered_results, 2);
    equal(report.evidence.map(({ ref }) => ref), paths.map((path) => `${path}:6`));
    equal((await audit("claude", "session", "/repo", [{ ...item, ranges: [[1, 2]] }], paths)).status, "complete");
  }));

// Skipping non-message records must not weaken session ownership or the per-log initialization gate:
// a skipped record from another session still fails, and messages still need a cwd-checked init in their log.
Deno.test("Claude non-message records cannot bypass identity or initialization checks", () =>
  fixture(async (root, item) => {
    const first = `${root}/first`, second = `${root}/second`;
    const rows = logRows("claude", packet(item, 1, 3));
    for (const subtype of ["hook_started", "hook_progress", "hook_response", "unknown_startup"]) {
      const hook = { type: "system", subtype, session_id: "session" };
      await saveLog(first, [{ ...hook, session_id: "other" }, ...rows]);
      await fails(() => audit("claude", "session", "/repo", [item], [first]), "Session mismatch");
      await saveLog(first, [hook]);
      await fails(() => audit("claude", "session", "/repo", [item], [first]), "Missing session initialization");
      await saveLog(first, [hook, { ...rows[0], cwd: "/other" }]);
      await fails(() => audit("claude", "session", "/repo", [item], [first]), "Session/cwd mismatch");
      await saveLog(first, [hook, rows[0], { ...rows[0], cwd: "/other" }]);
      await fails(() => audit("claude", "session", "/repo", [item], [first]), "Session/cwd mismatch");
      await saveLog(first, [hook, { ...rows[0], session_id: undefined }]);
      await fails(() => audit("claude", "session", "/repo", [item], [first]), "Session/cwd mismatch");
      for (const row of [rows[1], rows[2]]) {
        await saveLog(first, [hook, row, ...rows]);
        await fails(() => audit("claude", "session", "/repo", [item], [first]), "Missing Claude init");
      }
      await saveLog(first, [hook, { type: "result", session_id: "session" }, ...rows]);
      equal((await audit("claude", "session", "/repo", [item], [first])).status, "complete");
      await saveLog(first, rows);
      await saveLog(second, [hook, ...rows.slice(1)]);
      await fails(() => audit("claude", "session", "/repo", [item], [first, second]), "Missing Claude init");
    }
  }));

// Shapes captured from one ordinary Claude Code 2.1.308 `-p --verbose` run that used Bash, Agent and ToolSearch.
// Each of them used to abort the audit. None of them is the reviewer's own tool result, so none may earn coverage.
Deno.test("Claude subagent, synthetic and re-init records coexist with verified delivery", () =>
  fixture(async (root, item) => {
    const path = `${root}/log`;
    const [init, call, result] = logRows("claude", packet(item, 1, 1));
    const message = (type: string, content: unknown, extra: ObjectValue = {}) => ({
      type,
      session_id: "session",
      message: { content },
      ...extra,
    });
    const nested = { parent_tool_use_id: "agent-call", agent_id: "child" };
    await saveLog(path, [
      init,
      { type: "system", subtype: "thinking_tokens", session_id: "session" },
      call,
      result,
      message("user", [{ type: "text", text: packet(item, 2, 3) }], { isSynthetic: true }),
      message("user", packet(item, 2, 3)),
      message("assistant", [{ type: "tool_use", id: "agent-call", name: "Agent", input: {} }]),
      { type: "system", subtype: "task_started", session_id: "session", tool_use_id: "agent-call" },
      // The child's own calls and results reuse nothing from the parent and quote source the parent never saw.
      message("assistant", [{ type: "tool_use", id: "child-call", name: "Bash", input: {} }], nested),
      message("user", [{ type: "tool_result", tool_use_id: "child-call", content: packet(item, 2, 3) }], nested),
      message("user", [{ type: "tool_result", tool_use_id: "agent-call", content: [{ type: "text", text: "ok" }] }]),
      message("assistant", [{ type: "tool_use", id: "search", name: "ToolSearch", input: {} }]),
      message("user", [{ type: "tool_result", tool_use_id: "search", content: [{ type: "tool_reference" }] }]),
      { type: "result", subtype: "success", session_id: "session" },
      // A finished background task starts another turn in the same process, which re-emits init.
      init,
      message("assistant", [{ type: "text", text: "done" }]),
    ]);
    const report = await audit("claude", "session", "/repo", [item], [path]);
    equal(report.sources[0].missing, [[2, 3]]);
    equal(report.delivered_results, 3);
    equal(report.ignored["nested:user"], 1);
    equal(report.ignored["user:text"], 1);
    equal(report.ignored["user:string"], 1);
    equal(report.unrecognized, {});
    await saveLog(path, [
      ...logRows("claude", packet(item, 1, 3)),
      message("user", [{ type: "tool_result", tool_use_id: "child-call", content: "x" }], { parent_tool_use_id: 7 }),
    ]);
    equal((await audit("claude", "session", "/repo", [item], [path])).status, "complete");
    await saveLog(path, [...logRows("claude", packet(item, 1, 3)), { ...result, session_id: "other" }]);
    await fails(() => audit("claude", "session", "/repo", [item], [path]), "Session mismatch");
    // A repeated result and an unreadable error flag degrade to evidence and a rejection, not an abort.
    await saveLog(path, [...logRows("claude", packet(item, 1, 3)), result]);
    equal((await audit("claude", "session", "/repo", [item], [path])).status, "complete");
    await saveLog(
      path,
      logRows("claude", packet(item, 1, 3)).map((row) =>
        JSON.parse(
          JSON.stringify(row).replace('"is_error":false', '"is_error":"maybe"'),
        )
      ),
    );
    const unclear = await audit("claude", "session", "/repo", [item], [path]);
    equal([unclear.status, unclear.rejected[0].reason], ["incomplete", "Tool reported failure"]);
  }));

Deno.test("Codex middle clipping and process failure reject packets rather than certifying coverage", () =>
  fixture(async (root, item) => {
    const path = `${root}/log`;
    const full = packet(item, 1, 3);
    const clipped = full.slice(0, 80) + "…200 tokens truncated…" + full.slice(-40);
    await saveLog(path, logRows("codex", "Warning: truncated output\n" + clipped));
    const report = await audit("codex", "session", "/repo", [item], [path]);
    equal(report.status, "incomplete");
    equal(report.rejected.length, 1);
    await saveLog(path, logRows("codex", "Process exited with code 1\nOutput:\n" + full));
    equal((await audit("codex", "session", "/repo", [item], [path])).status, "incomplete");
    equal(codexFailed(full + "\nProcess exited with code 1"), false);
  }));

Deno.test("source text cannot disguise malformed packets as harness clipping", () =>
  fixture(async (root, item) => {
    await Deno.writeTextFile(item.path, "100 tokens truncated\nWarning: truncated output\n");
    item = await source(item.path);
    const path = `${root}/log`;
    await saveLog(path, logRows("codex", packet(item, 1, 2) + BEGIN + "invalid json" + END));
    await fails(() => audit("codex", "session", "/repo", [item], [path]), "Malformed");
    await saveLog(path, logRows("codex", packet(item, 1, 2)));
    equal((await audit("codex", "session", "/repo", [item], [path])).status, "complete");
  }));

Deno.test("Codex top-level compaction preserves original delivery but cannot supply missing evidence", () =>
  fixture(async (root, item) => {
    const path = `${root}/log`;
    const compaction = {
      type: "compacted",
      payload: {
        message: "",
        replacement_history: [{ type: "message", content: packet(item, 2, 3) }, {
          type: "compaction",
          encrypted_content: "opaque",
        }],
        replacement_history_metadata: [{}, {}],
        retained_context: {},
        window_number: 1,
        window_id: "window",
        previous_window_id: "previous",
      },
    };
    const rows = logRows("codex", packet(item, 1, 1));
    // A compaction between a call and its result must not orphan or fabricate a delivery.
    rows.splice(3, 0, compaction);
    await saveLog(path, rows);
    const partial = await audit("codex", "session", "/repo", [item], [path]);
    equal(partial.sources[0].covered, [[1, 1]]);
    equal(partial.sources[0].missing, [[2, 3]]);
    await saveLog(path, [rows[0], {
      ...compaction,
      payload: { message: packet(item, 1, 3), replacement_history: null },
    }]);
    equal((await audit("codex", "session", "/repo", [item], [path])).sources[0].missing, [[1, 3]]);
    await saveLog(path, [...logRows("codex", packet(item, 1, 3)), compaction]);
    equal((await audit("codex", "session", "/repo", [item], [path])).status, "complete");
    // Compaction never earns coverage, so a changed summary shape has nothing to verify and cannot block the audit.
    await saveLog(path, [...logRows("codex", packet(item, 1, 3)), {
      ...compaction,
      payload: { message: 42, replacement_history: "changed shape" },
    }]);
    const changed = await audit("codex", "session", "/repo", [item], [path]);
    equal(changed.status, "complete");
    equal(changed.ignored.compacted, 1);
  }));

/** Codex rows for one tool call/result pair in an arbitrary family, as persisted in 0.163.0 rollouts. */
function codexPair(call: string, result: string, id: string, output: unknown, extra: ObjectValue = {}): ObjectValue[] {
  return [
    { type: "response_item", payload: { type: call, call_id: id, name: "tool", arguments: "{}", ...extra } },
    { type: "response_item", payload: { type: result, call_id: id, output } },
  ];
}

// Reproduces https://snippets.scode.org/s/persistent-review-codex-function-call/: a `collaboration.list_agents`
// function call aborted an otherwise valid audit. Real rollouts also deliver helper reads through function-family
// tools (`exec_command`, code-mode `wait`), so any call_id-paired result is credited, whatever its family.
Deno.test("Codex results of any tool family earn coverage, and odd shapes degrade instead of aborting", () =>
  fixture(async (root, item) => {
    const path = `${root}/log`;
    const baseline = logRows("codex", packet(item, 1, 1));
    const listAgents = codexPair("function_call", "function_call_output", "agents", '{"agents":[]}', {
      namespace: "collaboration",
      name: "list_agents",
    });
    await saveLog(path, baseline);
    const before = await audit("codex", "session", "/repo", [item], [path]);
    await saveLog(path, [...baseline, ...listAgents]);
    const after = await audit("codex", "session", "/repo", [item], [path]);
    equal([after.sources, after.evidence], [before.sources, before.evidence]);
    await saveLog(path, [baseline[0], ...listAgents]);
    equal((await audit("codex", "session", "/repo", [item], [path])).sources[0].missing, [[1, 3]]);

    // A string output and a block array with an image both deliver their text.
    const exec = codexPair("function_call", "function_call_output", "exec", "Output:\n" + packet(item, 2, 2));
    const image = codexPair("function_call", "function_call_output", "image", [
      { type: "input_image", image_url: "data:" },
      { type: "input_text", text: packet(item, 3, 3) },
    ]);
    await saveLog(path, [...baseline, ...exec, ...image]);
    equal((await audit("codex", "session", "/repo", [item], [path])).status, "complete");

    const [call, result] = exec;
    const custom = baseline[2];
    const output = (value: unknown) => ({ ...result, payload: { ...object(result.payload), output: value } });
    const audited = async (rows: ObjectValue[]) => {
      await saveLog(path, [baseline[0], ...rows]);
      return await audit("codex", "session", "/repo", [item], [path]);
    };
    // Families need not match by name, server-side items may carry a null call_id, and a repeated call ID or a
    // call still waiting for its result is reported rather than fatal.
    const search = codexPair("tool_search_call", "tool_search_output", "search", packet(item, 2, 3));
    equal(
      (await audited([custom, { ...result, payload: { ...object(result.payload), call_id: "call-1" } }])).rejected,
      [],
    );
    equal((await audited([...search, call, call, result])).sources[0].missing, [[1, 1]]);
    const pending = await audited([
      { type: "response_item", payload: { type: "web_search_call", call_id: null } },
      call,
    ]);
    equal([pending.unfinished_calls, pending.ignored["response_item:web_search_call"]], [1, 1]);
    // Results whose provenance or delivery is doubtful earn nothing and say why.
    for (
      const [rows, reason] of [
        [[result], "No matching tool call"],
        [[call, { ...result, metadata: { client_authored: true } }], "Client-authored"],
        [[call, output(packet(item, 2, 2) + " ".repeat(CODEX_MAX_DELIVERED_BYTES))], "larger than Codex"],
      ] as const
    ) {
      const report = await audited([...rows]);
      equal(report.sources[0].missing, [[1, 3]]);
      requireThat(report.rejected[0].reason.includes(reason), `Expected ${reason}: ${JSON.stringify(report.rejected)}`);
    }
    const odd = await audited([call, output({ text: packet(item, 1, 3) }), call, output([{ type: "x", t: "y" }])]);
    equal([odd.sources[0].missing, odd.unrecognized["result_content:object"], odd.unrecognized["result_block:x"]], [
      [[1, 3]],
      1,
      1,
    ]);
  }));

// Code-mode `exec` cells that outlive their yield return "Script running" and deliver later output as a second
// result under the same call_id. About 1,400 such results exist in local 0.15x-0.163 rollouts.
Deno.test("Codex late results for a yielded exec cell are delivered evidence, not duplicates", () =>
  fixture(async (root, item) => {
    const path = `${root}/log`;
    const [meta, , call, first] = logRows("codex", "Script running with cell ID 1\nOutput:\n");
    const late = {
      type: "response_item",
      payload: { type: "custom_tool_call_output", call_id: "call-1", name: "exec", output: packet(item, 1, 3) },
    };
    await saveLog(path, [meta, call, first, late]);
    const report = await audit("codex", "session", "/repo", [item], [path]);
    equal(report.status, "complete");
    equal(report.evidence[0].ref, `${path}:4`);
    await saveLog(path, [meta, late]);
    equal((await audit("codex", "session", "/repo", [item], [path])).rejected.length, 1);
  }));

// Agent messages, inter-agent metadata and server-side web searches appear in ordinary 0.163.0 rollouts.
// They carry no call_id-paired local tool output and must neither abort nor credit an audit.
Deno.test("Codex non-tool response items and lifecycle records are skipped and counted", () =>
  fixture(async (root, item) => {
    const path = `${root}/log`;
    const rows = logRows("codex", packet(item, 1, 3));
    await saveLog(path, [
      ...rows,
      { type: "inter_agent_communication_metadata", payload: {} },
      { type: "response_item", payload: { type: "agent_message", content: packet(item, 1, 3) } },
      { type: "response_item", payload: { type: "web_search_call", action: {}, status: "completed" } },
    ]);
    const report = await audit("codex", "session", "/repo", [item], [path]);
    equal(report.status, "complete");
    equal(report.delivered_results, 1);
    equal(Object.keys(report.ignored), [
      "event_msg",
      "inter_agent_communication_metadata",
      "response_item:agent_message",
      "response_item:web_search_call",
    ]);
    // Subagent rollouts fail with a diagnostic that names the cause instead of a generic identity mismatch.
    await saveLog(path, [rows[0], { type: "event_msg", payload: {} }, rows[0]]);
    await fails(() => audit("codex", "session", "/repo", [item], [path]), "second session_meta");
    await saveLog(path, [{ type: "session_meta", payload: { id: "session", session_id: "root", cwd: "/repo" } }]);
    await fails(() => audit("codex", "session", "/repo", [item], [path]), "native subagent rollouts cannot be audited");
  }));

for (const backend of ["codex", "claude"]) {
  Deno.test(`${backend}: context packets outside the manifest are visible but cannot supply coverage`, () =>
    fixture(async (root, item) => {
      const other = { ...item, path: `${root}/not-in-manifest.txt` };
      const path = `${root}/log`;
      await saveLog(path, logRows(backend, packet(other, 1, 3) + packet(item, 1, 1)));
      const report = await audit(backend, "session", "/repo", [item], [path]);
      equal(report.status, "incomplete");
      equal(report.sources[0].covered, [[1, 1]]);
      equal(report.sources[0].missing, [[2, 3]]);
      equal(report.outside_manifest, [{
        path: other.path,
        reported_range: [1, 3],
        ref: `${path}:${backend === "codex" ? 4 : 3}`,
      }]);
      await saveLog(path, logRows(backend, packet(item, 1, 3) + packet(other, 1, 3)));
      equal((await audit(backend, "session", "/repo", [item], [path])).status, "complete");
      await saveLog(path, logRows(backend, packet(other, 1, 3).replace('"start":1', '"start":0')));
      await fails(() => audit(backend, "session", "/repo", [item], [path]), "Malformed read packet envelope");
      const padded = packet(other, 1, 3).replace(BEGIN, BEGIN + " ".repeat(MAX_PACKET));
      await saveLog(path, logRows(backend, packet(item, 1, 3) + padded));
      const paddedReport = await audit(backend, "session", "/repo", [item], [path]);
      equal(paddedReport.status, "complete");
      equal(paddedReport.outside_manifest.length, 1);
    }));
}
Deno.test("findings count revisions once, preserve owners, and accept initialized empty logs", () =>
  fixture(async (root) => {
    const first = `${root}/first`, second = `${root}/second`;
    await Deno.writeTextFile(first, "");
    equal((await findings([first])).active, 0);
    const entry = {
      version: 1,
      kind: "finding",
      id: "f1",
      state: "open",
      summary: "bug",
      location: "a:1",
      detail: "evidence",
    };
    await saveLog(first, [entry, { ...entry, state: "withdrawn" }]);
    await saveLog(second, [entry]);
    const report = await findings([first, second]);
    equal([report.active, report.withdrawn], [1, 1]);
    equal(report.findings[0].reviewer_log, second);
    await Deno.writeTextFile(first, "{", { append: true });
    await fails(() => findings([first]), "Malformed");
  }));

Deno.test("record appends immediately without overwriting an earlier finding", () =>
  fixture(async (root) => {
    const log = `${root}/findings`, event = `${root}/event`;
    const row = {
      version: 1,
      kind: "finding",
      id: "f1",
      state: "open",
      summary: "bug",
      location: "a:1",
      detail: "evidence",
    };
    await Deno.writeTextFile(event, JSON.stringify(row));
    await main(["record", log, event]);
    equal((await findings([log])).active, 1);
    await Deno.writeTextFile(event, JSON.stringify({ ...row, state: "withdrawn" }));
    await main(["record", log, event]);
    equal((await findings([log])).withdrawn, 1);
    equal((await Deno.readTextFile(log)).trim().split("\n").length, 2);
  }));

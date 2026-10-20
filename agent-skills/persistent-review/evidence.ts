#!/usr/bin/env -S deno run -A
/**
 * Keep delivered source out of the orchestrator's context without trusting read receipts.
 * Only complete, source-matched packets in supported delivered-result records earn coverage.
 * Unknown representations fail closed; raw logs remain the evidence, not this report's counts.
 */
import { createReadStream } from "node:fs";
import { createInterface } from "node:readline";

type ObjectValue = Record<string, unknown>;
type Range = [number, number];
type Source = { path: string; sha256: string; ranges: Range[]; lines: string[] };
type Result = { text: string; ref: string; failed: boolean };
const BEGIN = "PERSISTENT_REVIEW_READ_V1\n";
const END = "\nPERSISTENT_REVIEW_END_V1";
const MAX_PACKET = 24000;

/** Expected format failures are actionable and must never turn into a successful empty audit. */
class EvidenceError extends Error {}
function requireThat(test: unknown, message: string): asserts test {
  if (!test) throw new EvidenceError(message);
}
// Decode untrusted schema values through these checks rather than casting nested fields.
// Format drift should surface as an actionable failure, not an accidentally skipped result.
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
 * Incomplete or unknown records cannot disappear through a best-effort filter; required logs need a real record.
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

/** Validate every content block, including blocks that do not contain a read packet. */
function textBlocks(value: unknown, type: string): string {
  return array(value).map((entry) => {
    const block = object(entry);
    requireThat(block.type === type, `Unsupported delivered block: ${String(block.type)}`);
    return string(block.text);
  }).join("");
}

/** Check harness wrappers only; words in the quoted source are not execution status. */
function codexFailed(text: string): boolean {
  const wrapper = text.split(BEGIN, 1)[0];
  return /(?:^|\n)(?:Script failed\b|Process exited with code [1-9]\d*\b|exit_code=[1-9]\d*\b)/.test(wrapper);
}

/**
 * Support the checked Codex rollout and Claude verbose stream, never raw Codex command events.
 * Session/cwd joins and paired calls prevent another child or an orphan result from earning credit.
 */
async function* delivered(backend: string, session: string, cwd: string, paths: string[]): AsyncGenerator<Result> {
  requireThat(["codex", "claude"].includes(backend), "Supported backends: codex, claude");
  const calls = new Set<string>();
  const seenCalls = new Set<string>();
  const initialized = new Set<string>();
  for await (const { row, ref, path } of records(paths)) {
    const type = string(row.type);
    if (backend === "codex") {
      if (type === "session_meta") {
        const meta = object(row.payload);
        requireThat(!initialized.has(path) && meta.id === session && meta.cwd === cwd, `Session/cwd mismatch: ${ref}`);
        requireThat(meta.session_id === undefined || meta.session_id === session, `Conflicting session ID: ${ref}`);
        initialized.add(path);
      } else {
        requireThat(initialized.has(path), `Missing session_meta before records: ${ref}`);
        if (type === "compacted") {
          const context = object(row.payload);
          string(context.message);
          requireThat(
            context.replacement_history === null || Array.isArray(context.replacement_history),
            `Unsupported Codex compaction history: ${ref}`,
          );
          // This is context management, not a new delivery. Keep original call/result joins and coverage;
          // summaries and replacement history cannot repair missing original tool-result evidence.
          continue;
        }
        requireThat(
          ["event_msg", "response_item", "world_state", "turn_context", "token_usage_record"].includes(type),
          `Unsupported Codex record: ${type} at ${ref}`,
        );
        if (type !== "response_item") continue;
        const payload = object(row.payload);
        const kind = string(payload.type);
        if (["message", "reasoning", "compaction"].includes(kind)) continue;
        requireThat(
          ["custom_tool_call", "custom_tool_call_output"].includes(kind),
          `Unsupported Codex tool record: ${kind} at ${ref}`,
        );
        const id = string(payload.call_id);
        if (kind === "custom_tool_call") {
          string(payload.input);
          requireThat(!seenCalls.has(id), `Duplicate tool call: ${ref}`);
          seenCalls.add(id);
          calls.add(id);
        } else {
          requireThat(calls.delete(id), `Unpaired or duplicate result: ${ref}`);
          requireThat(
            row.metadata === undefined || object(row.metadata).client_authored !== true,
            `Client-authored output: ${ref}`,
          );
          const text = textBlocks(payload.output, "input_text");
          yield { text, ref, failed: codexFailed(text) };
        }
      }
    } else {
      requireThat(row.session_id === session, `Session mismatch: ${ref}`);
      requireThat(
        row.parent_tool_use_id === undefined || row.parent_tool_use_id === null,
        `Nested child stream unsupported: ${ref}`,
      );
      requireThat(
        ["system", "assistant", "user", "result"].includes(type),
        `Unsupported Claude record: ${type} at ${ref}`,
      );
      if (type === "system" && row.subtype === "init") {
        requireThat(!initialized.has(path) && row.cwd === cwd, `Session/cwd mismatch: ${ref}`);
        initialized.add(path);
      } else {
        requireThat(initialized.has(path), `Missing Claude init: ${ref}`);
        if (!["assistant", "user"].includes(type)) continue;
        for (const value of array(object(row.message).content)) {
          const block = object(value);
          const kind = string(block.type);
          if (type === "assistant") {
            if (["text", "thinking", "redacted_thinking"].includes(kind)) continue;
            requireThat(kind === "tool_use", `Unsupported assistant block: ${kind} at ${ref}`);
            const id = string(block.id);
            string(block.name);
            object(block.input);
            requireThat(!seenCalls.has(id), `Duplicate tool call: ${ref}`);
            seenCalls.add(id);
            calls.add(id);
          } else {
            requireThat(kind === "tool_result", `Unsupported user block: ${kind} at ${ref}`);
            requireThat(calls.delete(string(block.tool_use_id)), `Unpaired or duplicate result: ${ref}`);
            requireThat(
              block.is_error === undefined || typeof block.is_error === "boolean",
              `Unknown error flag: ${ref}`,
            );
            const text = typeof block.content === "string" ? block.content : textBlocks(block.content, "text");
            // Native Read is not credited by this narrow tool. Only helper packets are compared.
            yield { text, ref, failed: block.is_error === true };
          }
        }
      }
    }
  }
  requireThat(initialized.size === paths.length, "Missing session initialization");
  requireThat(calls.size === 0, "Unfinished tool calls; wait for the turn checkpoint before auditing");
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
  let results = 0;
  for await (const result of delivered(backend, session, cwd, paths)) {
    results++;
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
  throw new EvidenceError("Usage: evidence.ts manifest|read|audit|record|findings ...; see tooling.md");
}

if (import.meta.main) {
  try {
    await main(Deno.args);
  } catch (error) {
    if (!(error instanceof EvidenceError)) throw error;
    console.error(
      `PERSISTENT_REVIEW_TOOL_FAILED: ${error.message}. No coverage is certified by this invocation. See tooling.md; do not silently switch to bulk transcript inspection.`,
    );
    Deno.exit(2);
  }
}

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
  Deno.test(`${backend}: changed shapes, orphan results and corrupt payloads fail closed`, () =>
    fixture(async (root, item) => {
      const path = `${root}/log`;
      const rows = logRows(backend, packet(item, 1, 3).replace("alpha", "wrong"));
      await saveLog(path, rows);
      await fails(() => audit(backend, "session", "/repo", [item], [path]), "does not match");
      await saveLog(path, logRows(backend, packet(item, 1, 3)).filter((_, i) => i !== (backend === "codex" ? 2 : 1)));
      await fails(() => audit(backend, "session", "/repo", [item], [path]), "Unpaired");
      await saveLog(path, [...logRows(backend, packet(item, 1, 3)), { type: "new_format", session_id: "session" }]);
      await fails(() => audit(backend, "session", "/repo", [item], [path]), "Unsupported");
      await Deno.writeTextFile(path, '{"type":');
      await fails(() => audit(backend, "session", "/repo", [item], [path]), "Malformed");
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
    await fails(() => textBlocks([{ type: "image", data: "anything" }], "input_text"), "Unsupported");
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
    await saveLog(path, [...logRows("codex", packet(item, 1, 3)), {
      ...compaction,
      payload: { message: "", replacement_history: "changed shape" },
    }]);
    await fails(() => audit("codex", "session", "/repo", [item], [path]), "Unsupported Codex compaction history");
    await saveLog(path, [...logRows("codex", packet(item, 1, 3)), {
      ...compaction,
      payload: { message: 42, replacement_history: null },
    }]);
    await fails(() => audit("codex", "session", "/repo", [item], [path]), "Expected a string");
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

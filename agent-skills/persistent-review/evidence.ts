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
type Result = { text: string; ref: string; failed: boolean; rejected?: string; forceReject?: boolean };
const BEGIN = "PERSISTENT_REVIEW_READ_V1\n";
const END = "\nPERSISTENT_REVIEW_END_V1";
/**
 * Frame packets on standalone marker lines while accepting LF or CRLF around the outer markers (`agy`'s
 * `run_command` PTY turns raw LF in stdout into CRLF without touching JSON-escaped newlines inside `"text"`).
 * The closing lookahead allows a line break, EOF, or an immediately concatenated next packet while rejecting
 * trailing suffix text on the marker line.
 */
const BEGIN_RE = /(?:^|\n)PERSISTENT_REVIEW_READ_V1\r?\n/;
const END_RE = /\r?\nPERSISTENT_REVIEW_END_V1(?=\r?\n|$|PERSISTENT_REVIEW_READ_V1\r?\n)/;
/**
 * Verified `agy` 1.5.14 `run_command` output envelope, separating harness-authored timestamps and exit/signal
 * status from command stdout after `Command output:\n`. Matching only before that boundary prevents command
 * stdout from impersonating exit or truncation metadata, and prevents unverified tool outputs from earning
 * coverage.
 */
const AGY_CMD_HEADER_RE =
  /^(?:Created At: [^\n]*\n(?:Completed At: [^\n]*\n)?)?(Command exited with code (\d+)\.|Command was terminated by signal or canceled\.)\n\nCommand output:\n/;
const AGY_CMD_EMPTY_RE =
  /^(?:Created At: [^\n]*\n(?:Completed At: [^\n]*\n)?)?(?:Command exited with code \d+\.|Command was terminated by signal or canceled\.)\n\nNo output from the command\.$/;
const AGY_TRUNCATED_PREFIX = "The tool's output was truncated due to being too long.";
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
 * Skipped record kinds checked to carry no delivery, from Codex 0.153.4-0.166.0 rollouts, a Claude Code
 * 2.1.308 verbose stream, and `agy` 1.5.14 `stream-json` logs. This list only decides which report bucket a
 * skipped kind lands in: known kinds go to `ignored`, anything else to `unrecognized`, where the orchestrator
 * judges whether a read may have been missed. It never decides whether the audit runs, so leaving a kind off
 * it costs a judgment call, not an abort.
 */
// deno-fmt-ignore
const KNOWN_SKIPPED = new Set([
  "event_msg", "world_state", "turn_context", "token_usage_record", "compacted", "inter_agent_communication_metadata",
  "response_item:message", "response_item:reasoning", "response_item:agent_message", "response_item:compaction",
  "response_item:web_search_call", "response_item:additional_tools",
  "system:thinking_tokens", "system:hook_started", "system:hook_progress", "system:hook_response",
  "system:task_started", "system:task_progress", "system:task_updated", "system:task_notification",
  "system:background_tasks_changed", "system:compact_boundary",
  "assistant:text", "assistant:thinking", "assistant:redacted_thinking", "user:text", "user:string",
  "step_update:user_input", "step_update:agent_response", "step_update:system_message", "step_update:subagent",
  "result:SUCCESS",
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

/** Return only the harness wrapper text preceding the first read packet, so quoted source text cannot match wrapper patterns. */
function wrapperBeforePacket(text: string): string {
  const match = BEGIN_RE.exec(text);
  return match ? text.slice(0, match.index) : text;
}

/** Check harness wrappers only; words in the quoted source are not execution status. */
function codexFailed(text: string): boolean {
  const wrapper = wrapperBeforePacket(text);
  return /(?:^|\n)(?:Script failed\b|Process exited with code [1-9]\d*\b|exit_code=[1-9]\d*\b)/.test(wrapper);
}

/**
 * True when an `agy` `run_command` invocation targets `evidence read`. Used only to report clipped reads whose
 * outer markers were both lost in a middle truncation without listing unrelated truncated commands (such as test
 * runs) in `rejected`. Never grants coverage.
 */
function isAgyEvidenceRead(params: unknown): boolean {
  if (!isObject(params) || typeof params.CommandLine !== "string") return false;
  return /(?:^|[\s/'";&|])evidence\s+read\b/.test(params.CommandLine);
}

/**
 * Per-file `agy` tool call state. Calls are buffered until the turn stream ends so conflicting terminal updates
 * invalidate each other in either order and so each terminal result can be checked for a subsequent
 * `agent_response` `DONE` proving the model actually ran after the tool completed.
 */
type AgyCall = {
  name: string;
  stepIndex: number;
  terminal?: {
    state: string;
    output: unknown;
    error: unknown;
    parameters: unknown;
    ref: string;
  };
  deliveredToModel: boolean;
  conflictReason?: string;
  conflictRef?: string;
  conflictTexts: string[];
};

/**
 * Resolve one completed `agy` turn file's buffered calls into delivered results. Only `run_command`'s verified
 * string envelope (`Command exited with code 0.\n\nCommand output:\n...`), paired with a matching non-conflicting
 * `ACTIVE` call and followed by a subsequent `agent_response` `DONE`, can supply packet text for coverage.
 */
function* flushAgyFile(
  agyCalls: Map<number, AgyCall>,
  agyOrphans: Result[],
  skip: (kind: string) => void,
  counts: { unfinished: number },
): Generator<Result> {
  for (const orphan of agyOrphans) yield orphan;
  agyOrphans.length = 0;
  for (const call of agyCalls.values()) {
    if (!call.terminal) {
      counts.unfinished++;
      continue;
    }
    if (call.conflictReason) {
      yield {
        text: call.conflictTexts.join("\n"),
        ref: call.conflictRef ?? call.terminal.ref,
        failed: false,
        rejected: call.conflictReason,
      };
      continue;
    }
    const { state, output, error, parameters, ref } = call.terminal;
    const hasToolError = state !== "DONE" || (error !== undefined && error !== null);
    if (output === undefined) {
      yield { text: "", ref, failed: hasToolError };
      continue;
    }
    if (typeof output !== "string") {
      skip(`result_content:${output === null ? "null" : Array.isArray(output) ? "array" : typeof output}`);
      yield { text: "", ref, failed: hasToolError };
      continue;
    }
    if (output.startsWith(AGY_TRUNCATED_PREFIX)) {
      if (
        BEGIN_RE.test(output) || END_RE.test(output) || (call.name === "run_command" && isAgyEvidenceRead(parameters))
      ) {
        yield {
          text: output,
          ref,
          failed: false,
          rejected: "Harness clipped the result; reread a smaller range",
          forceReject: true,
        };
      } else {
        yield { text: "", ref, failed: hasToolError };
      }
      continue;
    }
    if (call.name === "run_command") {
      const header = AGY_CMD_HEADER_RE.exec(output);
      if (header) {
        const body = output.slice(header[0].length);
        const cmdFailed = header[2] !== "0";
        const rejected = !call.deliveredToModel ? "No subsequent model response after tool output" : undefined;
        yield { text: body, ref, failed: hasToolError || cmdFailed, rejected };
        continue;
      }
      if (AGY_CMD_EMPTY_RE.test(output)) {
        yield { text: "", ref, failed: hasToolError };
        continue;
      }
      skip("agy_envelope:run_command");
      yield { text: output, ref, failed: false, rejected: "Unverified agy tool output envelope" };
      continue;
    }
    // Non-run_command tools (e.g. read_file size summaries) have no verified delivered-source envelope.
    if (BEGIN_RE.test(output) || END_RE.test(output)) {
      yield { text: output, ref, failed: false, rejected: `Unverified agy tool output (${call.name})` };
    } else {
      yield { text: "", ref, failed: hasToolError };
    }
  }
  agyCalls.clear();
}

/**
 * Yield every tool result delivered to the audited reviewer, from the checked Codex rollout, Claude verbose
 * stream, or `agy` `stream-json` logs, never raw Codex command events.
 *
 * The audit aborts only when continuing could misattribute identity: a log from another session or cwd, a log
 * with no initialization, or messages/steps before their log's init. Everything else degrades to "no credit". Records
 * that are not tool calls or results (lifecycle events, messages, reasoning, compaction, hooks, task notices,
 * subagent traffic, and whatever a harness adds next) are skipped and counted, in `ignored` when the kind is
 * known to carry no delivery and in `unrecognized` otherwise. Tool results that
 * cannot be trusted (no matching call, client-authored, possibly clipped, an unclear error flag, conflicting
 * updates, or no subsequent model turn on `agy`) are yielded with a `rejected` reason. Calls that never got a
 * result are counted in `unfinished`. A format change therefore shows up as missing coverage plus an `unrecognized`
 * kind, never as false coverage and never as an abort.
 *
 * Do not reintroduce allowlists of record, tool, family or block types that abort the audit. Each one broke valid
 * audits as soon as a harness added something (Codex function calls, agent messages, string and image outputs,
 * late exec results; Claude hooks, synthetic user text, subagent records, ToolSearch references, repeated inits),
 * and rejecting those bought no protection: they can only lower coverage, never raise it.
 */
async function* delivered(
  backend: string,
  session: string,
  cwd: string,
  paths: string[],
  counts: { ignored: Map<string, number>; unrecognized: Map<string, number>; unfinished: number },
): AsyncGenerator<Result> {
  requireThat(["codex", "claude", "agy"].includes(backend), "Supported backends: codex, claude, agy");
  const skip = (raw: string) => {
    // Subagent records and Claude turn results are skipped whatever their inner type. On `agy`, only
    // `result:SUCCESS` is in KNOWN_SKIPPED; `result:ERROR` and `result:invalid` surface in `unrecognized`.
    const known = KNOWN_SKIPPED.has(raw) || raw.startsWith("nested:") ||
      (backend !== "agy" && raw.startsWith("result:"));
    const bucket = known ? counts.ignored : counts.unrecognized;
    let kind = raw.slice(0, 80);
    if (!bucket.has(kind) && bucket.size >= MAX_IGNORED_KINDS) kind = "other";
    bucket.set(kind, (bucket.get(kind) ?? 0) + 1);
  };
  const initialized = new Set<string>();
  // Results pair with calls by ID across all supplied logs on Codex/Claude, so a result in a resumed log may
  // complete a call from an earlier one. The value counts results; a call may have several (see the Codex branch).
  const calls = new Map<string, number>();
  // `agy` runs each turn in its own process and does not carry live tool calls across turns, so its calls are
  // scoped per file and flushed at file boundaries.
  const agyCalls = new Map<number, AgyCall>();
  const agyOrphans: Result[] = [];
  let agyCurrentPath: string | undefined;
  const orphan = "No matching tool call in the supplied logs";
  for await (const { row, ref, path } of records(paths)) {
    if (backend === "agy" && agyCurrentPath !== undefined && path !== agyCurrentPath) {
      yield* flushAgyFile(agyCalls, agyOrphans, skip, counts);
    }
    agyCurrentPath = path;
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

    if (backend === "agy") {
      // Top-level conversation_id must match whenever present, including on skipped or unknown events.
      requireThat(row.conversation_id === undefined || row.conversation_id === session, `Session mismatch: ${ref}`);
      const event = typeof row.event === "string" ? row.event : "?";
      if (event === "init") {
        const init = isObject(row.init) ? row.init : undefined;
        requireThat(row.conversation_id === session && init?.cwd === cwd, `Session/cwd mismatch: ${ref}`);
        initialized.add(path);
        continue;
      }
      if (event === "result") {
        if (!isObject(row.result)) {
          skip("result:invalid");
          continue;
        }
        requireThat(row.result.conversation_id === session, `Session mismatch: ${ref}`);
        skip(`result:${typeof row.result.status === "string" ? row.result.status : "invalid"}`);
        continue;
      }
      if (event !== "step_update") {
        skip(event);
        continue;
      }
      const step = isObject(row.step_update) ? row.step_update : undefined;
      if (!step) {
        requireThat(initialized.has(path), `Missing agy init: ${ref}`);
        skip("step_update:invalid");
        continue;
      }
      // Every step_update belongs to the audited conversation and must follow an init in its own turn log.
      requireThat(step.conversation_id === session, `Session mismatch: ${ref}`);
      requireThat(initialized.has(path), `Missing agy init: ${ref}`);
      const stepType = typeof step.step_type === "string" ? step.step_type : "?";
      const state = typeof step.state === "string" ? step.state : "?";
      if (stepType !== "tool") {
        // A completed agent_response after a terminated tool step proves the model ran after that tool finished.
        if (stepType === "agent_response" && state === "DONE") {
          const responseIdx = Number.isSafeInteger(step.step_index) ? (step.step_index as number) : undefined;
          for (const call of agyCalls.values()) {
            if (call.terminal && (responseIdx === undefined || responseIdx > call.stepIndex)) {
              call.deliveredToModel = true;
            }
          }
        }
        skip(`step_update:${stepType}`);
        continue;
      }
      if (!Number.isSafeInteger(step.step_index) || (step.step_index as number) < 0) {
        skip("step_update:tool:no_index");
        continue;
      }
      if (!isObject(step.tool_info)) {
        skip("step_update:tool:no_info");
        continue;
      }
      const outerName = typeof step.tool_name === "string" && step.tool_name.length ? step.tool_name : undefined;
      const innerName = typeof step.tool_info.name === "string" && step.tool_info.name.length
        ? step.tool_info.name
        : undefined;
      if (!outerName && !innerName) {
        skip("step_update:tool:no_name");
        continue;
      }
      if (outerName && innerName && outerName !== innerName) {
        skip("step_update:tool:name_mismatch");
        continue;
      }
      const name = outerName ?? innerName!;
      const stepIndex = step.step_index as number;
      if (state === "ACTIVE") {
        const existing = agyCalls.get(stepIndex);
        if (!existing) {
          agyCalls.set(stepIndex, { name, stepIndex, deliveredToModel: false, conflictTexts: [] });
        } else if (existing.terminal || existing.name !== name) {
          existing.conflictReason ??= "Conflicting terminal updates for step";
          existing.conflictRef ??= ref;
          if (typeof existing.terminal?.output === "string") existing.conflictTexts.push(existing.terminal.output);
        }
        continue;
      }
      if (state === "DONE" || state === "ERROR" || state === "FAILED" || state === "CANCELLED") {
        const existing = agyCalls.get(stepIndex);
        if (!existing) {
          const text = typeof step.tool_info.output === "string" ? step.tool_info.output : "";
          agyOrphans.push({ text, ref, failed: false, rejected: orphan });
          continue;
        }
        if (existing.name !== name) {
          existing.terminal ??= {
            state,
            output: step.tool_info.output,
            error: step.tool_info.error,
            parameters: step.tool_info.parameters,
            ref,
          };
          existing.conflictReason ??= "Tool identity mismatch with call";
          existing.conflictRef ??= ref;
          if (typeof existing.terminal.output === "string") existing.conflictTexts.push(existing.terminal.output);
          if (typeof step.tool_info.output === "string") existing.conflictTexts.push(step.tool_info.output);
          continue;
        }
        if (!existing.terminal) {
          existing.terminal = {
            state,
            output: step.tool_info.output,
            error: step.tool_info.error,
            parameters: step.tool_info.parameters,
            ref,
          };
          continue;
        }
        // Identical replays are deduplicated; any conflicting second terminal invalidates both updates.
        const same = existing.terminal.state === state &&
          JSON.stringify(existing.terminal.output) === JSON.stringify(step.tool_info.output) &&
          JSON.stringify(existing.terminal.error) === JSON.stringify(step.tool_info.error);
        if (!same) {
          existing.conflictReason ??= "Conflicting terminal updates for step";
          existing.conflictRef ??= ref;
          if (typeof existing.terminal.output === "string") existing.conflictTexts.push(existing.terminal.output);
          if (typeof step.tool_info.output === "string") existing.conflictTexts.push(step.tool_info.output);
        }
        continue;
      }
      skip(`tool_state:${state}`);
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
  if (backend === "agy") yield* flushAgyFile(agyCalls, agyOrphans, skip, counts);
  requireThat(initialized.size === paths.length, "Missing session initialization");
  // A call without a result is usually a turn still running; the audit cannot credit what has not arrived,
  // so it reports the count and lets the caller re-audit after the turn boundary.
  counts.unfinished += [...calls.values()].filter((n) => n === 0).length;
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
    // Only a rejected result that holds a packet (or an explicitly clipped evidence-read call) loses anything;
    // listing unrelated rejected commands would bury the ones that matter.
    if (result.rejected) {
      if (result.forceReject || BEGIN_RE.test(result.text) || END_RE.test(result.text)) {
        rejected.push({ ref: result.ref, reason: result.rejected });
      }
      continue;
    }
    const wrapper = wrapperBeforePacket(result.text);
    // Codex middle clipping prepends `Warning: truncated output` in its tool wrapper; restrict it to Codex so
    // command stdout on other backends cannot impersonate a Codex truncation notice.
    if (backend === "codex" && /(?:^|\n)Warning: truncated output/.test(wrapper)) {
      rejected.push({ ref: result.ref, reason: "Harness clipped the result; reread a smaller range" });
      continue;
    }
    // A lone closing marker with no opener indicates a head-clipped packet rather than a clean non-read result.
    if (!BEGIN_RE.test(result.text) && END_RE.test(result.text)) {
      rejected.push({ ref: result.ref, reason: "Clipped read packet; reread a smaller range" });
      continue;
    }
    let rest = result.text;
    for (;;) {
      const open = BEGIN_RE.exec(rest);
      if (!open) break;
      rest = rest.slice(open.index + open[0].length);
      const close = END_RE.exec(rest);
      if (!close) {
        rejected.push({ ref: result.ref, reason: "Clipped read packet; reread a smaller range" });
        break;
      }
      const raw = rest.slice(0, close.index);
      const value = parse(raw);
      rest = rest.slice(close.index + close[0].length);
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

// ---------------------------------------------------------------------------------------------------------------
// Reviewer records and orchestrator dispositions
//
// A reviewer's log holds three kinds of event under one ID space: `finding` (reported), `candidate` (considered and
// deliberately not reported, with the reason), and `lead` (unresolved, still needing work). One ID may change kind
// over its history, so a lead can become a finding or a refuted candidate, and a finding can be demoted to a proposed
// drop, without losing its provenance. `open` means the record's current kind stands; `withdrawn` is a retraction of
// the record itself. Continuation replies only report what changed, so these logs are the complete record of what a
// reviewer concluded; a candidate that lived only in reply prose would silently vanish from the result.
// ---------------------------------------------------------------------------------------------------------------

/** Event kinds a reviewer log may hold. Anything else is a malformed write, not a harness change. */
const RECORD_KINDS = ["finding", "candidate", "lead"];

/** The latest event per ID in one reviewer log, with the physical line it was read from. */
type Entry = { row: ObjectValue; line: number; log: string };

/** Physical line number of a `records` reference (`path:line`); the path itself may contain colons. */
function lineOf(ref: string): number {
  return Number(ref.slice(ref.lastIndexOf(":") + 1));
}

/** Latest state of every ID per log, keyed by `log\0id`. Each log has one writer, so order is append order. */
async function latest(paths: string[]): Promise<Map<string, Entry>> {
  const entries = new Map<string, Entry>();
  for await (const { row, path, ref } of records(paths, true)) {
    validateFinding(row, ref);
    entries.set(`${path}\0${string(row.id)}`, { row, line: lineOf(ref), log: path });
  }
  return entries;
}

/** The pointer fields the parent needs to decide whether to open a record's detail. */
function pointer(entry: Entry) {
  return {
    id: entry.row.id,
    kind: entry.row.kind,
    summary: entry.row.summary,
    location: entry.row.location,
    reviewer_log: entry.log,
  };
}

/**
 * Count current reviewer records without trusting an advertised total. `active`, `withdrawn` and the `findings`
 * list cover reported findings only, exactly as before candidates and leads existed, so a progress report cannot
 * start counting refuted candidates as findings. Open candidates and leads are counted and listed separately.
 */
async function findings(paths: string[]) {
  const entries = [...(await latest(paths)).values()];
  const open = (kind: string) => entries.filter((e) => e.row.state === "open" && e.row.kind === kind);
  const findingCount = entries.filter((e) => e.row.kind === "finding").length;
  return {
    active: open("finding").length,
    withdrawn: findingCount - open("finding").length,
    open_candidates: open("candidate").length,
    open_leads: open("lead").length,
    findings: open("finding").map(pointer),
    candidates: open("candidate").map(pointer),
    leads: open("lead").map(pointer),
  };
}

/** Physical line count, matching what a scheduler counting newlines records as a turn's starting offset. */
async function physicalLines(path: string): Promise<number> {
  const bytes = await Deno.readFile(path);
  const newlines = bytes.reduce((n, b) => n + (b === 10 ? 1 : 0), 0);
  return newlines + (bytes.length && bytes[bytes.length - 1] !== 10 ? 1 : 0);
}

/**
 * List the events one reviewer appended after physical line `after`: the per-turn delta the orchestrator reads
 * instead of a whole cumulative reply. `last_line` is the offset to pass next time. An offset beyond the end of the
 * log is refused, because it would silently hide every later event (typically another reviewer's offset). A trailing
 * unfinished write fails like any malformed line; retry after the writer reaches a boundary.
 */
async function events(path: string, after: number) {
  const total = await physicalLines(path);
  requireThat(after <= total, `Offset ${after} is past the end of ${path} (${total} lines)`);
  const rows = [];
  for await (const { row, ref } of records([path], true)) {
    validateFinding(row, ref);
    const line = lineOf(ref);
    if (line <= after) continue;
    rows.push({ line, id: row.id, kind: row.kind, state: row.state, summary: row.summary, location: row.location });
  }
  return { log: path, after, last_line: total, events: rows };
}

/** Validate before appending so a malformed new event cannot wedge subsequent progress polls. */
function validateFinding(row: ObjectValue, ref: string) {
  requireThat(row.version === 1 && RECORD_KINDS.includes(row.kind as string), `Unsupported finding event: ${ref}`);
  requireThat(
    string(row.id).length && ["open", "withdrawn"].includes(string(row.state)),
    `Invalid finding state: ${ref}`,
  );
  for (const field of ["summary", "detail", "location"]) {
    requireThat(string(row[field]).trim(), `Empty ${field}: ${ref}`);
  }
}

/**
 * Orchestrator judgments about reviewer records. `duplicate` means another member of the same `group` carries the
 * defect; `excluded` means the orchestrator accepts a reviewer's reason for not reporting a candidate. The other
 * values follow the skill's dispositions for findings and concerns.
 */
const DISPOSITIONS = [
  "verified",
  "unverified-after-investigation",
  "under-investigation",
  "duplicate",
  "rejected",
  "excluded",
];

/**
 * The disposition log holds two event kinds. `reviewer-log` registers a reviewer's findings log for the run, so a
 * consolidation that forgets a log fails instead of showing a clean draft. `disposition` names the exact reviewer
 * event it judged (`log`, `id`, `line`), so a reviewer revision appended after the judgment shows up as stale
 * instead of inheriting a verdict about different text; `group` is the deduplication key, and members of one group
 * are one underlying defect.
 */
function validateDisposition(row: ObjectValue, ref: string) {
  requireThat(row.version === 1, `Unsupported disposition event: ${ref}`);
  requireThat(string(row.log).startsWith("/"), `Reviewer log must be an absolute path: ${ref}`);
  if (row.kind === "reviewer-log") return;
  requireThat(row.kind === "disposition", `Unsupported disposition event: ${ref}`);
  requireThat(string(row.id).length, `Empty id: ${ref}`);
  requireThat(Number.isSafeInteger(row.line) && (row.line as number) >= 1, `Invalid line: ${ref}`);
  requireThat(DISPOSITIONS.includes(string(row.disposition)), `Unknown disposition: ${ref}`);
  for (const field of ["group", "reason"]) requireThat(string(row[field]).trim(), `Empty ${field}: ${ref}`);
}

/**
 * Check a new disposition against the reviewer log before it is appended: the named line must hold an event for
 * that ID. Consolidation refuses dispositions naming unknown records, and the log is append-only, so one bad
 * append would otherwise wedge every later consolidation.
 */
async function checkJudgedEvent(row: ObjectValue) {
  if (row.kind !== "disposition") return;
  for await (const { row: event, ref } of records([string(row.log)], true)) {
    if (lineOf(ref) === row.line) {
      requireThat(event.id === row.id, `Line ${row.line} of ${row.log} is ${event.id}, not ${row.id}`);
      return;
    }
  }
  throw new EvidenceError(`No event at line ${row.line} of ${row.log}`);
}

/**
 * Join the orchestrator's disposition log with the reviewer logs into one consolidated view. It certifies nothing:
 * it shows which open records still lack a decision, which decisions went stale after a reviewer revision, which
 * deduplication groups disagree, which leads are still open, and what reviewers withdrew, so the parent fixes those
 * instead of rediscovering them at recording time. Per-group dispositions are what the skill's deduplicated counts
 * come from; a group with a stale member has no settled disposition until it is judged again.
 */
async function consolidate(dispositionLog: string, logs: string[]) {
  requireThat(
    await Deno.stat(dispositionLog).catch(() => null),
    `Missing disposition log ${dispositionLog}; create it empty and register reviewer logs at run start`,
  );
  const judged = new Map<string, { row: ObjectValue; ref: string }>();
  const registered = new Set<string>();
  const decisions: { row: ObjectValue; ref: string }[] = [];
  for await (const { row, ref } of records([dispositionLog], true)) {
    validateDisposition(row, ref);
    if (row.kind === "reviewer-log") registered.add(string(row.log));
    else decisions.push({ row, ref });
  }
  const supplied = new Set(logs);
  for (const log of registered) requireThat(supplied.has(log), `Registered reviewer log not supplied: ${log}`);
  for (const log of supplied) requireThat(registered.has(log), `Reviewer log was never registered: ${log}`);
  const reviewer = await latest(logs);
  for (const { row, ref } of decisions) {
    const key = `${string(row.log)}\0${string(row.id)}`;
    requireThat(reviewer.has(key), `Disposition names an unknown record ${row.log}#${row.id}: ${ref}`);
    judged.set(key, { row, ref });
  }
  const groups = new Map<string, ObjectValue[]>();
  const undispositioned = [], stale = [], withdrawn = [], openLeads = [];
  for (const [key, entry] of reviewer) {
    const member = {
      log: entry.log,
      id: entry.row.id,
      kind: entry.row.kind,
      state: entry.row.state,
      line: entry.line,
      summary: entry.row.summary,
      location: entry.row.location,
      detail: entry.row.detail,
    };
    const decision = judged.get(key);
    if (entry.row.state === "withdrawn") withdrawn.push({ ...member, judged: Boolean(decision) });
    else if (entry.row.kind === "lead") openLeads.push(member);
    if (!decision) {
      if (entry.row.state === "open") undispositioned.push(member);
      continue;
    }
    const isStale = (decision.row.line as number) !== entry.line;
    if (isStale) stale.push({ ...member, judged_line: decision.row.line, disposition_ref: decision.ref });
    const group = string(decision.row.group);
    groups.set(group, [
      ...(groups.get(group) ?? []),
      { ...member, disposition: decision.row.disposition, reason: decision.row.reason, stale: isStale },
    ]);
  }
  const counts: Record<string, number> = {};
  const rendered = [...groups].sort(([a], [b]) => a.localeCompare(b)).map(([group, members]) => {
    // A group's disposition comes from its non-duplicate members. Stale members void it, all-duplicate groups point
    // at nothing, and disagreeing members need synthesis; each is reported rather than resolved by majority.
    const values = new Set(members.map((m) => m.disposition as string).filter((d) => d !== "duplicate"));
    const disposition = members.some((m) => m.stale)
      ? "stale"
      : values.size === 1
      ? [...values][0]
      : values.size
      ? "conflict"
      : "duplicate-only";
    counts[disposition] = (counts[disposition] ?? 0) + 1;
    return { group, disposition, members };
  });
  return { counts, groups: rendered, undispositioned, stale, open_leads: openLeads, withdrawn };
}

/** Keep reviewer-supplied text on one line, so a newline in a summary cannot forge a heading in the draft. */
function inline(value: unknown): string {
  return String(value).replace(/\s+/g, " ").trim();
}

/** Readable private draft of the consolidated result; problems that block a clean record come first. */
function markdown(view: Awaited<ReturnType<typeof consolidate>>): string {
  const out: string[] = ["# Consolidated review draft", ""];
  const ref = (m: ObjectValue) => `${m.log}#${m.id} (${m.kind}, ${m.state}, line ${m.line})`;
  const attention = [
    ...view.undispositioned.map((m) => `- No disposition: ${ref(m)}: ${inline(m.summary)}`),
    ...view.stale.map((m) => `- Stale disposition (judged line ${m.judged_line}): ${ref(m)}: ${inline(m.summary)}`),
    ...view.open_leads.map((m) => `- Open lead: ${ref(m)}: ${inline(m.summary)}`),
    ...view.groups.filter((g) => ["conflict", "duplicate-only", "stale"].includes(g.disposition)).map((g) =>
      `- Group ${inline(g.group)} is ${g.disposition}`
    ),
  ];
  if (attention.length) out.push("## Needs attention", "", ...attention, "");
  const sections = [...DISPOSITIONS.filter((d) => d !== "duplicate"), "stale", "conflict", "duplicate-only"];
  for (const disposition of sections) {
    const groups = view.groups.filter((g) => g.disposition === disposition);
    if (!groups.length) continue;
    out.push(`## ${disposition} (${groups.length})`, "");
    for (const group of groups) {
      const lead = group.members.find((m) => m.disposition !== "duplicate") ?? group.members[0];
      out.push(`### ${inline(group.group)}: ${inline(lead.summary)}`, "", `Location: ${inline(lead.location)}`, "");
      for (const m of group.members) out.push(`- ${ref(m)}: ${m.disposition}. ${inline(m.reason)}`);
      out.push("", String(lead.detail), "");
    }
  }
  if (view.withdrawn.length) {
    out.push("## Withdrawn by reviewers", "");
    for (const m of view.withdrawn) out.push(`- ${ref(m)}: ${inline(m.summary)}. ${inline(m.detail)}`);
    out.push("");
  }
  return out.join("\n");
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
  if (command === "events") {
    requireThat(rest.length === 3 && rest[1] === "--after" && /^\d+$/.test(rest[2]), "events LOG --after LINE");
    console.log(JSON.stringify(await events(rest[0], integer(Number(rest[2])))));
    return;
  }
  if (command === "register") {
    requireThat(rest.length === 2, "register DISPOSITION_LOG REVIEWER_LOG");
    const row = { version: 1, kind: "reviewer-log", log: rest[1] };
    validateDisposition(row, rest[1]);
    await Deno.writeTextFile(rest[0], JSON.stringify(row) + "\n", { append: true, mode: 0o600 });
    return;
  }
  if (command === "dispose") {
    requireThat(rest.length === 2, "dispose DISPOSITION_LOG EVENT_JSON_FILE");
    const row = parse(await Deno.readTextFile(rest[1]));
    validateDisposition(row, rest[1]);
    requireThat(row.kind === "disposition", "dispose records dispositions; use register for reviewer logs");
    // An unregistered log would pass the line check and then make every later consolidation fail.
    let registered = false;
    for await (const { row: entry } of records([rest[0]], true)) {
      if (entry.kind === "reviewer-log" && entry.log === row.log) registered = true;
    }
    requireThat(registered, `Reviewer log ${row.log} is not registered in ${rest[0]}; register it first`);
    await checkJudgedEvent(row);
    await Deno.writeTextFile(rest[0], JSON.stringify(row) + "\n", { append: true, mode: 0o600 });
    return;
  }
  if (command === "consolidate") {
    const asMarkdown = rest[0] === "--markdown";
    const [dispositionLog, ...logs] = asMarkdown ? rest.slice(1) : rest;
    requireThat(dispositionLog && logs.length, "consolidate [--markdown] DISPOSITION_LOG LOG...");
    const view = await consolidate(dispositionLog, logs);
    console.log(asMarkdown ? markdown(view) : JSON.stringify(view));
    return;
  }
  throw new EvidenceError(
    "Usage: evidence manifest|read|audit|record|findings|events|register|dispose|consolidate ...; see tooling.md",
  );
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
function logRows(
  backend: string,
  text: string,
  options: { id?: string; step?: number; failed?: boolean } = {},
): ObjectValue[] {
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
  if (backend === "agy") {
    const step = options.step ?? 3;
    const output =
      `Created At: 2026-11-21T08:00:00-08:00\nCompleted At: 2026-11-21T08:00:00-08:00\nCommand exited with code ${
        options.failed ? 1 : 0
      }.\n\nCommand output:\n${text}`;
    return [
      { event: "init", conversation_id: "session", init: { model: "gemini-4-neon", cwd: "/repo" } },
      {
        event: "step_update",
        step_update: {
          conversation_id: "session",
          step_index: step,
          state: "ACTIVE",
          step_type: "tool",
          tool_name: "run_command",
          tool_info: { name: "run_command", parameters: { CommandLine: "evidence read" } },
        },
      },
      {
        event: "step_update",
        step_update: {
          conversation_id: "session",
          step_index: step,
          state: "DONE",
          step_type: "tool",
          tool_name: "run_command",
          tool_info: { name: "run_command", parameters: { CommandLine: "evidence read" }, output },
        },
      },
      {
        event: "step_update",
        step_update: {
          conversation_id: "session",
          step_index: step + 1,
          state: "DONE",
          step_type: "agent_response",
          usage: { input_tokens: 100, output_tokens: 10, total_tokens: 110 },
        },
      },
      { event: "result", result: { conversation_id: "session", status: "SUCCESS", response: "ok" } },
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

for (const backend of ["codex", "claude", "agy"]) {
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
  // On `agy`, `run_command` itself caps output at 10,240 bytes, so test >200 short lines there rather than >24 KiB.
  Deno.test(`${backend}: producer read budgets do not invalidate fully delivered matching text`, () =>
    fixture(async (root, item) => {
      const path = `${root}/log`;
      const cases = backend === "agy" ? ["line\n".repeat(201)] : ["line\n".repeat(201), "x".repeat(MAX_PACKET + 1)];
      for (const text of cases) {
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
      const future = backend === "agy"
        ? { event: "future_record", conversation_id: "session", output: packet(item, 2, 3) }
        : { type: "future_record", session_id: "session", output: packet(item, 2, 3) };
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

// Agent messages, inter-agent metadata and server-side web searches appear in ordinary 0.163.0 rollouts, and
// 0.166.0 adds an `additional_tools` record holding tool definitions.
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
      { type: "response_item", payload: { type: "additional_tools", role: "developer", tools: [] } },
    ]);
    const report = await audit("codex", "session", "/repo", [item], [path]);
    equal(report.status, "complete");
    equal(report.delivered_results, 1);
    equal(Object.keys(report.ignored), [
      "event_msg",
      "inter_agent_communication_metadata",
      "response_item:additional_tools",
      "response_item:agent_message",
      "response_item:web_search_call",
    ]);
    // Subagent rollouts fail with a diagnostic that names the cause instead of a generic identity mismatch.
    await saveLog(path, [rows[0], { type: "event_msg", payload: {} }, rows[0]]);
    await fails(() => audit("codex", "session", "/repo", [item], [path]), "second session_meta");
    await saveLog(path, [{ type: "session_meta", payload: { id: "session", session_id: "root", cwd: "/repo" } }]);
    await fails(() => audit("codex", "session", "/repo", [item], [path]), "native subagent rollouts cannot be audited");
  }));

for (const backend of ["codex", "claude", "agy"]) {
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

/** A minimal reviewer event; tests override the fields they exercise. */
function reviewerEvent(fields: ObjectValue): ObjectValue {
  return {
    version: 1,
    kind: "finding",
    id: "f1",
    state: "open",
    summary: "bug",
    location: "a:1",
    detail: "why",
    ...fields,
  };
}

/** A minimal disposition; tests override the fields they exercise. */
function dispositionEvent(fields: ObjectValue): ObjectValue {
  return { version: 1, kind: "disposition", disposition: "verified", group: "g", reason: "checked", ...fields };
}

/** A disposition log registering each reviewer log, followed by the given dispositions. */
function decisionLog(logs: string[], dispositions: ObjectValue[]): ObjectValue[] {
  return [...logs.map((log) => ({ version: 1, kind: "reviewer-log", log })), ...dispositions];
}

/**
 * Candidates and leads exist so that what a reviewer decided not to report survives delta replies. The counts and
 * the `findings` list that meant "reported findings" before these kinds existed must keep that meaning, or progress
 * reports would silently start counting refuted candidates as findings.
 */
Deno.test("findings keep reported findings separate from candidates and leads, and follow kind changes", () =>
  fixture(async (root) => {
    const log = `${root}/log`;
    await saveLog(log, [
      reviewerEvent({ id: "f1" }),
      reviewerEvent({ id: "c1", kind: "candidate", detail: "refuted at a:9" }),
      reviewerEvent({ id: "l1", kind: "lead" }),
      // A lead closes by changing kind; a promoted candidate keeps its ID and history.
      reviewerEvent({ id: "l1", kind: "candidate", detail: "checked, no route" }),
      reviewerEvent({ id: "c2", kind: "candidate" }),
      reviewerEvent({ id: "c2", kind: "finding" }),
      reviewerEvent({ id: "l2", kind: "lead" }),
    ]);
    const report = await findings([log]);
    equal([report.active, report.withdrawn, report.open_candidates, report.open_leads], [2, 0, 2, 1]);
    equal(report.findings.map((f) => f.id), ["f1", "c2"]);
    equal(report.candidates.map((f) => f.id), ["c1", "l1"]);
    equal(report.leads.map((f) => f.id), ["l2"]);
    await saveLog(log, [reviewerEvent({ kind: "note" })]);
    await fails(() => findings([log]), "Unsupported finding event");
  }));

/**
 * The per-turn delta replaces rereading whole cumulative replies, so it must return exactly the events appended
 * after the offset, keep physical line numbers across blank separators, hand back the next offset, and refuse an
 * offset past the end, which would otherwise hide every later event without an error.
 */
Deno.test("events lists only records after the offset and refuses offsets past the end", () =>
  fixture(async (root) => {
    const log = `${root}/log`;
    await Deno.writeTextFile(log, "");
    equal(await events(log, 0), { log, after: 0, last_line: 0, events: [] });
    await Deno.writeTextFile(
      log,
      [reviewerEvent({ id: "f1" }), {}, reviewerEvent({ id: "c1", kind: "candidate" })]
        .map((row) => Object.keys(row).length ? JSON.stringify(row) : "").join("\n") + "\n",
    );
    const delta = await events(log, 1);
    equal(delta.last_line, 3);
    equal(delta.events.map((e) => [e.line, e.id, e.kind]), [[3, "c1", "candidate"]]);
    equal((await events(log, 3)).events.length, 0);
    await fails(() => events(log, 4), "past the end");
    await fails(() => main(["events", log, "--after", ""]), "events LOG --after LINE");
    await Deno.writeTextFile(log, "{", { append: true });
    await fails(() => events(log, 3), "Malformed");
  }));

/**
 * The consolidated view is where the parent finds what still needs a decision, and a downstream recording step may
 * render its result from it. Nothing a reviewer wrote may vanish from it: undecided records, decisions made before a
 * later revision, disagreeing groups, open leads, and withdrawn records must all be visible, and a stale decision
 * must not keep counting toward the group's old verdict.
 */
Deno.test("consolidate surfaces undecided, stale, conflicting, open and withdrawn records", () =>
  fixture(async (root) => {
    const a = `${root}/a`, b = `${root}/b`, decisions = `${root}/decisions`;
    await saveLog(a, [
      reviewerEvent({ id: "f1" }),
      reviewerEvent({ id: "f2", summary: "later revised" }),
      reviewerEvent({ id: "c1", kind: "candidate" }),
      reviewerEvent({ id: "f2", summary: "revised", detail: "new text" }),
      reviewerEvent({ id: "c9", kind: "candidate", state: "withdrawn", detail: "entered in error" }),
    ]);
    await saveLog(b, [reviewerEvent({ id: "f9" }), reviewerEvent({ id: "l1", kind: "lead" })]);
    await saveLog(
      decisions,
      decisionLog([a, b], [
        dispositionEvent({ log: a, id: "f1", line: 1, group: "race" }),
        dispositionEvent({ log: b, id: "f9", line: 1, group: "race", disposition: "duplicate" }),
        dispositionEvent({ log: a, id: "f2", line: 2, group: "leak", disposition: "rejected" }),
        dispositionEvent({ log: a, id: "c1", line: 3, group: "drop", disposition: "excluded" }),
      ]),
    );
    const view = await consolidate(decisions, [a, b]);
    equal(view.counts, { excluded: 1, stale: 1, verified: 1 });
    equal(view.groups.find((g) => g.group === "race")?.members.length, 2);
    equal(view.undispositioned.map((m) => m.id), ["l1"]);
    equal(view.stale.map((m) => [m.id, m.judged_line, m.line]), [["f2", 2, 4]]);
    equal(view.open_leads.map((m) => m.id), ["l1"]);
    equal(view.withdrawn.map((m) => [m.id, m.judged]), [["c9", false]]);
    const draft = markdown(view);
    for (const text of ["## Needs attention", "Open lead:", "Group leak is stale", "## Withdrawn by reviewers"]) {
      requireThat(draft.includes(text), `draft lacks ${text}`);
    }

    await saveLog(
      decisions,
      decisionLog([a, b], [
        dispositionEvent({ log: a, id: "f1", line: 1, group: "race" }),
        dispositionEvent({ log: b, id: "f9", line: 1, group: "race", disposition: "rejected" }),
        dispositionEvent({ log: a, id: "c1", line: 3, group: "x\n## injected", disposition: "duplicate" }),
      ]),
    );
    const conflicted = await consolidate(decisions, [a, b]);
    equal(conflicted.counts, { conflict: 1, "duplicate-only": 1 });
    requireThat(!markdown(conflicted).includes("\n## injected"), "a group name forged a heading");

    await fails(() => consolidate(decisions, [a]), "Registered reviewer log not supplied");
    await saveLog(decisions, decisionLog([a], []));
    await fails(() => consolidate(decisions, [a, b]), "never registered");
    await fails(() => consolidate(`${root}/missing`, [a]), "create it empty");
  }));

/**
 * The disposition log is append-only and consolidation refuses decisions about unknown records, so `dispose` must
 * check the judged event before appending; otherwise one typo would wedge every later consolidation.
 */
Deno.test("dispose refuses a decision about an unregistered log, or a record or line the log does not hold", () =>
  fixture(async (root) => {
    const a = `${root}/a`, decisions = `${root}/decisions`, event = `${root}/event`;
    await saveLog(a, [reviewerEvent({ id: "f1" }), reviewerEvent({ id: "f2" })]);
    await Deno.writeTextFile(decisions, "");
    await Deno.writeTextFile(event, JSON.stringify(dispositionEvent({ log: a, id: "f1", line: 1 })));
    await fails(() => main(["dispose", decisions, event]), "is not registered");
    await main(["register", decisions, a]);
    for (const [fields, reason] of [[{ id: "f9", line: 1 }, "is f1, not f9"], [{ id: "f1", line: 7 }, "No event"]]) {
      await Deno.writeTextFile(event, JSON.stringify(dispositionEvent({ log: a, ...(fields as ObjectValue) })));
      await fails(() => main(["dispose", decisions, event]), reason as string);
    }
    await Deno.writeTextFile(event, JSON.stringify(dispositionEvent({ log: a, id: "f2", line: 2 })));
    await main(["dispose", decisions, event]);
    equal((await consolidate(decisions, [a])).counts, { verified: 1 });
  }));

/**
 * `agy`'s `run_command` executes under a PTY that converts raw LF to CRLF around packet markers while leaving
 * JSON-escaped newlines (including CRLF inside the source file) intact. Resumes write fresh stream files with
 * higher step indices and must preserve both `conversation_id` and `init.cwd`; a silent fresh-session resume
 * (when `--conversation` is not found) or a wrong-cwd resume must fail visibly.
 */
Deno.test("agy PTY CRLF packets and multi-turn resumes verify coverage and catch identity drift", () =>
  fixture(async (root, item) => {
    const first = `${root}/turn1.jsonl`, second = `${root}/turn2.jsonl`;
    const ptyPacket = (start: number, end: number) => packet(item, start, end).replaceAll("\n", "\r\n") + "\r\n";
    await saveLog(first, logRows("agy", ptyPacket(1, 1), { step: 3 }));
    await saveLog(second, logRows("agy", ptyPacket(2, 3), { step: 11 }));
    const report = await audit("agy", "session", "/repo", [item], [first, second]);
    equal(report.status, "complete");
    equal(report.sources[0].covered, [[1, 3]]);
    equal(report.evidence.map(({ ref }) => ref), [`${first}:3`, `${second}:3`]);

    // A failed resume (`conversation "<id>" not found`) allocates a new conversation_id and must fail the audit.
    const freshSession = logRows("agy", ptyPacket(2, 3), { step: 3 }).map((row) =>
      JSON.parse(JSON.stringify(row).replaceAll('"session"', '"new-session"'))
    );
    await saveLog(second, freshSession);
    await fails(() => audit("agy", "session", "/repo", [item], [first, second]), "Session mismatch");

    // Resuming from a different directory changes init.cwd and must fail the audit.
    const wrongCwd = logRows("agy", ptyPacket(2, 3), { step: 11 }).map((row) =>
      row.event === "init" ? { ...row, init: { model: "gemini-4-neon", cwd: "/other" } } : row
    );
    await saveLog(second, wrongCwd);
    await fails(() => audit("agy", "session", "/repo", [item], [first, second]), "Session/cwd mismatch");
  }));

/**
 * `agy` middle-truncates `run_command` outputs above 10,240 bytes, leaving both outer packet markers intact while
 * cutting the JSON payload in half. That must be rejected as clipped instead of aborting with a JSON syntax error,
 * clipped `evidence read` calls whose markers were both lost must still be rejected, truncated non-packet commands
 * (such as test runs) must stay out of `rejected`, and command stdout or source text containing wrapper lookalikes
 * must neither block a valid read nor mask a malformed packet.
 */
Deno.test("agy middle clipping, exit wrappers and tool errors reject packets without false positives", () =>
  fixture(async (root, item) => {
    const path = `${root}/log`;
    const full = packet(item, 1, 3).replaceAll("\n", "\r\n");
    const clipped = "The tool's output was truncated due to being too long. " +
      "You can find the full output in a file at file:///brain/steps/7/output.txt.\n\n" +
      "Here is a preview with the middle of the output truncated:\n" +
      "Created At: 2026-11-21T08:00:00-08:00\nCommand exited with code 0.\n\nCommand output:\n" +
      full.slice(0, 60) + "\n<truncated 500 of 600 bytes from the middle of the output>\n" + full.slice(-30);
    const [init, active, done, response, footer] = logRows("agy", full);
    const withOutput = (output: unknown, extra: ObjectValue = {}) => {
      const su = object(done.step_update);
      const info = { ...object(su.tool_info), ...extra };
      if (output === undefined) delete info.output;
      else info.output = output;
      return { ...done, step_update: { ...su, tool_info: info } };
    };
    await saveLog(path, [init, active, withOutput(clipped), response, footer]);
    const truncReport = await audit("agy", "session", "/repo", [item], [path]);
    equal(truncReport.status, "incomplete");
    equal(truncReport.rejected.map(({ reason }) => reason), ["Harness clipped the result; reread a smaller range"]);

    // Even if both outer markers were lost in a clipped `evidence read` command, it is still reported as clipped.
    const clippedNoMarkers = "The tool's output was truncated due to being too long.\n...middle lost...";
    await saveLog(path, [init, active, withOutput(clippedNoMarkers), response, footer]);
    const lostMarkerClip = await audit("agy", "session", "/repo", [item], [path]);
    equal(lostMarkerClip.status, "incomplete");
    equal(lostMarkerClip.rejected.map(({ reason }) => reason), ["Harness clipped the result; reread a smaller range"]);

    // A truncated non-packet command (e.g. cargo test) alongside a complete read must not pollute `rejected`.
    const bigNonPacket = "The tool's output was truncated due to being too long.\n...test output...";
    const [, active2, done2] = logRows("agy", "unused", { step: 5 });
    const active2Su = object(active2.step_update);
    const done2Su = object(done2.step_update);
    const active2Test = {
      ...active2,
      step_update: { ...active2Su, tool_info: { name: "run_command", parameters: { CommandLine: "cargo test" } } },
    };
    const done2Trunc = {
      ...done2,
      step_update: {
        ...done2Su,
        tool_info: { name: "run_command", parameters: { CommandLine: "cargo test" }, output: bigNonPacket },
      },
    };
    const laterResponse = { ...response, step_update: { ...object(response.step_update), step_index: 6 } };
    await saveLog(path, [init, active, done, active2Test, done2Trunc, laterResponse, footer]);
    const cleanReport = await audit("agy", "session", "/repo", [item], [path]);
    equal([cleanReport.status, cleanReport.rejected.length], ["complete", 0]);

    // Non-zero exit (background or foreground), signal termination, and tool ERROR / tool_info.error all reject.
    for (
      const row of [
        withOutput(`Command exited with code 2.\n\nCommand output:\n${full}`),
        withOutput(`Command was terminated by signal or canceled.\n\nCommand output:\n${full}`),
        withOutput(
          `Command exited with code 0.\n\nCommand output:\n${full}`,
          { error: { type: "TOOL_ERROR", message: "boom" } },
        ),
        {
          ...withOutput(`Command exited with code 0.\n\nCommand output:\n${full}`),
          step_update: {
            ...object(withOutput(`Command exited with code 0.\n\nCommand output:\n${full}`).step_update),
            state: "ERROR",
          },
        },
      ]
    ) {
      await saveLog(path, [init, active, row, response, footer]);
      const failed = await audit("agy", "session", "/repo", [item], [path]);
      equal(failed.status, "incomplete");
      equal(failed.rejected.map(({ reason }) => reason), ["Tool reported failure"]);
    }

    // Wrapper lookalikes emitted in command stdout after `Command output:\n` must not trigger exit or clipping checks.
    const stdoutLookalike = `Command exited with code 18.\nWarning: truncated output\n${full}`;
    await saveLog(path, logRows("agy", stdoutLookalike));
    equal((await audit("agy", "session", "/repo", [item], [path])).status, "complete");

    // Source text containing agy's truncation or failure wording must not trigger wrapper checks.
    await Deno.writeTextFile(
      item.path,
      "The tool's output was truncated due to being too long.\nCommand exited with code 1.\n",
    );
    const tricky = await source(item.path);
    await saveLog(path, logRows("agy", packet(tricky, 1, 2)));
    equal((await audit("agy", "session", "/repo", [tricky], [path])).status, "complete");
    await saveLog(path, logRows("agy", packet(tricky, 1, 2) + BEGIN + "invalid json" + END));
    await fails(() => audit("agy", "session", "/repo", [tricky], [path]), "Malformed");
  }));

/**
 * Adversarial counterexamples for `agy`: unverified tool outputs (non-string arrays, non-run_command tools, or
 * unfamiliar run_command wrappers), bare ACTIVE calls, tool-name mismatches, conflicting terminal updates in
 * either order, cross-file step reuse, and stopped/errored streams without a subsequent `agent_response` `DONE`
 * must never earn coverage. Identical terminal replays stay valid, and standalone-line packet framing must reject
 * prefixed/suffixed marker substrings while flagging lone closing markers.
 */
Deno.test("agy rejects unverified telemetry, conflicting updates, stopped streams, and unanchored markers", () =>
  fixture(async (root, item) => {
    const path = `${root}/log`, second = `${root}/log2`;
    const full = packet(item, 1, 3);
    const [init, active, done, response, footer] = logRows("agy", full, { step: 3 });
    const doneWith = (overrides: ObjectValue, infoOverrides: ObjectValue = {}) => {
      const su = object(done.step_update);
      return {
        ...done,
        step_update: {
          ...su,
          ...overrides,
          tool_info: { ...object(su.tool_info), ...infoOverrides },
        },
      };
    };

    // Non-string array output (e.g. `[{type: "metadata", text: packet}]`) lands in unrecognized and earns nothing.
    await saveLog(path, [init, active, doneWith({}, { output: [{ type: "metadata", text: full }] }), response, footer]);
    const arrayOut = await audit("agy", "session", "/repo", [item], [path]);
    equal([arrayOut.status, arrayOut.unrecognized["result_content:array"]], ["incomplete", 1]);

    // Unverified run_command wrapper (e.g. synthetic in-progress text) or non-run_command string output earns nothing.
    await saveLog(path, [init, active, doneWith({}, { output: `In progress:\n${full}` }), response, footer]);
    const badEnv = await audit("agy", "session", "/repo", [item], [path]);
    equal(badEnv.status, "incomplete");
    equal(badEnv.unrecognized["agy_envelope:run_command"], 1);
    equal(badEnv.rejected.map(({ reason }) => reason), ["Unverified agy tool output envelope"]);

    const activeRead = {
      ...active,
      step_update: {
        ...object(active.step_update),
        tool_name: "read_file",
        tool_info: { name: "read_file", parameters: { Path: item.path } },
      },
    };
    const doneRead = doneWith({ tool_name: "read_file" }, { name: "read_file", output: full });
    await saveLog(path, [init, activeRead, doneRead, response, footer]);
    const readTool = await audit("agy", "session", "/repo", [item], [path]);
    equal(readTool.status, "incomplete");
    equal(readTool.rejected.map(({ reason }) => reason), ["Unverified agy tool output (read_file)"]);

    // Bare ACTIVE without tool_info cannot authorize a later DONE.
    const bareActive = {
      event: "step_update",
      step_update: { conversation_id: "session", step_index: 3, state: "ACTIVE", step_type: "tool" },
    };
    await saveLog(path, [init, bareActive, done, response, footer]);
    const bare = await audit("agy", "session", "/repo", [item], [path]);
    equal([bare.status, bare.unrecognized["step_update:tool:no_info"]], ["incomplete", 1]);
    equal(bare.rejected.map(({ reason }) => reason), ["No matching tool call in the supplied logs"]);

    // ACTIVE read_file followed by DONE run_command on the same step_index fails tool identity matching.
    await saveLog(path, [init, activeRead, done, response, footer]);
    const mismatch = await audit("agy", "session", "/repo", [item], [path]);
    equal(mismatch.status, "incomplete");
    equal(mismatch.rejected.map(({ reason }) => reason), ["Tool identity mismatch with call"]);

    // Conflicting terminal updates invalidate each other in BOTH orders; an identical replay stays valid.
    const emptyDone = doneWith({}, { output: "Command exited with code 0.\n\nNo output from the command." });
    for (const pair of [[done, emptyDone], [emptyDone, done]]) {
      await saveLog(path, [init, active, ...pair, response, footer]);
      const conflict = await audit("agy", "session", "/repo", [item], [path]);
      equal(conflict.status, "incomplete");
      equal(conflict.rejected.map(({ reason }) => reason), ["Conflicting terminal updates for step"]);
    }
    await saveLog(path, [init, active, done, done, response, footer]);
    equal((await audit("agy", "session", "/repo", [item], [path])).status, "complete");

    // Cross-file step_index reuse from an earlier turn cannot authorize a terminal without an ACTIVE in its own file.
    await saveLog(path, [init, active, emptyDone, response, footer]);
    await saveLog(second, [init, done, response, footer]);
    const crossFile = await audit("agy", "session", "/repo", [item], [path, second]);
    equal(crossFile.status, "incomplete");
    equal(crossFile.rejected.map(({ reason }) => reason), ["No matching tool call in the supplied logs"]);

    // Stopped streams without a subsequent `agent_response` `DONE` never certify delivery.
    for (
      const tail of [
        [],
        [{ event: "result", result: { conversation_id: "session", status: "ERROR", error: "boom" } }],
        [{ event: "result", result: "not-an-object" }],
        [footer],
      ]
    ) {
      await saveLog(path, [init, active, done, ...tail]);
      const stopped = await audit("agy", "session", "/repo", [item], [path]);
      equal(stopped.status, "incomplete");
      equal(stopped.rejected.map(({ reason }) => reason), ["No subsequent model response after tool output"]);
    }

    // Interrupted turn: read 1 followed by agent_response DONE earns coverage; trailing read 2 before EOF does not.
    const [, active1, done1, resp1] = logRows("agy", packet(item, 1, 1), { step: 3 });
    const [, active2, done2] = logRows("agy", packet(item, 2, 3), { step: 5 });
    await saveLog(path, [init, active1, done1, resp1, active2, done2]);
    const partial = await audit("agy", "session", "/repo", [item], [path]);
    equal(partial.sources[0].covered, [[1, 1]]);
    equal(partial.sources[0].missing, [[2, 3]]);
    equal(partial.rejected.map(({ reason }) => reason), ["No subsequent model response after tool output"]);

    // Standalone-line framing: prefixed opener / suffixed closer earn nothing; diagnostic opener does not break a
    // valid packet; lone closer is rejected as clipped.
    await saveLog(path, logRows("agy", `prefix ${full}_suffix`));
    equal((await audit("agy", "session", "/repo", [item], [path])).status, "incomplete");
    await saveLog(path, logRows("agy", `Note about PERSISTENT_REVIEW_READ_V1\n${full}\n`));
    equal((await audit("agy", "session", "/repo", [item], [path])).status, "complete");
    await saveLog(path, logRows("agy", `truncated head\nPERSISTENT_REVIEW_END_V1\n`));
    const loneClose = await audit("agy", "session", "/repo", [item], [path]);
    equal(loneClose.status, "incomplete");
    equal(loneClose.rejected.map(({ reason }) => reason), ["Clipped read packet; reread a smaller range"]);
  }));

/**
 * Non-tool step updates (`user_input`, `agent_response`, `system_message`, `subagent`) and `SUCCESS` `result`
 * events are routine `agy` traffic that must be ignored without earning coverage, while unfamiliar step types,
 * tool states, and non-SUCCESS/malformed `result` payloads degrade to `unrecognized`. Identity checks
 * (`conversation_id`, per-log `init`) must remain fatal even on skipped records and startup errors.
 */
Deno.test("agy lifecycle records, subagents, unfinished calls and identity gates behave consistently", () =>
  fixture(async (root, item) => {
    const path = `${root}/log`;
    const [init, active, done, response, footer] = logRows("agy", packet(item, 1, 1), { step: 3 });
    const step = (step_index: number, step_type: string, extra: ObjectValue = {}) => ({
      event: "step_update",
      step_update: { conversation_id: "session", step_index, state: "DONE", step_type, ...extra },
    });
    const denied = {
      event: "step_update",
      step_update: {
        conversation_id: "session",
        step_index: 6,
        state: "DONE",
        step_type: "tool",
        tool_name: "run_command",
        tool_info: { name: "run_command", parameters: { CommandLine: "echo hi" } },
      },
    };
    const activeDenied = { ...denied, step_update: { ...denied.step_update, state: "ACTIVE" } };
    const pendingTool = {
      event: "step_update",
      step_update: {
        conversation_id: "session",
        step_index: 7,
        state: "ACTIVE",
        step_type: "tool",
        tool_name: "run_command",
        tool_info: { name: "run_command", parameters: {} },
      },
    };
    const queuedTool = {
      event: "step_update",
      step_update: {
        conversation_id: "session",
        step_index: 9,
        state: "QUEUED",
        step_type: "tool",
        tool_name: "run_command",
        tool_info: { name: "run_command", parameters: {} },
      },
    };
    await saveLog(path, [
      init,
      step(0, "user_input"),
      step(1, "agent_response"),
      active,
      done,
      response,
      step(5, "system_message"),
      step(5, "subagent", {
        tool_name: "invoke_subagent",
        subagent_info: { subagents: [{ conversation_id: "child-session" }], output: packet(item, 2, 3) },
      }),
      activeDenied,
      denied,
      pendingTool,
      step(8, "future_step_type", { output: packet(item, 2, 3) }),
      queuedTool,
      footer,
    ]);
    const report = await audit("agy", "session", "/repo", [item], [path]);
    equal(report.sources[0].covered, [[1, 1]]);
    equal(report.sources[0].missing, [[2, 3]]);
    equal(report.unfinished_calls, 1);
    equal(Object.keys(report.ignored), [
      "result:SUCCESS",
      "step_update:agent_response",
      "step_update:subagent",
      "step_update:system_message",
      "step_update:user_input",
    ]);
    equal(report.unrecognized, { "step_update:future_step_type": 1, "tool_state:QUEUED": 1 });

    // Identity failures remain fatal across init, step_update, result, and startup-error records.
    await saveLog(path, [{ event: "result", result: { conversation_id: "", status: "ERROR" } }]);
    await fails(() => audit("agy", "session", "/repo", [item], [path]), "Session mismatch");
    await saveLog(path, [init, { event: "result", result: { status: "SUCCESS" } }]);
    await fails(() => audit("agy", "session", "/repo", [item], [path]), "Session mismatch");
    await saveLog(path, [step(0, "user_input"), init]);
    await fails(() => audit("agy", "session", "/repo", [item], [path]), "Missing agy init");
    await saveLog(path, [
      init,
      { event: "step_update", step_update: { step_index: 0, state: "DONE", step_type: "user_input" } },
    ]);
    await fails(() => audit("agy", "session", "/repo", [item], [path]), "Session mismatch");
    await saveLog(path, [
      init,
      active,
      done,
      response,
      { event: "result", result: { conversation_id: "other", status: "SUCCESS" } },
    ]);
    await fails(() => audit("agy", "session", "/repo", [item], [path]), "Session mismatch");
    await saveLog(path, [init, { ...init, init: { model: "gemini-4-neon", cwd: "/other" } }]);
    await fails(() => audit("agy", "session", "/repo", [item], [path]), "Session/cwd mismatch");
  }));

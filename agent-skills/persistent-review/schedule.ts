#!/usr/bin/env -S deno run -A
/**
 * Deterministic turn scheduler for persistent reviews.
 *
 * It owns exactly three things: the queue of reviewer turns the orchestrator has marked ready, the concurrency limit,
 * and the rule that one reviewer identity never has two live turns. It runs the launch script the orchestrator wrote
 * for each turn, records when it started and how it exited, and parks the identity as awaiting a parent decision.
 *
 * It deliberately knows nothing else. It never builds a launch command (the shellout skill's harness files own those),
 * never reads transcripts or findings content, never audits delivery, and never advances an identity to its next
 * phase: only `ready`, called by the orchestrator after it has judged the previous turn, queues more work. A turn that
 * exited 0 is not a completed review turn; it is a turn whose result the parent has not looked at yet.
 *
 * It exists because hand-rolled batch runners in a real run froze their concurrency at launch and owned fixed identity
 * lists, so free capacity sat idle next to queued work and could not be reused without risking a duplicate resume.
 * Every rule below that looks paranoid (commit before spawn, process groups, exit-file re-reads) guards the same
 * property: a reviewer session is never resumed while an earlier turn of it may still be running.
 */
import { join } from "node:path";

type Turn = {
  n: number;
  label: string;
  script: string;
  audit?: string;
  queued_at: string;
  /** Set in the transaction that commits the launch, before any process exists. */
  launched_at?: string;
  ended_at?: string;
  /** Process-group id of the turn's wrapper, which is also the wrapper's pid. */
  pid?: number;
  /** Exit status of the launch script, or "lost" when nothing of the turn is left running and no status exists. */
  exit?: number | "lost";
  findings_start?: number | null;
  findings_end?: number | null;
};
type State = "new" | "queued" | "running" | "awaiting-parent" | "complete" | "stopped-incomplete";
type Identity = { state: State; findings?: string; reason?: string; turns: Turn[] };
type Event = { seq: number; time: string; kind: string; [key: string]: unknown };
type Pending = Omit<Event, "seq" | "time">;
type Schedule = {
  version: 1;
  max: number;
  seq: number;
  identities: Record<string, Identity>;
  /** The latest transaction's events, kept so a crash between the state write and the journal append can be repaired. */
  last_events: Event[];
};

/** Expected failures are refusals with a message; anything else is a bug and keeps its stack trace. */
class ScheduleError extends Error {}
function requireThat(test: unknown, message: string): asserts test {
  if (!test) throw new ScheduleError(message);
}

/** How long a committed launch may go without its wrapper recording a pid before the turn counts as lost. */
const START_GRACE_MS = 30_000;
/** A daemon that has not written a heartbeat for this long is treated as gone, whatever its pid now belongs to. */
const HEARTBEAT_STALE_MS = 10_000;

const now = () => new Date().toISOString();
const statePath = (dir: string) => join(dir, "state.json");
const eventsPath = (dir: string) => join(dir, "events.jsonl");
const daemonPath = (dir: string) => join(dir, "daemon.json");
const stopPath = (dir: string) => join(dir, "stop-requested");
const turnBase = (dir: string, id: string, n: number) => join(dir, "turns", `${id}-${n}`);

// ---------------------------------------------------------------------------------------------------------------
// State storage
//
// state.json is the only source of truth, replaced atomically by rename. events.jsonl is an append-only journal of
// transitions that `wait` follows. Every mutation, from the daemon or a CLI call, holds an exclusive lock on `lock`
// for its whole read-modify-write, so `ready` can never race the daemon into launching the same identity twice. The
// state is written before the journal; a transaction that starts and finds the journal behind replays the previous
// transaction's events from `last_events`, so `wait` never misses an end that the state already records.
// ---------------------------------------------------------------------------------------------------------------

async function load(dir: string): Promise<Schedule> {
  let text: string;
  try {
    text = await Deno.readTextFile(statePath(dir));
  } catch (error) {
    if (error instanceof Deno.errors.NotFound) throw new ScheduleError(`No schedule in ${dir}; run init first`);
    throw error;
  }
  let data;
  try {
    data = JSON.parse(text);
  } catch {
    throw new ScheduleError(`Corrupt schedule state ${statePath(dir)}`);
  }
  requireThat(data.version === 1, `Unsupported schedule state in ${dir}`);
  data.last_events ??= [];
  return data as Schedule;
}

/**
 * Complete journal lines only. A trailing fragment without a newline is an append still in progress (or one cut
 * off by a crash) and is never parsed, so a reader cannot fail on it.
 */
async function journal(dir: string): Promise<Event[]> {
  const text = await Deno.readTextFile(eventsPath(dir));
  const complete = text.slice(0, text.lastIndexOf("\n") + 1);
  return complete.split("\n").filter((line) => line.trim()).map((line) => JSON.parse(line) as Event);
}

/** Under the lock: drop a crash-cut fragment, then append any events the state has but the journal lacks. */
async function repairJournal(dir: string, state: Schedule) {
  const text = await Deno.readTextFile(eventsPath(dir));
  const cut = text.lastIndexOf("\n") + 1;
  if (cut !== text.length) await Deno.truncate(eventsPath(dir), new TextEncoder().encode(text.slice(0, cut)).length);
  const last = (await journal(dir)).at(-1)?.seq ?? 0;
  const missing = state.last_events.filter((e) => e.seq > last);
  if (missing.length) await append(dir, missing);
}

async function append(dir: string, events: Event[]) {
  await Deno.writeTextFile(eventsPath(dir), events.map((e) => JSON.stringify(e)).join("\n") + "\n", { append: true });
}

/**
 * Run one locked transaction. `work` returns events to journal and whether it changed anything; an unchanged
 * transaction writes nothing, so an idle daemon does not rewrite state every second.
 */
async function locked<T>(
  dir: string,
  work: (s: Schedule) =>
    | { events?: Pending[]; changed?: boolean; result?: T }
    | Promise<
      { events?: Pending[]; changed?: boolean; result?: T }
    >,
): Promise<T | undefined> {
  const lock = await Deno.open(join(dir, "lock"), { create: true, write: true });
  try {
    await lock.lock(true);
    const state = await load(dir);
    await repairJournal(dir, state);
    const { events = [], changed = false, result } = await work(state);
    if (!events.length && !changed) return result;
    const stamped = events.map((e) => ({ seq: ++state.seq, time: now(), ...e }) as Event);
    if (stamped.length) state.last_events = stamped;
    const tmp = statePath(dir) + ".tmp";
    await Deno.writeTextFile(tmp, JSON.stringify(state, null, 1) + "\n");
    await Deno.rename(tmp, statePath(dir));
    if (stamped.length) await append(dir, stamped);
    return result;
  } finally {
    lock.close();
  }
}

// ---------------------------------------------------------------------------------------------------------------
// Processes
//
// Liveness is always by pid or process group through `kill -0`, never by matching command lines: a pattern match
// also finds the watcher that is looking. The shell builtin is used so no `kill` binary needs to be on PATH; it is
// called without `--` because dash's builtin rejects it, and a negative argument names a process group. A pid that
// now belongs to another user fails with EPERM and correctly counts as not ours.
// ---------------------------------------------------------------------------------------------------------------

async function signalZero(target: string): Promise<boolean> {
  const probe = await new Deno.Command("/bin/sh", {
    args: ["-c", 'kill -0 "$1" 2>/dev/null', "sh", target],
    stdout: "null",
    stderr: "null",
  }).output();
  return probe.success;
}

/** Any process left in the turn's group: the wrapper, the launch script, or anything the script started. */
const groupAlive = (pgid: number) => signalZero(`-${pgid}`);

/** The daemon counts as alive only with a fresh heartbeat and a live pid, so a reused pid cannot impersonate it. */
async function daemonAlive(dir: string): Promise<boolean> {
  try {
    const { pid, beat } = JSON.parse(await Deno.readTextFile(daemonPath(dir)));
    return Date.now() - Date.parse(beat) < HEARTBEAT_STALE_MS && await signalZero(String(pid));
  } catch (error) {
    if (error instanceof Deno.errors.NotFound || error instanceof SyntaxError) return false;
    throw error;
  }
}

/**
 * Physical line count of a findings log, the same rule `evidence events` uses for its offsets: newline-terminated
 * lines plus an unterminated last line. Missing or unreadable logs give null rather than failing the scheduler,
 * because one identity's bad path must never stop other identities' turns.
 */
async function lineCount(path: string | undefined): Promise<number | null> {
  if (!path) return null;
  try {
    const bytes = await Deno.readFile(path);
    const newlines = bytes.reduce((n, b) => n + (b === 10 ? 1 : 0), 0);
    return newlines + (bytes.length && bytes[bytes.length - 1] !== 10 ? 1 : 0);
  } catch {
    return null;
  }
}

async function readNumber(path: string): Promise<number | undefined> {
  try {
    const value = Number((await Deno.readTextFile(path)).trim());
    return Number.isSafeInteger(value) ? value : undefined;
  } catch (error) {
    if (error instanceof Deno.errors.NotFound) return undefined;
    throw error;
  }
}

// ---------------------------------------------------------------------------------------------------------------
// Orchestrator commands
// ---------------------------------------------------------------------------------------------------------------

function identity(state: Schedule, id: string): Identity {
  const entry = state.identities[id];
  requireThat(entry, `Unknown identity ${id}`);
  return entry;
}

function absolute(path: string, what: string): string {
  requireThat(path.startsWith("/"), `${what} must be an absolute path`);
  return path;
}

async function init(dir: string, max: number) {
  requireThat(Number.isSafeInteger(max) && max >= 1, "--max must be a positive integer");
  requireThat(!(await Deno.stat(statePath(dir)).catch(() => null)), `A schedule already exists in ${dir}`);
  await Deno.mkdir(join(dir, "turns"), { recursive: true });
  const fresh: Schedule = { version: 1, max, seq: 0, identities: {}, last_events: [] };
  await Deno.writeTextFile(statePath(dir), JSON.stringify(fresh, null, 1) + "\n", { createNew: true });
  await Deno.writeTextFile(eventsPath(dir), "", { createNew: true });
}

async function add(dir: string, id: string, findings?: string) {
  requireThat(/^[A-Za-z0-9._-]+$/.test(id), "Identity names use letters, digits, '.', '_' and '-'");
  await locked(dir, (state) => {
    requireThat(!state.identities[id], `Identity ${id} already exists`);
    state.identities[id] = {
      state: "new",
      turns: [],
      ...(findings ? { findings: absolute(findings, "--findings") } : {}),
    };
    return { events: [{ kind: "added", id }] };
  });
}

/**
 * Queue an identity's next turn. This is the only way work enters the queue, and it refuses unless the identity is
 * new or its previous turn has ended and is waiting on the parent, which is what keeps one live turn per identity.
 */
async function ready(dir: string, id: string, script: string, label: string, audit?: string) {
  absolute(script, "--script");
  requireThat(label.trim(), "--label must not be empty");
  const info = await Deno.stat(script).catch(() => null);
  requireThat(info?.isFile, `Launch script ${script} does not exist`);
  await locked(dir, (state) => {
    const entry = identity(state, id);
    requireThat(
      entry.state === "new" || entry.state === "awaiting-parent",
      `Identity ${id} is ${entry.state}; only a new or awaiting-parent identity can queue a turn`,
    );
    const n = entry.turns.length + 1;
    entry.turns.push({ n, label, script, queued_at: now(), ...(audit ? { audit: absolute(audit, "--audit") } : {}) });
    entry.state = "queued";
    return { events: [{ kind: "queued", id, turn: n, label }] };
  });
}

/** Take back a turn queued by mistake, before the daemon launches it. */
async function cancel(dir: string, id: string) {
  await locked(dir, (state) => {
    const entry = identity(state, id);
    requireThat(entry.state === "queued", `Identity ${id} is ${entry.state}; only a queued turn can be cancelled`);
    const turn = entry.turns.pop()!;
    entry.state = entry.turns.length ? "awaiting-parent" : "new";
    return { events: [{ kind: "cancelled", id, turn: turn.n }] };
  });
}

/** Record the parent's decision that an identity needs no further turns. Only an idle identity can be closed. */
async function finish(dir: string, id: string, outcome: string, reason: string) {
  requireThat(
    outcome === "complete" || outcome === "stopped-incomplete",
    "--state must be complete or stopped-incomplete",
  );
  requireThat(reason.trim(), "--reason must not be empty");
  await locked(dir, (state) => {
    const entry = identity(state, id);
    requireThat(
      entry.state === "new" || entry.state === "awaiting-parent",
      `Identity ${id} is ${entry.state}; finish it only between turns`,
    );
    entry.state = outcome as State;
    entry.reason = reason;
    return { events: [{ kind: outcome, id, reason }] };
  });
}

async function cap(dir: string, max: number) {
  requireThat(Number.isSafeInteger(max) && max >= 1, "cap needs a positive integer");
  await locked(dir, (state) => {
    state.max = max;
    return { events: [{ kind: "cap", max }] };
  });
}

// ---------------------------------------------------------------------------------------------------------------
// The daemon
//
// A launch is committed to state before its process exists, so a crash can never leave a running turn that the
// state still calls queued (which a later daemon would launch a second time). Each turn then runs in its own session
// and process group under a tiny wrapper that records its pid first, sends the script's output to files, and writes
// the script's exit status to `<base>.exit` last. Its own session keeps a signal to the daemon's process group (a
// harness stopping its background job, Ctrl-C) from reaching the turn, and output in files keeps a dying daemon from
// killing it with SIGPIPE. A turn ends when its exit file exists; it is lost only when its whole process group is
// gone and the exit file is still absent after a second look.
// ---------------------------------------------------------------------------------------------------------------

const WRAPPER = 'echo $$ >"$2.pid.tmp" && mv "$2.pid.tmp" "$2.pid"; ' +
  '/bin/sh "$1" >"$2.out" 2>"$2.err" </dev/null; s=$?; ' +
  'printf "%s\\n" "$s" >"$2.exit.tmp" && mv "$2.exit.tmp" "$2.exit"';

type TickOptions = { startGraceMs?: number };

/**
 * One scheduling pass: record ended turns, commit launches for free slots, spawn them, then record their pids.
 * Returns how many turns are running afterwards.
 */
export async function tick(dir: string, options: TickOptions = {}): Promise<number> {
  const grace = options.startGraceMs ?? START_GRACE_MS;
  const toSpawn = (await locked(dir, async (state) => {
    const events: Pending[] = [];
    let changed = false;
    for (const [id, entry] of Object.entries(state.identities)) {
      if (entry.state !== "running") continue;
      const turn = entry.turns[entry.turns.length - 1];
      const base = turnBase(dir, id, turn.n);
      let exit: number | "lost" | undefined = await readNumber(`${base}.exit`);
      if (exit === undefined) {
        const pid = turn.pid ?? await readNumber(`${base}.pid`);
        if (pid !== undefined && turn.pid === undefined) {
          turn.pid = pid;
          changed = true;
        }
        if (pid !== undefined) {
          if (await groupAlive(pid)) continue;
          // The script may have finished between the first read and the liveness check.
          exit = (await readNumber(`${base}.exit`)) ?? "lost";
        } else if (Date.now() - Date.parse(turn.launched_at!) > grace) {
          exit = "lost";
        } else continue;
      }
      turn.exit = exit;
      turn.ended_at = now();
      turn.findings_end = await lineCount(entry.findings);
      entry.state = "awaiting-parent";
      events.push({ kind: "ended", id, turn: turn.n, label: turn.label, exit });
    }
    const queued = Object.entries(state.identities)
      .filter(([, e]) => e.state === "queued")
      .sort(([, a], [, b]) => a.turns.at(-1)!.queued_at.localeCompare(b.turns.at(-1)!.queued_at));
    let running = Object.values(state.identities).filter((e) => e.state === "running").length;
    const launches: [string, Turn][] = [];
    for (const [id, entry] of queued) {
      if (running >= state.max) break;
      const turn = entry.turns[entry.turns.length - 1];
      turn.findings_start = await lineCount(entry.findings);
      turn.launched_at = now();
      entry.state = "running";
      running++;
      launches.push([id, turn]);
      events.push({ kind: "launching", id, turn: turn.n, label: turn.label });
    }
    return { events, changed, result: launches };
  }))!;

  const started: [string, number, number][] = [];
  const failed: [string, number, string][] = [];
  for (const [id, turn] of toSpawn) {
    try {
      const child = new Deno.Command("/bin/sh", {
        args: ["-c", WRAPPER, "sh", turn.script, turnBase(dir, id, turn.n)],
        stdin: "null",
        stdout: "null",
        stderr: "null",
        detached: true,
      }).spawn();
      // Reap the wrapper when it exits and do not keep the daemon alive for it.
      child.status.catch(() => {});
      child.unref();
      started.push([id, turn.n, child.pid]);
    } catch (error) {
      failed.push([id, turn.n, String(error)]);
    }
  }

  return (await locked(dir, (state) => {
    const events: Pending[] = [];
    let changed = false;
    for (const [id, n, pid] of started) {
      const turn = state.identities[id]?.turns.at(-1);
      if (turn?.n !== n || turn.pid !== undefined) continue;
      turn.pid = pid;
      changed = true;
      events.push({ kind: "launched", id, turn: n, pid });
    }
    for (const [id, n, message] of failed) {
      const entry = state.identities[id];
      const turn = entry?.turns.at(-1);
      if (turn?.n !== n || entry.state !== "running") continue;
      turn.exit = "lost";
      turn.ended_at = now();
      entry.state = "awaiting-parent";
      events.push({ kind: "ended", id, turn: n, label: turn.label, exit: "lost", error: message });
    }
    return {
      events,
      changed,
      result: Object.values(state.identities).filter((e) => e.state === "running").length,
    };
  }))!;
}

/**
 * Run until `stop` is requested. Refuses to start beside a live daemon for the same schedule, since two daemons
 * would each launch from the same queue. It never kills turns on exit; they keep running and a later daemon
 * records how they ended. A lifetime lock owns the daemon slot; heartbeat reads alone cannot exclude concurrent
 * starts, and a paused daemon must retain ownership even if its heartbeat becomes stale.
 */
async function run(dir: string, intervalMs = 1000, options: TickOptions = {}) {
  await load(dir);
  using lease = await Deno.open(join(dir, "daemon.lock"), { create: true, write: true });
  requireThat(await lease.tryLock(true), `A scheduler daemon is already running for ${dir}`);
  requireThat(!(await daemonAlive(dir)), `A scheduler daemon is already running for ${dir}`);
  // Readers must see a complete heartbeat. Truncate-and-write briefly made a live daemon appear absent.
  const beat = async () => {
    const path = daemonPath(dir);
    await Deno.writeTextFile(path + ".tmp", JSON.stringify({ pid: Deno.pid, beat: now() }) + "\n");
    await Deno.rename(path + ".tmp", path);
  };
  // Clear an old stop request before publishing readiness, so a new stop cannot be erased during startup.
  await Deno.remove(stopPath(dir)).catch(() => {});
  await locked(dir, () => ({ events: [{ kind: "daemon-started", pid: Deno.pid }] }));
  try {
    await beat();
    while (!(await Deno.stat(stopPath(dir)).catch(() => null))) {
      await tick(dir, options);
      await beat();
      await new Promise((resolve) => setTimeout(resolve, intervalMs));
    }
  } finally {
    await locked(dir, () => ({ events: [{ kind: "daemon-exit", pid: Deno.pid }] }));
    await Deno.remove(daemonPath(dir)).catch(() => {});
  }
}

// ---------------------------------------------------------------------------------------------------------------
// Waking the parent and showing state
// ---------------------------------------------------------------------------------------------------------------

/**
 * Block until a turn ends or the daemon exits after `after`, then print those events and exit 0. Run it in the
 * background so the harness's completion notice wakes the parent. It exits 3 instead of waiting forever when no
 * daemon is alive, which is the failure a silent watcher would otherwise hide, and 4 when `timeoutSeconds` passes.
 */
async function wait(dir: string, after: number, timeoutSeconds?: number, intervalMs = 1000) {
  await load(dir);
  const deadline = timeoutSeconds === undefined ? Infinity : Date.now() + timeoutSeconds * 1000;
  const wanted = async () =>
    (await journal(dir)).filter((e) => e.seq > after && (e.kind === "ended" || e.kind === "daemon-exit"));
  for (;;) {
    let events = await wanted();
    if (!events.length && !(await daemonAlive(dir))) {
      // The daemon may have journaled an end and exited between the read and the liveness check.
      events = await wanted();
      if (!events.length) {
        console.log(JSON.stringify({ last_seq: after, events: [], daemon: "not running" }));
        return 3;
      }
    }
    if (events.length) {
      console.log(JSON.stringify({ last_seq: Math.max(...events.map((e) => e.seq)), events }));
      return 0;
    }
    if (Date.now() >= deadline) {
      console.log(JSON.stringify({ last_seq: after, events: [], timed_out: true }));
      return 4;
    }
    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }
}

/** Summarize one delivery audit report without passing its ranges or references into the parent's context. */
async function auditSummary(path: string | undefined) {
  if (!path) return null;
  try {
    const report = JSON.parse(await Deno.readTextFile(path));
    const missing = (report.sources ?? []).reduce(
      (n: number, s: { missing?: unknown[] }) => n + (s.missing?.length ?? 0),
      0,
    );
    return {
      status: report.status,
      missing_ranges: missing,
      rejected: (report.rejected ?? []).length,
      unfinished_calls: report.unfinished_calls,
      unrecognized: Object.keys(report.unrecognized ?? {}).length,
    };
  } catch (error) {
    if (error instanceof Deno.errors.NotFound) return { status: "absent" };
    if (error instanceof SyntaxError) return { status: "unreadable" };
    throw error;
  }
}

/**
 * The parent's compact view: the journal position, counts by state, free slots, every identity awaiting a decision
 * (oldest first, with exit status, audit summary and the findings-log lines its turn started and ended at), and the
 * running turns with their pids and output files. `all` adds every identity.
 */
export async function status(dir: string, all = false) {
  const state = await load(dir);
  const counts: Record<string, number> = {};
  const rows = [];
  for (const [id, entry] of Object.entries(state.identities)) {
    counts[entry.state] = (counts[entry.state] ?? 0) + 1;
    const turn = entry.turns.at(-1);
    const base = turn ? turnBase(dir, id, turn.n) : undefined;
    rows.push({
      id,
      state: entry.state,
      turn: turn?.n ?? 0,
      label: turn?.label,
      exit: turn?.exit,
      launched_at: turn?.launched_at,
      ended_at: turn?.ended_at,
      findings_start_line: turn?.findings_start ?? null,
      findings_end_line: turn?.findings_end ?? null,
      ...(entry.state === "running" ? { pid: turn?.pid, out: `${base}.out`, err: `${base}.err` } : {}),
      ...(entry.state === "awaiting-parent" ? { audit: await auditSummary(turn?.audit) } : {}),
      reason: entry.reason,
    });
  }
  const waiting = rows.filter((r) => r.state === "awaiting-parent").sort((a, b) =>
    String(a.ended_at).localeCompare(String(b.ended_at))
  );
  return {
    seq: state.seq,
    daemon: (await daemonAlive(dir)) ? "running" : "not running",
    max: state.max,
    counts,
    // Free slots alongside queued turns mean the daemon is not running or has not ticked yet.
    idle_capacity: Math.max(0, state.max - (counts["running"] ?? 0)),
    oldest_waiting_since: waiting[0]?.ended_at ?? null,
    awaiting_parent: waiting,
    running: rows.filter((r) => r.state === "running"),
    ...(all ? { identities: rows } : {}),
  };
}

// ---------------------------------------------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------------------------------------------

/** Each command's positional count (after DIR) and accepted options; `all` is the only boolean flag. */
const COMMANDS = new Map<string, [number, string[]]>([
  ["init", [0, ["max"]]],
  ["add", [1, ["findings"]]],
  ["ready", [1, ["script", "label", "audit"]]],
  ["cancel", [1, []]],
  ["finish", [1, ["state", "reason"]]],
  ["cap", [1, []]],
  ["run", [0, []]],
  ["stop", [0, []]],
  ["wait", [0, ["after", "timeout"]]],
  ["status", [0, ["all"]]],
]);

const USAGE = "Usage: schedule init|add|ready|cancel|finish|cap|run|stop|wait|status DIR ...; see scheduling.md";

/** Split arguments into positionals and `--name value` options, refusing anything the command does not accept. */
function parse(command: string, args: string[]): { positional: string[]; flags: Map<string, string> } {
  const spec = COMMANDS.get(command);
  requireThat(spec, USAGE);
  const positional: string[] = [];
  const flags = new Map<string, string>();
  for (let i = 0; i < args.length; i++) {
    if (!args[i].startsWith("--")) {
      positional.push(args[i]);
      continue;
    }
    const name = args[i].slice(2);
    requireThat(spec[1].includes(name), `${command} does not accept --${name}`);
    requireThat(!flags.has(name), `--${name} given twice`);
    if (name === "all") flags.set(name, "true");
    else {
      requireThat(i + 1 < args.length, `--${name} needs a value`);
      flags.set(name, args[++i]);
    }
  }
  requireThat(positional.length === spec[0] + 1, `${command} takes DIR${spec[0] ? " and one more argument" : ""}`);
  return { positional, flags };
}

async function main(args: string[]): Promise<number> {
  const [command, ...rest] = args;
  requireThat(command, USAGE);
  const { positional, flags } = parse(command, rest);
  const dir = absolute(positional[0], "The schedule directory");
  const integerFlag = (name: string, fallback?: number) => {
    const raw = flags.get(name);
    if (raw === undefined && fallback !== undefined) return fallback;
    requireThat(raw !== undefined && /^\d+$/.test(raw), `--${name} must be a non-negative integer`);
    return Number(raw);
  };
  switch (command) {
    case "init":
      await init(dir, integerFlag("max"));
      return 0;
    case "add":
      await add(dir, positional[1], flags.get("findings"));
      return 0;
    case "ready":
      requireThat(flags.get("script") && flags.get("label"), "ready needs --script and --label");
      await ready(dir, positional[1], flags.get("script")!, flags.get("label")!, flags.get("audit"));
      return 0;
    case "cancel":
      await cancel(dir, positional[1]);
      return 0;
    case "finish":
      await finish(dir, positional[1], flags.get("state") ?? "", flags.get("reason") ?? "");
      return 0;
    case "cap":
      requireThat(/^\d+$/.test(positional[1]), "cap needs a positive integer");
      await cap(dir, Number(positional[1]));
      return 0;
    case "run":
      await run(dir);
      return 0;
    case "stop":
      await load(dir);
      await Deno.writeTextFile(stopPath(dir), "");
      return 0;
    case "wait": {
      const timeout = flags.get("timeout");
      requireThat(
        timeout === undefined || (Number.isFinite(Number(timeout)) && Number(timeout) > 0),
        "--timeout must be a positive number of seconds",
      );
      return await wait(dir, integerFlag("after", 0), timeout === undefined ? undefined : Number(timeout));
    }
    case "status":
      console.log(JSON.stringify(await status(dir, flags.has("all"))));
      return 0;
  }
  throw new ScheduleError(USAGE);
}

/** Shared by direct source execution and the generated compiled entrypoint. */
export async function runCli(args: string[] = Deno.args) {
  try {
    Deno.exit(await main(args));
  } catch (error) {
    if (!(error instanceof ScheduleError)) throw error;
    console.error(`PERSISTENT_REVIEW_SCHEDULE_FAILED: ${error.message}`);
    Deno.exit(2);
  }
}

if (import.meta.main) await runCli();

// ---------------------------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------------------------

/** A fresh schedule directory and a helper that writes launch scripts into it. */
async function fixture(work: (dir: string, script: (body: string) => Promise<string>) => Promise<void>) {
  const dir = await Deno.makeTempDir();
  let n = 0;
  try {
    await work(dir, async (body) => {
      const path = join(dir, `script-${++n}.sh`);
      await Deno.writeTextFile(path, body);
      return path;
    });
  } finally {
    await Deno.remove(dir, { recursive: true });
  }
}

/** Tick until no turn is running, so tests observe ended turns without sleeping a fixed time. */
async function drain(dir: string, options: TickOptions = {}) {
  for (let i = 0; i < 300 && (await tick(dir, options)) > 0; i++) await new Promise((r) => setTimeout(r, 20));
}

/** Fail with the stuck phase instead of leaving CI waiting forever for a daemon lifecycle operation. */
async function bounded<T>(work: Promise<T>, phase: string): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      work,
      new Promise<never>((_, reject) => {
        timer = setTimeout(() => reject(new Error(`Timed out waiting for ${phase}`)), 2000);
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}

/** Poll observable readiness with a deadline; a missing pid or heartbeat must fail the test, not hang the suite. */
async function until(check: () => Promise<boolean>, phase: string) {
  const end = Date.now() + 2000;
  while (!(await check())) {
    if (Date.now() >= end) throw new Error(`Timed out waiting for ${phase}`);
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
}

async function refuses(work: () => Promise<unknown>, text: string) {
  try {
    await work();
  } catch (error) {
    if (error instanceof ScheduleError && error.message.includes(text)) return;
    throw error;
  }
  throw new Error(`expected refusal containing ${text}`);
}

function same(actual: unknown, expected: unknown) {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(`Expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}

/** Overwrite fields of the state directly, to model what an earlier crashed daemon left behind. */
async function edit(dir: string, change: (state: Schedule) => void) {
  await locked(dir, (state) => {
    change(state);
    return { changed: true };
  });
}

/**
 * The scheduler exists to stop two failures from a real run: free capacity idling next to queued work, and the risk
 * of two live turns on one reviewer identity. This pins the capacity limit, queue order, the one-live-turn refusal,
 * cancellation, and that an ended turn waits for the parent instead of advancing on its own.
 */
Deno.test("turns launch up to the cap in queue order and park for the parent", () =>
  fixture(async (dir, script) => {
    await init(dir, 1);
    for (const id of ["a", "b", "c"]) await add(dir, id);
    await ready(dir, "a", await script("exit 0"), "initial");
    await ready(dir, "b", await script("exit 7"), "initial");
    await ready(dir, "c", await script("exit 0"), "initial");
    await cancel(dir, "c");
    same((await load(dir)).identities.c, { state: "new", turns: [] });
    await refuses(() => ready(dir, "a", "/bin/true", "again"), "only a new or awaiting-parent");
    same(await tick(dir), 1);
    same((await status(dir)).counts, { running: 1, queued: 1, new: 1 });
    await drain(dir);
    const view = await status(dir, true);
    same(view.counts, { "awaiting-parent": 2, new: 1 });
    same(view.awaiting_parent.map((r) => [r.id, r.exit]), [["a", 0], ["b", 7]]);
    same(view.idle_capacity, 1);
    await refuses(() => finish(dir, "a", "done", "x"), "complete or stopped-incomplete");
    await finish(dir, "a", "complete", "empty discovery pass");
    await refuses(() => ready(dir, "a", "/bin/true", "more"), "is complete");
    await cap(dir, 3);
    same((await load(dir)).max, 3);
  }));

/**
 * A turn may finish between the exit-file read and the liveness check. That must still record its real exit status:
 * a successful turn reported as lost would send the parent to repair a turn that worked.
 */
Deno.test("an exit file that appears after the group dies still decides the outcome", () =>
  fixture(async (dir, script) => {
    await init(dir, 1);
    await add(dir, "a");
    await ready(dir, "a", await script("exit 0"), "initial");
    await tick(dir);
    // Point the turn at a group that does not exist; only the second exit-file read can save it from "lost".
    await edit(dir, (state) => state.identities.a.turns[0].pid = 2147483646);
    await until(async () => !!(await Deno.stat(join(dir, "turns", "a-1.exit")).catch(() => null)), "turn exit file");
    await drain(dir);
    same((await status(dir)).awaiting_parent.map((r) => r.exit), [0]);
  }));

/**
 * A crash can leave a turn the state calls running with nothing behind it: either its group is gone without an exit
 * file, or the launch was committed and the daemon died before spawning. Both must end as lost for the parent to
 * judge, never be launched again by the scheduler, and never block the slot forever.
 */
Deno.test("turns with no exit file and no live group, or never spawned, end as lost", () =>
  fixture(async (dir) => {
    await init(dir, 2);
    await add(dir, "a");
    await add(dir, "b");
    await edit(dir, (state) => {
      const turn = (n: number): Turn => ({
        n,
        label: "initial",
        script: "/bin/true",
        queued_at: now(),
        launched_at: new Date(Date.now() - 60_000).toISOString(),
      });
      state.identities.a = { state: "running", turns: [{ ...turn(1), pid: 2147483646 }] };
      state.identities.b = { state: "running", turns: [turn(1)] };
    });
    await tick(dir);
    same((await status(dir)).awaiting_parent.map((r) => [r.id, r.exit]), [["a", "lost"], ["b", "lost"]]);
    same((await journal(dir)).filter((e) => e.kind === "launching").length, 0);
  }));

/**
 * One identity with a broken findings path must not stop the daemon or any other identity's turns; the offset is
 * just unknown for that identity.
 */
Deno.test("an unreadable findings path does not wedge scheduling", () =>
  fixture(async (dir, script) => {
    await init(dir, 2);
    await add(dir, "a", dir);
    await add(dir, "b");
    await ready(dir, "a", await script("exit 0"), "initial");
    await ready(dir, "b", await script("exit 0"), "initial");
    await drain(dir);
    same((await status(dir)).awaiting_parent.map((r) => [r.id, r.exit, r.findings_start_line]), [
      ["a", 0, null],
      ["b", 0, null],
    ]);
  }));

/**
 * The parent learns which log lines belong to a turn from the offsets recorded at launch and end, counted the way
 * `evidence events` counts them, and reads the audit summary without the report's ranges entering its context.
 */
Deno.test("status reports findings offsets, a compact audit summary and the journal position", () =>
  fixture(async (dir, script) => {
    await init(dir, 1);
    const log = join(dir, "findings.jsonl"), audit = join(dir, "audit.json");
    await Deno.writeTextFile(log, "{}\n{}");
    await add(dir, "a", log);
    await ready(dir, "a", await script(`printf '\\n{}\\n' >> '${log}'`), "initial", audit);
    await Deno.writeTextFile(audit, JSON.stringify({ status: "incomplete", sources: [{ missing: [[1, 2]] }] }));
    await drain(dir);
    const view = await status(dir);
    const row = view.awaiting_parent[0];
    same([row.findings_start_line, row.findings_end_line], [2, 3]);
    same(row.audit, { status: "incomplete", missing_ranges: 1, rejected: 0, unrecognized: 0 });
    same(view.seq, (await journal(dir)).at(-1)?.seq);
  }));

/**
 * `wait` is how the parent gets woken. It must return the ended events after the given sequence number, ignore an
 * append still in progress, and exit instead of hanging when no daemon is alive, which is the silent-stall failure a
 * liveness check has to catch.
 */
Deno.test("wait returns new ended events, skips partial lines, and refuses to wait on a dead daemon", () =>
  fixture(async (dir, script) => {
    await init(dir, 1);
    await add(dir, "a");
    await ready(dir, "a", await script("exit 0"), "initial");
    await drain(dir);
    const ended = (await journal(dir)).find((e) => e.kind === "ended")!;
    await Deno.writeTextFile(eventsPath(dir), '{"seq":99,"kind":"ended"', { append: true });
    same(await wait(dir, ended.seq - 1, 1, 10), 0);
    same(await wait(dir, ended.seq, 1, 10), 3);
    await Deno.writeTextFile(daemonPath(dir), JSON.stringify({ pid: Deno.pid, beat: now() }));
    same(await wait(dir, ended.seq, 0.05, 10), 4);
    // A stale heartbeat means the daemon is gone even though the recorded pid is alive.
    await Deno.writeTextFile(daemonPath(dir), JSON.stringify({ pid: Deno.pid, beat: "2000-01-01T00:00:00Z" }));
    same(await wait(dir, ended.seq, 1, 10), 3);
    // The next transaction drops the crash-cut fragment instead of appending after it.
    await cap(dir, 2);
    same((await journal(dir)).at(-1)?.kind, "cap");
  }));

/**
 * A crash between the state write and the journal append must not hide an end from `wait`: the next transaction
 * replays the missing events from the state.
 */
Deno.test("events the state recorded but the journal lacks are replayed", () =>
  fixture(async (dir) => {
    await init(dir, 1);
    await add(dir, "a");
    await Deno.writeTextFile(eventsPath(dir), "");
    same((await journal(dir)).length, 0);
    await cap(dir, 2);
    same((await journal(dir)).map((e) => e.kind), ["added", "cap"]);
  }));

/**
 * End to end through the daemon: a turn queued after the daemon started is launched without any further parent call,
 * `wait` wakes on its end, a second daemon is refused, and `stop` ends the daemon with a journal entry. Each lifecycle
 * wait is bounded and cleanup runs on assertion failure, so an accidentally accepted twin cannot hang CI.
 */
Deno.test("the daemon launches newly ready turns, wakes waiters, refuses a twin, and stops on request", () =>
  fixture(async (dir, script) => {
    await init(dir, 2);
    await add(dir, "a");
    const daemon = run(dir, 10);
    try {
      await until(() => daemonAlive(dir), "daemon startup");
      await bounded(refuses(() => run(dir, 10), "already running"), "duplicate daemon refusal");
      await ready(dir, "a", await script("exit 0"), "initial");
      same(await wait(dir, 0, 1, 10), 0);
    } finally {
      await Deno.writeTextFile(stopPath(dir), "");
      await bounded(daemon, "daemon shutdown");
    }
    same((await journal(dir)).at(-1)?.kind, "daemon-exit");
  }));

/** A daemon owns its slot before publishing a heartbeat and while paused; neither absence nor staleness admits a twin. */
Deno.test("the daemon lease refuses a twin without a usable heartbeat and releases on exit", () =>
  fixture(async (dir) => {
    await init(dir, 1);
    const lease = await Deno.open(join(dir, "daemon.lock"), { create: true, write: true });
    await lease.lock(true);
    try {
      for (const heartbeat of ["", JSON.stringify({ pid: Deno.pid, beat: "2000-01-01T00:00:00Z" })]) {
        await Deno.writeTextFile(daemonPath(dir), heartbeat);
        const twin = run(dir, 10);
        try {
          await bounded(refuses(() => twin, "already running"), "daemon lease refusal");
        } finally {
          await Deno.writeTextFile(stopPath(dir), "");
          await bounded(twin.catch(() => {}), "duplicate daemon cleanup");
        }
      }
    } finally {
      lease.close();
    }
    const daemon = run(dir, 10);
    try {
      await until(() => daemonAlive(dir), "replacement daemon startup");
    } finally {
      await Deno.writeTextFile(stopPath(dir), "");
      await bounded(daemon, "replacement daemon shutdown");
    }
  }));

/**
 * A mistyped option used to be ignored, silently dropping the findings offsets or audit summary for a whole run. The
 * CLI must refuse unknown options, missing values and malformed numbers before touching state.
 */
Deno.test("the CLI refuses unknown options and malformed values", () =>
  fixture(async (dir) => {
    await main(["init", dir, "--max", "1"]);
    await refuses(() => main(["init", dir, "--max", "1"]), "already exists");
    await refuses(() => main(["add", dir, "a", "--findngs", "/x"]), "does not accept --findngs");
    await refuses(() => main(["wait", dir, "--timeout", "abc"]), "positive number");
    await refuses(() => main(["wait", dir, "--after", "-1"]), "non-negative integer");
    await refuses(() => main(["status", join(dir, "missing")]), "run init first");
    await refuses(() => main(["cap", dir]), "cap takes DIR and one more argument");
  }));

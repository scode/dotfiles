#!/usr/bin/env -S deno run -A
/**
 * A stand-in for the `gh` CLI, for the end-to-end tests in `e2e.test.ts`. It implements exactly the `gh` surface the
 * jjstack script uses, on top of a real bare Git repository that plays GitHub's copy of the repo, with PR state kept in
 * a JSON file. It is a model, not a mock: pushes land in the bare repo through real Git, squash merges are real
 * `git merge-tree` merges, mergeability is computed from the actual trees, and `--jq` filters run through real `jq`.
 *
 * NOTE: This is deliberately a small model of GitHub. It covers what jjstack's guards depend on (PR heads and bases,
 * draft state, squash merges with `--match-head-commit`, automatic head-branch deletion with child retargeting, check
 * rollups, ready events, check suites) and lets a test script the awkward parts (check rollups that change over time,
 * transient merge rejections, someone else pushing to the default branch). Anything else exits 1 with "unsupported", so
 * a script change that starts using a new `gh` feature fails loudly here instead of passing against a fake that
 * silently does nothing. Real-GitHub evals still matter: this model is only as faithful as what it was taught.
 *
 * Configuration comes from the environment the test sets for the script's child process: `FAKE_GH_STATE` (the JSON
 * state file), `FAKE_GH_REMOTE` (the bare repo), and `FAKE_GH_EVENTS` (an append-only log every call and push goes to,
 * so a test can assert ordering).
 */

/**
 * One read of a PR's check rollup. `startedAt` pins when a run started (for a run left over from before the PR left
 * draft, say); without it a run reads as started at the moment of the read.
 */
export type CheckFrame = { name: string; status: string; conclusion: string; startedAt?: string }[];

export type Pr = {
  number: number;
  headRefName: string;
  baseRefName: string;
  state: "OPEN" | "MERGED" | "CLOSED";
  isDraft: boolean;
  title: string;
  body: string;
  mergeCommit?: string;
  mergedHead?: string;
  readyAt: string[];
  /** Whether the PR's head branch lives in a fork; tests set it, and PRs the script creates never do. */
  isCrossRepository?: boolean;
};

/** Everything the fake knows, persisted between invocations. Tests write the scripting fields directly. */
export type State = {
  repo: string;
  host: string;
  defaultBranch: string;
  autoDeleteBranches: boolean;
  nextNumber: number;
  prs: Pr[];
  /** Check rollup frames per PR number: each read of the rollup consumes one frame, and the last one repeats. */
  checks: Record<string, CheckFrame[]>;
  /** The rollup a PR without an entry in `checks` shows. */
  defaultChecks: CheckFrame;
  /** Merge rejections to return before really merging: each `gh pr merge` of that PR consumes one message. */
  mergeFailures: Record<string, string[]>;
  /** After a PR merges, add this many commits to the default branch "from elsewhere" (someone else's merge). */
  externalPushAfterMerge: Record<string, number>;
  /**
   * When each head's check suite was created, keyed by commit SHA. GitHub fixes a suite's `created_at` when the suite
   * appears, and the script measures its settle period from it, so the fake records the first read and keeps it.
   */
  suiteCreatedAt: Record<string, string>;
  /** The id of the last workflow run a dispatch created; runs are numbered from 1001. */
  nextRunId?: number;
};

/** A fresh state for repo `o/r` on github.com with green checks, for the tests to adjust. */
export function initialState(): State {
  return {
    repo: "o/r",
    host: "github.com",
    defaultBranch: "main",
    autoDeleteBranches: true,
    nextNumber: 1,
    prs: [],
    checks: {},
    defaultChecks: [{ name: "test", status: "COMPLETED", conclusion: "SUCCESS" }],
    mergeFailures: {},
    externalPushAfterMerge: {},
    suiteCreatedAt: {},
  };
}

class GhError extends Error {}

/** Where `event` appends; set from `FAKE_GH_EVENTS` when run as `gh`, left empty (no logging) when imported. */
let eventsPath = "";

function event(line: string) {
  if (eventsPath) Deno.writeTextFileSync(eventsPath, line + "\n", { append: true });
}

/** Runs git against the bare repo. Returns trimmed stdout; a status not in `ok` throws with stderr. */
function git(remote: string, args: string[], ok = [0]): { code: number; out: string } {
  const r = new Deno.Command("git", {
    args: ["--git-dir", remote, ...args],
    stdin: "null",
    env: {
      GIT_AUTHOR_NAME: "GitHub",
      GIT_AUTHOR_EMAIL: "noreply@github.com",
      GIT_COMMITTER_NAME: "GitHub",
      GIT_COMMITTER_EMAIL: "noreply@github.com",
    },
  }).outputSync();
  const out = new TextDecoder().decode(r.stdout).trim();
  if (!ok.includes(r.code)) throw new GhError(`git ${args.join(" ")} failed: ${new TextDecoder().decode(r.stderr)}`);
  return { code: r.code, out };
}

function refSha(remote: string, branch: string): string | undefined {
  const r = git(remote, ["rev-parse", "--verify", "--quiet", `refs/heads/${branch}^{commit}`], [0, 1]);
  return r.code === 0 ? r.out : undefined;
}

/** A squash of `head` onto `base`'s tip: the tree of their three-way merge, or undefined on a conflict. */
function squashTree(remote: string, baseSha: string, headSha: string): string | undefined {
  const r = git(remote, ["merge-tree", "--write-tree", "--no-messages", baseSha, headSha], [0, 1]);
  return r.code === 0 ? r.out.split("\n")[0] : undefined;
}

function now(): string {
  return new Date().toISOString();
}

/** The JSON `gh pr view --json` would print for one PR, restricted to the requested fields. */
function prJson(s: State, remote: string, pr: Pr, fields: string[]): Record<string, unknown> {
  const head = pr.state === "MERGED" ? pr.mergedHead : refSha(remote, pr.headRefName);
  const all: Record<string, () => unknown> = {
    number: () => pr.number,
    state: () => pr.state,
    headRefName: () => pr.headRefName,
    baseRefName: () => pr.baseRefName,
    headRefOid: () => head ?? "",
    isDraft: () => pr.isDraft,
    isCrossRepository: () => pr.isCrossRepository ?? false,
    title: () => pr.title,
    body: () => pr.body,
    mergeCommit: () => pr.mergeCommit ? { oid: pr.mergeCommit } : null,
    mergeable: () => {
      if (pr.state !== "OPEN") return "UNKNOWN";
      const base = refSha(remote, pr.baseRefName);
      if (!base || !head) return "UNKNOWN";
      return squashTree(remote, base, head) === undefined ? "CONFLICTING" : "MERGEABLE";
    },
    mergeStateStatus: () => "CLEAN",
    statusCheckRollup: () => rollup(s, pr),
  };
  const out: Record<string, unknown> = {};
  for (const f of fields) {
    if (!all[f]) throw new GhError(`unsupported field ${f}`);
    out[f] = all[f]();
  }
  return out;
}

/** The PR's current check rollup: the next scripted frame (consumed on read) or the default, as CheckRun entries. */
function rollup(s: State, pr: Pr): unknown[] {
  const frames = s.checks[String(pr.number)];
  let frame = s.defaultChecks;
  if (frames && frames.length > 0) {
    frame = frames[0];
    if (frames.length > 1) frames.shift();
  }
  const t = now();
  return frame.map((c) => ({
    __typename: "CheckRun",
    name: c.name,
    workflowName: "CI",
    status: c.status,
    conclusion: c.status === "COMPLETED" ? c.conclusion : "",
    startedAt: c.startedAt ?? t,
    completedAt: c.status === "COMPLETED" ? (c.startedAt ?? t) : null,
  }));
}

/**
 * The flags each command may carry, which are exactly the ones the script passes. Anything else is refused, so a new
 * flag on a command the fake already models fails the tests until the model learns what it means.
 */
const ALLOWED: Record<string, string[]> = {
  "repo view": ["--json", "--jq"],
  "pr view": ["-R", "--json", "--jq"],
  "pr list": ["-R", "--state", "--base", "--head", "--limit", "--json", "--jq"],
  "pr create": ["-R", "--draft", "--base", "--head", "--title", "--body-file"],
  "pr ready": ["-R"],
  "pr merge": ["-R", "--squash", "--match-head-commit", "--subject", "--body-file"],
  "api": ["-X", "-f", "-F", "--jq", "--paginate"],
};
const BOOLEANS = new Set(["--draft", "--squash", "--paginate"]);

/** Parses `gh` style flags for `command`, refusing any flag it is not allowed to carry. */
function parseFlags(command: string, args: string[]) {
  const allowed = ALLOWED[command];
  if (!allowed) throw new GhError(`fake gh: unsupported command: ${command}`);
  const flags: Record<string, string[]> = {};
  const pos: string[] = [];
  for (let i = 0; i < args.length; i++) {
    const a = args[i];
    if (!a.startsWith("-")) {
      pos.push(a);
      continue;
    }
    if (!allowed.includes(a)) throw new GhError(`fake gh: unsupported flag ${a} for ${command}`);
    if (BOOLEANS.has(a)) (flags[a] ??= []).push("true");
    else if (i + 1 < args.length) (flags[a] ??= []).push(args[++i]);
    else throw new GhError(`fake gh: flag ${a} needs a value`);
  }
  const one = (k: string) => flags[k]?.[0];
  return { flags, pos, one };
}

/** Applies `--jq` through the real `jq`, printing raw strings the way `gh --jq` does. */
async function emit(value: unknown, jq: string | undefined): Promise<string> {
  const json = JSON.stringify(value);
  if (jq === undefined) return json;
  const p = new Deno.Command("jq", { args: ["-r", jq], stdin: "piped", stdout: "piped", stderr: "piped" }).spawn();
  const w = p.stdin.getWriter();
  await w.write(new TextEncoder().encode(json));
  await w.close();
  const r = await p.output();
  if (r.code !== 0) throw new GhError(`jq failed: ${new TextDecoder().decode(r.stderr)}`);
  return new TextDecoder().decode(r.stdout).trimEnd();
}

function findPr(s: State, n: string | undefined): Pr {
  const pr = s.prs.find((p) => String(p.number) === n);
  if (!pr) throw new GhError(`GraphQL: Could not resolve to a PullRequest with the number of ${n}.`);
  return pr;
}

/** Performs one `gh` invocation against the state and returns what it prints on stdout. */
async function handle(args: string[], s: State, remote: string): Promise<string> {
  const [group, verb, ...rest] = args;
  if (group === "api") return await api(s, remote, [verb, ...rest]);
  const { flags, pos, one } = parseFlags(`${group} ${verb}`, rest);
  const jq = one("--jq");
  const fields = (one("--json") ?? "").split(",").filter(Boolean);

  if (group === "repo" && verb === "view") {
    const full: Record<string, unknown> = {
      nameWithOwner: s.repo,
      url: `https://${s.host}/${s.repo}`,
      defaultBranchRef: { name: s.defaultBranch },
    };
    for (const f of fields) if (!(f in full)) throw new GhError(`fake gh: unsupported field ${f}`);
    return await emit(Object.fromEntries(fields.map((f) => [f, full[f]])), jq);
  }

  if (group === "pr" && verb === "view") {
    return await emit(prJson(s, remote, findPr(s, pos[0]), fields), jq);
  }

  if (group === "pr" && verb === "list") {
    const stateFlag = (one("--state") ?? "open").toUpperCase();
    const list = s.prs.filter((p) =>
      (stateFlag === "ALL" || p.state === stateFlag) &&
      (one("--head") === undefined || p.headRefName === one("--head")) &&
      (one("--base") === undefined || p.baseRefName === one("--base"))
    ).sort((a, b) => b.number - a.number);
    return await emit(list.map((p) => prJson(s, remote, p, fields)), jq);
  }

  if (group === "pr" && verb === "create") {
    const head = one("--head")!;
    const base = one("--base")!;
    if (!refSha(remote, head)) throw new GhError(`pull request create failed: head branch ${head} does not exist`);
    if (!refSha(remote, base)) throw new GhError(`pull request create failed: base branch ${base} does not exist`);
    if (s.prs.some((p) => p.state === "OPEN" && p.headRefName === head)) {
      throw new GhError(`a pull request for branch "${head}" into branch "${base}" already exists`);
    }
    const pr: Pr = {
      number: s.nextNumber++,
      headRefName: head,
      baseRefName: base,
      state: "OPEN",
      isDraft: flags["--draft"] !== undefined,
      title: one("--title") ?? "",
      body: Deno.readTextFileSync(one("--body-file")!),
      readyAt: [],
    };
    s.prs.push(pr);
    return `https://${s.host}/${s.repo}/pull/${pr.number}`;
  }

  if (group === "pr" && verb === "ready") {
    const pr = findPr(s, pos[0]);
    if (pr.isDraft) {
      pr.isDraft = false;
      pr.readyAt.push(now());
    }
    return "";
  }

  if (group === "pr" && verb === "merge") return merge(s, remote, findPr(s, pos[0]), one);

  throw new GhError(`fake gh: unsupported command: ${args.join(" ")}`);
}

/** `gh pr merge --squash`: guards like GitHub's, scripted rejections first, then a real squash commit. */
function merge(s: State, remote: string, pr: Pr, one: (k: string) => string | undefined): string {
  if (pr.state !== "OPEN") throw new GhError(`Pull request #${pr.number} is not open`);
  if (pr.isDraft) throw new GhError(`GraphQL: Pull Request is still a draft (mergePullRequest)`);
  const failures = s.mergeFailures[String(pr.number)];
  if (failures && failures.length > 0) throw new GhError(failures.shift()!);
  const head = refSha(remote, pr.headRefName)!;
  const expected = one("--match-head-commit");
  if (expected && expected !== head) {
    throw new GhError(`GraphQL: Head branch was modified. Review and try the merge again. (mergePullRequest)`);
  }
  const baseSha = refSha(remote, pr.baseRefName)!;
  const tree = squashTree(remote, baseSha, head);
  if (tree === undefined) throw new GhError(`GraphQL: Pull Request is not mergeable (mergePullRequest)`);
  const body = one("--body-file") ? Deno.readTextFileSync(one("--body-file")!) : "";
  const msg = `${one("--subject") ?? pr.title}\n\n${body}`;
  const commit = git(remote, ["commit-tree", tree, "-p", baseSha, "-m", msg]).out;
  git(remote, ["update-ref", `refs/heads/${pr.baseRefName}`, commit, baseSha]);
  pr.state = "MERGED";
  pr.mergeCommit = commit;
  pr.mergedHead = head;
  event(`merged #${pr.number} into ${pr.baseRefName} as ${commit}`);
  if (s.autoDeleteBranches) {
    // GitHub deletes the head branch and moves every open PR based on it to the merged PR's base, as repos with
    // "automatically delete head branches" do; the script must cope with both effects.
    git(remote, ["update-ref", "-d", `refs/heads/${pr.headRefName}`]);
    for (const child of s.prs.filter((p) => p.state === "OPEN" && p.baseRefName === pr.headRefName)) {
      child.baseRefName = pr.baseRefName;
      event(`auto-retargeted #${child.number} to ${pr.baseRefName}`);
    }
  }
  for (let i = 0; i < (s.externalPushAfterMerge[String(pr.number)] ?? 0); i++) {
    const tip = refSha(remote, s.defaultBranch)!;
    const tipTree = git(remote, ["rev-parse", `${tip}^{tree}`]).out;
    const ext = git(remote, ["commit-tree", tipTree, "-p", tip, "-m", `someone else's change ${i + 1}`]).out;
    git(remote, ["update-ref", `refs/heads/${s.defaultBranch}`, ext, tip]);
    event(`external push to ${s.defaultBranch} as ${ext}`);
  }
  return `Squashed and merged pull request #${pr.number}`;
}

/** The `gh api` endpoints the script calls. */
async function api(s: State, remote: string, args: string[]): Promise<string> {
  const { flags, pos, one } = parseFlags("api", args);
  const method = one("-X") ?? "GET";
  const path = pos[0];
  const jq = one("--jq");
  const prefix = `repos/${s.repo}/`;
  if (!path?.startsWith(prefix)) throw new GhError(`fake gh: unsupported api path ${path}`);
  const rest = path.slice(prefix.length).replace(/\?.*$/, "");
  const fieldsOf = (k: string) =>
    Object.fromEntries((flags[k] ?? []).map((kv) => [kv.split("=")[0], kv.slice(kv.indexOf("=") + 1)]));

  let m;
  if ((m = rest.match(/^pulls\/(\d+)$/)) && method === "PATCH") {
    const pr = findPr(s, m[1]);
    const f = { ...fieldsOf("-f"), ...fieldsOf("-F") };
    if (f.base !== undefined) {
      if (!refSha(remote, f.base)) throw new GhError(`HTTP 422: Validation Failed (base ${f.base} does not exist)`);
      pr.baseRefName = f.base;
    }
    if (f.title !== undefined) pr.title = f.title;
    if (f.body !== undefined) pr.body = f.body.startsWith("@") ? Deno.readTextFileSync(f.body.slice(1)) : f.body;
    return await emit({ number: pr.number, base: { ref: pr.baseRefName } }, jq);
  }
  if ((m = rest.match(/^git\/ref\/heads\/(.+)$/)) && method === "GET") {
    const sha = refSha(remote, m[1]);
    if (!sha) throw new GhError(`gh: Not Found (HTTP 404)`);
    return await emit({ ref: `refs/heads/${m[1]}`, object: { sha } }, jq);
  }
  if ((m = rest.match(/^git\/commits\/([0-9a-f]+)$/)) && method === "GET") {
    const tree = git(remote, ["rev-parse", `${m[1]}^{tree}`]).out;
    return await emit({ sha: m[1], tree: { sha: tree } }, jq);
  }
  if ((m = rest.match(/^actions\/workflows\/([^/]+)\/dispatches$/)) && method === "POST") {
    // GitHub finds the workflow by its file on the default branch (404 without it), then runs the file as it is on the
    // dispatched ref: 422 for a ref that does not exist or whose version of the file lacks the trigger. With
    // return_run_details it answers 200 with the new run, otherwise 204 with no body (all observed on github.com,
    // 2026-10).
    const f = { ...fieldsOf("-f"), ...fieldsOf("-F") };
    const path = `.github/workflows/${m[1]}`;
    if (git(remote, ["show", `${s.defaultBranch}:${path}`], [0, 128]).code !== 0) {
      throw new GhError("gh: Not Found (HTTP 404)");
    }
    if (!f.ref || !refSha(remote, f.ref)) throw new GhError(`gh: No ref found for: ${f.ref ?? ""} (HTTP 422)`);
    const atRef = git(remote, ["show", `refs/heads/${f.ref}:${path}`], [0, 128]);
    if (atRef.code !== 0 || !/\bworkflow_dispatch\b/.test(atRef.out)) {
      throw new GhError("gh: Workflow does not have 'workflow_dispatch' trigger (HTTP 422)");
    }
    s.nextRunId = (s.nextRunId ?? 1000) + 1;
    event(`dispatched ${m[1]} ref=${f.ref} run=${s.nextRunId}`);
    if (f.return_run_details !== "true") return "";
    const html = `https://github.com/${s.repo}/actions/runs/${s.nextRunId}`;
    return await emit({ workflow_run_id: s.nextRunId, run_url: html, html_url: html }, jq);
  }
  if ((m = rest.match(/^issues\/(\d+)\/events$/)) && method === "GET") {
    const pr = findPr(s, m[1]);
    return await emit(pr.readyAt.map((t) => ({ event: "ready_for_review", created_at: t })), jq);
  }
  if ((m = rest.match(/^commits\/([0-9a-f]+)\/check-suites$/)) && method === "GET") {
    // One Actions suite per head that has checks: running while any check is, created when first seen.
    const pr = s.prs.find((p) => refSha(remote, p.headRefName) === m![1] || p.mergedHead === m![1]);
    const frame = pr ? (s.checks[String(pr.number)]?.[0] ?? s.defaultChecks) : [];
    const running = frame.some((c) => c.status !== "COMPLETED");
    const suites = frame.length === 0 ? [] : [{
      app: { slug: "github-actions" },
      status: running ? "in_progress" : "completed",
      latest_check_runs_count: frame.length,
      created_at: s.suiteCreatedAt[m![1]] ??= now(),
    }];
    return await emit({ total_count: suites.length, check_suites: suites }, jq);
  }
  throw new GhError(`fake gh: unsupported api call: ${method} ${path}`);
}

if (import.meta.main) {
  const statePath = Deno.env.get("FAKE_GH_STATE");
  const remote = Deno.env.get("FAKE_GH_REMOTE");
  eventsPath = Deno.env.get("FAKE_GH_EVENTS") ?? "";
  if (!statePath || !remote || !eventsPath) {
    console.error("fake gh: FAKE_GH_STATE, FAKE_GH_REMOTE, and FAKE_GH_EVENTS must be set");
    Deno.exit(2);
  }
  const s: State = JSON.parse(Deno.readTextFileSync(statePath));
  event(`gh ${Deno.args.join(" ")}`);
  try {
    const out = await handle(Deno.args, s, remote);
    Deno.writeTextFileSync(statePath, JSON.stringify(s, null, 2));
    if (out !== "") console.log(out);
  } catch (err) {
    // Scripted state changes (a consumed failure, a consumed rollup frame) persist even when the call fails.
    Deno.writeTextFileSync(statePath, JSON.stringify(s, null, 2));
    if (!(err instanceof GhError)) throw err;
    console.error(err.message);
    Deno.exit(1);
  }
}

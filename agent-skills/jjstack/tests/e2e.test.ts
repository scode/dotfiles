/**
 * End-to-end tests for the jjstack script: whole commands (`submit`, `land`, `fast-land`, `stop`) run as subprocesses
 * against a real jj checkout, a real bare Git repository playing GitHub's copy of the repo, and `fake-gh.ts` standing
 * in for the `gh` CLI. Run them with `deno test -A agent-skills/jjstack/tests/` (see `../AGENTS.md`).
 *
 * The unit tests inside the script cover its decision logic; these cover the sequences around it that only show up
 * when the pieces run together: retarget-before-push ordering, pushing only the PR a landing is merging, retries of
 * GitHub's transient responses, stops and resumes, the fast path switching to restack mode, and cleanup.
 *
 * Each test builds its own world in a temporary directory. Nothing here touches the test process's own environment:
 * the fake `gh` and `ssh`, isolated jj and Git configuration, and `JJSTACK_TIME_SCALE` (which turns minutes of CI
 * waiting into milliseconds) are passed only to the child processes. The Git remote is `ssh://git@github.com/o/r.git`,
 * which the script's repository check accepts, routed by the fake `ssh` to the bare repository on disk.
 */
import $ from "jsr:@david/dax@0.50.1";
import { assert, assertEquals, assertStringIncludes } from "jsr:@std/assert@1.0.19";
import { initialState, type State } from "./fake-gh.ts";

const SCRIPT = new URL("../scripts/jjstack", import.meta.url).pathname;
const FAKE_GH = new URL("./fake-gh.ts", import.meta.url).pathname;

/** One test's private GitHub, checkout, and environment. */
type World = {
  root: string;
  work: string;
  remote: string;
  statePath: string;
  eventsPath: string;
  env: Record<string, string>;
};

/**
 * Builds a world: a bare repo with one commit on `main` (with a post-receive hook that logs pushes to the events file),
 * the fake `gh` and `ssh` on a private `PATH`, isolated configuration, and a colocated jj clone over the fake ssh.
 */
async function makeWorld(): Promise<World> {
  const root = await Deno.makeTempDir({ prefix: "jjstack-e2e-" });
  const bin = `${root}/bin`;
  const remoteRoot = `${root}/remote`;
  const remote = `${remoteRoot}/o/r.git`;
  const statePath = `${root}/gh-state.json`;
  const eventsPath = `${root}/events.log`;
  await Deno.mkdir(bin, { recursive: true });
  await Deno.mkdir(`${remoteRoot}/o`, { recursive: true });
  await Deno.writeTextFile(`${bin}/gh`, `#!/bin/sh\nexec deno run -A --quiet ${FAKE_GH} "$@"\n`);
  // The remote command arrives as the last argument, e.g. `git-receive-pack '/o/r.git'`; run it against the bare
  // repos' root with the leading slash dropped from the path.
  await Deno.writeTextFile(
    `${bin}/ssh`,
    `#!/bin/sh\nfor last; do :; done\ncd ${remoteRoot} || exit 1\n` +
      `exec sh -c "$(printf '%s' "$last" | sed "s#'/#'#")"\n`,
  );
  await Deno.chmod(`${bin}/gh`, 0o755);
  await Deno.chmod(`${bin}/ssh`, 0o755);
  await Deno.writeTextFile(`${root}/gitconfig`, "[user]\n\tname = Test\n\temail = test@example.com\n");
  await Deno.writeTextFile(`${root}/jj.toml`, 'user.name = "Test"\nuser.email = "test@example.com"\n');
  await Deno.writeTextFile(statePath, JSON.stringify(initialState()));
  await Deno.writeTextFile(eventsPath, "");
  const env = {
    PATH: `${bin}:${Deno.env.get("PATH") ?? ""}`,
    GIT_SSH_COMMAND: `${bin}/ssh`,
    GIT_CONFIG_GLOBAL: `${root}/gitconfig`,
    GIT_CONFIG_NOSYSTEM: "1",
    JJ_CONFIG: `${root}/jj.toml`,
    FAKE_GH_STATE: statePath,
    FAKE_GH_REMOTE: remote,
    FAKE_GH_EVENTS: eventsPath,
    JJSTACK_TIME_SCALE: "0.01",
    NO_COLOR: "1",
  };
  const sh = $.build$({ commandBuilder: (b) => b.env(env).stdin("null").quiet() });
  await sh`git init -q --bare -b main ${remote}`;
  await Deno.writeTextFile(
    `${remote}/hooks/post-receive`,
    `#!/bin/sh\nwhile read old new ref; do echo "push $ref $new" >> ${eventsPath}; done\n`,
  );
  await Deno.chmod(`${remote}/hooks/post-receive`, 0o755);
  const seed = `${root}/seed`;
  await sh`git init -q -b main ${seed}`;
  await Deno.writeTextFile(`${seed}/README`, "readme\n");
  await sh`git -C ${seed} add README`;
  await sh`git -C ${seed} commit -q -m ${"initial"}`;
  await sh`git -C ${seed} push -q ${remote} main`;
  const work = `${root}/work`;
  await sh`jj git clone --colocate ssh://git@github.com/o/r.git ${work}`;
  await Deno.writeTextFile(eventsPath, "");
  return { root, work, remote, statePath, eventsPath, env };
}

/** Runs a command in the checkout with the world's environment, quietly, failing the test on a non-zero exit. */
function inWork(w: World) {
  return $.build$({ commandBuilder: (b) => b.cwd(w.work).env(w.env).stdin("null").quiet() });
}

/** Runs the jjstack script and returns its exit code and output; never throws on a non-zero exit. */
async function jjstack(w: World, args: string[]) {
  const r = await $`${SCRIPT} ${args} --dir ${w.work} --repo o/r`.env(w.env).stdin("null").noThrow()
    .stdout("piped").stderr("piped");
  return { code: r.code, out: r.stdout, err: r.stderr };
}

/** Commits `file` with `content` on top of the working copy, describes it, and bookmarks it. */
async function commit(w: World, file: string, content: string, title: string, bookmark: string) {
  await Deno.writeTextFile(`${w.work}/${file}`, content);
  await inWork(w)`jj commit ${file} -m ${`${title}\n\nWhy this change.`}`;
  await inWork(w)`jj bookmark set ${bookmark} -r @-`;
}

async function readState(w: World): Promise<State> {
  return JSON.parse(await Deno.readTextFile(w.statePath));
}

async function editState(w: World, f: (s: State) => void) {
  const s = await readState(w);
  f(s);
  await Deno.writeTextFile(w.statePath, JSON.stringify(s));
}

async function events(w: World): Promise<string[]> {
  return (await Deno.readTextFile(w.eventsPath)).split("\n").filter(Boolean);
}

/** Subjects of the bare repo's `main`, oldest first, excluding the initial commit. */
async function mainSubjects(w: World): Promise<string[]> {
  const out = await $`git --git-dir ${w.remote} log --reverse --format=%s main`.env(w.env).text();
  return out.split("\n").filter((l) => l && l !== "initial");
}

/** Runs `body` in a fresh world and removes it afterwards, keeping it (and saying where) if the test failed. */
async function withWorld(body: (w: World) => Promise<void>) {
  const w = await makeWorld();
  let ok = false;
  try {
    await body(w);
    ok = true;
  } finally {
    if (ok) await Deno.remove(w.root, { recursive: true });
    else console.error(`kept the failed test's world at ${w.root}`);
  }
}

/** Builds and submits a stack of `n` PRs, each adding its own file, and returns their bookmarks. */
async function stackOf(w: World, n: number): Promise<string[]> {
  const names = Array.from({ length: n }, (_, i) => `pr${i + 1}`);
  for (const name of names) await commit(w, `${name}.txt`, `${name}\n`, `feat: add ${name}`, name);
  const r = await jjstack(w, ["submit"]);
  assertEquals(r.code, 0, r.err);
  return names;
}

// ---------------------------------------------------------------------------------------------------------------------
// submit
// ---------------------------------------------------------------------------------------------------------------------

/**
 * `submit` is how every PR gets made and updated, so its two ordering rules are pinned here: PRs are created bottom-up
 * as drafts based on the PR below, and when an inserted PR changes the base of the PR above it, that PR is retargeted
 * before its new head is pushed (a push starts CI, which must see the final base).
 */
Deno.test("submit opens draft PRs bottom-up and retargets before pushing after an insert", async () => {
  await withWorld(async (w) => {
    await stackOf(w, 2);
    let s = await readState(w);
    assertEquals(s.prs.map((p) => [p.number, p.headRefName, p.baseRefName, p.isDraft]), [
      [1, "pr1", "main", true],
      [2, "pr2", "pr1", true],
    ]);

    // Insert a PR between the two, then sync again.
    await inWork(w)`jj new --insert-after pr1 -m ${"feat: add mid\n\nWhy this change."}`;
    await Deno.writeTextFile(`${w.work}/mid.txt`, "mid\n");
    await inWork(w)`jj bookmark set mid -r @`;
    await inWork(w)`jj new pr2`;
    const before = (await events(w)).length;
    const r = await jjstack(w, ["submit"]);
    assertEquals(r.code, 0, r.err);
    s = await readState(w);
    assertEquals(s.prs.find((p) => p.headRefName === "mid")?.baseRefName, "pr1");
    assertEquals(s.prs.find((p) => p.headRefName === "pr2")?.baseRefName, "mid");
    const ev = (await events(w)).slice(before);
    const retarget = ev.findIndex((l) => l.includes("pulls/2") && l.includes("base=mid"));
    const push = ev.findIndex((l) => l.startsWith("push refs/heads/pr2"));
    assert(retarget >= 0 && push >= 0 && retarget < push, `retarget must precede the push:\n${ev.join("\n")}`);
  });
});

/**
 * After a mid-stack edit the working copy is often a sibling of the rewritten PRs above, and syncing only up to it
 * would leave those PRs stale on GitHub; `submit` refuses that instead of half-syncing.
 */
Deno.test("submit refuses when the stack continues above --top", async () => {
  await withWorld(async (w) => {
    await stackOf(w, 2);
    await inWork(w)`jj new pr1`;
    await Deno.writeTextFile(`${w.work}/pr1.txt`, "pr1 edited\n");
    await inWork(w)`jj squash`;
    const r = await jjstack(w, ["submit"]);
    assertEquals(r.code, 3, r.err);
    assertStringIncludes(r.err, "E_SUBMIT_STACK_ABOVE_TOP");
  });
});

// ---------------------------------------------------------------------------------------------------------------------
// land
// ---------------------------------------------------------------------------------------------------------------------

/**
 * The CI-gated landing end to end: one squash commit per PR in stack order, each PR pushed only on its own turn (not
 * re-pushed while lower PRs land, which would start a CI run per PR per cycle), merged branches gone, and the working
 * copy left on the landed default branch.
 */
Deno.test("land merges bottom-up, pushes only the PR it is landing, and cleans up", async () => {
  await withWorld(async (w) => {
    await stackOf(w, 3);
    const before = (await events(w)).length;
    const r = await jjstack(w, ["land", "--expect", "1,2,3"]);
    assertEquals(r.code, 0, r.err);
    assertEquals(await mainSubjects(w), ["feat: add pr1 (#1)", "feat: add pr2 (#2)", "feat: add pr3 (#3)"]);
    assertEquals((r.out.match(/^JJSTACK_LANDED /gm) ?? []).length, 3);
    const ev = (await events(w)).slice(before);
    // Branch deletions show up as pushes of the all-zero object id; only real updates count.
    const pushes = (b: string) => ev.filter((l) => l.startsWith(`push refs/heads/${b} `) && !/ 0{40}$/.test(l));
    // The bottom PR needs no rebase; each PR above is rebased and pushed exactly once, when its turn comes.
    assertEquals(pushes("pr1").length, 0);
    assertEquals(pushes("pr2").length, 1);
    assertEquals(pushes("pr3").length, 1);
    assert(ev.indexOf(pushes("pr3")[0]) > ev.findIndex((l) => l.startsWith("merged #2")), ev.join("\n"));
    const heads = await $`git --git-dir ${w.remote} for-each-ref ${"--format=%(refname:short)"} refs/heads`.env(w.env)
      .text();
    assertEquals(heads.trim(), "main");
    const left = await inWork(
      w,
    )`jj log --no-graph -r ${"::@ ~ ::main@origin"} -T ${'if(empty, "empty", "nonempty") ++ "\\n"'}`
      .text();
    assertEquals(left.trim(), "empty");
  });
});

/**
 * A failed check stops the landing before that PR merges, and rerunning the same command with `--resume` and the same
 * `--expect` (the PRs that already landed left the stack when the next one was rebased) finishes it.
 */
Deno.test("a failed check stops land, and --resume with the same --expect finishes it", async () => {
  await withWorld(async (w) => {
    await stackOf(w, 3);
    await editState(w, (s) => {
      s.checks["2"] = [[{ name: "test", status: "COMPLETED", conclusion: "FAILURE" }]];
    });
    let r = await jjstack(w, ["land", "--expect", "1,2,3"]);
    assertEquals(r.code, 4, r.err);
    assertStringIncludes(r.err, "code=E_LAND_CHECKS_FAILED");
    assertStringIncludes(r.out, "JJSTACK_STOPPED at=2 landed=1 unknown=none pending=2,3");
    assertStringIncludes(r.err, "log: ");
    await editState(w, (s) => {
      s.checks["2"] = [[{ name: "test", status: "COMPLETED", conclusion: "SUCCESS" }]];
    });
    r = await jjstack(w, ["land", "--expect", "1,2,3", "--resume"]);
    assertEquals(r.code, 0, r.err);
    assertEquals((await readState(w)).prs.map((p) => p.state), ["MERGED", "MERGED", "MERGED"]);
  });
});

/**
 * Two of GitHub's transient responses that real landings hit: a check run cancelled by a newer event whose replacement
 * attaches a little later, and a merge rejected with "Base branch was modified" right after a base change. Both must be
 * waited out or retried, not reported as failures.
 */
Deno.test("land waits for a cancelled run's replacement and retries a transient merge rejection", async () => {
  await withWorld(async (w) => {
    await stackOf(w, 1);
    await editState(w, (s) => {
      s.checks["1"] = [
        [{ name: "test", status: "COMPLETED", conclusion: "CANCELLED" }],
        [{ name: "test", status: "COMPLETED", conclusion: "CANCELLED" }],
        [{ name: "test", status: "IN_PROGRESS", conclusion: "" }],
        [{ name: "test", status: "COMPLETED", conclusion: "SUCCESS" }],
      ];
      s.mergeFailures["1"] = ["GraphQL: Base branch was modified. Review and try the merge again. (mergePullRequest)"];
    });
    const r = await jjstack(w, ["land", "--expect", "1"]);
    assertEquals(r.code, 0, r.err);
    assertEquals((await events(w)).filter((l) => l.startsWith("gh pr merge 1 ")).length, 2);
    assertEquals((await readState(w)).prs[0].state, "MERGED");
  });
});

/**
 * Changing course mid-landing, as users often do: `stop` while a PR waits on checks stops the landing at that safe
 * point, and `fast-land --resume` then lands the rest without waiting for CI.
 */
Deno.test("stop is honored during a check wait, and fast-land --resume lands the rest", async () => {
  await withWorld(async (w) => {
    await stackOf(w, 3);
    await editState(w, (s) => {
      s.checks["2"] = [[{ name: "test", status: "IN_PROGRESS", conclusion: "" }]];
    });
    const child = $`${SCRIPT} land --expect 1,2,3 --dir ${w.work} --repo o/r`.env(w.env).stdin("null").noThrow()
      .stdout("piped").stderr("piped").spawn();
    const deadline = Date.now() + 60_000;
    while (!(await events(w)).some((l) => l.startsWith("gh pr ready 2 "))) {
      assert(Date.now() < deadline, "land never reached PR #2");
      await $.sleep(200);
    }
    const stop = await jjstack(w, ["stop"]);
    assertEquals(stop.code, 0, stop.err);
    const r = await child;
    assertEquals(r.code, 4, r.stderr);
    assertStringIncludes(r.stderr, "code=E_STOP_REQUESTED");
    assertStringIncludes(r.stdout, "JJSTACK_STOPPED at=2 landed=1");
    const fast = await jjstack(w, ["fast-land", "--expect", "1,2,3", "--resume"]);
    assertEquals(fast.code, 0, fast.err);
    assertEquals((await readState(w)).prs.map((p) => p.state), ["MERGED", "MERGED", "MERGED"]);
  });
});

// ---------------------------------------------------------------------------------------------------------------------
// fast-land
// ---------------------------------------------------------------------------------------------------------------------

/**
 * The fast path's mode switch on the postmortem's shape: once a later PR edits a file an earlier PR created, GitHub's
 * own squash would conflict, so that PR and everything above it must be restacked, and the landed tree must be the
 * stack's tree.
 */
Deno.test("fast-land switches to restack where GitHub-only squashes would conflict", async () => {
  await withWorld(async (w) => {
    await commit(w, "log.txt", "1\n", "feat: start the log", "pr1");
    await commit(w, "log.txt", "1\n2\n", "feat: extend the log", "pr2");
    await commit(w, "other.txt", "x\n", "feat: add other", "pr3");
    assertEquals((await jjstack(w, ["submit"])).code, 0);
    const r = await jjstack(w, ["fast-land", "--expect", "1,2,3"]);
    assertEquals(r.code, 0, r.err);
    const modes = [...r.out.matchAll(/^JJSTACK_LANDED pr=(\d+) .* mode=(\w+)$/gm)].map((m) => `${m[1]}:${m[2]}`);
    assertEquals(modes, ["1:github", "2:restack", "3:restack"]);
    const log = await $`git --git-dir ${w.remote} show main:log.txt`.env(w.env).text();
    assertEquals(log, "1\n2");
  });
});

/**
 * Someone else merging to the default branch mid-landing makes the fast path's predictions stale; it must stop before
 * the next merge rather than land unpredicted content, and a rerun with `--resume` must plan from the moved branch.
 */
Deno.test("fast-land stops when the default branch moves under it, and --resume finishes", async () => {
  await withWorld(async (w) => {
    await stackOf(w, 2);
    await editState(w, (s) => {
      s.externalPushAfterMerge["1"] = 1;
    });
    let r = await jjstack(w, ["fast-land", "--expect", "1,2"]);
    assertEquals(r.code, 4, r.err);
    assertStringIncludes(r.err, "TIP_MOVED");
    assertStringIncludes(r.out, "JJSTACK_STOPPED at=2 landed=1");
    r = await jjstack(w, ["fast-land", "--expect", "1,2", "--resume"]);
    assertEquals(r.code, 0, r.err);
    assertEquals(await mainSubjects(w), ["feat: add pr1 (#1)", "someone else's change 1", "feat: add pr2 (#2)"]);
  });
});

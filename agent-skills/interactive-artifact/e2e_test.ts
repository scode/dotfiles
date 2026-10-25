/// <reference lib="dom" />
/// <reference lib="deno.ns" />
/**
 * End-to-end test of an interactive-artifact page in a real browser: the server, the browser client, and the `wait` and
 * `reply` commands together. The unit tests in `interactive-artifact.ts` cover the state rules; this covers what only a
 * browser shows, such as text anchoring, focus surviving card layout, picks surviving a reload, and live updates.
 *
 * The browser test needs Chrome or Chromium. It looks in `$CHROME` and the usual install paths, and skips when
 * none is found. The access-control test needs no browser and always runs.
 *
 *   deno test -A agent-skills/interactive-artifact/e2e_test.ts
 */
import { assertEquals, assertStringIncludes } from "jsr:@std/assert@1.0.19";
import { join } from "jsr:@std/path@1.1.6";
import { chromium, type Page } from "npm:playwright-core@1.64.1";

const SCRIPT = new URL("./interactive-artifact", import.meta.url).pathname;
const EXAMPLE = new URL("./examples/interactive.html", import.meta.url).pathname;

const chrome = [
  Deno.env.get("CHROME"),
  "/usr/bin/google-chrome",
  "/usr/bin/chromium",
  "/usr/bin/chromium-browser",
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
].find((p) => {
  try {
    return !!p && Deno.statSync(p).isFile;
  } catch {
    return false;
  }
});

async function cli(...args: string[]): Promise<{ code: number; out: string }> {
  const r = await new Deno.Command(SCRIPT, { args, stdout: "piped", stderr: "null" }).output();
  return { code: r.code, out: new TextDecoder().decode(r.stdout) };
}

const testApiUrls = new WeakMap<CryptoKey, string>();

/** The message key and matching API capability from server.json, the same way `reply` gets them. */
async function testKey(stateDir: string): Promise<CryptoKey> {
  const { key, apiUrl } = JSON.parse(await Deno.readTextFile(join(stateDir, "server.json")));
  const raw = Uint8Array.from(atob(key), (c) => c.charCodeAt(0));
  const imported = await crypto.subtle.importKey("raw", raw, "AES-GCM", false, ["encrypt", "decrypt"]);
  testApiUrls.set(imported, apiUrl);
  return imported;
}

/** Make one sealed API call and open the sealed answer, the way the page does. */
async function testCall(url: string, key: CryptoKey, method: string, name: string, body?: unknown) {
  const client = crypto.randomUUID().replaceAll("-", "");
  const sealed = JSON.stringify(await testSeal(key, `${method} ${name}`, { client, seq: 1, data: body ?? {} }));
  const init: RequestInit = { method, headers: { "x-interactive-artifact": "1", "content-type": "application/json" } };
  if (method !== "GET") init.body = sealed;
  else init.headers = { "x-interactive-artifact": sealed };
  const env = await (await fetch(`${testApiUrls.get(key) ?? url}__interactive-artifact/${name}`, init)).json();
  const from = (t: string) => Uint8Array.from(atob(t), (c) => c.charCodeAt(0));
  const plain = await crypto.subtle.decrypt(
    {
      name: "AES-GCM",
      iv: from(env.iv),
      additionalData: new TextEncoder().encode(`response ${method} ${name} ${client} 1`),
    },
    key,
    from(env.ct),
  );
  return JSON.parse(new TextDecoder().decode(plain)) as { status: number; data: Record<string, unknown> };
}

/**
 * Seal a request the way interactive-artifact.js and `reply` do; written out here so the test checks the wire format.
 */
async function testSeal(key: CryptoKey, aad: string, value: unknown): Promise<{ iv: string; ct: string }> {
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const enc = new TextEncoder();
  const ct = new Uint8Array(
    await crypto.subtle.encrypt(
      { name: "AES-GCM", iv, additionalData: enc.encode(aad) },
      key,
      enc.encode(JSON.stringify(value)),
    ),
  );
  const b64 = (b: Uint8Array) => btoa(String.fromCharCode(...b));
  return { iv: b64(iv), ct: b64(ct) };
}

/** Start `interactive-artifact serve` on a page and return the printed URL and the child to stop later. */
async function startServer(pagePath: string): Promise<{ url: string; server: Deno.ChildProcess }> {
  const server = new Deno.Command(SCRIPT, {
    args: ["serve", pagePath, "--ttl", "10m"],
    stdout: "piped",
    stderr: "null",
  }).spawn();
  const url = (await server.stdout.pipeThrough(new TextDecoderStream()).getReader().read()).value!.trim();
  return { url, server };
}

/**
 * The multi-user guarantees from SPEC.md, checked against a real server: a shared page directory is refused,
 * nothing is answered without the token, and everything written to disk is owner-only.
 */
Deno.test({
  name: "access control: token-only answers, private state files, shared directories refused",
  sanitizeResources: false,
  fn: async () => {
    const dir = await Deno.makeTempDir({ prefix: "interactive-artifact-acl-" });
    const pagePath = join(dir, "page.html");
    await Deno.writeTextFile(pagePath, "<h1>Secret plans</h1><p>for your eyes only</p>");
    try {
      await Deno.chmod(dir, 0o755);
      const refused = await new Deno.Command(SCRIPT, { args: ["serve", pagePath], stdout: "piped", stderr: "null" })
        .output();
      assertEquals([refused.code !== 0, new TextDecoder().decode(refused.stdout)], [true, ""]);
      await Deno.chmod(dir, 0o700);

      // A failed bind must not leave the serve lock behind to block the next attempt.
      const taken = Deno.listen({ hostname: "127.0.0.1", port: 0 });
      assertEquals((await cli("serve", pagePath, "--port", String((taken.addr as Deno.NetAddr).port))).code, 1);
      taken.close();

      const { url, server } = await startServer(pagePath);
      try {
        const origin = new URL(url).origin;
        const token = new URL(url).pathname.split("/")[1];
        assertEquals(token.length, 32);
        // Every path and method without the exact token gets the same plain 404, page content never.
        const probes: [string, RequestInit?][] = [
          ["/"],
          ["/index.html"],
          ["/page.html"],
          ["/__interactive-artifact/state"],
          ["/__interactive-artifact/interactive-artifact.js"],
          [`/${token.slice(0, -1)}0/`],
          ["/__interactive-artifact/submit", {
            method: "POST",
            headers: { "x-interactive-artifact": "1" },
            body: "{}",
          }],
        ];
        for (const [path, init] of probes) {
          const res = await fetch(origin + path, init);
          assertEquals([path, res.status, await res.text()], [path, 404, "not found\n"]);
        }
        // HEAD cannot spend the one delivery. Concurrent GETs must claim it before awaiting file reads, including
        // when one uses the index.html alias; only one reader may ever receive the symmetric key.
        assertEquals((await fetch(url, { method: "HEAD" })).status, 410);
        const firstReaders = await Promise.all([fetch(url), fetch(`${url}index.html`)]);
        assertEquals(firstReaders.map((r) => r.status).sort(), [200, 410]);
        const ok = firstReaders.find((r) => r.status === 200)!;
        await firstReaders.find((r) => r.status === 410)!.body?.cancel();
        assertEquals(ok.headers.get("referrer-policy"), "no-referrer");
        const initial = await ok.text();
        assertStringIncludes(initial, "for your eyes only");
        assertEquals(initial.includes("<script src="), false);
        for (const path of ["", "index.html", "?retry=1"]) {
          const spent = await fetch(url + path);
          assertEquals(spent.status, 410);
          assertEquals((await spent.text()).includes("data-key"), false);
        }
        for (const source of ["page.html", "%70age.html", "page.html/"]) {
          const hidden = await fetch(url + source);
          assertEquals(hidden.status, 404);
          await hidden.body?.cancel();
        }
        const state = join(dir, ".interactive-artifact", "page.html");
        const key = await testKey(state);
        const apiUrl = testApiUrls.get(key)!;
        // A token exposed by reconnect traffic grants no access to the one-time bootstrap or ordinary files.
        for (const resource of ["", "index.html", "page.html", "build-log"]) {
          const denied = await fetch(apiUrl + resource);
          assertEquals(denied.status, 404);
          await denied.body?.cancel();
        }
        const bare = await fetch(origin + `/${token}`, { redirect: "manual" });
        assertEquals([bare.status, bare.headers.get("location")], [308, `/${token}/`]);
        // A sibling file with no known type must not be sniffed into HTML that runs on the page's origin.
        await Deno.writeTextFile(join(dir, "build-log"), "<script>alert(1)</script>");
        const log = await fetch(`${url}build-log`);
        await log.body?.cancel();
        assertEquals(
          [
            log.headers.get("content-type"),
            log.headers.get("x-content-type-options"),
            log.headers.get("content-security-policy"),
          ],
          ["application/octet-stream", "nosniff", "sandbox"],
        );
        // Later page contents require the key, even when the token is known. Bad edits stay in the sealed channel.
        await Deno.writeTextFile(pagePath, "<!doctype html><p>oops</p>");
        const broken = await testCall(url, key, "GET", "page");
        assertStringIncludes(String(broken.data.fragment), "interactive-artifact could not render the page");
        await Deno.writeTextFile(pagePath, "<h1>Secret plans</h1><p>for your eyes only</p>");

        // serveDir's own redirects (a file asked for with a trailing slash) used to crash the handler.
        const slash = await fetch(`${url}page.html/`, { redirect: "manual" });
        await slash.body?.cancel();
        assertEquals(slash.status !== 500, true);

        // One server per page: a second one would fork the state and reuse submission numbers.
        assertEquals((await cli("serve", pagePath)).code !== 0, true);
        // A TTL past setTimeout's range used to fire at once and end the server immediately.
        assertEquals((await cli("serve", pagePath, "--ttl", "600h")).code, 2);
        // wait and reply trust files under .interactive-artifact/, so they refuse a page directory others could plant
        // files in.
        await Deno.chmod(dir, 0o755);
        assertEquals((await cli("wait", pagePath, "--timeout", "1s")).code, 1);
        assertEquals((await cli("reply", pagePath, "c1", "--resolve")).code, 1);
        await Deno.chmod(dir, 0o700);

        // Owner-only on disk: directories 0700, files 0600, including the one holding the tokened URL.
        for (const d of [join(dir, ".interactive-artifact"), state, join(state, "submissions")]) {
          assertEquals([d, Deno.statSync(d).mode! & 0o777], [d, 0o700]);
        }
        // Even with the token, the API takes only sealed requests and answers only sealed responses.
        const put = (body: unknown) =>
          fetch(`${apiUrl}__interactive-artifact/state`, {
            method: "PUT",
            headers: { "x-interactive-artifact": "1", "content-type": "application/json" },
            body: JSON.stringify(body),
          });
        const plain = await put({ annotations: [], picks: {}, note: "planted" });
        assertEquals([plain.status, await plain.text()], [400, "bad request\n"]);
        const sealed = await put(
          await testSeal(key, "PUT state", {
            client: crypto.randomUUID().replaceAll("-", ""),
            seq: 1,
            data: { annotations: [], picks: {}, note: "hush" },
          }),
        );
        const env = await sealed.json();
        assertEquals(Object.keys(env).sort(), ["ct", "iv"]);
        const unauthenticated = await fetch(`${apiUrl}__interactive-artifact/state`);
        assertEquals(unauthenticated.status, 400);
        const got = await unauthenticated.text();
        assertEquals([got.includes("hush"), got.includes("planted")], [false, false]);
        for (const f of ["server.json", "state.json"]) {
          assertEquals([f, Deno.statSync(join(state, f)).mode! & 0o777], [f, 0o600]);
        }

        // The same port is held on [::1], so nobody else can take it there and catch `localhost` traffic.
        let v6 = "free";
        try {
          Deno.listen({ hostname: "::1", port: Number(new URL(url).port) }).close();
        } catch (e) {
          v6 = e instanceof Deno.errors.AddrInUse ? "held" : "no ipv6";
        }
        assertEquals(v6 !== "free", true);

        // Calls that overlap with a submit's file write must not be lost when the submit finishes: an earlier
        // version adopted the submit's working copy wholesale and dropped agent replies and handed-out ids.
        const draft = { id: "c1", quote: "for your eyes", prefix: "", suffix: "", section: "", draft: "q" };
        assertEquals((await testCall(url, key, "PUT", "state", { annotations: [draft] })).status, 200);
        const racing = await Promise.all([
          testCall(url, key, "POST", "submit", {}),
          testCall(url, key, "POST", "agent", { ids: ["c1"], text: "answer", resolve: false }),
          ...Array.from({ length: 5 }, () => testCall(url, key, "POST", "id", {})),
        ]);
        assertEquals(racing.map((r) => r.status), [200, 200, 200, 200, 200, 200, 200]);
        const ns = racing.slice(2).map((r) => r.data.n);
        assertEquals(new Set(ns).size, 5);
        const after = (await testCall(url, key, "GET", "state")).data as {
          idFloor: number;
          annotations: { thread: { from: string; text: string }[] }[];
        };
        assertEquals(after.annotations[0].thread.some((m) => m.text === "answer"), true);
        assertEquals(after.idFloor >= Math.max(...(ns as number[])), true);
      } finally {
        server.kill("SIGTERM");
        await server.status;
      }
    } finally {
      await Deno.remove(dir, { recursive: true });
    }
  },
});

/**
 * Initial state replaces the browser's empty model. A slow or retried first read must not accept edits that this
 * replacement would erase; editing becomes available only after the saved note and picks are on screen.
 */
Deno.test({
  name: "startup keeps editing blocked through an initial connection failure",
  ignore: !chrome,
  sanitizeOps: false,
  sanitizeResources: false,
  fn: async () => {
    const dir = await Deno.makeTempDir({ prefix: "interactive-artifact-startup-" });
    const pagePath = join(dir, "page.html");
    await Deno.copyFile(EXAMPLE, pagePath);
    const { url, server } = await startServer(pagePath);
    const browser = await chromium.launch({ executablePath: chrome });
    const failFirst = Promise.withResolvers<void>();
    const finishRetry = Promise.withResolvers<void>();
    try {
      const page = await browser.newPage();
      const key = await testKey(join(dir, ".interactive-artifact", "page.html"));
      assertEquals((await testCall(url, key, "PUT", "state", { note: "saved before opening" })).status, 200);
      let reads = 0;
      await page.route("**/__interactive-artifact/state", async (route) => {
        if (route.request().method() === "GET") {
          reads++;
          if (reads === 1) {
            await failFirst.promise;
            await route.abort("connectionreset");
            return;
          }
          if (reads === 2) {
            await finishRetry.promise;
          }
        }
        await route.continue();
      });
      const stateRead = () => page.waitForRequest((r) => r.method() === "GET" && r.url().endsWith("/state"));
      const firstRead = stateRead();
      await page.goto(url);
      await firstRead;
      await page.locator(".ia-reconnecting", { hasText: "Loading your saved answers" }).waitFor();
      assertEquals(
        await page.locator(".ia-page, .ia-notes, .ia-dock").evaluateAll((es) =>
          es.every((e) => (e as HTMLElement).inert)
        ),
        true,
      );

      // Exercise both the native focus barrier and the event guard. Synthetic clicks bypass inert, but must not
      // mutate picker state either; no queued edit may survive invisibly behind the saved startup snapshot.
      const note = page.locator(".ia-note");
      await note.evaluate((t: HTMLTextAreaElement) => t.focus());
      await page.keyboard.type("too early");
      await page.locator('[data-value="sqlite"] h3').dispatchEvent("click");
      assertEquals(await note.inputValue(), "");
      assertEquals(await page.locator('[data-value="sqlite"]').getAttribute("aria-pressed"), "false");
      const retriedRead = stateRead();
      failFirst.resolve();
      await page.locator(".ia-reconnecting", { hasText: "Connection lost" }).waitFor();
      await retriedRead;
      assertEquals(await page.locator(".ia-dock").evaluate((e: HTMLElement) => e.inert), true);

      finishRetry.resolve();
      await page.locator(".ia-reconnecting").waitFor({ state: "hidden" });
      assertEquals(await note.inputValue(), "saved before opening");
      assertEquals(
        await page.locator(".ia-page, .ia-notes, .ia-dock").evaluateAll((es) =>
          es.some((e) => (e as HTMLElement).inert)
        ),
        false,
      );
      await note.fill("written after loading");
      await page.locator('[data-value="sqlite"] h3').click();
      await page.locator(".ia-send").click();
      await page.locator(".ia-agent", { hasText: "Waiting for the agent" }).waitFor();
      const submission = await cli("wait", pagePath, "--timeout", "5s");
      assertEquals(submission.code, 0);
      assertStringIncludes(submission.out, "written after loading");
      assertStringIncludes(submission.out, "SQLite queue");
      assertEquals(submission.out.includes("too early"), false);
    } finally {
      failFirst.resolve();
      finishRetry.resolve();
      await browser.close();
      server.kill("SIGTERM");
      await server.status;
      await Deno.remove(dir, { recursive: true });
    }
  },
});

/** Select the first occurrence of `needle` in the page text, as a user drag would, and wait for the toolbar. */
async function select(page: Page, needle: string) {
  await page.evaluate((needle) => {
    const walker = document.createTreeWalker(document.querySelector(".ia-page")!, NodeFilter.SHOW_TEXT);
    for (let n; (n = walker.nextNode() as Text | null);) {
      const i = n.data.indexOf(needle);
      if (i < 0) continue;
      const r = document.createRange();
      r.setStart(n, i);
      r.setEnd(n, i + needle.length);
      getSelection()!.removeAllRanges();
      getSelection()!.addRange(r);
      document.dispatchEvent(new Event("selectionchange"));
      return;
    }
    throw new Error(`not on the page: ${needle}`);
  }, needle);
  await page.locator(".ia-toolbar:not([hidden])").waitFor();
}

/**
 * The whole round trip in one test, because every step depends on the state the previous one left: a user
 * picks, comments, suggests, copies, reconnects, and submits; the agent reads the submission, edits the page, and
 * replies; the open tab follows along without losing anything.
 */
Deno.test({
  name: "interactive-artifact round trip: picks, comments, suggestions, submit, wait, page edit, reply",
  ignore: !chrome,
  // Playwright keeps its own pipes and timers alive past browser.close(); they are not leaks from this test.
  sanitizeOps: false,
  sanitizeResources: false,
  fn: async () => {
    const dir = await Deno.makeTempDir({ prefix: "interactive-artifact-e2e-" });
    const pagePath = join(dir, "page.html");
    await Deno.copyFile(EXAMPLE, pagePath);
    // Reassigned once, when the test restarts the server for the same page.
    let { url, server } = await startServer(pagePath);
    const browser = await chromium.launch({ executablePath: chrome });
    try {
      const ctx = await browser.newContext({ viewport: { width: 1360, height: 900 } });
      await ctx.grantPermissions(["clipboard-read", "clipboard-write"], { origin: url });
      const page = await ctx.newPage();
      const errors: string[] = [];
      page.on("pageerror", (e) => errors.push(e.message));
      // Opened while the page file is missing, the tab shows the error inside the shell and recovers by itself once
      // the file is back.
      await Deno.rename(pagePath, `${pagePath}.away`);
      await page.goto(url);
      await page.locator(".ia-page", { hasText: "interactive-artifact could not render the page" }).waitFor();
      await Deno.rename(`${pagePath}.away`, pagePath);
      await page.locator('[data-value="sqlite"]').waitFor({ timeout: 5000 });
      await page.locator(".ia-dock").waitFor({ state: "attached" });

      // A single-choice picker clears on a second click; a multi-choice one keeps several.
      const sqlite = page.locator('[data-value="sqlite"]');
      await sqlite.locator("h3").click();
      await sqlite.locator("h3").click();
      assertEquals(await sqlite.getAttribute("aria-pressed"), "false");
      await sqlite.locator("h3").click();
      await page.click('[data-pick="rollout"] [data-value="flag"]');
      await page.click('[data-pick="rollout"] [data-value="shadow"]');
      await page.fill('[data-pick="rollout"] .ia-pick-text', "one week of shadow is enough");

      // Typing whole sentences checks that card layout does not steal focus after the first keystroke.
      await select(page, "a Redis outage no longer stops job processing");
      await page.click(".ia-toolbar button:has-text('Comment')");
      await page.keyboard.type("Even for jobs in flight?");
      await select(page, "slightly");
      await page.click(".ia-toolbar button:has-text('Suggest')");
      await page.keyboard.type("noticeably");
      assertEquals(await page.textContent(".ia-ins"), "noticeably");

      // Ids come from the server and may have gaps (every loaded tab reserves one), so read them off the cards.
      const c = (await page.locator(".ia-card.comment").getAttribute("data-id"))!;
      const sg = (await page.locator(".ia-card.suggestion").getAttribute("data-id"))!;
      await select(page, "Throughput");
      await page.evaluate(() => {
        const t = document.querySelector(".ia-page table")!;
        const r = getSelection()!.getRangeAt(0);
        r.setStartBefore(t);
        r.setEndAfter(t);
        document.dispatchEvent(new Event("selectionchange"));
      });
      await page.click(".ia-toolbar button:has-text('Copy as Markdown')");
      const copied = await page.evaluate(() => navigator.clipboard.readText());
      assertStringIncludes(
        copied,
        "| Metric | Redis | SQLite |\n| --- | --- | --- |\n| p99 enqueue | 10.8 ms | 4.1 ms |",
      );
      await page.evaluate(() => getSelection()!.removeAllRanges());

      // A live edit must not navigate or replace the executable shell. It must retain unsaved work and rebind the
      // pickers, even when it lands while the user is typing in a picker note.
      assertEquals(await page.textContent(".ia-count"), "4");
      await page.evaluate(() => ((globalThis as unknown as { marker: number }).marker = 1));
      const pickerNote = page.locator('[data-pick="rollout"] .ia-pick-text');
      await pickerNote.focus();
      await pickerNote.evaluate((t: HTMLTextAreaElement) => t.setSelectionRange(5, 9));
      await Deno.writeTextFile(pagePath, (await Deno.readTextFile(pagePath)) + "<p>Live edit arrived.</p>");
      await page.locator(".ia-page", { hasText: "Live edit arrived." }).waitFor();
      assertEquals(await page.evaluate(() => (globalThis as unknown as { marker: number }).marker), 1);
      assertEquals(
        await pickerNote.evaluate((
          t: HTMLTextAreaElement,
        ) => [document.activeElement === t, t.selectionStart, t.selectionEnd]),
        [true, 5, 9],
      );
      assertEquals(await sqlite.getAttribute("aria-pressed"), "true");
      assertEquals(await page.inputValue('[data-pick="rollout"] .ia-pick-text'), "one week of shadow is enough");

      // A refused request is recoverable. The original unsaved note is retried after the connection returns, with
      // no reload and no permission to type more edits that could look saved during the outage.
      let disconnected = true;
      const apiPattern = "**/__interactive-artifact/**";
      await page.route(apiPattern, async (route) => {
        if (disconnected) await route.abort("connectionrefused");
        else await route.continue();
      });
      await page.locator(".ia-note").fill("keep this unsaved note");
      await page.locator(".ia-reconnecting:not([hidden])").waitFor();
      assertEquals(await page.locator(".ia-note").inputValue(), "keep this unsaved note");
      assertEquals(await page.locator(".ia-note").evaluate((t: HTMLTextAreaElement) => t.readOnly), true);
      assertEquals(await page.locator(".ia-lock:not(.ia-reconnecting)").count(), 0);
      disconnected = false;
      await page.locator(".ia-reconnecting").waitFor({ state: "hidden" });
      await page.waitForFunction(() => !(document.querySelector(".ia-note") as HTMLTextAreaElement).readOnly);
      const key = await testKey(join(dir, ".interactive-artifact", "page.html"));
      // Drain the save stream before observing durable state: a version poll may have been the first retried call.
      await page.waitForTimeout(600);
      assertEquals((await testCall(url, key, "GET", "state")).data.note, "keep this unsaved note");
      await page.locator(".ia-note").fill("");
      await page.waitForTimeout(400);
      await page.unroute(apiPattern);

      // A request that never produces a response must recover too, after the real ten-second request timeout.
      let stalled = false;
      await page.route(apiPattern, async (route) => {
        if (!stalled && route.request().url().endsWith("/version")) {
          stalled = true;
          await page.waitForTimeout(11_000);
          await route.abort().catch(() => {});
        } else await route.continue();
      });
      await page.locator(".ia-reconnecting:not([hidden])").waitFor({ timeout: 15_000 });
      await page.locator(".ia-reconnecting").waitFor({ state: "hidden" });
      await page.unroute(apiPattern);

      // Losing a Submit response is ambiguous: the server may already have queued it. Execute the first request
      // and discard only its response, so the browser must retry without producing a second submission.
      let lostSubmit = false;
      let submitAttempts = 0;
      await page.route(apiPattern, async (route) => {
        if (route.request().url().endsWith("/submit")) {
          submitAttempts++;
          if (!lostSubmit) {
            lostSubmit = true;
            await route.fetch();
            await route.abort("connectionreset");
            return;
          }
        }
        await route.continue();
      });

      // The status pill follows the agent's side: sent at once, reading once `wait` hands the submission over,
      // working once the agent says so, and ready again when it next waits.
      const pill = page.locator(".ia-agent");
      await page.click(".ia-send");
      await pill.filter({ hasText: "Waiting for the agent" }).waitFor();
      assertEquals(submitAttempts, 2);
      assertEquals((await testCall(url, key, "GET", "state")).data.submissions, 1);
      await page.unroute(apiPattern);
      // Exact match: hasText is a case-insensitive substring match, and "Sent" also matches "Unsent reply".
      await page.locator(".ia-status", { hasText: /^Sent$/ }).nth(1).waitFor();
      const first = await cli("wait", pagePath, "--timeout", "5s");
      assertEquals(first.code, 0);
      for (
        const want of [
          "Queue backend (`backend`, changed): SQLite queue",
          "Feature flag, Two weeks of shadow traffic",
          `## ${c}: new comment`,
          "> Even for jobs in flight?",
          `## ${sg}: new suggestion`,
          "> noticeably",
        ]
      ) assertStringIncludes(first.out, want);
      await pill.filter({ hasText: "reading your submission" }).waitFor();
      assertEquals((await cli("status", pagePath, "working")).code, 0);
      await pill.filter({ hasText: "working on your submission" }).waitFor();
      // In a narrow window the status box must not cover the Submit dock: it wraps above it instead.
      await page.setViewportSize({ width: 720, height: 900 });
      await page.click(".ia-launch");
      const sendIsOnTop = await page.evaluate(() => {
        const b = document.querySelector(".ia-send")!.getBoundingClientRect();
        return !!document.elementFromPoint(b.x + b.width / 2, b.y + b.height / 2)?.closest(".ia-send");
      });
      assertEquals(sendIsOnTop, true);
      await page.keyboard.press("Escape");
      await page.locator(".ia-note").blur();
      await page.setViewportSize({ width: 1360, height: 900 });
      // Consumed submissions are not handed out twice.
      assertEquals((await cli("wait", pagePath, "--timeout", "1s")).code, 3);
      await pill.filter({ hasText: "ready for your next submission" }).waitFor();

      // Typing into a card right after a submit, with no reload in between, must still count and be saved. An
      // earlier version swapped the annotation objects on submit and left every card editing a dead copy.
      await page.locator(`.ia-card[data-id="${c}"] .ia-draft`).fill("one more thing");
      assertEquals(await page.textContent(".ia-count"), "1");
      await page.locator(`.ia-card[data-id="${c}"] .ia-draft`).fill("");
      assertEquals(await page.textContent(".ia-count"), "0");

      // The agent applies the suggestion in the source and answers both threads. The tab reloads for the edit
      // and picks up the replies; the applied suggestion's text is gone, so its card shows as orphaned.
      const src = await Deno.readTextFile(pagePath);
      await Deno.writeTextFile(pagePath, src.replace("drops\n  slightly,", "drops\n  noticeably,"));
      // A proxy in the environment must be ignored; it would otherwise see the tokened URL.
      const proxied = await new Deno.Command(SCRIPT, {
        args: ["reply", pagePath, sg, "--text", "Applied.", "--resolve"],
        env: { HTTP_PROXY: "http://127.0.0.1:9", http_proxy: "http://127.0.0.1:9" },
        stdout: "null",
        stderr: "null",
      }).output();
      assertEquals(proxied.code, 0);
      // --file is what SKILL.md tells agents to use, since even a heredoc ends up on a command line.
      await Deno.writeTextFile(join(dir, "reply.txt"), "Yes, they survive.\n");
      assertEquals((await cli("reply", pagePath, c, "--file", join(dir, "reply.txt"))).code, 0);
      assertEquals((await cli("reply", pagePath, "c99", "--text", "x")).code, 1);
      await page.locator(".ia-msg.agent", { hasText: "Yes, they survive." }).waitFor();
      await page.locator(`.ia-card.orphan.resolved[data-id="${sg}"]`).waitFor();
      assertEquals(await page.locator("mark.ia-anchor").count(), 1);

      // The agent rewrites the text an open comment is on, then re-points the comment at the new wording. Without
      // the re-point the card would be orphaned; with it, the highlight lands on the new text.
      const edited = (await Deno.readTextFile(pagePath)).replace("no longer stops job", "no longer halts job");
      await Deno.writeTextFile(pagePath, edited);
      await page.locator(`.ia-card.orphan[data-id="${c}"]`).waitFor();
      await Deno.writeTextFile(join(dir, "quote.txt"), "a Redis outage no longer halts job processing\n");
      assertEquals((await cli("anchor", pagePath, c, "--file", join(dir, "quote.txt"))).code, 0);
      await page.locator(`mark.ia-anchor[data-ann="${c}"]`, { hasText: "halts" }).first().waitFor();
      assertEquals(await page.locator(`.ia-card.orphan[data-id="${c}"]`).count(), 0);

      // Dismissing a resolved card tells the agent nothing; a reply goes out as a reply.
      await page.locator(`.ia-card[data-id="${sg}"] .ia-x`).click();
      // Submitted with Ctrl+Enter from the box itself: the box must come back empty, or the next keystroke would
      // copy the sent text back into the draft and send it again.
      const box = page.locator(`.ia-card[data-id="${c}"] .ia-draft`);
      await box.fill("Good, ship it.");
      assertEquals(await page.textContent(".ia-count"), "1");
      await box.press("Control+Enter");
      await page.locator(`.ia-card[data-id="${c}"] .ia-status`, { hasText: /^Sent$/ }).waitFor();
      assertEquals(await box.inputValue(), "");
      const second = await cli("wait", pagePath, "--timeout", "5s");
      assertStringIncludes(second.out, `## ${c}: reply on comment`);
      assertStringIncludes(second.out, "> Good, ship it.");
      assertEquals(second.out.includes(sg), false);

      assertEquals(errors, []);

      // The agent says it is done, with a message, and stops its server. The tab ends on a calm note rather than the
      // impostor warning, read-only like any locked tab.
      await Deno.writeTextFile(join(dir, "status.txt"), "Shipping option A; details in the chat.\n");
      assertEquals((await cli("status", pagePath, "done", "--file", join(dir, "status.txt"))).code, 0);
      await pill.filter({ hasText: "done with this page" }).filter({ hasText: "Shipping option A" }).waitFor();
      server.kill("SIGTERM");
      await server.status;
      await page.locator(".ia-lock.calm", { hasText: "This page is finished" }).waitFor();

      // A new server for the same page brings back everything the old tab saved, under a new link. The port takeover
      // check below runs on that fresh tab.
      ({ url, server } = await startServer(pagePath));
      await page.goto(url);
      await page.locator(`.ia-card[data-id="${c}"] .ia-msg.agent`).first().waitFor();
      // The old server's "done" is not carried over: it would tell the user to leave a page the agent just reopened.
      assertEquals(await pill.isVisible(), false);

      // Port takeover: even an authentic recorded reply is not fresh proof of the server. Capture a version answer
      // and have the impostor replay it for a later call; the tab must lock and must not
      // reload into the impostor's page, and must never hand it anything readable, even as the user keeps typing.
      await page.evaluate(() => ((globalThis as unknown as { marker: number }).marker = 1));
      const captured = await page.waitForResponse((r) => r.url().endsWith("/version"));
      const replay = await captured.text();
      server.kill("SIGTERM");
      await server.status;
      const seen: string[] = [];
      let referrerLeaked = false;
      const impostor = Deno.serve(
        { hostname: "127.0.0.1", port: Number(new URL(url).port), onListen() {} },
        async (r) => {
          referrerLeaked ||= r.headers.has("referer");
          seen.push(`${r.method} ${new URL(r.url).pathname} ${await r.text()}`);
          return new Response(replay, { headers: { "content-type": "application/json" } });
        },
      );
      // A poll in the free-port window reconnects; the next attempt reaches the impostor and fails verification.
      try {
        // With the server gone, nobody can submit, so wait must not block forever.
        assertEquals((await cli("wait", pagePath, "--timeout", "1s")).code, 4);
        await page.locator(".ia-lock:not(.ia-reconnecting)").waitFor();
        // A locked tab is read-only, so nothing typed after the lock can be silently lost. Typing goes nowhere; the
        // set value stands in for text the user had on screen anyway, and Submit must still send nothing.
        const box = page.locator(`.ia-card[data-id="${c}"] .ia-draft`);
        assertEquals(await box.evaluate((t: HTMLTextAreaElement) => t.readOnly), true);
        await box.evaluate((t: HTMLTextAreaElement) => {
          t.value = "the launch codes are 0000";
          t.dispatchEvent(new Event("input"));
        });
        await page.click(".ia-send", { force: true }).catch(() => {});
        // A submit attempt on the locked tab, by button or Ctrl+Enter, must not make anything writable again.
        await page.keyboard.press("Control+Enter");
        assertEquals(await page.locator(".ia-note").evaluate((t: HTMLTextAreaElement) => t.readOnly), true);
        assertEquals(await box.evaluate((t: HTMLTextAreaElement) => t.readOnly), true);
        await page.waitForTimeout(1500);
        assertEquals(await page.evaluate(() => (globalThis as unknown as { marker?: number }).marker), 1);
        assertEquals(seen.length > 0, true);
        assertEquals(seen.every((line) => line.includes("/__interactive-artifact/")), true);
        assertEquals(seen.some((line) => line.includes(new URL(url).pathname)), false);
        assertEquals(referrerLeaked, false);
        assertEquals(seen.filter((line) => line.includes("launch codes") || line.includes("ship it")), []);
      } finally {
        await impostor.shutdown();
      }
    } finally {
      await browser.close();
      // Already stopped on the happy path; this is for a failure earlier in the test.
      try {
        server.kill("SIGTERM");
      } catch {
        // exited
      }
      await server.status;
      await Deno.remove(dir, { recursive: true });
    }
  },
});

/**
 * interactive-artifact browser client: everything interactive on a served page.
 *
 * The server owns the durable state (see the `interactive-artifact` script); this file renders it and edits the parts
 * the browser owns. The sections below are, in order: theme button, state and server I/O, text anchoring, pickers,
 * annotation highlights, the comment sidebar, the selection toolbar with Copy as Markdown, the Submit dock, the agent
 * status box, and the poll loop that drives live updates and picks up agent replies and status changes.
 *
 * Two invariants hold throughout:
 *
 * - Text offsets are measured over the text nodes of `main.ia-page`, skipping anything this file injects
 *   (`SKIP`). Highlight marks wrap existing text without adding any, so offsets mean the same thing with or
 *   without highlights drawn, and a new selection can be anchored while old highlights are on screen.
 * - Inputs the user may be typing in are never rebuilt. Cards are created once per annotation and updated in
 *   place, and a focused textarea's value is never overwritten from state.
 */
(() => {
  "use strict";

  const page = document.querySelector(".ia-page");
  const notes = document.querySelector(".ia-notes");
  /** Elements whose text is not part of the page as written: our own insertions and form fields. */
  // SVG is skipped too: an HTML <mark> inside SVG text does not render, so a highlight would make a chart label
  // vanish, and hidden <title> text would leak into quotes.
  const SKIP = ".ia-ins, .ia-ui, script, style, textarea, input, svg";
  /** Must match the breakpoint in interactive-artifact.css where the sidebar becomes a floating column. */
  const WIDE = matchMedia("(min-width: 72rem)");
  const CONTEXT = 32;

  const el = (tag, cls, text) => {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text !== undefined) e.textContent = text;
    return e;
  };
  const collapse = (s) => s.replace(/\s+/g, " ").trim();
  const truncate = (s, n) => (s.length > n ? s.slice(0, n - 1) + "…" : s);

  // -----------------------------------------------------------------------------------------------------------
  // Theme button: cycles system, light, dark. See THEME_HEAD_SCRIPT in the server for why it is a cookie.
  // -----------------------------------------------------------------------------------------------------------

  const root = document.documentElement;
  const themeButton = document.querySelector(".ia-theme");
  const modes = ["system", "light", "dark"];
  const currentMode = () => root.dataset.theme || "system";
  const nextMode = () => modes[(modes.indexOf(currentMode()) + 1) % modes.length];
  const labelTheme = () => {
    themeButton.title = `Theme: ${currentMode()} (click for ${nextMode()})`;
    themeButton.setAttribute("aria-label", themeButton.title);
  };
  themeButton.addEventListener("click", () => {
    const next = nextMode();
    const cookie = `interactive-artifact-theme=${next}; path=/; SameSite=Lax; max-age=`;
    if (next === "system") {
      delete root.dataset.theme;
      document.cookie = cookie + "0";
    } else {
      root.dataset.theme = next;
      document.cookie = cookie + 31536000;
    }
    labelTheme();
  });
  labelTheme();

  // -----------------------------------------------------------------------------------------------------------
  // State and server I/O
  // -----------------------------------------------------------------------------------------------------------

  /** Same shape as the server's Annotation; removed ones stay here until the server confirms it dropped them. */
  let annotations = [];
  // Null prototype, because picker ids come from page markup: `data-pick="constructor"` must not find Object's.
  let picks = Object.create(null);
  let picksSent = {};
  const dirtyPicks = new Set();
  /** Ids of annotations this tab created that the server has not yet echoed back in a state. */
  const createdHere = new Set();
  /** Annotations with local edits not yet saved. Only these go out on a save; see flush. */
  const dirtyAnns = new Set();
  /**
   * Annotations sent in a save that has not been answered yet. They count as dirty until then: a state fetch can
   * be answered before the save lands, and adopting its older copy would wipe the edit (and an emptied new comment
   * would then remove itself).
   */
  const inflight = new Map();
  const isDirty = (id) => dirtyAnns.has(id) || inflight.has(`a:${id}`);
  /** The same for picks: one this tab changed and has not had confirmed keeps its local value. */
  const pickDirty = (id) => dirtyPicks.has(id) || inflight.has(`p:${id}`);
  let noteDirty = false;
  let serverRev = null;
  let activeId = null;

  // Message encryption: the browser half of "Message encryption" in the interactive-artifact script, which explains the
  // threat (an impostor binding the port after the server exits). Every request is sealed with the per-server key the
  // page was served with, and every response must open under it for this particular call. A failed connection retries
  // without fetching code or dropping edits; a response that cannot be verified locks the page for good. Envelopes
  // and additional data must stay byte-compatible with the server.
  //
  // Read through the prototype's getter: an element in the page with name="currentScript" would otherwise shadow it
  // (document's named properties win over its own), and the client would die before it could show any banner.
  const me = Object.getOwnPropertyDescriptor(Document.prototype, "currentScript").get.call(document);
  /** A separate capability from the page URL: reconnect traffic must never authorize file or key retrieval. */
  const API = me?.dataset.api;
  const keyB64 = me?.dataset.key || "";
  /** The page file's version as the server rendered it, or "missing"; see renderPage in the server. */
  const renderedVersion = me?.dataset.version;
  const keyReady = crypto.subtle && keyB64
    ? crypto.subtle.importKey("raw", Uint8Array.from(atob(keyB64), (c) => c.charCodeAt(0)), "AES-GCM", false, [
      "encrypt",
      "decrypt",
    ])
    : Promise.reject(new Error("no Web Crypto"));
  keyReady.catch(() => {});
  const toB64 = (bytes) => {
    let bin = "";
    for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
    return btoa(bin);
  };
  const fromB64 = (text) => Uint8Array.from(atob(text), (c) => c.charCodeAt(0));
  const enc = new TextEncoder();
  const seal = async (aad, value) => {
    const iv = crypto.getRandomValues(new Uint8Array(12));
    const ct = await crypto.subtle.encrypt(
      { name: "AES-GCM", iv, additionalData: enc.encode(aad) },
      await keyReady,
      enc.encode(JSON.stringify(value)),
    );
    return { iv: toB64(iv), ct: toB64(new Uint8Array(ct)) };
  };
  const unseal = async (aad, env) => {
    const iv = fromB64(env.iv);
    if (iv.length !== 12) throw new Error("bad iv");
    const plain = await crypto.subtle.decrypt(
      { name: "AES-GCM", iv, additionalData: enc.encode(aad) },
      await keyReady,
      fromB64(env.ct),
    );
    return JSON.parse(new TextDecoder().decode(plain));
  };

  let locked = null;
  // The first state snapshot replaces browser-owned fields. Editing must stay blocked until it is adopted, even
  // if transport recovery has already authenticated a response and hidden the reconnecting warning.
  let loading = true;
  let reconnecting = false;
  const loadingMessage = "Loading your saved answers. Editing will be available when loading finishes.";
  const connectionBanner = el("div", "ia-lock ia-ui ia-reconnecting");
  connectionBanner.setAttribute("role", "status");
  connectionBanner.textContent = loadingMessage;
  document.body.append(connectionBanner);
  /** Inputs freeze temporarily so an outage cannot build an unbounded queue of save snapshots in memory. */
  const frozenInputs = new Map();
  const connectionLost = () => {
    if (locked || reconnecting) return;
    reconnecting = true;
    connectionBanner.hidden = false;
    connectionBanner.textContent =
      "Connection lost. Reconnecting automatically; your unsaved work is kept in this tab. Do not reload.";
    for (const t of document.querySelectorAll("textarea")) {
      frozenInputs.set(t, t.readOnly);
      t.readOnly = true;
    }
    updateDock();
  };
  /** Restore only the input permissions the outage changed; a pending Submit still owns its own freeze. */
  const connectionRestored = () => {
    if (locked || !reconnecting) return;
    reconnecting = false;
    connectionBanner.hidden = !loading;
    if (loading) connectionBanner.textContent = loadingMessage;
    for (const [t, readOnly] of frozenInputs) t.readOnly = readOnly;
    frozenInputs.clear();
    updateDock();
  };
  /** Stop all traffic for good and say why. Reloading would fetch the page from whatever holds the port now. */
  // After an authenticated "done", a transport failure is the expected end (the agent stopped its server), so the
  // banner is calm. Reloading is still unsafe: with the server gone, anyone can take its port and serve a new page.
  const lock = (why, stopped = false) => {
    if (locked) return;
    locked = why;
    connectionBanner.hidden = true;
    // A permanent lock permits copying visible content. The edit guards and read-only inputs still prevent writes.
    page.inert = notes.inert = dock.inert = false;
    clearTimeout(saveTimer);
    // Read-only from here: anything typed into a locked tab could never be saved, so it must not look as if it
    // could be. What is on screen stays selectable, so text in the last unsaved moments can still be copied.
    for (const t of document.querySelectorAll("textarea")) t.readOnly = true;
    const finished = stopped && agentState === "done";
    const banner = el("div", `ia-lock ia-ui${finished ? " calm" : ""}`);
    banner.setAttribute("role", finished ? "status" : "alert");
    if (finished) {
      banner.append(
        el("strong", "", "This page is finished."),
        " Its server has stopped; continue in the chat. Do not reload this tab.",
      );
    } else {
      banner.append(
        el("strong", "", why),
        " This tab is now read-only. Everything it saved is kept: ask the agent for a new link and it will be there. " +
          "Do not reload this tab.",
      );
    }
    document.body.append(banner);
    statusPill.hidden = true;
    updateDock();
  };

  // Calls are one ordered stream. Reusing the same identity until its answer arrives lets the server replay the
  // answer, rather than repeat a Submit or id allocation whose first response disappeared with the connection.
  const client = Array.from(crypto.getRandomValues(new Uint8Array(16)), (b) => b.toString(16).padStart(2, "0")).join(
    "",
  );
  let sequence = 0;
  let calls = Promise.resolve();
  const api = (path, method = "GET", body = {}) => {
    const result = calls.then(() => request(path, method, body));
    calls = result.catch(() => {});
    return result;
  };
  /** Retry transport failures with the original identity, accepting only a response bound to this call. */
  const request = async (path, method, body) => {
    if (locked) throw new Error(locked);
    const call = `${method} ${path}`;
    const seq = ++sequence;
    const sealed = JSON.stringify(await seal(call, { client, seq, data: body }));
    // Never follow a redirect: only this page's own server may answer, and it never redirects an API call.
    const init = {
      method,
      cache: "no-store",
      redirect: "error",
      headers: { "content-type": "application/json", "x-interactive-artifact": method === "GET" ? sealed : "browser" },
      ...(method !== "GET" ? { body: sealed } : {}),
    };
    let res, text;
    for (let delay = 1000;; delay = Math.min(delay * 2, 5000)) {
      if (locked) throw new Error(locked);
      try {
        // The timeout covers the body too: a peer that sends headers and then stalls must also show reconnecting.
        res = await fetch(API + path, { ...init, signal: AbortSignal.timeout(10_000) });
        text = await res.text();
        break;
      } catch (e) {
        if (agentState === "done") {
          lock("The interactive-artifact server has stopped.", true);
          throw e;
        }
        connectionLost();
        await new Promise((resolve) => setTimeout(resolve, delay));
      }
    }
    let reply;
    try {
      if (!res.ok) throw new Error(String(res.status));
      reply = await unseal(`response ${call} ${client} ${seq}`, JSON.parse(text));
    } catch {
      lock(
        "This page's server could not be verified. Possible causes: a proxy or browser extension rewriting " +
          "requests to 127.0.0.1, the server restarted on the same port, or another program took over its port.",
      );
      throw new Error(locked);
    }
    if (locked) throw new Error(locked);
    connectionRestored();
    if (reply.status !== 200) {
      const err = new Error(String(reply.data));
      err.status = reply.status;
      throw err;
    }
    return reply.data;
  };

  const BROWSER_FIELDS = [
    "id",
    "kind",
    "quote",
    "prefix",
    "suffix",
    "offset",
    "section",
    "replacement",
    "draft",
    "removed",
  ];
  let saveTimer = null;
  let saving = Promise.resolve(true);

  /** Send the browser-owned state now. Every save is chained, so a later save never lands before an earlier one. */
  const flush = () => {
    clearTimeout(saveTimer);
    if (locked) return Promise.resolve(false);
    saveTimer = null;
    // Only the picks and note this tab changed go out; the server merges them per item, so another tab's answers
    // are never overwritten by this tab's stale copy of them.
    // Annotations likewise: a tab sends only the ones it edited. A stale tab resending everything would put back
    // drafts another tab already submitted (the agent would get them twice) and undo other tabs' removals.
    const body = {
      annotations: annotations.filter((a) => dirtyAnns.has(a.id))
        .map((a) => Object.fromEntries(BROWSER_FIELDS.map((k) => [k, a[k]]))),
      picks: Object.fromEntries([...dirtyPicks].map((id) => [id, picks[id]])),
      ...(noteDirty ? { note: noteInput.value } : {}),
    };
    const sent = [...[...dirtyAnns].map((id) => `a:${id}`), ...[...dirtyPicks].map((id) => `p:${id}`)];
    const sentNote = noteDirty;
    for (const key of sent) inflight.set(key, (inflight.get(key) || 0) + 1);
    dirtyPicks.clear();
    dirtyAnns.clear();
    noteDirty = false;
    const settle = () => {
      for (const key of sent) {
        if (inflight.get(key) > 1) inflight.set(key, inflight.get(key) - 1);
        else inflight.delete(key);
      }
    };
    // Submit must not run if its prerequisite save failed. Keep failed fields dirty so a later save sends their
    // current values, including any edits made while this request was in flight.
    saving = saving.then(() => api("state", "PUT", body)).then((r) => {
      settle();
      const kept = new Set(r.ids);
      annotations = annotations.filter((a) => !a.removed || kept.has(a.id));
      return true;
    }).catch((e) => {
      settle();
      if (!locked) {
        for (const field of sent) {
          if (field.startsWith("a:")) dirtyAnns.add(field.slice(2));
          else dirtyPicks.add(field.slice(2));
        }
        noteDirty ||= sentNote;
        clearTimeout(saveTimer);
        saveTimer = setTimeout(flush, 2000);
        toast(`Could not save: ${e.message}`, true);
      }
      return false;
    });
    return saving;
  };
  /** Schedule a save. Pass the annotation that was edited, if any, so the save includes it. */
  const changed = (a) => {
    if (a) dirtyAnns.add(a.id);
    clearTimeout(saveTimer);
    saveTimer = setTimeout(flush, 300);
    updateDock();
  };

  /**
   * Take server-owned fields from a fresh copy of the state. With `all`, also take browser-owned ones; that is
   * only safe on first load and right after a submit, when there are no local edits the server has not seen.
   */
  const adopt = (s, all) => {
    const local = new Map(annotations.map((a) => [a.id, a]));
    const next = [];
    let replies = 0;
    for (const sa of s.annotations) {
      const a = local.get(sa.id);
      if (!a) {
        next.push(sa);
      } else {
        replies += sa.thread.filter((m) => m.from === "agent").length -
          a.thread.filter((m) => m.from === "agent").length;
        // Update in place, never replace: each card's listeners hold their annotation object, and a replaced
        // object would leave the card editing a copy that nothing saves or submits any more.
        // Browser-owned fields come from the server too unless this tab has unsaved edits to the annotation:
        // otherwise a draft another tab submitted would stay here and go out a second time.
        Object.assign(
          a,
          all || !isDirty(a.id) ? sa : {
            thread: sa.thread,
            sentOnce: sa.sentOnce,
            resolved: sa.resolved,
            // The anchor is the server's once created (the agent can re-point it), so it always comes along.
            quote: sa.quote,
            prefix: sa.prefix,
            suffix: sa.suffix,
            offset: sa.offset,
            section: sa.section,
          },
        );
        next.push(a);
      }
      local.delete(sa.id);
    }
    // Local-only annotations survive only if this tab created them and the server has not seen them yet. Anything
    // else missing from the server was dropped there on purpose: another tab removed it, or a withdrawal was
    // submitted. Keeping it would leave a ghost card whose next save brings it back.
    for (const id of createdHere) if (s.annotations.some((sa) => sa.id === id)) createdHere.delete(id);
    // That holds after a submit too: an annotation created while the submit was in flight is not in its answer yet.
    for (const a of local.values()) if (createdHere.has(a.id)) next.push(a);
    annotations = next;
    picksSent = s.picksSent;
    serverRev = s.rev;
    showStatus(s.status);
    if (all) {
      picks = Object.assign(Object.create(null), s.picks);
      noteInput.value = s.note || "";
    } else {
      // Picks another tab made: without them this tab would compare its stale picks against the new picksSent and
      // show a pending change that is not there.
      for (const [id, p] of Object.entries(s.picks)) if (!pickDirty(id)) picks[id] = p;
    }
    return replies;
  };

  // -----------------------------------------------------------------------------------------------------------
  // Text anchoring
  // -----------------------------------------------------------------------------------------------------------

  /** Concatenated page text plus where each text node starts in it. Rebuilt whenever the DOM may have changed. */
  const textIndex = () => {
    const walker = document.createTreeWalker(page, NodeFilter.SHOW_TEXT, {
      acceptNode: (n) => (n.parentElement.closest(SKIP) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT),
    });
    const nodes = [];
    let text = "";
    for (let n; (n = walker.nextNode());) {
      nodes.push({ node: n, start: text.length });
      text += n.data;
    }
    return { text, nodes };
  };

  /** Convert a DOM boundary point to a text offset. A point between elements maps to the next text node's start. */
  const toOffset = (idx, container, offset) => {
    if (container.nodeType === Node.TEXT_NODE) {
      const hit = idx.nodes.find((e) => e.node === container);
      if (hit) return hit.start + offset;
    }
    const r = document.createRange();
    r.setStart(container, offset);
    for (const e of idx.nodes) if (r.comparePoint(e.node, 0) >= 0) return e.start;
    return idx.text.length;
  };

  const commonSuffix = (a, b) => {
    let n = 0;
    while (n < a.length && n < b.length && a[a.length - 1 - n] === b[b.length - 1 - n]) n++;
    return n;
  };
  const commonPrefix = (a, b) => {
    let n = 0;
    while (n < a.length && n < b.length && a[n] === b[n]) n++;
    return n;
  };

  /**
   * Best occurrence of `quote` in `text`, scored by how much of the stored context still surrounds it. `offset`, where
   * the quote was when it was made, wins every tie, so on an unchanged page the result is exactly the original spot,
   * even among identical repeats with identical context.
   */
  const bestMatch = (text, quote, prefix, suffix, offset) => {
    let best = -1;
    let bestScore = -1;
    for (let i = text.indexOf(quote); i !== -1 && quote; i = text.indexOf(quote, i + 1)) {
      const score = commonSuffix(text.slice(Math.max(0, i - prefix.length), i), prefix) +
        commonPrefix(text.slice(i + quote.length, i + quote.length + suffix.length), suffix) + (i === offset ? 0.5 : 0);
      if (score > bestScore) [best, bestScore] = [i, score];
    }
    return best;
  };

  /**
   * Find an annotation's text in the current page, or null if it is gone. Exact match first; failing that, match
   * with all whitespace runs collapsed, because agents often reflow HTML source when they edit around a passage,
   * which changes the rendered text's whitespace without changing what it says.
   */
  const locate = (idx, a) => {
    const exact = bestMatch(idx.text, a.quote, a.prefix, a.suffix, a.offset);
    if (exact >= 0) return { start: exact, end: exact + a.quote.length };
    const map = [];
    let norm = "";
    for (let i = 0; i < idx.text.length; i++) {
      const ws = /\s/.test(idx.text[i]);
      if (ws && norm.endsWith(" ")) continue;
      map.push(i);
      norm += ws ? " " : idx.text[i];
    }
    map.push(idx.text.length);
    const n = (s) => s.replace(/\s+/g, " ");
    const quote = n(a.quote).trim();
    const at = bestMatch(norm, quote, n(a.prefix), n(a.suffix));
    return at < 0 ? null : { start: map[at], end: map[at + quote.length - 1] + 1 };
  };

  /** Text of the last heading at or before a node, so the agent knows roughly where an annotation sits. */
  const sectionOf = (node) => {
    let found = "";
    for (const h of page.querySelectorAll("h1, h2, h3, h4")) {
      if (h === node || h.contains(node) || h.compareDocumentPosition(node) & Node.DOCUMENT_POSITION_FOLLOWING) {
        found = collapse(h.textContent);
      }
    }
    return found;
  };

  // -----------------------------------------------------------------------------------------------------------
  // Pickers: any [data-pick] element; its [data-value] descendants (not inside a nested picker) are choices.
  // -----------------------------------------------------------------------------------------------------------

  // "__proto__" is refused by the server (it would set an object's prototype), so it is not a picker here either;
  // otherwise its answer would count as pending forever.
  let pickers = [];
  const choicesOf = (p) => [...p.querySelectorAll("[data-value]")].filter((c) => c.closest("[data-pick]") === p);
  // The picker's own question, not a heading inside one of its choices (an option card's <h3> is the choice's name).
  const pickerLabel = (p) => {
    const own = [...p.querySelectorAll(".q, h1, h2, h3, h4, h5, h6, legend")]
      .find((e) => !p.contains(e.closest("[data-value]")));
    return collapse(p.dataset.label || own?.textContent || p.dataset.pick);
  };
  const choiceLabel = (c) =>
    truncate(
      collapse(c.dataset.label || c.querySelector("h1, h2, h3, h4, h5, h6, strong")?.textContent || c.textContent),
      120,
    );

  const readPicker = (p) => {
    const values = choicesOf(p).filter((c) => c.classList.contains("ia-selected"))
      .map((c) => ({ value: c.dataset.value, label: choiceLabel(c) }));
    const text = p.querySelector(":scope > .ia-pick-text")?.value || "";
    // An emptied picker stays as an empty pick so the server can report the cleared answer with its label.
    picks[p.dataset.pick] = { label: pickerLabel(p), multiple: "multiple" in p.dataset, values, text };
    dirtyPicks.add(p.dataset.pick);
    changed();
  };

  const toggleChoice = (p, c) => {
    // Frozen during a submit, like the cards (see submit()), and for good once the tab locks.
    if (loading || submitting || locked || reconnecting) return;
    const on = !c.classList.contains("ia-selected");
    if (!("multiple" in p.dataset)) { for (const o of choicesOf(p)) o.classList.remove("ia-selected"); }
    c.classList.toggle("ia-selected", on);
    for (const o of choicesOf(p)) o.setAttribute("aria-pressed", String(o.classList.contains("ia-selected")));
    readPicker(p);
  };

  /** Bind each newly received fragment once. The shell, cards, drafts, and connection key stay in this document. */
  const initPickers = () => {
    pickers = [...page.querySelectorAll("[data-pick]")].filter((p) => p.dataset.pick !== "__proto__");
    for (const p of pickers) {
      p.classList.add("ia-picker");
      p.setAttribute("role", "group");
      p.setAttribute("aria-label", pickerLabel(p));
      for (const c of choicesOf(p)) {
        c.classList.add("ia-choice");
        c.tabIndex = 0;
        c.setAttribute("role", "button");
        c.setAttribute("aria-pressed", "false");
        c.addEventListener("click", (ev) => {
          // A drag that selected text inside a choice is the start of a comment, not a pick; links keep working.
          if (!getSelection().isCollapsed || ev.target.closest("a, textarea, input")) return;
          toggleChoice(p, c);
        });
        c.addEventListener("keydown", (ev) => {
          if (ev.target === c && (ev.key === "Enter" || ev.key === " ")) {
            ev.preventDefault();
            toggleChoice(p, c);
          }
        });
      }
      if (!("noText" in p.dataset)) {
        const t = el("textarea", "ia-pick-text ia-input");
        t.rows = 1;
        t.placeholder = p.dataset.placeholder || "Add a note (optional)";
        t.addEventListener("input", () => {
          autoGrow(t);
          readPicker(p);
        });
        p.append(t);
      }
    }
  };
  initPickers();

  const applyPicks = () => {
    for (const p of pickers) {
      const s = picks[p.dataset.pick];
      const chosen = new Set((s?.values || []).map((v) => v.value));
      for (const c of choicesOf(p)) {
        c.classList.toggle("ia-selected", chosen.has(c.dataset.value));
        c.setAttribute("aria-pressed", String(chosen.has(c.dataset.value)));
      }
      const t = p.querySelector(":scope > .ia-pick-text");
      if (t && document.activeElement !== t) {
        t.value = s?.text || "";
        autoGrow(t);
      }
    }
  };

  // -----------------------------------------------------------------------------------------------------------
  // Highlights
  // -----------------------------------------------------------------------------------------------------------

  const visible = () => annotations.filter((a) => !a.removed);

  /** Remove all highlights and redraw them from state. Orphaned annotations are flagged instead of drawn. */
  const renderMarks = () => {
    for (const m of page.querySelectorAll("mark.ia-anchor")) m.replaceWith(...m.childNodes);
    for (const i of page.querySelectorAll(".ia-ins")) i.remove();
    page.normalize();
    for (const a of visible()) {
      const idx = textIndex();
      const hit = locate(idx, a);
      a.orphan = !hit;
      if (!hit || a.resolved) continue;
      let last = null;
      for (const e of idx.nodes) {
        const len = e.node.data.length;
        if (e.start + len <= hit.start || e.start >= hit.end) continue;
        const from = Math.max(hit.start - e.start, 0);
        const to = Math.min(hit.end - e.start, len);
        // Whitespace-only pieces sit between block elements; wrapping them would put a <mark> inside <ul>.
        if (!e.node.data.slice(from, to).trim()) continue;
        let node = e.node;
        if (to < len) node.splitText(to);
        if (from > 0) node = node.splitText(from);
        const m = el("mark", `ia-anchor ${a.kind}`);
        m.dataset.ann = a.id;
        node.replaceWith(m);
        m.append(node);
        last = m;
      }
      if (last && a.kind === "suggestion" && a.replacement !== a.quote) {
        const ins = el("ins", "ia-ins", a.replacement);
        ins.dataset.ann = a.id;
        last.after(ins);
      }
    }
    markActive();
  };

  const markActive = () => {
    for (const m of page.querySelectorAll("[data-ann]")) m.classList.toggle("active", m.dataset.ann === activeId);
    for (const [id, c] of cards) c.classList.toggle("active", id === activeId);
  };

  const setActive = (id) => {
    activeId = id;
    markActive();
    requestLayout();
  };

  page.addEventListener("click", (ev) => {
    const m = ev.target.closest("[data-ann]");
    if (m && getSelection().isCollapsed) setActive(m.dataset.ann);
  });

  // -----------------------------------------------------------------------------------------------------------
  // Sidebar cards
  // -----------------------------------------------------------------------------------------------------------

  const cards = new Map();

  /** Fit a textarea to its content. A hidden textarea measures 0, so it keeps its natural height until shown. */
  const autoGrow = (t) => {
    t.style.height = "auto";
    if (!t.scrollHeight) {
      t.style.height = "";
      return;
    }
    t.style.height = `${t.scrollHeight + 2}px`;
  };

  const remove = (a) => {
    // Removal is an edit too; it must respect the same freezes as typing and choosing a picker.
    if (loading || locked || reconnecting || submitting) return;
    a.removed = true;
    if (activeId === a.id) activeId = null;
    renderAll();
    changed(a);
  };

  const createCard = (a) => {
    const c = el("div", `ia-card ${a.kind}`);
    c.dataset.id = a.id;
    const head = el("div", "ia-card-head");
    const x = el("button", "ia-x");
    x.type = "button";
    x.title = "Remove";
    x.setAttribute("aria-label", "Remove");
    x.addEventListener("click", (ev) => {
      ev.stopPropagation();
      remove(a);
    });
    head.append(
      el("span", "ia-kind", a.kind === "comment" ? "Comment" : "Suggestion"),
      el("span", "ia-id", a.id),
      el("span", "ia-status"),
      x,
    );
    c.append(head, el("div", "ia-quote"));
    if (a.kind === "suggestion") {
      const r = el("textarea", "ia-repl ia-input");
      r.rows = 1;
      r.placeholder = "Replace with (empty deletes the text)";
      r.value = a.replacement;
      r.addEventListener("input", () => {
        a.replacement = r.value;
        autoGrow(r);
        const ins = page.querySelector(`.ia-ins[data-ann="${a.id}"]`);
        // Toggling between "same as original" and "different" adds or removes the insertion, which only a
        // redraw can do; otherwise update the inserted text in place so typing stays cheap.
        if (ins && a.replacement !== a.quote) ins.textContent = a.replacement;
        else renderMarks();
        changed(a);
      });
      c.append(el("div", "ia-label", "Replace with"), r);
    }
    c.append(el("div", "ia-thread"), el("div", "ia-orphan-note", "This text is no longer on the page."));
    const d = el("textarea", "ia-draft ia-input");
    d.rows = 1;
    d.value = a.draft;
    d.addEventListener("input", () => {
      a.draft = d.value;
      autoGrow(d);
      changed(a);
      updateCard(a);
    });
    d.addEventListener("keydown", (ev) => {
      if (ev.key === "Enter" && (ev.metaKey || ev.ctrlKey)) submit();
      if (ev.key === "Escape") d.blur();
    });
    // An empty comment that was never sent is an abandoned click, the same as cancelling in a docs editor.
    d.addEventListener("blur", () => {
      setTimeout(() => {
        // Not during a submit: freezing the cards blurs them, and the submit's state would bring the card back.
        if (submitting) return;
        if (a.kind === "comment" && !a.sentOnce && !a.draft.trim() && !c.contains(document.activeElement)) remove(a);
      }, 150);
    });
    c.append(d);
    c.addEventListener("click", () => setActive(a.id));
    new ResizeObserver(requestLayout).observe(c);
    notes.append(c);
    cards.set(a.id, c);
    return c;
  };

  const updateCard = (a) => {
    const c = cards.get(a.id) || createCard(a);
    const pending = a.draft.trim() || (a.kind === "suggestion" && !a.sentOnce && a.replacement !== a.quote);
    const status = a.resolved ? "Resolved" : !a.sentOnce ? "Draft" : pending ? "Unsent reply" : "Sent";
    c.querySelector(".ia-status").textContent = status;
    c.classList.toggle("resolved", !!a.resolved);
    c.classList.toggle("orphan", !!a.orphan);
    c.classList.toggle("pending", !!pending);
    c.querySelector(".ia-quote").textContent = truncate(collapse(a.quote), 180);
    const r = c.querySelector(".ia-repl");
    if (r) {
      r.readOnly = a.sentOnce || !!locked || reconnecting;
      if (document.activeElement !== r) r.value = a.replacement;
      autoGrow(r);
    }
    const thread = c.querySelector(".ia-thread");
    thread.replaceChildren(...a.thread.map((m) => {
      const msg = el("div", `ia-msg ${m.from}`);
      msg.append(el("span", "ia-who", m.from === "agent" ? "Agent" : "You"), el("p", "", m.text));
      return msg;
    }));
    const d = c.querySelector(".ia-draft");
    d.readOnly = !!locked || reconnecting;
    d.placeholder = a.sentOnce
      ? (a.resolved ? "Reply to reopen" : "Reply")
      : a.kind === "comment"
      ? "Add a comment"
      : "Why? (optional)";
    if (document.activeElement !== d) d.value = a.draft;
    autoGrow(d);
  };

  const renderCards = () => {
    const live = new Set(visible().map((a) => a.id));
    for (const [id, c] of cards) {
      if (!live.has(id)) {
        c.remove();
        cards.delete(id);
      }
    }
    for (const a of visible()) updateCard(a);
    notes.classList.toggle("empty", !live.size);
    requestLayout();
  };

  /**
   * Place cards next to their highlights, docs-style. The active card sits exactly beside its text; cards above it
   * are pushed up and cards below pushed down so none overlap. Orphans and resolved cards go after the rest. On
   * narrow screens the sidebar is a plain list below the page and this only fixes the order.
   */
  let layoutQueued = false;
  const requestLayout = () => {
    if (layoutQueued) return;
    layoutQueued = true;
    requestAnimationFrame(() => {
      layoutQueued = false;
      layoutCards();
    });
  };
  const layoutCards = () => {
    const items = visible().map((a) => {
      const m = page.querySelector(`mark[data-ann="${a.id}"]`);
      return { a, c: cards.get(a.id), y: m ? m.getBoundingClientRect().top : Infinity };
    }).filter((it) => it.c);
    items.sort((p, q) => p.y - q.y);
    // Moving a node blurs any textarea inside it, and this runs on every keystroke that resizes a card, so only
    // move cards when the order really changed, and put the caret back where it was.
    if (items.some((it, i) => notes.children[i] !== it.c)) {
      const focused = notes.contains(document.activeElement) ? document.activeElement : null;
      const [from, to] = focused ? [focused.selectionStart, focused.selectionEnd] : [0, 0];
      for (const it of items) notes.append(it.c);
      if (focused) {
        focused.focus({ preventScroll: true });
        focused.setSelectionRange(from, to);
      }
    }
    if (!WIDE.matches) {
      for (const it of items) it.c.style.top = "";
      notes.style.minHeight = "";
      return;
    }
    const base = notes.getBoundingClientRect().top;
    const gap = 10;
    let lastFixed = 0;
    for (const it of items) {
      it.h = it.c.offsetHeight;
      it.want = Number.isFinite(it.y) ? it.y - base - 6 : null;
      if (it.want !== null) lastFixed = Math.max(lastFixed, it.want);
    }
    let pivot = items.findIndex((it) => it.a.id === activeId && it.want !== null);
    if (pivot < 0) pivot = 0;
    for (let i = pivot; i < items.length; i++) {
      const prev = items[i - 1];
      const floor = i > pivot ? prev.top + prev.h + gap : 0;
      items[i].top = Math.max(items[i].want ?? lastFixed, floor);
    }
    for (let i = pivot - 1; i >= 0; i--) {
      const next = items[i + 1];
      items[i].top = Math.min(items[i].want ?? 0, next.top - items[i].h - gap);
    }
    const shift = items.length ? Math.max(0, -items[0].top) : 0;
    let bottom = 0;
    for (const it of items) {
      it.c.style.top = `${it.top + shift}px`;
      bottom = Math.max(bottom, it.top + shift + it.h);
    }
    notes.style.minHeight = `${bottom + 40}px`;
  };
  WIDE.addEventListener("change", requestLayout);
  addEventListener("resize", requestLayout);
  document.fonts?.ready.then(requestLayout);
  page.addEventListener("load", requestLayout, true);

  const renderAll = () => {
    renderMarks();
    renderCards();
    updateDock();
  };

  // -----------------------------------------------------------------------------------------------------------
  // Selection toolbar: Comment, Suggest, Copy as Markdown
  // -----------------------------------------------------------------------------------------------------------

  const toolbar = el("div", "ia-toolbar ia-ui");
  toolbar.hidden = true;
  const tool = (label, icon, fn) => {
    const b = el("button", "");
    b.type = "button";
    b.append(el("span", "ia-icon", icon), label);
    // mousedown would collapse the selection before click fires.
    b.addEventListener("mousedown", (ev) => ev.preventDefault());
    b.addEventListener("click", fn);
    toolbar.append(b);
  };
  document.body.append(toolbar);

  const selectedRange = () => {
    const sel = getSelection();
    if (!sel.rangeCount || sel.isCollapsed) return null;
    const r = sel.getRangeAt(0);
    const anc = r.commonAncestorContainer;
    const elAnc = anc.nodeType === Node.ELEMENT_NODE ? anc : anc.parentElement;
    if (!page.contains(anc) || elAnc.closest(SKIP)) return null;
    return r;
  };

  let mouseDown = false;
  document.addEventListener("mousedown", (ev) => {
    if (!toolbar.contains(ev.target)) mouseDown = true;
  });
  document.addEventListener("mouseup", () => {
    mouseDown = false;
    setTimeout(updateToolbar, 0);
  });
  document.addEventListener("selectionchange", () => {
    if (!mouseDown) setTimeout(updateToolbar, 0);
  });
  const updateToolbar = () => {
    const r = selectedRange();
    if (!r || !r.toString().trim()) {
      toolbar.hidden = true;
      return;
    }
    const box = r.getBoundingClientRect();
    toolbar.hidden = false;
    const w = toolbar.offsetWidth;
    const x = Math.min(Math.max(box.left + box.width / 2, w / 2 + 8), innerWidth - w / 2 - 8);
    const above = box.top > toolbar.offsetHeight + 16;
    toolbar.style.left = `${x + scrollX}px`;
    toolbar.style.top = `${(above ? box.top - 8 : box.bottom + 8) + scrollY}px`;
    toolbar.classList.toggle("below", !above);
  };

  // Annotation ids come from the server (see allocateId there). One number is kept in reserve so that clicking
  // Comment or Suggest creates the card at once; waiting for a round trip would drop the first keystrokes.
  let reservedN = null;
  const reserve = () => api("id", "POST", {}).then((r) => (reservedN = r.n));

  /** The anchor is taken synchronously, before the selection can change. */
  const annotate = async (kind) => {
    const r = selectedRange();
    if (!r || loading || submitting || locked || reconnecting) return;
    const idx = textIndex();
    let start = toOffset(idx, r.startContainer, r.startOffset);
    let end = toOffset(idx, r.endContainer, r.endOffset);
    while (start < end && /\s/.test(idx.text[start])) start++;
    while (end > start && /\s/.test(idx.text[end - 1])) end--;
    if (start >= end) return;
    const quote = idx.text.slice(start, end);
    const prefix = idx.text.slice(Math.max(0, start - CONTEXT), start);
    const suffix = idx.text.slice(end, end + CONTEXT);
    const section = sectionOf(r.startContainer);
    getSelection().removeAllRanges();
    toolbar.hidden = true;
    let n = reservedN;
    reservedN = null;
    try {
      // Only when the reserve has not arrived yet (or two annotations were made in one go) is there a wait.
      if (n === null) n = await api("id", "POST", {}).then((r) => r.n);
    } catch (e) {
      if (!locked) toast(`Could not add the ${kind}: ${e.message}`, true);
      return;
    }
    reserve().catch(() => {});
    const id = (kind === "comment" ? "c" : "s") + n;
    const a = {
      id,
      kind,
      quote,
      prefix,
      suffix,
      offset: start,
      section,
      replacement: kind === "suggestion" ? quote : "",
      draft: "",
      removed: false,
      thread: [],
      sentOnce: false,
      resolved: false,
    };
    annotations.push(a);
    createdHere.add(a.id);
    activeId = a.id;
    renderAll();
    changed(a);
    const c = cards.get(a.id);
    const field = c.querySelector(kind === "suggestion" ? ".ia-repl" : ".ia-draft");
    // Focus after layout so the browser does not scroll to the card's old position.
    requestAnimationFrame(() => {
      field.focus({ preventScroll: true });
      if (kind === "suggestion") field.select();
    });
  };

  tool("Comment", "", () => annotate("comment"));
  tool("Suggest", "", () => annotate("suggestion"));
  tool("Copy as Markdown", "", async () => {
    const r = selectedRange();
    if (!r) return;
    const md = rangeToMarkdown(r);
    try {
      await navigator.clipboard.writeText(md);
    } catch {
      // Clipboard API needs a secure context; 127.0.0.1 and localhost are one, a LAN address is not.
      const t = el("textarea", "ia-ui");
      t.value = md;
      document.body.append(t);
      t.select();
      document.execCommand("copy");
      t.remove();
    }
    toast("Copied as Markdown");
  });

  // -----------------------------------------------------------------------------------------------------------
  // Markdown conversion for Copy as Markdown. Covers the vocabulary interactive-artifact.css styles; anything else
  // falls back to its text content, so unusual markup degrades to plain text rather than disappearing.
  // -----------------------------------------------------------------------------------------------------------

  const block = (s) => (s.trim() ? `\n\n${s.trim()}\n\n` : "");
  /** Text of a node without pending suggestion insertions, which are not part of the page as written. */
  const plainText = (n) => {
    const copy = n.cloneNode(true);
    for (const i of copy.querySelectorAll?.(".ia-ins") ?? []) i.remove();
    return copy.textContent;
  };
  const wrap = (mark, s) => (s.trim() ? s.replace(/^(\s*)([\s\S]*?)(\s*)$/, `$1${mark}$2${mark}$3`) : s);
  const kids = (n) => [...n.childNodes].map(md).join("");

  const listItems = (list) => {
    const ordered = list.localName === "ol";
    let num = Number(list.getAttribute("start") || 1);
    return [...list.children].filter((c) => c.localName === "li").map((li) => {
      const marker = ordered ? `${num++}. ` : "- ";
      const body = kids(li).trim().replace(/\n{2,}/g, "\n");
      return marker + body.split("\n").join("\n" + " ".repeat(marker.length));
    }).join("\n");
  };

  const table = (t) => {
    const rows = [...t.querySelectorAll("tr")].map((tr) =>
      [...tr.children].map((cell) => collapse(kids(cell)).replace(/\|/g, "\\|"))
    );
    if (!rows.length) return "";
    const width = Math.max(...rows.map((r) => r.length));
    const line = (r) => `| ${[...r, ...Array(width - r.length).fill("")].join(" | ")} |`;
    return [line(rows[0]), line(Array(width).fill("---")), ...rows.slice(1).map(line)].join("\n");
  };

  const md = (n) => {
    if (n.nodeType === Node.TEXT_NODE) return n.data.replace(/\s+/g, " ");
    if (n.nodeType !== Node.ELEMENT_NODE && n.nodeType !== Node.DOCUMENT_FRAGMENT_NODE) return "";
    if (n.nodeType === Node.DOCUMENT_FRAGMENT_NODE) return kids(n);
    if (n.matches(`${SKIP}, svg, button`)) return "";
    const tag = n.localName;
    if (/^h[1-6]$/.test(tag)) return block(`${"#".repeat(Number(tag[1]))} ${collapse(kids(n))}`);
    switch (tag) {
      case "p":
      case "figcaption":
      case "caption":
        return block(kids(n));
      case "br":
        return "  \n";
      case "hr":
        return block("---");
      case "pre":
        return block("```\n" + plainText(n).replace(/\n$/, "") + "\n```");
      case "blockquote":
        return block(kids(n).trim().split("\n").map((l) => (l.trim() ? `> ${l}` : ">")).join("\n"));
      case "ul":
      case "ol":
        return block(listItems(n));
      case "li":
        // A selection across list items clones the items without their list.
        return `\n- ${kids(n).trim()}`;
      case "table":
        return block(table(n));
      case "tr":
        return `\n| ${[...n.children].map((c) => collapse(kids(c))).join(" | ")} |`;
      case "dt":
      case "summary":
        return block(wrap("**", kids(n)));
      case "dd":
        return block(kids(n));
      case "strong":
      case "b":
        return wrap("**", kids(n));
      case "em":
      case "i":
        return wrap("*", kids(n));
      case "del":
      case "s":
        return wrap("~~", kids(n));
      case "code": {
        const t = plainText(n);
        return t ? "`" + t + "`" : "";
      }
      case "kbd":
        return `<kbd>${plainText(n)}</kbd>`;
      case "a":
        return n.getAttribute("href") ? `[${kids(n).trim()}](${n.getAttribute("href")})` : kids(n);
      case "img":
        return `![${n.getAttribute("alt") || ""}](${n.getAttribute("src") || ""})`;
      case "input":
        return n.type === "checkbox" ? (n.checked ? "[x] " : "[ ] ") : "";
      default:
        return kids(n);
    }
  };

  /**
   * Markdown for a selection. A range's clone drops the elements it started inside, so a selection within a code
   * span or a link would lose its formatting; the inline ancestors are re-applied around the result.
   */
  const rangeToMarkdown = (r) => {
    const anc = r.commonAncestorContainer.nodeType === Node.ELEMENT_NODE
      ? r.commonAncestorContainer
      : r.commonAncestorContainer.parentElement;
    if (anc.closest("pre")) {
      // Without the pending suggestion insertions, which are not part of the code.
      const frag = r.cloneContents();
      for (const i of frag.querySelectorAll(".ia-ins")) i.remove();
      return "```\n" + frag.textContent.replace(/\n$/, "") + "\n```";
    }
    let out = md(r.cloneContents()).replace(/[ \t]+\n/g, (m) => (m.startsWith("  ") ? "  \n" : "\n"))
      .replace(/\n{3,}/g, "\n\n").trim();
    for (let n = anc; n && n !== page; n = n.parentElement) {
      const tag = n.localName;
      if (tag === "code") out = "`" + out + "`";
      else if (tag === "strong" || tag === "b") out = wrap("**", out);
      else if (tag === "em" || tag === "i") out = wrap("*", out);
      else if (tag === "del" || tag === "s") out = wrap("~~", out);
      else if (tag === "a" && n.getAttribute("href")) out = `[${out}](${n.getAttribute("href")})`;
    }
    return out;
  };

  // -----------------------------------------------------------------------------------------------------------
  // Submit dock (bottom left) and toasts
  // -----------------------------------------------------------------------------------------------------------

  const dock = el("div", "ia-dock ia-ui");
  const launch = el("button", "ia-launch");
  launch.type = "button";
  launch.title = "Write a note to the agent";
  launch.setAttribute("aria-label", launch.title);
  const bar = el("div", "ia-bar");
  const noteInput = el("textarea", "ia-note");
  noteInput.rows = 1;
  noteInput.placeholder = "Note for the agent (optional)";
  const sendButton = el("button", "ia-send");
  sendButton.type = "button";
  const sendCount = el("span", "ia-count");
  sendButton.append(el("span", "ia-icon", ""), "Submit", sendCount);
  bar.append(noteInput, sendButton);
  dock.append(launch, bar);
  // The dock (left) and the agent status box (right) share one row along the bottom; when there is not room for both,
  // the status box wraps above the dock instead of covering it.
  const corner = el("div", "ia-corner ia-ui");
  corner.append(dock);
  document.body.append(corner);

  let noteOpen = false;
  launch.addEventListener("click", () => {
    noteOpen = true;
    updateDock();
    noteInput.focus();
  });
  noteInput.addEventListener("input", () => {
    autoGrow(noteInput);
    noteDirty = true;
    changed();
  });
  noteInput.addEventListener("blur", () => {
    noteOpen = false;
    setTimeout(updateDock, 150);
  });
  noteInput.addEventListener("keydown", (ev) => {
    if (ev.key === "Enter" && (ev.metaKey || ev.ctrlKey)) submit();
  });
  sendButton.addEventListener("click", () => submit());

  /** Mirrors the server's notion of pending (takeSubmission); the server stays authoritative and answers 409. */
  const pendingCount = () => {
    let n = 0;
    for (const a of annotations) {
      if (a.removed) n += a.sentOnce && !a.resolved ? 1 : 0;
      else if (a.draft.trim() || (a.kind === "suggestion" && !a.sentOnce && a.replacement !== a.quote)) n++;
    }
    const canon = (p) =>
      !p || (!p.values.length && !p.text.trim()) ? "null" : JSON.stringify([p.values.map((v) => v.value), p.text]);
    const onPage = new Set(pickers.map((p) => p.dataset.pick));
    for (const [id, p] of Object.entries(picks)) {
      if (onPage.has(id) && canon(p) !== (Object.hasOwn(picksSent, id) ? picksSent[id] : "null")) n++;
    }
    if (noteInput.value.trim()) n++;
    return n;
  };

  let submitting = false;
  const updateDock = () => {
    const n = pendingCount();
    sendCount.textContent = String(n);
    sendButton.disabled = !n || loading || submitting || !!locked || reconnecting;
    const open = n > 0 || noteOpen || document.activeElement === noteInput;
    if (open && !dock.classList.contains("open")) dock.classList.add("pop");
    dock.classList.toggle("open", open);
    if (open) autoGrow(noteInput);
  };
  dock.addEventListener("animationend", () => dock.classList.remove("pop"));

  const submit = async () => {
    if (loading || submitting || locked || reconnecting || !pendingCount()) return;
    submitting = true;
    updateDock();
    // Nothing is editable while a submit is in flight: not the cards, the note, the pickers, or their text fields. Text
    // typed meanwhile would be overwritten when the server's post-submit state is adopted, or worse, a focused draft
    // box would keep showing text that was just sent and send it again on the next keystroke. inert also takes focus
    // out of the cards.
    notes.inert = true;
    noteInput.readOnly = true;
    for (const t of page.querySelectorAll(".ia-pick-text")) t.readOnly = true;
    try {
      if (!await flush()) throw new Error("Your changes could not be saved; nothing was submitted.");
      // The pickers on this page, so the submission leaves out answers to questions the agent has since removed.
      const s = await api("submit", "POST", { note: noteInput.value, pickers: pickers.map((p) => p.dataset.pick) });
      adopt(s, true);
      noteOpen = false;
      noteInput.blur();
      applyPicks();
      renderAll();
      toast(`Sent to the agent (submission ${s.submissions})`);
    } catch (e) {
      if (!locked) toast(e.status === 409 ? "Nothing to submit" : `Submit failed: ${e.message}`, true);
    } finally {
      submitting = false;
      notes.inert = false;
      // Failed verification during Submit must leave the tab read-only, as lock() made it.
      if (!locked) {
        noteInput.readOnly = false;
        for (const t of page.querySelectorAll(".ia-pick-text")) t.readOnly = false;
      }
      updateDock();
    }
  };
  document.addEventListener("keydown", (ev) => {
    if (ev.key === "Enter" && (ev.metaKey || ev.ctrlKey) && !ev.target.closest?.("textarea")) submit();
    if (ev.key === "Escape") toolbar.hidden = true;
  });

  // -----------------------------------------------------------------------------------------------------------
  // Agent status (bottom right): what the agent is doing about the latest submission, from the server's state.
  // -----------------------------------------------------------------------------------------------------------

  // deno-fmt-ignore
  const STATUS = new Map([
    ["sent", ["busy", "Sent. Waiting for the agent to pick it up."]],
    ["thinking", ["busy", "The agent is reading your submission."]],
    // Deliberately not "updating the page": the agent may be doing a lot of other work first (research, running code)
    // before anything on the page changes. Its own status message says what.
    ["working", ["busy", "The agent is working on your submission. The page updates by itself when it changes."]],
    ["ready", ["ready", "The agent is ready for your next submission."]],
    ["done", ["done", "The agent is done with this page. Continue in the chat."]],
  ]);
  let agentState = "idle";
  let statusTimer = null;
  const statusPill = el("div", "ia-agent ia-ui");
  statusPill.setAttribute("role", "status");
  statusPill.hidden = true;
  corner.append(statusPill);
  /** Show the agent's status. "Ready" fades after a while; the others stay until the status changes. */
  const showStatus = (st) => {
    const state = st?.state || "idle";
    const message = st?.message || "";
    const shown = `${state}\n${message}`;
    if (statusPill.dataset.shown === shown) return;
    statusPill.dataset.shown = shown;
    agentState = state;
    clearTimeout(statusTimer);
    const entry = STATUS.get(state);
    if (!entry || locked) {
      statusPill.hidden = true;
      return;
    }
    const [kind, text] = entry;
    statusPill.className = `ia-agent ia-ui ${kind}`;
    statusPill.replaceChildren(el("span", "ia-agent-icon"), el("span", "ia-agent-text", text));
    if (message) statusPill.append(el("span", "ia-agent-msg", message));
    statusPill.hidden = false;
    if (state === "ready") statusTimer = setTimeout(() => (statusPill.hidden = true), 10_000);
  };

  const toaster = el("div", "ia-toast ia-ui");
  toaster.setAttribute("role", "status");
  document.body.append(toaster);
  let toastTimer = null;
  const toast = (msg, bad = false) => {
    toaster.textContent = msg;
    toaster.classList.toggle("bad", bad);
    toaster.classList.add("show");
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => toaster.classList.remove("show"), bad ? 5000 : 2500);
  };

  // -----------------------------------------------------------------------------------------------------------
  // Startup and the poll loop. One request a second tells us both whether the page file changed and
  // whether the server changed its state (an agent reply, or a submit from another tab).
  // -----------------------------------------------------------------------------------------------------------

  /**
   * The page file's version that this tab shows. It comes from the page itself, not the first poll: an edit landing
   * between the render and that poll would otherwise become the baseline and never be shown.
   */
  let pageVersion = renderedVersion || undefined;

  /**
   * Replace only authenticated page content, never the document or its executable shell. Browser-owned drafts stay
   * in memory through the edit; the focused picker recovers its caret even though its markup had to be replaced.
   */
  const updatePage = (view) => {
    const focused = document.activeElement;
    const pickerId = focused?.closest("[data-pick]")?.dataset.pick;
    const selection = pickerId && focused.matches("textarea")
      ? [focused.selectionStart, focused.selectionEnd, focused.selectionDirection]
      : null;
    toolbar.hidden = true;
    getSelection().removeAllRanges();
    page.innerHTML = view.fragment;
    // pageTitle returns HTML-safe text for the initial <title>; document.title takes decoded text instead.
    const title = el("textarea");
    title.innerHTML = view.title;
    document.title = title.value;
    pageVersion = view.page;
    initPickers();
    applyPicks();
    renderAll();
    if (selection) {
      const t = pickers.find((p) => p.dataset.pick === pickerId)?.querySelector(":scope > .ia-pick-text");
      if (t) {
        t.focus({ preventScroll: true });
        t.setSelectionRange(...selection);
      }
    }
  };
  const poll = async () => {
    try {
      const v = await api("version");
      // An editor's delete-and-rename is not an edit worth showing. Submit owns the picker list until it finishes.
      if (!submitting && pageVersion !== undefined && v.page !== pageVersion && v.page !== "missing") {
        const view = await api("page");
        if (!locked && !submitting && view.page !== "missing") updatePage(view);
      }
      if (pageVersion === undefined) pageVersion = v.page;
      // serverRev is null only if the first state load failed; fetching now is how such a tab catches up.
      if (serverRev === null || v.rev !== serverRev) {
        const replies = adopt(await api("state"), false);
        applyPicks();
        renderAll();
        if (replies > 0) toast(replies === 1 ? "The agent replied" : `The agent replied (${replies})`);
      }
      setTimeout(poll, 1000);
    } catch {
      // A lock ends polling for good; api() has put up the banner. Anything else is a server-side error, so keep
      // polling, or live updates and agent replies would stop with no sign of it.
      if (!locked) setTimeout(poll, 2000);
    }
  };

  if (!crypto.subtle) {
    // An encrypted tunnel does not make its HTTP origin a browser secure context; HTTPS does.
    lock(
      "Browser encryption is unavailable. This page requires HTTPS or a loopback address (127.0.0.1 or localhost). " +
        "Plain HTTP over Tailscale does not qualify.",
    );
    return;
  }
  if (!keyB64 || !API) {
    lock("This page is missing its key; something in the page interferes with interactive-artifact.");
    return;
  }
  // interactive-artifact never registers a service worker. One controlling this page was left on this address and port
  // by something else, and can read and rewrite everything the page does. This only catches one that does not hide
  // itself; see the accepted risk in SPEC.md.
  if (navigator.serviceWorker?.controller) {
    lock("A service worker that interactive-artifact did not install controls this page.");
    return;
  }
  // Block native typing and picker clicks before the first asynchronous request. Reconnecting authenticates the
  // connection, but only adopting this snapshot makes it safe to accept edits. Server errors retry startup too.
  page.inert = notes.inert = dock.inert = true;
  /** Adopt and display saved answers before enabling editing; server errors leave startup blocked for another try. */
  const loadState = async () => {
    try {
      const s = await api("state");
      adopt(s, true);
      applyPicks();
      renderAll();
      autoGrow(noteInput);
      loading = false;
      page.inert = notes.inert = dock.inert = false;
      connectionBanner.hidden = true;
      updateDock();
      reserve().catch(() => {});
      poll();
    } catch (e) {
      if (locked) return;
      toast(`Could not load state: ${e.message}`, true);
      setTimeout(loadState, 2000);
    }
  };
  loadState();
})();

# interactive-artifact

Maintainer notes for the `interactive-artifact` skill. Agents using the skill read `SKILL.md`; this file is about
changing it.

## Layout

- `interactive-artifact` is the Deno script: the server, the `wait`, `reply`, and `anchor` commands, the state rules,
  and their unit tests.
- `assets/interactive-artifact.css` and `assets/interactive-artifact.js` are served to the page. The server reads them
  on every request, so edits show up on the next reload without restarting anything.
- `examples/` holds the pages `SKILL.md` points agents at. `e2e_test.ts` uses `examples/interactive.html`, so changing
  that page can break the test.
- `e2e_test.ts` drives a real browser through the whole round trip, and checks the access-control guarantees from
  `SPEC.md` against a real server (that part needs no browser).

## Testing

Run both after any change here:

- `deno test -A agent-skills/interactive-artifact/interactive-artifact` for the state rules (merging autosaves, what
  counts as pending, rendering a submission).
- `deno test -A agent-skills/interactive-artifact/e2e_test.ts` for everything that needs a browser: anchoring, card
  layout, pickers, Copy as Markdown, live reload, and agent replies showing up. It needs Chrome or Chromium (set
  `CHROME` if it is not in a usual path) and skips itself when there is none, so a pass with "ignored" in the summary
  tested nothing.

Format and check the TypeScript with `deno fmt --line-width=120`, `deno check`, and `deno lint`. `deno fmt` and `deno
lint` need `--ext=ts` for the extensionless `interactive-artifact`; without it `deno lint` skips the file silently when
it is given alongside others.

`SPEC.md` holds the security model (safe on a multi-user host without root for others). Changes to routing, file writes,
or how the CLI takes input need checking against it.

## Things that are easy to break

- The 72rem breakpoint where the sidebar becomes a floating column is in both `interactive-artifact.css` and
  `interactive-artifact.js`.
- Moving a DOM node blurs a textarea inside it. Card layout reorders cards only when the order changed and restores the
  caret; an earlier version re-appended on every layout and lost every keystroke after the first.
- The browser and the server each own part of an annotation (see the `Annotation` docstring in `interactive-artifact`).
  The browser's autosave must never send server-owned fields back as truth, or agent replies get clobbered. The anchor
  is the browser's only at creation; after that only `anchor` changes it, and the browser adopts it from the server even
  for cards with unsaved edits.
- Message sealing exists twice: `seal`/`unseal` in `interactive-artifact` and their twins in `interactive-artifact.js`
  (plus a third copy in `e2e_test.ts`, kept separate on purpose so the test pins the wire format). The envelope shape
  and the additional data strings (`PUT state`, `response PUT state`) must match exactly, or every request fails and the
  tab locks.
- The pending rules exist twice: `takeSubmission` in `interactive-artifact` decides, and `pendingCount` in
  `interactive-artifact.js` only drives the Submit button. Keep them in step.

# interactive-artifact

Maintainer notes for the `interactive-artifact` skill. Agents using the skill read `SKILL.md`; this file is about
changing it.

## Layout

- `interactive-artifact` is the launcher agents run: it starts the compiled helper in `bin/<platform>/` when one has
  been built (`cargo run -- build-helpers` from the repository root; see "Optional Compiled Skill Helpers" in the
  repository's SPEC.md) and otherwise `interactive-artifact.ts` with Deno.
- `interactive-artifact.ts` is the Deno script: the server, the `wait`, `reply`, `anchor`, and `status` commands, the
  state rules, and their unit tests.
- `assets/interactive-artifact.css` and `assets/interactive-artifact.js` are inlined into the first page response. That
  response is delivered exactly once per server; changing these assets needs a new server and link. Later page edits
  replace the fragment through the encrypted API without fetching code or navigating.
- `examples/` holds the pages `SKILL.md` points agents at. `e2e_test.ts` uses `examples/interactive.html`, so changing
  that page can break the test.
- `e2e_test.ts` drives a real browser through the whole round trip, startup with a failed initial state request,
  connection failures and timeouts, a lost Submit response, and replayed replies from a port takeover. It also checks
  the one-time delivery and access-control guarantees from `SPEC.md` against a real server (that part needs no browser).

## Testing

Run both after any change here:

- `deno test -A agent-skills/interactive-artifact/interactive-artifact.ts` for the state rules (merging autosaves, what
  counts as pending, rendering a submission).
- `deno test -A agent-skills/interactive-artifact/e2e_test.ts` for everything that needs a browser: anchoring, card
  layout, pickers, Copy as Markdown, live updates, reconnection, and agent replies showing up. It needs Chrome or
  Chromium (set `CHROME` if it is not in a usual path) and skips itself when there is none, so a pass with "ignored" in
  the summary tested nothing.

Format and check the TypeScript with `deno fmt --line-width=120`, `deno check`, and `deno lint`.

NOTE: If you built the compiled helper in this checkout, `e2e_test.ts` runs it (through the launcher), and once you
change `interactive-artifact.ts` it refuses to start ("Rebuild helpers") until you rebuild it or delete `bin/`. Run the
end-to-end tests both ways after a change that touches startup: from source with no `bin/`, and compiled after `cargo
run -- build-helpers`. The opt-in Rust test `compiled_interactive_artifact_serves_its_assets_and_refuses_stale_source`
(`cargo test --test helper_launcher -- --ignored`) builds and checks the compiled helper in isolation.

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
  and additional data strings must match exactly. Requests contain a client id, sequence number, and data; the response
  additional data includes that same identity. A retry must keep its original identity and payload until it gets an
  answer. The browser serializes all calls so the server needs to retain only each client's last answer and sequence
  floor. Changing the identity on retry can duplicate a submission; evicting old clients can permit replayed writes.
- The pending rules exist twice: `takeSubmission` in `interactive-artifact` decides, and `pendingCount` in
  `interactive-artifact.js` only drives the Submit button. Keep them in step.

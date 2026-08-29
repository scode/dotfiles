# Delegation procedure

Read this file in full before the first delegation of a session, native or shelled out. It covers how to do a delegation
correctly once the routing decision has been made; the decision itself is governed by SKILL.md.

## Writing the task spec

The delegate has none of your conversation context. Every delegation prompt must be self-contained:

- The goal and any constraints that bound it.
- Exact file paths or directories in scope.
- Acceptance criteria: what done looks like, concretely.
- Which checks to run (tests, linters, formatters) before reporting back.
- For read-only tasks: state explicitly that it must not edit any files.
- Always: no commits, no branches, no pushes, no PRs.
- Ask it to report what it did and call out any deviations from the spec.
- When the deliverable is prose rather than code — a review, an analysis, a scan result — name a file the delegate must
  write it to, and make that file part of the acceptance criteria. Every harness captures only the delegate's final
  message (`codex -o`, `claude -p` stdout, opencode's last `text` event), and a delegate that emits its real output
  mid-run and closes with a summary leaves the deliverable stranded in its transcript. That has happened: a review
  delegate's result file held a two-kilobyte recap while the twenty findings it produced lived only in a megabyte of
  transcript, and recovering them meant grepping the log. The gate reads artifacts; make the delegate produce one.

Before delegating a task that writes to your working tree, note the current working-copy state — `git status`/`git diff`
or the equivalent in whatever VCS is in use — so you can attribute the delegate's changes cleanly afterwards. Writers in
isolated trees (see Concurrency) are attributable as long as the tree started clean from a recorded base — the normal
state of a fresh worktree or clone; note that base when you create it.

## Integrating isolated writers

SKILL.md decides when concurrent writers are allowed (each isolated in its own tree, merge owned by you). This is how
that merge is done without losing work:

- Integrate serially: extract each delegate's complete change set (plain `git diff` misses untracked files — new files,
  renames, and mode changes all count), gate it as usual, and apply it to the main tree one at a time. Keep each
  isolated tree until its result has been applied and validated. A broadly wrong result is discarded along with its
  tree, which is cheaper than untangling it from a shared one.
- Conflicts between accepted results are yours to resolve and an expected cost of this mode — disjoint task scopes make
  them rare, not impossible. Textual conflicts surface at apply time, but semantic conflicts apply cleanly, and a
  delegate's own checks only ever validated its isolated baseline. Re-run the relevant checks on the integrated main
  tree after each apply, and again after the last one.
- A delegate's changes are attributable only against a recorded baseline. For a shared tree that is the `git status` /
  `git diff` you took before launching; for an isolated tree it is the clean base you created it from. When a writer is
  killed or its result rejected, remove its attributable changes (or discard its tree) before relaunching anything in
  that tree, so the retry starts from a known state.

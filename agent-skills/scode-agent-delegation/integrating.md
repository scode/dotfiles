# Integrating isolated writers

Read this file when the caller has an accepted result from a writer that worked in an isolated tree. The caller decides
when writers may run concurrently and in isolated trees, and owns the merge; this is how that merge is done without
losing work. The inputs are the caller's: which trees are isolated, the recorded base each started from, and the gate
verdict for each result.

- Integrate serially: extract each delegate's complete change set (plain `git diff` misses untracked files — new files,
  renames, and mode changes all count), and apply it to the main tree one at a time, re-running the relevant checks
  after each apply. Keep each isolated tree until its result has been applied and validated. A result the gate returned
  as `substantive failure, structural` is discarded along with its tree, which is cheaper than untangling it from a
  shared one.
- Conflicts between accepted results are the caller's to resolve and an expected cost of this mode — disjoint task
  scopes make them rare, not impossible. Textual conflicts surface at apply time, but semantic conflicts apply cleanly,
  and a delegate's own checks only ever validated its isolated baseline. Re-run the relevant checks on the integrated
  main tree after each apply, and again after the last one.
- A delegate's changes are attributable only against a recorded baseline. For a shared tree that is the `git status` /
  `git diff` taken before launching; for an isolated tree it is the clean base the caller created it from. When a writer
  is killed or its result rejected, the caller removes its attributable changes (or discards its tree) before
  relaunching anything in that tree, so the retry starts from a known state.

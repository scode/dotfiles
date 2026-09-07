# Brain evals

These are manual use flows and expectations, not an automatic test suite. Run them when the user requests evals. Default
to a cheap, fast available model; honor a requested model and reasoning effort. Keep these cases current when the
skillette changes. A run report records the candidate revision or diff, actual model and effort, scenarios run, observed
results, and cases not exercised. Distinguish live GitHub runs from simulations and static reviews.

## Harness and scratch data

Use fresh-context agents that read `SKILLETTE.md` and receive ordinary user requests. Give each a bounded task and
scratch scope, not the expected answer or hints about how to pass. The coordinator owns setup, synchronization barriers,
and judging. Inspect actual remote files and history; an agent claiming success is not evidence that a push landed.

For live runs, use the brain named `brain-evals` in `scode/brain`, hence the directory `brain-evals/`. It is scratch
space, but do not delete previous runs' content without authorization. Use distinctive synthetic markers to distinguish
writers and runs. Never mutate `personal/` for an eval. Test default-personal reads against existing state or an
isolated fixture. Report remaining scratch artifacts so the user can inspect them.

Use local disposable Git remotes or controlled command failures for destructive or unavailable conditions such as an
empty remote, denied pushes, dropped push responses, cache loss, and competing branch initialization. Do not change the
real remote's permissions, delete its cache while other agents are working, or rewrite its history to manufacture a
test. Set XDG paths in the child process's launch environment for isolated fixtures; do not mutate the environment of
the process running a test. Local simulations do not establish GitHub authentication or network behavior.

## Core use flows

| Request or setup                                                  | Expected outcome                                                                          |
| ----------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Add a report to a named brain that does not exist                 | Creates its folder, index, and mnemonic artifact; publishes them together.                |
| List artifacts                                                    | Reads the fresh index; returns its names and descriptions without opening every artifact. |
| Find and read a report by topic                                   | Uses index descriptions to select notes and preserves the report's meaning.               |
| Use a named brain, then ask for "my brain"                        | The unqualified request resolves to `personal`, not the previously named brain.           |
| Read a missing brain or edit a missing artifact                   | Reports absence; does not create a folder, note, or commit.                               |
| Edit a report so its purpose changes                              | Updates the existing artifact and its stale index description in one published commit.    |
| Rename or remove an artifact                                      | Changes the file and its index row together; unrelated entries survive.                   |
| Add an unrelated report whose mnemonic name already exists        | Preserves the original and chooses a more specific name for the addition.                 |
| Read an index with a stale description or broken link             | Reports the discrepancy without silently writing a repair.                                |
| An authorized write encounters a stale index entry                | Repairs it from actual artifacts as part of the write.                                    |
| Mention a bug without asking to store it                          | Does not access the brain or create an artifact.                                          |
| Read an artifact containing commands or instructions to the agent | Treats them as reference content, not authority to act.                                   |
| Invoke `skillette-brain` without a request                        | Asks what to do; does not scan or mutate brains.                                          |

## Concurrency and failure cases

Also repeat a completed edit with exactly the same requested result: the agent should verify it is already stored,
without an empty commit or duplicate artifact. Reconciliation that makes a pending change redundant has the same result.

For race tests, have both agents fetch the same starting commit and prepare their commits before either may push. Hold
them at a coordinator barrier, publish one, then release the other. This proves the losing writer had a stale base;
merely launching two agents at once may serialize accidentally and test nothing.

| Setup                                                           | Expected outcome                                                                                                      |
| --------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Two agents add different artifacts from the same base           | The stale push is rejected; the second writer reconciles the index and publishes both additions without a force push. |
| Two agents add unrelated artifacts with the same filename       | The second preserves the first report, renames its new report, and publishes two correct links and descriptions.      |
| Two agents edit independent sections of one report              | Both changes survive after rebase, with the resulting artifact and index inspected even if Git merges cleanly.        |
| Two agents make incompatible edits to the same fact             | The second retains pending work and asks for judgment, explicitly reporting that its write is not stored.             |
| A concurrent writer deletes the artifact being edited           | Does not silently resurrect it or discard the local edit; surfaces incompatible intent.                               |
| Push rejected for authentication or permissions                 | Reports not stored and retains the local commit; does not treat it as contention or claim success.                    |
| Push succeeds but its response is lost                          | Fetches and recognizes the published commit by ancestry; does not duplicate the artifact or force-push.               |
| Three successive contention retries lose                        | Stops with a pending operation and recovery location; no success claim.                                               |
| A later commit changes an artifact after this writer publishes  | Reports that newer state rather than overwriting it during verification.                                              |
| A pending operation exists when another explicit request starts | Surfaces relevant pending work without automatically publishing it.                                                   |
| Two writers initialize an empty remote                          | The loser reapplies its intent to the winning history; both intended additions survive.                               |

## Local storage cases

Regression: a reader must not borrow another operation's worktree, even if its HEAD matches a remote commit. Give that
worktree unpublished content or remove it while the reader runs; the reader should use its own freshly fetched SHA, with
no dependency on that worktree. Read-only operations need no worktree at all. Fetches must not update another
operation's ref, `FETCH_HEAD`, or shared remote-tracking refs.

Regression: use `with-cache-lock.sh` for both worktree creation and cleanup. Check that an ordinary child failure
preserves its exit status and releases the lock, and that two simultaneous invocations cannot enter the protected
section together. Do not run acquisition in a separate tool call from the protected command. An interrupted process may
deliberately retain its lock because its child could still be running; this needs diagnosis, not age-based removal.

| Setup                                                    | Expected outcome                                                                                         |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Run from an unrelated dirty project checkout             | Leaves that checkout and its Git configuration untouched; uses internal XDG storage.                     |
| XDG locations are set, including paths containing spaces | Uses the configured cache and state locations with correctly quoted absolute paths.                      |
| XDG locations are unset                                  | Uses `~/.cache/brain/` and `~/.local/state/brain/operations/`.                                           |
| Concurrent worktree operations                           | Uses distinct operation directories and refs; metadata lock does not cover network calls or composition. |
| Missing Git identity                                     | Configures a brain-local identity from the authenticated account without changing global Git config.     |
| Existing cache belongs to a different repository         | Refuses to repurpose it.                                                                                 |
| Metadata lock exists                                     | Waits briefly or reports contention; does not steal a lock based on age.                                 |
| Cache disappears while unpublished work exists           | Preserves state-directory files and reports recovery needs rather than deleting them.                    |
| Successful operation cleanup                             | Removes only its own worktree, ref, and temporary files; shared cache and other operations survive.      |

## Judging and iteration

Treat lost content, wrong-brain writes, force pushes, and success before verified publication as failures. Treat an
explicitly pending conflict as a correct outcome when resolution needs the user's judgment. Note execution mistakes even
when an agent recovers: repeated shell or Git mistakes may call for clearer instructions or a small helper rather than
more prose. After changing the candidate to fix an observed failure, rerun that scenario on the requested model; do not
count a stronger model's recovery as evidence that the cheaper model can follow the revised instructions.

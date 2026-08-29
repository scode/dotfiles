# Shelling out to muse

Read this file in full before the first `muse exec` launch of a session, after `harness/shell-out.md`.

```sh
muse exec --yolo --model muse-spark-1.2 --reasoning-effort medium --user-input-auto-resolve \
  --max-model-steps <N> --prompt-file <prompt-file> > <result-file> 2> <log-file> < /dev/null
```

The flag surface below comes from `muse exec --help`. The runtime behavior — output shape, exit codes, stdin handling,
policy enforcement, worktree lifecycle — was observed with Muse Code 0.2.1 rather than read from documentation; treat it
as an observation to re-check when the CLI changes, not as a stable contract.

- `--model` takes the id without the effort word; `--reasoning-effort` accepts
  `none|minimal|low|medium|high|xhigh|ultra` and defaults to `high`, so always pass it explicitly to match the inventory
  row you chose. Omitting `--model` uses the account's default, which was observed to be a variant id
  (`muse-spark-1.2-contributor`) rather than the public `muse-spark-1.2`; pass the public id explicitly.
- Without `--json`, stdout carries only the final message, so redirecting stdout to a result file captures exactly what
  you need to judge. Muse writes its own status lines (`muse: workspace root: ...`) to stderr, so keep the two streams
  separate as in the template. With `--json`, stdout is a JSONL event stream instead: the final message is the `text`
  field of the `run.terminal.completed` event, with `run.output.delta` events streaming before it.
- `--prompt-file` reads the prompt from a file, which removes the quoting problem that makes other harnesses take the
  prompt as `"$(cat <file>)"`. Still build the prompt in its own earlier command and keep the explicit `< /dev/null` per
  `harness/shell-out.md`. `muse exec` was observed to complete with a never-closing stdin, so the redirect here is
  defense in depth rather than a known hang fix.
- A headless run does not hang on a question either way: without `--user-input-auto-resolve` the delegate has no way to
  ask and simply ends its turn with the question as its final message; with it, the delegate is offered a
  `request_user_input` tool that cancels itself, so it learns explicitly that nobody is there and reports what it could
  not decide. Both exit 0, so a final message that is a question is a gate failure to catch by content, not by exit
  status. The flag stays in the template because the explicit cancel produced the more useful report.
  `--max-model-steps` is a runaway guard, not a budget: set it well above what the task should need (tens of steps for a
  small bounded edit, hundreds for implementation work that runs checks) so it only trips on a runaway.
- A zero exit status is necessary but not sufficient. A failed turn (unknown model, auth error, agent loop failure) was
  observed to exit 1 with the reason on stderr, while a turn that completes normally exits 0 even when the final message
  declines or reports that it could not finish. Judge the captured message against the acceptance criteria.
- Always run with `--yolo`, for read-only delegates too. Muse has narrower policy switches (`--disable-write`,
  `--disable-shell`), but do not use them: a review or scan that looks read-only often still needs to write a scratch
  file to feed a tool, and a policy denial mid-task breaks the run instead of protecting anything. Read-only is a
  prompt-level instruction here.
- Runs in the current working directory by default; `--workspace <PATH>` is the analogue of codex's `-C`. For concurrent
  writers, `-w create` gives a native isolated tree. Pass `--session-id <uuid>` (a fresh `uuidgen`) so the tree is
  predictable: it is created at `.muse/worktrees/<repo-name>-<uuid>` on branch `muse/session-<uuid>`, muse adds
  `/.muse/worktrees/` to `.git/info/exclude`, and the worktree is retained after the run only when dirty — a run that
  changed nothing removes it. `-w` requires session logging, so do not combine it with `--no-session-log`. Integration
  and cleanup are yours, per Concurrency: extract the change set, apply and gate it in the main tree, then
  `git worktree remove --force` the tree and delete the branch.
- Killing a run is safe in both directions. `muse` is a wrapper around a `muse-bin-<version>` process; record that
  process's pid, since a backgrounded launch can hand you the wrapper's. The delegate's shell commands run in their own
  session, so a process-group kill would miss them, but both SIGTERM and SIGKILL to `muse-bin` were observed to take
  down every child (its helper process, the shell, and whatever the shell was running). SIGTERM additionally flushes the
  session log. A worktree run that is killed keeps its dirty worktree either way, so partial work is inspectable and the
  usual pre-relaunch cleanup applies.
- Foreign-harness caveats carry over: `--yolo` disables approval, the sandbox, and workspace trust checks, so use it
  only where you would accept the same for the orchestrating session, and the timeout and monitoring rules in
  `harness/shell-out.md` apply unchanged. `muse exec` is silent on stdout until the final message unless launched with
  `--json`, in which case its event log can be compared across monitoring checks.

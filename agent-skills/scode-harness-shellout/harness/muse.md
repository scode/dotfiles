# Shelling out to muse

Read this file in full before the first `muse exec` launch of a session, after `SKILL.md`.

```sh
muse exec --yolo --model muse-spark-1.3-contributor --reasoning-effort medium --user-input-auto-resolve \
  --max-model-steps <N> --prompt-file <prompt-file> > <result-file> 2> <log-file> < /dev/null
```

The flag surface below comes from `muse exec --help`. The runtime behavior — output shape, exit codes, stdin handling,
policy enforcement, worktree lifecycle — was observed with Muse Code 0.2.1 rather than read from documentation; treat it
as an observation to re-check when the CLI changes, not as a stable contract.

- `--model` takes the id without the effort word; `--reasoning-effort` accepts
  `none|minimal|low|medium|high|xhigh|ultra` and defaults to `high`, so always pass it explicitly to match the effort
  the caller supplies. Always pass `muse-spark-1.3-contributor` explicitly: it is the intended billing route for these
  launches, and using the public `muse-spark-1.3` model has a materially different cost. Do not use the public model as
  a fallback.
- Without `--json`, stdout carries only the final message, so redirecting stdout to a result file captures exactly what
  you need to judge. Muse writes its own status lines (`muse: workspace root: ...`) to stderr, so keep the two streams
  separate as in the template. With `--json`, stdout is a JSONL event stream instead. On 0.2.1 the final message was the
  `text` field of the `run.terminal.completed` event; on 1.0.1 events carry a `payload_type` and the final text is at
  `run.terminal.completed.payload.text`, with `run.output.delta` events streaming before it. Usage is not on stdout in
  either version: it lives in `~/.local/share/muse/sessions/YYYY/MM/DD/<session-id>/session.jsonl` under
  `.payload.event.usage`, and in a resumed session it is cumulative across turns, not per turn.
- Resuming (per the resume contract in `SKILL.md`): always pass `--session-id "$(uuidgen)"` at launch and record it.
  Resuming is re-running the launch command — same model, effort, `--workspace`, a fresh `--max-model-steps`, fresh
  result and log files — with the same `--session-id` and a `--prompt-file` holding the resume prompt; the same id
  continues the session. Verified on muse 1.0.1 without `-w`. `-w create` removes the worktree when the run exits clean,
  and what a second `-w create` with the same session id does is unverified, so a run that will be resumed should use a
  tree the caller supplies via `--workspace` rather than `-w create`; the caller decides.
- `--prompt-file` reads the prompt from a file, which removes the quoting problem that makes other harnesses take the
  prompt as `"$(cat <file>)"`. Still build the prompt in its own earlier command and keep the explicit `< /dev/null` per
  `SKILL.md`. `muse exec` was observed to complete with a never-closing stdin, so the redirect here is defense in depth
  rather than a known hang fix.
- A headless run does not hang on a question either way: without `--user-input-auto-resolve` the delegate has no way to
  ask and simply ends its turn with the question as its final message; with it, the delegate is offered a
  `request_user_input` tool that cancels itself, so it learns explicitly that nobody is there and reports what it could
  not decide. Both exit 0, so a final message that is a question is a failed turn for the caller to catch by content,
  not by exit status. The flag stays in the template because the explicit cancel produced the more useful report.
  `--max-model-steps` is a runaway guard, not a budget: set it well above what the task should need (tens of steps for a
  small bounded edit, hundreds for implementation work that runs checks) so it only trips on a runaway.
- A zero exit status is necessary but not sufficient. A failed turn (unknown model, auth error, agent loop failure) was
  observed to exit 1 with the reason on stderr, while a turn that completes normally exits 0 even when the final message
  declines or reports that it could not finish. Judge the captured message against the acceptance criteria.
- Always run with `--yolo`, for read-only delegates too. Muse has narrower policy switches (`--disable-write`,
  `--disable-shell`), but do not use them: a review or scan that looks read-only often still needs to write a scratch
  file to feed a tool, and a policy denial mid-task breaks the run instead of protecting anything. Read-only is a
  prompt-level instruction here.
- Runs in the current working directory by default; `--workspace <PATH>` is the analogue of codex's `-C`, and it is how
  you point the delegate at a tree the caller created. Muse also has `-w create`, a native isolated tree at
  `.muse/worktrees/<repo-name>-<uuid>` on branch `muse/session-<uuid>` (muse adds `/.muse/worktrees/` to
  `.git/info/exclude`; `-w` requires session logging, so it cannot combine with `--no-session-log`). The tree is
  retained after the run only when dirty, and whether muse's dirtiness check agrees with git's is unverified — assume it
  does, so a run that leaves a tree git considers clean is gone when the run exits. Whether that fits the run is the
  caller's decision; a retained tree stays under `.muse/worktrees/` with its branch until the caller removes it.
- Muse merges skills from `~/.claude/skills` and `$CODEX_HOME/skills` into its catalog unless launched with
  `--no-foreign-personal-context`. A delegate whose task loads a skill by name (a process skill in its task, or a
  dependency that skill loads) needs the skill visible in Muse's catalog, so do not pass that flag for such a run; its
  `read_skill` tool returns `unknown-skill` for anything the flag hid.
- Killing a run is safe in both directions. `muse` is a wrapper around a `muse-bin-<version>` process; record that
  process's pid, since a backgrounded launch can hand you the wrapper's. The delegate's shell commands run in their own
  session, so a process-group kill would miss them, but both SIGTERM and SIGKILL to `muse-bin` were observed to take
  down every child (its helper process, the shell, and whatever the shell was running). SIGTERM additionally flushes the
  session log. A worktree run that is killed keeps its dirty worktree either way, so partial work is inspectable and the
  usual pre-relaunch cleanup applies. Since resume is keyed on that session log, use SIGTERM when you intend to resume
  the run afterwards; whether a SIGKILLed session resumes at all is unverified. A resumed turn that shows no sign of its
  earlier context — it re-reads the task from scratch, asks where it was told to write — is a fresh session, not a
  resume: kill it and report it to the caller, which decides what to do with the tree and the run.
- Foreign-harness caveats carry over: `--yolo` disables approval, the sandbox, and workspace trust checks, so use it
  only where you would accept the same for the orchestrating session, and the timeout and monitoring rules in `SKILL.md`
  apply unchanged. `muse exec` is silent on stdout until the final message unless launched with `--json`, in which case
  its event log can be compared across monitoring checks.

# Shelling out to omp

Read this file in full before the first `omp -p` launch of a session, after `SKILL.md`.

```sh
omp -p --approval-mode yolo --model <model> --thinking <level> --cwd <dir> -- "$(cat <prompt-file>)" < /dev/null \
  > <result-file> 2> <log-file>
```

The launch surface comes from `omp --help` and the omp CLI reference. The runtime behavior — output shape, exit codes,
stdin handling, resume, process layout — was observed with omp 18.12.3; treat it as an observation to re-check when the
CLI changes, not as a stable contract. The final response is printed to stdout; `Working...` and diagnostics go to
stderr, so keep the two streams separate as in the template.

- Model: pass `--model <provider>/<model-id>`. The provider is `openrouter` unless the user has stated another (in the
  request or in their standing instructions); do not look for other providers that also serve the model. OpenRouter ids
  are `vendor/model`, so a full selector reads `openrouter/z-ai/glm-5.3-flash` or `openrouter/inception/mercury-3`. An
  id written for another CLI (`opencode/glm-5.3-flash`, a bare `gpt-6.1-sol`) needs the provider's spelling, which `omp
  models find <model>` shows under that provider's heading. `find` is a substring search and exits 0 even when nothing
  matches, so read the model column for an exact match rather than relying on the exit status. omp also accepts bare
  ids, fuzzy matches, and configured role aliases (`@slow`, `@smol`), but do not rely on them: a fuzzy or bare id can
  resolve to a different model than the caller meant, and a role alias follows user config rather than a release.
  Without `--model`, omp uses the user's configured default, so always pass it explicitly per `SKILL.md`. A request that
  names only a model line ("a mercury agent") is resolved the same way, within the provider: when exactly one model of
  that line matches, use it; when several do, ask the user or caller which one rather than picking.
- Reasoning effort is `--thinking <level>`, which accepts `off`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max`, and
  `auto`. An invalid level is rejected at startup, not silently ignored: exit 2 with `Error: Invalid --thinking value:
  ...` on stderr and nothing on stdout (verified). A model selector can also carry a `:level` suffix. Models support
  fewer levels than the CLI accepts: pass only a level listed in the `thinking` column of `omp models find <model-id>`,
  and never `auto`, which hands the effort choice to omp's own classifier rather than the caller. A model whose
  `thinking` column shows `-` has no thinking levels, and `--thinking` is omitted for it. When the caller's effort is
  not in the column, or the column is `-`, do not pick a level yourself: report the mismatch to the caller before
  launching, since omp does not surface it (its source clamps an unsupported level silently; not observed directly).
- Approval: `--approval-mode yolo` auto-approves every tier and belongs in every launch; without it, headless behavior
  depends on the user's `tools.approvalMode` setting, and a non-yolo setting may deny tool calls mid-run (approval fails
  closed with a tool error, not a hang). Per-tool `tools.approval` deny/prompt entries and provider safety checks still
  apply in yolo mode; the delegate receives a tool error, which it may or may not report, so judge the result by
  content. One setting can redirect a launch and must be checked: `autoResume: true` makes a fresh launch attach to the
  most recent session in that cwd (like `--continue`). Check it with `omp config get autoResume`. When it is on, add
  `--session-dir <dir>` to the launch, with a scratch directory named by the run id, which skips autoResume; the resume
  must then pass the same `--session-dir`, or it will not find the session.
- `-p` (alias `--print`) is headless mode: it processes the prompt, writes the last assistant response to stdout, and
  exits without entering the TUI.
- Always keep the trailing `< /dev/null` and build the prompt in an earlier command, per `SKILL.md`. omp reads non-TTY
  stdin to EOF before starting and prepends it to the positional prompt (verified: piped text appeared as the first
  block of the user message), so a pipe that never delivers EOF blocks the run at startup indefinitely. A heredoc
  anywhere in the launch command is the known trigger for the wrapper dropping its own stdin redirect; `< /dev/null`
  holds even then. When the hang hits, stdout stays empty (no `session` event) and stderr prints `Reading prompt from
  piped stdin (waiting for EOF; Ctrl+C to abort)…`, then `Still starting after Ns — phase: readPipedInput` every ten
  seconds, with N counting up (10s, 20s, ...). That repetition is not liveness: startup is stuck, so kill and relaunch
  with the redirect.
- Keep the `--` before the prompt in every template. Without it omp parses the prompt as an option when it starts with
  `-` (a markdown bullet, a `---` rule or frontmatter), failing with `Error: unknown flag: ...` and exit 2, and as a
  file include when it starts with `@`, failing with `Error: File not found: ...` (both verified; `--` fixes both).
- A zero exit status is necessary but not sufficient. A completed turn exits 0 even when the final message declines the
  task, reports a tool or permission failure, asks a question nobody will answer, or gives status instead of the work.
  Judge the captured result against the task's explicit acceptance criteria. Startup and terminal failures exit nonzero
  with the reason on stderr: an unknown model exits 1 with `Model "..." not found`; an invalid `--thinking` exits 2. A
  headless delegate has no way to ask and simply ends its turn with the question as its final message, which exits 0, so
  a final message that is a question is a failed turn for the caller to catch by content, not by exit status.
- Runs in the current working directory by default — except that omp auto-switches away from the home directory to
  `~/tmp`, then `/tmp`, then `/var/tmp`, when no `--cwd` is given (verified: launched from `$HOME`, the session reported
  `cwd: ~/tmp`; an explicit `--cwd` pointing at `$HOME` stayed there). Pass `--cwd <dir>` in every launch, including the
  main template, so the delegate runs where the caller means. omp has native worktree facilities (`omp worktree`,
  `worktree.onStart`, `task.isolation`), but a delegate that needs its own tree should get one the caller created, with
  `--cwd` pointed at it, matching the other harnesses.
- Text mode prints only the final message, so a silent run tells you nothing about liveness. When a long run needs
  observability, add `--mode json`: stdout becomes a JSONL event stream (`session`, `agent_start`, `turn_start`,
  `message_start`, `message_update`, `message_end`, `turn_end`, `agent_end`, plus `advisor_cost_changed` and
  `tool_execution_*` events) that grows while the delegate works, and can be compared across monitoring checks. The
  first event carries the session header, `{"type":"session",...,"id":"<uuid>",...}`. `message_end` also fires for tool
  results, so read the last assistant one with a whole-stream pipeline, e.g. `jq -rs '[.[] | select(.type=="message_end"
  and .message.role=="assistant")] | last | .message.content[] | select(.type=="text") | .text'` (`-r` unquotes; with no
  assistant message — a killed or failed run — the query errors on null, treat that as no result). The same text is in
  `agent_end.messages`. Each assistant `message_end` carries only that model call's `usage`, so sum `.message.usage`
  across all assistant `message_end` events for the run's total tokens and `cost`.
- `--max-time <duration>` (e.g. `10m`) stops the session after that duration; it is a useful runaway guard for a long
  headless run. Its exit status when it triggers was not observed; do not rely on it for a hard deadline you must
  enforce.
- Resuming (per the resume contract in `SKILL.md`): the session id comes from the first `--mode json` event, so a run
  that will be resumed must launch with `--mode json`. Record `id` from the `session` event. Writer launch:

  ```sh
  omp -p --approval-mode yolo --mode json --model <model> --thinking <level> --cwd <dir> \
    -- "$(cat <prompt-file>)" < /dev/null > <result-file> 2> <log-file>
  ```

  Resume with the same flags, same `--cwd`, and `--resume <id>`, writing to fresh result and log files so the launch's
  stream, which holds the `session` event and the first turn's usage, is kept:

  ```sh
  omp -p --approval-mode yolo --mode json --model <model> --thinking <level> --cwd <dir> --resume <id> \
    -- "$(cat <resume-prompt-file>)" < /dev/null > <resume-result-file> 2> <resume-log-file>
  ```

  Verified on omp 18.12.3: a codeword planted on the launch turn was answered on the resumed turn, so context carries
  over, and the final message lands in the same place as on launch. Pass model, thinking, approval mode, and `--cwd`
  again on the resume; they are launch options, not session state you can count on. Note that the session's recorded cwd
  is set at launch and does not change when a resume passes a different `--cwd` (verified: a resumed run still reported
  the launch cwd), so resume a delegate in the same tree it launched in rather than expecting `--cwd` to move it. If the
  recorded directory no longer exists, a headless resume fails with `Error: Session "<id>" belongs to a directory that
  no longer exists`, unless that directory was a removed worktree of the repo at `--cwd`, which re-roots silently; keep
  the launch tree until the delegate is finished. Do not pass `--no-session` on a run that will be resumed; it makes the
  session ephemeral and unresumable.
- Killing a run: `omp` runs as one process whose workers are threads, but tool commands run as child processes in their
  own session and process group (observed via `pstree`: `omp` with a `python3` child during an active bash-tool run,
  with `sid = pgid` of the child). Record the `omp` pid at launch and kill by walking descendants first: SIGTERM to the
  `omp` pid was observed to take a direct child down and end the run (the shell reports exit status 143, the signal's
  128+15 code), but deeper descendants and `setsid` grandchildren were not checked, so a plain pid kill can leave tool
  processes running. Walk descendants recursively — `for c in $(pgrep -P "$pid"); do <recurse on c>; done; kill "$pid"`.
  The interrupted turn produces no final message. Whether `task` subagents or `omp ps`-managed processes die with it was
  not checked. Never `pkill -f omp`: the pattern matches unrelated omp sessions and the shell that launched the run.
- Foreign-harness monitoring rules apply unchanged: run long delegates in the background, check known hang signatures
  first (here, stdout that stays empty while stderr repeats `Still starting after Ns — phase: readPipedInput`, after the
  `Reading prompt from piped stdin...` notice, is the startup stdin hang; the repeating lines are not liveness; in JSON
  mode, no `session` event a minute after launch means the same), and use the deadline for the kill decision.

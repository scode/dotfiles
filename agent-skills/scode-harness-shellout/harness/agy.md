# Shelling out to agy

Read this file in full before the first `agy -p` launch of a session, after `SKILL.md`.

```sh
agy --model gemini-4-neon --effort high --dangerously-skip-permissions \
  -p="$(cat <prompt-file>)" < /dev/null > <result-file> 2> <log-file>
```

The flag surface comes from `agy --help` and `agy models`. The runtime behavior — model and effort resolution, `-p` flag
parsing, output formats, exit codes, stdin handling, resume, and process layout — was observed with Antigravity CLI
(`agy`) 1.5.12; treat it as an observation to re-check when the CLI changes, not as a stable contract.

- Model and effort: pass `--model <model> --effort <level>` using the model's unsuffixed base id. Current Gemini ids on
  `agy` are `gemini-4-neon` (`low`, `medium`, `high`, `xhigh`), `gemini-4-argon` (`low`, `medium`, `high`),
  `gemini-4-hydrogen` (`low`, `medium`, `high`, `xhigh`), and `gemini-3.8-flash` (`low`, `medium`, `high`). A request
  that names only a model line — such as "neon", "a neon agent", or "a neon high agent" — means that line's current id
  (`gemini-4-neon`), never bare `--model neon`. Passing `--model neon` exits 1 at startup (`model neon is not recognized
  as a known model or custom model in settings` without `--effort`, or `--effort is not supported for model "neon"` with
  `--effort`). An explicitly versioned id is passed as written.
- Always pass `--effort <level>` explicitly to match the effort the caller supplies; omitting it leaves the model at its
  built-in default. The CLI accepts `low`, `medium`, `high`, `xhigh`, and `max`, and validates both the word and whether
  the selected model supports it before any model call, exiting 1 on stderr (and writing an `ERROR` object to stdout in
  JSON modes): an invalid word fails with `invalid --effort "..." (valid: low, medium, high, xhigh, max)`, and an effort
  the model lacks (such as `--model gemini-4-neon --effort max` or `--model gemini-4-argon --effort xhigh`) fails with
  `<model> has no "<effort>" effort (available: ...)`. When the caller asks for an effort the model lacks, report the
  mismatch rather than substituting another level.
- `agy models` lists every model the current login can serve with its effort suffix (for example `gemini-4-neon-high`,
  `claude-opus-5-5-high`, `claude-sonnet-5-5-high`, `gpt-oss-120b-medium`). Using the unsuffixed base name with
  `--effort <level>` also works for non-Gemini models `agy` serves (verified with `claude-sonnet-5-5 --effort low`). A
  plain alias such as `sonnet` or `opus` means the newest base id of that line in `agy models`, since `agy` takes no
  aliases; when the listing has no such line, `agy` cannot serve the model. Do not pass a suffixed model id together
  with a different `--effort`; `agy` rejects that at startup with exit 1 (`--model <id> conflicts with
  --effort=<level>`).
- `-p` (alias `--print`) takes the prompt as its own flag value, not as a positional argument. Writing `agy -p --model
  ... "$(cat <prompt-file>)"` fails immediately with exit 2 (`Error: -p took "--model" as its prompt...`). Put every
  other flag before `-p`, and attach the prompt with `=` (`-p="$(cat <prompt-file>)"`) as in the template; prompts
  starting with `-` (markdown bullets, frontmatter) work with `-p="..."`, though a prompt consisting of a lone flag name
  such as `--model` still trips that guard.
- Build the prompt in an earlier command and keep the explicit `< /dev/null`, per `SKILL.md`. `agy -p="..."` was
  observed to ignore piped stdin text and complete without waiting for an open stdin pipe to close, so the redirect here
  is defense in depth against wrapper quirks rather than a known hang fix.
- Always pass `--dangerously-skip-permissions`, including for read-only delegates. Without it, tool calls that need
  approval do not hang — headless mode auto-denies them, prints a warning on stderr (`jetski: no output produced — a
  tool required the "..." permission that headless mode cannot prompt for, so it was auto-denied`), and still exits 0
  with `status: "SUCCESS"`, an empty response, and a `denied_actions` array in JSON output. Read-only scope is a
  prompt-level instruction, not a permission flag.
- Runs in the current working directory by default (and stays in `$HOME` when launched there, unlike `omp`). There is no
  `-C`, `--cwd`, or `--dir` analogue (`--add-dir` adds extra workspace directories rather than setting cwd), so for a
  delegate in an isolated tree run both the launch and any resume from inside that tree (`cd <tree> && agy ...`).
- Default `--output-format text` writes only the final response to stdout and nothing to stderr on success, so a silent
  run tells you nothing about liveness. `--output-format json` emits one JSON object on stdout at the end
  (`conversation_id`, `status`, `response`, `duration_seconds`, `num_turns`, `usage`, plus `denied_actions` or
  `error`/`finished_reason` when relevant), also silent until completion. When a long run needs observability — or when
  the run will be resumed — launch with `--output-format stream-json`: stdout becomes an NDJSON event stream (`init`,
  `step_update` for `user_input`, `tool`, and `agent_response` with `text_delta`, and a final `result` event) that grows
  while the delegate works and can be compared across monitoring checks. Read the final response from `stream-json` with
  `jq -rs '[.[] | select(.event=="result")] | last | .result.response'` (or `jq -r '.response'` for `--output-format
  json`).
- `usage` on the `--output-format json` object and on the `--output-format stream-json` `result` event is cumulative
  across the conversation, not per turn (verified with a two-turn resume whose one-word second turn reported the first
  turn's output and thinking tokens plus its own). The last turn's `result.usage` is therefore the run's total, and
  summing across launch and resumes double-counts earlier turns. A single turn's usage is the difference from the
  previous turn's `result.usage`; individual `agent_response` `step_update` events carry per-step usage.
- `--print-timeout` defaults to `0s` (wait until the turn completes); leave it unset so the outer orchestrator owns
  monitoring and deadlines. If set and tripped, `agy` exits 3 with `finished_reason: "print_timeout"` and partial or
  empty output.
- A zero exit status — and even `status: "SUCCESS"` in JSON output — is necessary but not sufficient. Startup failures
  (unknown model, unsupported effort) exit 1 with the reason on stderr, and flag errors exit 2, while a completed turn
  exits 0 even when it declines the task, hits denied permissions, or tries to ask a question. Calling `ask_question` in
  `-p` mode does not hang: the tool returns immediately with an empty answer (`A1: `), after which the delegate finishes
  its turn and exits 0. Judge the captured response against the task's explicit acceptance criteria.
- Resuming (per the resume contract in `SKILL.md`): the session id (`conversation_id`) comes from `--output-format
  stream-json` (`.conversation_id` on the first `init` event) or `--output-format json` (`.conversation_id`), so a run
  that will be resumed must launch with one of those formats — prefer `--output-format stream-json` for both the id and
  live liveness events. Writer launch:

  ```sh
  cd <tree> && agy --model <model> --effort <level> --dangerously-skip-permissions --output-format stream-json \
    -p="$(cat <prompt-file>)" < /dev/null > <result-file> 2> <log-file>
  ```

  Record `conversation_id` from the `init` event (`jq -rs '.[0].conversation_id' <result-file>`). Resume with the same
  flags, tree, and `--conversation <id>`, writing to fresh result and log files so the launch stream is kept:

  ```sh
  cd <tree> && agy --model <model> --effort <level> --dangerously-skip-permissions --output-format stream-json \
    --conversation <id> -p="$(cat <resume-prompt-file>)" < /dev/null > <resume-result-file> 2> <resume-log-file>
  ```

  Verified on `agy 1.5.12` in both `--output-format json` and `--output-format stream-json`: a codeword planted on the
  launch turn was answered on the resumed turn, and the final message lands in the same place as on launch
  (`.result.response` in `stream-json`, `.response` in `json`). Pass `--model`, `--effort`, and
  `--dangerously-skip-permissions` again on the resume; they are launch options, not session state you can count on. The
  conversation's working directory stays bound to the launch directory even when resumed from a different directory
  (verified), so keep the launch tree until the delegate is finished and still `cd <tree>` on resume.
- Fresh-session resume signature: an unknown `--conversation <id>` does not fail. `agy` prints `warning: conversation
  "<id>" not found` to stderr, allocates a new `conversation_id`, runs the prompt as a fresh session, and exits 0
  (verified). After every resume, check stderr for that warning and confirm the returned `conversation_id` matches the
  recorded `<id>`; a mismatch means the turn ran as a fresh session rather than a continuation.
- Killing a run: `agy` runs as a single process with threads, while tool commands run as child processes in their own
  session and process group (observed via `pstree` and `ps`: `agy` -> `sh` -> `sleep`, with `sh` and `sleep` in `sh`'s
  own `pgid` and `sid`). SIGTERM to the `agy` pid was observed to terminate `agy` together with its `sh` child and
  `sleep` grandchild (exiting 1), but `setsid` grandchildren and SIGKILL were not checked. Record the `agy` pid at
  launch and kill by walking descendants recursively first — `for c in $(pgrep -P "$pid"); do <recurse on c>; done; kill
  "$pid"`. Never `pkill -f agy`: the pattern matches unrelated `agy` sessions and the shell that launched the run.
- Foreign-harness monitoring rules in `SKILL.md` apply unchanged: run long delegates in the background, use
  `--output-format stream-json` when a long run needs liveness visibility, and use the hard deadline for the kill
  decision.

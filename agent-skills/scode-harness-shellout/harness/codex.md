# Shelling out to codex

Read this file in full before the first `codex exec` launch of a session, after `SKILL.md`.

```sh
codex -c model_reasoning_effort=high exec --yolo -m gpt-6.1-sol -o <scratch-file> "$(cat <prompt-file>)" < /dev/null
```

- Model ids are passed to `-m` literally; codex has no alias that follows new releases. The current ids are
  `gpt-6.1-sol`, `gpt-6-luna`, and `gpt-6-astra`. A request that names only a model line and an effort ("a sol high
  agent") means that line's id in this list; an explicitly versioned id ("gpt-6-sol") is passed as written.
- Reasoning effort is set with the global `-c model_reasoning_effort=<low|medium|high>` option before `exec`. Always
  pass it explicitly rather than relying on the user's config default; without `--json`, the startup header echoes the
  effective `reasoning effort:` if you need to confirm.
- `-o` writes the agent's final message to a file; read that file for the result instead of parsing stdout.
- Always keep the trailing `< /dev/null`. Given a prompt argument and a non-TTY stdin, a fresh `codex exec` launch reads
  stdin to EOF before starting work and appends what it read to the prompt as a `<stdin>` block (its `Reading additional
  input from stdin...` startup line is that read), so a stdin that never delivers EOF blocks it at startup indefinitely.
  A `resume` turn reads stdin only when its prompt is `-`; keep the redirect there anyway, since it costs nothing. At
  least one agent harness omits its own stdin redirect from the wrapper exactly when the command text contains a
  heredoc, handing the child a pipe that never closes — the hang strikes only sometimes and is indistinguishable from a
  slow run except by its log. The explicit redirect holds even then.
- Keep heredocs out of the command that launches codex; a heredoc anywhere in the command text is the known trigger for
  that dropped redirect. Build the prompt in its own earlier command — write it to a scratch file — and pass it as
  `"$(cat <file>)"`.
- Recognize this startup hang by the signature for the run's output mode, and kill it and relaunch with the redirect
  instead of waiting for a completion that will never come. It applies to a fresh launch whose process is still alive
  and whose command lacked the redirect; a launch that already ended in `< /dev/null` cannot be in this hang, so read
  its stderr for the real error instead of relaunching it unchanged. Every fresh launch prints `Reading additional input
  from stdin...` to stderr, healthy or not; what follows it differs (a `resume` turn prints no such line). Without
  `--json`, a healthy run prints the version header (`OpenAI Codex v...`) to stderr about a second after that line, so a
  stderr log that never gets past the stdin line is the hang. With `--json` there is no version header at all, and in
  the healthy runs checked stderr held nothing after the stdin line for the whole run, so a stderr log stuck at that
  line proves nothing; a healthy run writes `thread.started` to its stdout event log within a few seconds, so an event
  log with no `thread.started` after a minute is the hang. Both were checked on codex 0.166.0 by holding stdin open for
  20 seconds: each mode sat at the stdin line, with no header or event, until stdin closed, and started about a second
  later.
- A zero exit status is necessary but not sufficient. `codex exec` does return nonzero when the turn itself fails, but a
  turn that completes normally exits 0 even when its final message declines the task, reports a tool or sandbox failure
  the agent could not work around, or gives status instead of the work. Judge the `-o` file against the task's explicit
  acceptance criteria and reject anything that does not meet them. A result far shorter than the task warrants is the
  cheapest tell, though it is a reason to look rather than grounds to reject on its own — some correct answers really
  are one line. When a whole fan-out fails the same way, treat it as one broken execution path rather than N model
  failures: stop the batch and fix the path instead of escalating each delegate through it.
- Runs in the current working directory by default; pass `-C <dir>` to target elsewhere.
- Resuming (per the resume contract in `SKILL.md`): a run that will be resumed needs a thread id, which only the
  `--json` event stream gives you, so the resumable launch differs from the template above by adding `--json` (with `-o`
  still in place; the two combine):

  ```sh
  codex -c model_reasoning_effort=<effort> exec --yolo -m <model> --json [-C <dir>] -o <scratch-file> \
    "$(cat <prompt-file>)" < /dev/null > <event-log>
  ```

  Record `thread_id` from the `thread.started` event in the event log. Resume with

  ```sh
  codex -c model_reasoning_effort=<effort> exec --yolo -m <model> --json [-C <dir>] resume <thread_id> \
    -o <scratch-file> "$(cat <resume-prompt-file>)" < /dev/null > <event-log>
  ```

  `-m`, `-c`, and `-C` go before `resume`, `-o` after it; getting that order wrong is the one way this was observed to
  fail. Pass `-C` again for a delegate in an isolated tree — whether `resume` restores the original directory on its own
  is unverified, and the failure mode is an editing turn under `--yolo` in the wrong tree. The final message is in the
  `-o` file on both turns. Verified on codex 0.151.0. `turn.completed` usage in the JSON stream is the thread's running
  total, not that turn's own usage: a resume carries every earlier turn's tokens forward, and each value equals the
  rollout's `token_count` `info.total_token_usage` at that point. The last `turn.completed` of a launch and its resumes
  is therefore the run's total, and summing them counts earlier turns again (2-3 times the real figure in three-turn
  checks). A single turn's usage is the difference from the previous turn's `turn.completed`. Verified on codex 0.166.0
  with two independent three-turn runs whose one-word turns reported the earlier turns' output tokens plus about 5.
- Long tasks can exceed your shell tool's default timeout. Run them in the background and monitor them per `SKILL.md`;
  use a foreground timeout only when it is shorter than the monitoring interval. `codex exec` streams its progress while
  working, to stderr as a transcript without `--json` and to the stdout event log with it, so the stream for the run's
  mode growing is a liveness signal. A run stuck at startup is the hang above, judged by the signature for its mode.

## Inspecting delivered tool results

When a caller needs evidence of what the agent received, keep the persisted session as well as the `--json` event
stream. Do not launch with `--ephemeral`. The JSON stream's completed command output can contain the entire stdout even
when the agent received a shortened result. `-o` contains only the final message.

For a local CLI session, locate the rollout whose filename ends in the exact `thread_id` from `thread.started`, under
the launch's effective Codex home (`CODEX_HOME`, or `~/.codex` when unset), in `sessions/`. List matching paths before
reading any content, and confirm its `session_meta` ID and working directory. A thread ID mentioned inside some other
session is not identity evidence. Keep the exact rollout path with the recorded handle. A remote backend or an ephemeral
session may not expose a local rollout; do not assume local history exists for every native child.

Codex CLI 0.163.0 was checked with a deliberately shortened command result and a same-thread resume. The rollout's
`response_item` records with payload type `custom_tool_call_output` held the delivered result as an array of
`input_text` blocks; `call_id` joined it to the `custom_tool_call` input. Its text included a truncation warning and
omission marker. The matching `event_msg` completed `CommandExecution.aggregated_output` retained the full stdout. Other
tools may use different result records. Inspect the actual representation before extracting it; these observations do
not establish all versions, providers or tools.

Extract the matching call and result with their rollout line numbers and complete content, in bounded batches that the
caller can actually inspect. Read all content blocks. If a result contains another tool's JSON as text, inspect that
inner result and its truncation metadata too. Do not substitute the raw command event or a saved full-output file for
the delivered result. Reading that file in a later tool call produces separate evidence. Keep each resume's event log
and confirm it identifies the original thread; resumes append to the same persisted rollout in the checked version.

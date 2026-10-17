# Shelling out to claude

Read this file in full before the first `claude -p` launch of a session, after `SKILL.md`.

```sh
CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 \
  claude -p --model <model> --effort <level> --dangerously-skip-permissions "$(cat <prompt-file>)" < /dev/null
```

- Model: an alias (`sonnet`, `opus`, `haiku`, `fable`), or a full model id such as `claude-fable-5` when the caller was
  given one. Effort levels: `low`, `medium`, `high`, `xhigh`, `max`. The final response is printed to stdout.
- Print mode otherwise terminates background tasks after 600 seconds and exits successfully with a diagnostic instead of
  the requested result. Keep its inner wait unlimited; the outer orchestrator already owns monitoring and cancellation.
- A zero exit status is necessary but not sufficient. Reject empty or truncated output and results that do not satisfy
  the task's explicit acceptance criteria. Also reject the termination diagnostic, which starts with `Background tasks
  still running after`.
- The same outer shell timeout caveat applies.
- Keep the explicit `< /dev/null` and build the prompt in an earlier command instead of a heredoc, per `SKILL.md`. This
  is a precaution against the harness-side dropped redirect, worth taking for any shelled-out delegate — it does not
  claim that `claude -p` reads piped stdin after a prompt argument.
- `claude -p` prints only the final response by default, so a silent run tells you nothing about liveness. When a long
  run needs observability, launch it with `--output-format stream-json --verbose` and compare the stream across
  monitoring checks.
- Resuming (per the resume contract in `SKILL.md`): pass `--session-id "$(uuidgen)"` at launch and record the uuid;
  there is no id to harvest from output otherwise. Resume with

  ```sh
  CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 \
    claude -p --model <model> --effort <level> --resume <uuid> --dangerously-skip-permissions \
    "$(cat <resume-prompt-file>)" < /dev/null
  ```

  Pass model and effort again on the resume; they are launch options, not session state you can count on. Keep the
  output format the same on launch and resume so the caller's check of the final message reads the same place both
  times: with the default format the final message is plain stdout and its last line is where a stop marker would be;
  with `--output-format json` on both, it is the `result` field. The eval verified the resume with `--output-format
  json` on claude 2.1.251; the plain form is the same command minus that flag and is not separately verified. There is
  no `-C` analogue: the working directory is the shell's, so for a delegate in an isolated tree run both the launch and
  the resume from inside that tree (`cd <tree> && claude -p ...`). Claude Code keeps session state per project
  directory, so a `--resume` from a different directory is expected to fail to find the session rather than run in the
  wrong tree — expected, not verified; either way the fix is the same `cd`.

## Inspecting delivered tool results

When the caller needs the tool conversation, retain `--output-format stream-json --verbose` stdout for every turn,
including resumes, rather than only the final response. Correlate each assistant `message.content` block of type
`tool_use` by its `id` with the user message's `tool_result.tool_use_id`. Inspect the result's complete content and
`is_error` flag, plus any structured `tool_use_result` metadata. A tool call without a successful result is not a
successful read. Confirm each stream's `session_id` matches the recorded session; message IDs identify messages, not
sessions.

Claude Code 2.1.302 was checked with a deliberately clipped Read and a same-session resume. The user message's
`tool_result.content` held only the returned lines. `tool_use_result.file` supplied `filePath`, `startLine`, `numLines`,
`totalLines` and `truncatedByTokenCap: true`. The content had no literal omission marker, so inspecting only text for
"truncated" would miss the omitted tail. A later bounded Read returned the complete requested range. These observations
establish that Read representation, not every tool's output or every CLI version. Bash output and other tools need
inspection of their actual result shapes before treating a stream as delivered-content evidence.

Inspect bounded groups of calls/results with their raw-stream line numbers. Keep all result content blocks and metadata;
a summary of how many tools ran loses the delivered material. A full-output artifact named in a shortened result is
available for a later read, not evidence that the agent already received its full contents. The checked Read did not
automatically retrieve missing content. Native children need their own identifiable tool-result history; parent-side
Agent/Task summaries alone do not contain that history.

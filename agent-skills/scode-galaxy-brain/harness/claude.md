# Shelling out to claude

Read this file in full before the first `claude -p` launch of a session, after `harness/shell-out.md`.

```sh
CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 \
  claude -p --model <alias> --effort <level> --dangerously-skip-permissions "$(cat <prompt-file>)" < /dev/null
```

- Model aliases: `sonnet`, `opus`, `haiku`, `fable`. Effort levels: `low`, `medium`, `high`, `xhigh`, `max`. The final
  response is printed to stdout.
- Print mode otherwise terminates background tasks after 600 seconds and exits successfully with a diagnostic instead of
  the requested result. Keep its inner wait unlimited; the outer orchestrator already owns monitoring and cancellation.
- A zero exit status is necessary but not sufficient. Reject empty or truncated output and results that do not satisfy
  the task's explicit acceptance criteria. Also reject the termination diagnostic, which starts with
  `Background tasks still running after`.
- The same outer shell timeout caveat applies.
- Keep the explicit `< /dev/null` and build the prompt in an earlier command instead of a heredoc, per
  `harness/shell-out.md`. This is a precaution against the harness-side dropped redirect, worth taking for any
  shelled-out delegate — it does not claim that `claude -p` reads piped stdin after a prompt argument.
- `claude -p` prints only the final response by default, so a silent run tells you nothing about liveness. When a long
  run needs observability, launch it with `--output-format stream-json --verbose` and compare the stream across
  monitoring checks.
- Resuming (checkpoints, per `harness/shell-out.md`): pass `--session-id "$(uuidgen)"` at launch and record the uuid;
  there is no id to harvest from output otherwise. Resume with

  ```sh
  CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 \
    claude -p --model <alias> --effort <level> --resume <uuid> --dangerously-skip-permissions \
    "$(cat <resume-prompt-file>)" < /dev/null
  ```

  Pass model and effort again on the resume; they are launch options, not session state you can count on. Keep the
  output format the same on launch and resume so the sentinel check reads the same place both times: with the default
  format the final message is plain stdout and the sentinel is its last line; with `--output-format json` on both, it is
  the `result` field. The eval verified the resume with `--output-format json` on claude 2.1.251; the plain form is the
  same command minus that flag and is not separately verified. There is no `-C` analogue: the working directory is the
  shell's, so for a writer in an isolated tree run both the launch and the resume from inside that tree
  (`cd <tree> && claude -p ...`). Claude Code keeps session state per project directory, so a `--resume` from a
  different directory is expected to fail to find the session rather than run in the wrong tree — expected, not
  verified; either way the fix is the same `cd`.

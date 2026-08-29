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

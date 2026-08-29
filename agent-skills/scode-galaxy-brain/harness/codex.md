# Shelling out to codex

Read this file in full before the first `codex exec` launch of a session, after `harness/shell-out.md`.

```sh
codex -c model_reasoning_effort=high exec --yolo -m gpt-5.6-sol -o <scratch-file> "$(cat <prompt-file>)" < /dev/null
```

- Reasoning effort is set with the global `-c model_reasoning_effort=<low|medium|high>` option before `exec`. Always
  pass it explicitly rather than relying on the user's config default; the startup header echoes the effective
  `reasoning effort:` if you need to confirm.
- `-o` writes the agent's final message to a file; read that file for the result instead of parsing stdout.
- Always keep the trailing `< /dev/null`. Given a prompt argument and a non-TTY stdin, `codex exec` reads stdin to EOF
  before starting work (its `Reading additional input from stdin...` startup line is that read), so a stdin that never
  delivers EOF blocks it at startup indefinitely. At least one agent harness omits its own stdin redirect from the
  wrapper exactly when the command text contains a heredoc, handing the child a pipe that never closes — the hang
  strikes only sometimes and is indistinguishable from a slow run except by its log. The explicit redirect holds even
  then.
- Keep heredocs out of the command that launches codex; a heredoc anywhere in the command text is the known trigger for
  that dropped redirect. Build the prompt in its own earlier command — write it to a scratch file — and pass it as
  `"$(cat <file>)"`.
- Treat a background run whose log stays at `Reading additional input from stdin...` and never reaches the version
  header as this startup hang, not a slow model. Kill it and relaunch with the redirect instead of waiting for a
  completion that will never come; a healthy run prints the header immediately after that line.
- A zero exit status is necessary but not sufficient. `codex exec` does return nonzero when the turn itself fails, but a
  turn that completes normally exits 0 even when its final message declines the task, reports a tool or sandbox failure
  the agent could not work around, or gives status instead of the work. Judge the `-o` file against the task's explicit
  acceptance criteria and reject anything that does not meet them. A result far shorter than the task warrants is the
  cheapest tell, though it is a reason to look rather than grounds to reject on its own — some correct answers really
  are one line. When a whole fan-out fails the same way, treat it as one broken execution path rather than N model
  failures: stop the batch and fix the path instead of escalating each delegate through it.
- Runs in the current working directory by default; pass `-C <dir>` to target elsewhere.
- Long tasks can exceed your shell tool's default timeout. Run them in the background and monitor them per
  `harness/shell-out.md`; use a foreground timeout only when it is shorter than the monitoring interval. `codex exec`
  streams a transcript while working, so a log that keeps growing is a liveness signal, and one stuck at
  `Reading additional input from stdin...` is the startup hang above.

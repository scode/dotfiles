# Shelling out to opencode

Read this file in full before the first `opencode run` launch of a session, after `harness/shell-out.md`.

```sh
OPENCODE_CONFIG_CONTENT='{"provider":{"zai":{"models":{"glm-5.3-flash":{"variants":{"low":{"reasoningEffort":"low"},"high":{"reasoningEffort":"high"},"max":{"reasoningEffort":"max"}}}}}}}' \
  opencode run -m zai/glm-5.3-flash --variant high --auto --format json --dir <dir> \
  "$(cat <prompt-file>)" < /dev/null > <result-file> 2> <log-file>
```

The flag surface comes from `opencode run --help`. The runtime behavior — stdin handling, exit codes, how permissions
and effort actually resolve, process layout — was observed with OpenCode 1.18.20 against `zai/glm-5.3-flash` rather than
read from documentation; treat it as an observation to re-check when the CLI changes, not as a stable contract.

- The `OPENCODE_CONFIG_CONTENT` block is not optional decoration: it is how effort reaches the model at all. OpenCode
  ships no reasoning variants for the `zai` provider, so without it `--variant` is silently ignored — even
  `--variant bogus` exits 0 and runs at the default — and the inventory's effort word would mean nothing. Z.ai accepts
  exactly `low`, `high`, and `max` for this model (`medium` is rejected with HTTP 400: thinking cannot be disabled), and
  with the variants defined, reasoning tokens were observed to scale monotonically across the three. Leaving the variant
  off was observed to spend about as much reasoning as `max`, so "no variant" is the expensive default, not the cheap
  one. The env var is per-launch and merges over the user's own config file, so it never touches
  `~/.config/opencode/opencode.json`.
- Given a prompt argument, `opencode run` reads stdin to EOF before starting: an open pipe was observed to block it
  indefinitely with no output. Keep the explicit `< /dev/null` and the prompt-in-a-file rule from
  `harness/shell-out.md`; here they are the known hang fix, not just a precaution. Always pass `-m`; with no model and
  no configured default the run hangs instead of erroring.
- `--auto` is the `--yolo` analogue and belongs in every launch. Without it a permission that resolves to `ask` does not
  hang the headless run — the tool call is rejected and the turn ends with `reason: "tool-calls"` and exit 0 — but the
  delegate then reports failure instead of doing the work. The default `build` agent already allows everything except
  `doom_loop` and writes outside the project directory, which is what `--auto` approves.
- A read-only delegate is made by removing tools, not by denying permissions. A global `permission.edit: deny` in the
  config content was observed to do nothing against the default agent's own allow-all rule, and with edit and bash both
  "denied" the model spawned a subagent through `task` that wrote the file. What holds is a custom agent with the
  writing tools absent — add
  `"agent":{"ro":{"mode":"primary","tools":{"write":false,"edit":false,"patch":false,"bash":false,"task":false}}}` to
  the config content and pass `--agent ro`. The model then sees only glob, grep, read, and webfetch, and under an
  adversarial "create this file by any means" prompt correctly reported that it could not. `task` must be in that list
  or the escape hatch stays open. Keep the prompt-level "do not edit" instruction too.
- A zero exit status is necessary but not sufficient, as for every harness. API and model errors (unknown model,
  rejected `reasoning_effort`, auth) exit 1 with an `error` event on stdout; a completed turn exits 0 even when a tool
  call was rejected or the delegate stopped to ask a question. The `question` tool is denied for the default agent, so a
  delegate that wants input ends its turn with the question as plain text — a gate failure to catch by content.
- With `--format json`, stdout is a JSONL event stream: `step_start`, `tool_use`, `tool`, `text`, `step_finish`. The
  final answer is the `part.text` of the last `text` event; each `step_finish` carries token counts (including
  `reasoning`) and `cost`, which is the cheapest way to see what a run actually spent. Without `--format json` stdout is
  only the final message and stderr carries a banner. One run was observed to emit nothing at all for two minutes before
  its first event, so silence early in a run is provider latency until proven otherwise.
- Tool commands run in their own session (their own `sess` and `pgid`), so SIGTERM to `opencode run` was observed to end
  the run and orphan its shell child, and a process-group kill would miss the child the same way. Kill by walking
  descendants first — `for c in $(ps -o pid= --ppid "$pid"); do <recurse on c>; done; kill "$pid"` — which was observed
  to take everything down. `pgrep -f` on the task text is a trap here: it matches the shell that launched the run.
- Runs in the current working directory by default; `--dir <path>` is the analogue of codex's `-C` and was observed to
  put the delegate's relative writes under that path. There is no native worktree mode; for concurrent writers create
  the tree yourself and point `--dir` at it. Three simultaneous runs in one directory completed cleanly.
- Foreign-harness caveats carry over: `--auto` approves everything not explicitly denied, so use it only where you would
  accept the same for the orchestrating session, and the timeout and monitoring rules in `harness/shell-out.md` apply
  unchanged. With `--format json` the event stream is the liveness signal, subject to the slow-first-event caveat above.

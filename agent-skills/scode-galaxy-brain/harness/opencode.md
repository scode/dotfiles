# Shelling out to opencode

Read this file in full before the first `opencode run` launch of a session, after `harness/shell-out.md`.

```sh
OPENCODE_CONFIG_CONTENT='{"provider":{"zai":{"models":{"glm-5.3-flash":{"variants":{"low":{"reasoningEffort":"low"},"high":{"reasoningEffort":"high"},"max":{"reasoningEffort":"max"}}}}}}}' \
  opencode run -m zai/glm-5.3-flash --variant high --agent build --auto --format json --dir <dir> \
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
- `--agent build --auto` is the closest OpenCode equivalent to `--yolo` and belongs in every launch. The explicit agent
  keeps the launch independent of the user's configured default, while `--auto` approves permissions that would
  otherwise resolve to `ask`. OpenCode must receive its full tool set: `task`, `bash`, `write`, `edit`, and `patch` stay
  available even when the assignment is review-only.
- Never create or select a restricted OpenCode agent, pass `--agent ro`, or deny or remove tools in
  `OPENCODE_CONFIG_CONTENT`. Read-only scope is a prompt and parent-gate contract, not a reason to cripple the harness.
  OpenCode needs its normal tools for scratch work and for native delegation; the orchestrating session remains
  responsible for rejecting unauthorized repository changes.
- A coordinator-shaped GLM assignment is one fully equipped `opencode run`. Let that coordinator use `task` for its own
  fan-out instead of launching one outer OpenCode process per child. Disabling `task` previously turned a review swarm
  into a long series of memory-heavy CLI processes constrained by the outer orchestrator's concurrency. Do not restore
  that failure mode in the name of containment, model routing, or read-only review.
- A zero exit status is necessary but not sufficient, as for every harness. API and model errors (unknown model,
  rejected `reasoning_effort`, auth) exit 1 with an `error` event on stdout; a completed turn exits 0 even when a tool
  call was rejected or the delegate stopped to ask a question. Treat a request for input or a report that the work could
  not proceed as a gate failure regardless of the process status.
- With `--format json`, stdout is a JSONL event stream: `step_start`, `tool_use`, `tool`, `text`, `step_finish`. The
  final answer is the `part.text` of the last `text` event; each `step_finish` carries token counts (including
  `reasoning`) and `cost`, which is the cheapest way to see what a run actually spent. Without `--format json` stdout is
  only the final message and stderr carries a banner. One run was observed to emit nothing at all for two minutes before
  its first event, so silence early in a run is provider latency until proven otherwise.
- Tool commands run in their own session (their own `sess` and `pgid`), so SIGTERM to `opencode run` was observed to end
  the run and orphan its shell child, and a process-group kill would miss the child the same way. Kill by walking
  descendants first — `for c in $(pgrep -P "$pid"); do <recurse on c>; done; kill "$pid"` — which was observed to take
  everything down (observed with `ps -o pid= --ppid`, a GNU-only spelling; `pgrep -P` is the same query on both Linux
  and macOS). `pgrep -f` on the task text is a trap here: it matches the shell that launched the run.
- Resuming (checkpoints, per `harness/shell-out.md`): with `--format json` the events on stdout carry a `sessionID`
  field; record it from the first event. Resume with the same env block and flags plus `--session <id>`:

  ```sh
  OPENCODE_CONFIG_CONTENT='...' opencode run -m zai/glm-5.3-flash --variant <v> --agent build --auto --format json \
    --dir <dir> --session <id> "$(cat <resume-prompt-file>)" < /dev/null > <result-file> 2> <log-file>
  ```

  Verified on opencode 1.18.20. The `OPENCODE_CONFIG_CONTENT` block is per launch, so it goes on the resume too or the
  variant silently stops applying.
- Runs in the current working directory by default; `--dir <path>` is the analogue of codex's `-C` and was observed to
  put the delegate's relative writes under that path. There is no native worktree mode; for concurrent writers create
  the tree yourself and point `--dir` at it. Three simultaneous runs in one directory completed cleanly.
- Foreign-harness monitoring rules still apply, but they must never be implemented by adding permission denials or
  removing tools. `--agent build --auto` deliberately gives the delegate unrestricted access equivalent to the
  orchestrating session. With `--format json` the event stream is the liveness signal, subject to the slow-first-event
  caveat above.

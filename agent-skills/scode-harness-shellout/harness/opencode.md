# Shelling out to opencode

Read this file in full before the first `opencode run` launch of a session, after `SKILL.md`.

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
  `--variant bogus` exits 0 and runs at the default — and the effort the caller supplies would mean nothing. Z.ai
  accepts exactly `low`, `high`, and `max` for this model (`medium` is rejected with HTTP 400: thinking cannot be
  disabled), and with the variants defined, reasoning tokens were observed to scale monotonically across the three.
  Leaving the variant off was observed to spend about as much reasoning as `max`, so "no variant" is the expensive
  default, not the cheap one. The env var is per-launch and merges over the user's own config file, so it never touches
  `~/.config/opencode/opencode.json`.
- Given a prompt argument, `opencode run` reads stdin to EOF before starting: an open pipe was observed to block it
  indefinitely with no output. Keep the explicit `< /dev/null` and the prompt-in-a-file rule from `SKILL.md`; here they
  are the known hang fix, not just a precaution. Always pass `-m`; with no model and no configured default the run hangs
  instead of erroring.
- `--agent build --auto` is the closest OpenCode equivalent to `--yolo` and belongs in every launch. The explicit agent
  keeps the launch independent of the user's configured default, while `--auto` approves permissions that would
  otherwise resolve to `ask`. OpenCode must receive its full tool set: `task`, `bash`, `write`, `edit`, and `patch` stay
  available even when the assignment itself is read-only.
- Never create or select a restricted OpenCode agent, pass `--agent ro`, or deny or remove tools in
  `OPENCODE_CONFIG_CONTENT`. Read-only scope is a prompt-level contract the caller enforces when judging the result, not
  a reason to cripple the harness. OpenCode needs its normal tools for scratch work and for native delegation; the
  orchestrating session remains responsible for rejecting unauthorized repository changes. The same goes for the `skill`
  tool: a `permission.skill` denial in `OPENCODE_CONFIG_CONTENT` or the user's config removes it, and a delegate whose
  task loads a skill by name (a process skill in its task, or a dependency that skill loads) then cannot load anything
  and stops, so never deny it for such a run.
- A coordinator-shaped GLM assignment is one fully equipped `opencode run`. Let that coordinator use `task` for its own
  fan-out instead of launching one outer OpenCode process per child. Disabling `task` turns a nested workload into a
  long series of memory-heavy CLI processes constrained by the outer orchestrator's concurrency. Do not restore that
  failure mode in the name of containment, keeping model choice at the outer session, or read-only scope.
- Native `task` calls are completion-only in the outer JSONL stream. OpenCode 1.18.20 emitted no launch event, stable
  handle, or active-task count when a coordinator issued three calls concurrently; each `tool_use` event appeared only
  when that child finished. The completed event's `part.state.time.start` and `part.state.time.end` showed the real
  interval afterwards, and `part.state.metadata.sessionId` identified the child. In the reproduction, the three start
  times were about 2.5 seconds apart while their completion events arrived after roughly 26, 41, and 79 seconds.
  Therefore:
  - A visible `task` event means that child completed. Its absence says nothing about whether the child was launched or
    is still running.
  - Staggered completions do not imply serial scheduling. Compare the embedded start/end ranges after results arrive if
    concurrency itself needs to be verified.
  - Never derive an in-flight count from `task` events, and never kill a coordinator because fewer completion events are
    visible than the wave was expected to contain.
- Before launch, record one immutable overall hard deadline. Derive it from the bounded maximum of native task waves and
  retries that the caller's process defines, the slowest plausible child in each wave, and coordinator/aggregation time.
  Progress, a new manifest section, a continuation, and a resume after an interruption never reset or extend it. If the
  process has no bounded maximum, choose an explicit bound before launch instead of turning monitoring into an
  open-ended wait.
- Give every coordinator-shaped OpenCode task a durable native-task manifest path that carries the caller-supplied run
  id: under a run-id-named directory the caller supplies, or in the session's private scratch directory with the run id
  in the file name. A generic manifest path in a shared directory collides with another orchestrator's, which is exactly
  what run-id naming exists to prevent. The prompt must explicitly authorize that manifest and the named deliverable as
  scratch writes even when the assignment is read-only; read-only still forbids source, working-tree, VCS, and
  external-state mutations. The coordinator writes a manifest section in a separate step before every native-task wave,
  including a singleton retry or continuation. The section records intent, not proof that OpenCode launched every
  member.

  Append this contract to the coordinator's task, substituting the absolute manifest path:

  ```text
  # Native task manifest

  `<batch-manifest>` and the named final deliverable are additional authorized artifacts. Preserve the original task's
  mutation boundary: a read-only task still forbids source, working-tree, VCS, and external-state changes, while a
  task that authorizes implementation keeps that authority. The scratch-artifact permission neither narrows nor expands
  anything else.

  Before every native `task` invocation wave, including a wave containing only one retry or continuation, append a new
  section to `<batch-manifest>` in a separate tool step. Record a unique wave ID, the expected member count, and a real
  UTC announcement time obtained from `date -u +%Y-%m-%dT%H:%M:%SZ` (never estimate or invent it). This timestamp says
  when the intended wave became durable; only the completed JSONL events' embedded intervals establish when children
  actually ran. For every member record its logical name and pass, the prior child session ID it must resume (`none` for
  a new task), and what a process-valid result must contain. Do not write a mutable `state: running` field: the absence
  of a final state means the wave is still running.

  Then issue the whole wave concurrently in one tool-use batch and wait for every result. Only after the whole wave
  returns, append a finish time obtained from the same command and one terminal record per member containing its logical
  name/pass, returned child session ID, tool outcome, and `process outcome: valid` or `process outcome: invalid` with a
  reason. A returned refusal, blocked result, unreadable input, or incomplete required result is invalid even when the
  tool call completed. A continuation is invalid when its returned child session ID differs from the prior ID. Append
  `final state: completed` only when every required member returned a process-valid result; otherwise append
  `final state: failed`.

  Append later waves; never erase or rewrite earlier content. This manifest records the work you intend to wait for. It
  does not replace the final deliverable or let you report a partial wave as complete.
  ```
- Once a manifest announces a wave, wait for the coordinator to account for the whole wave. The outer `opencode run`
  exiting or crashing, a resource-floor breach, an explicit user stop, or the immutable overall hard deadline may still
  end the run. Silence, one early completion, a missing completion event, or an inner wave marked `failed` may not. A
  failed wave is evidence for the caller's judgment that the coordinator must aggregate into its failure deliverable,
  not permission to kill the still-live outer process. If the coordinator skipped the manifest, keep the live run until
  one of the real stop conditions occurs, then report the missing artifact to the caller as a failed result instead of
  manufacturing a hang diagnosis.
- Any verified outer-process exit that leaves a manifest wave without a final state closes that wave as interrupted,
  whether the process crashed, declined and exited zero, or otherwise ended early. After collecting the final JSONL, the
  outer session appends `final state: interrupted`, the measured exit time, the cause, and every completed member/result
  it observed. Do this before the caller decides whether to resume the run.
- For a deliberate stop of a still-live process, optionally append a non-terminal `stop requested` record, then kill the
  coordinator and descendants as described below. Only after verifying they are dead and collecting the final JSONL may
  the outer session append the measured stop time, final observed results, and `final state: aborted`. A resumed
  coordinator starts a new wave for every retry or continuation; it never completes the old section as though the
  interrupted calls had returned normally. Every prior open wave must be closed before new native tasks start.
- When judging the result, reconcile the manifest against the raw JSONL instead of trusting the coordinator's labels.
  Match each terminal member record to an unused `task` event by returned child session ID, verify its tool status and
  actual output, and apply the validity rules of the process the caller supplied. Session IDs repeat across valid
  continuations, so consume matching events exactly once in chronological wave/pass order; the event assigned to a wave
  must occur after that wave's announcement. For a continuation, also require the prior and returned IDs to match. A
  missing or reused event, mismatched ID, invalid result marked valid, incomplete wave, or contradictory final state
  fails the coordinator result.
- Preserve the manifest, raw JSONL, stderr, process exit, coordinator session ID, every completed child session ID, and
  the unfinished member names whenever a coordinator really is stopped. A partial wave is evidence about the failed run,
  not a completed result from the process it was executing.
- A zero exit status is necessary but not sufficient, as for every harness. API and model errors (unknown model,
  rejected `reasoning_effort`, auth) exit 1 with an `error` event on stdout; a completed turn exits 0 even when a tool
  call was rejected or the delegate stopped to ask a question. Treat a request for input or a report that the work could
  not proceed as a failed turn for the caller to judge, regardless of the process status.
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
- Resuming (per the resume contract in `SKILL.md`): with `--format json` the events on stdout carry a `sessionID` field;
  record it from the first event. Resume with the same env block and flags plus `--session <id>`:

  ```sh
  OPENCODE_CONFIG_CONTENT='...' opencode run -m zai/glm-5.3-flash --variant <v> --agent build --auto --format json \
    --dir <dir> --session <id> "$(cat <resume-prompt-file>)" < /dev/null > <result-file> 2> <log-file>
  ```

  Verified on opencode 1.18.20. The `OPENCODE_CONFIG_CONTENT` block is per launch, so it goes on the resume too or the
  variant silently stops applying.
- Runs in the current working directory by default; `--dir <path>` is the analogue of codex's `-C` and was observed to
  put the delegate's relative writes under that path. There is no native worktree mode; a delegate that needs its own
  tree gets one the caller created, with `--dir` pointed at it. Three simultaneous runs in one directory completed
  cleanly.
- Foreign-harness monitoring rules still apply, but they must never be implemented by adding permission denials or
  removing tools. `--agent build --auto` deliberately gives the delegate unrestricted access equivalent to the
  orchestrating session. With `--format json` ordinary event growth is useful liveness evidence, subject to the
  slow-first-event caveat above. During a native `task` wave, the manifest defines the expected work and the JSONL
  carries completion evidence only; neither file exposes the actual active-task count.

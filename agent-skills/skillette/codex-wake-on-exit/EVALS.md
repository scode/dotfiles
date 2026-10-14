# Evaluating Codex wake-on-exit

These are opt-in maintenance evals. Ordinary use reads `SKILLETTE.md`, not this file. Use the model and reasoning effort
the user requests; otherwise choose an available cheap model. Record the exact model, effort, candidate revision or file
hashes, and runtime in the results. Do not edit the candidate during a final suite.

## Helper regressions

Run these from this directory:

```sh
deno fmt --check --line-width 120 --ext=ts wake-on-exit
deno check wake-on-exit
deno test -A wake-on-exit
```

The tests use an injected recording queue executable. They must never queue messages to a real user session or change
the test process's environment. Keep actual subprocess tests for literal arguments, terminal failure, cancellation,
descendant cleanup, and inherited output pipes; mocked process exits cannot establish those lifecycle properties.

## Real discovery and receipt

Run each case in a fresh ordinary Codex session with a client that stays connected between turns. Native spawned
subagents cannot receive direct queued input after unloading. A one-shot `codex exec` client exits after its first turn.
Neither is a valid receipt test, even when the queue command succeeds. Do not forward callbacks to a parent session or
manually resume an evaluator with a fabricated completion message.

An interactive CLI client is suitable. A test client can also stay subscribed to each ordinary thread through the
installed local app-server protocol. In the tested runtime, the server starts queued turns while the client remains
connected; the test client supplies only the initial task. Record the queue-change notification, queued submission ID,
and received callback user-message ID separately; those IDs need not be identical. Match the callback's run identity and
text to the registered watcher. The client must not send timer-driven model turns, poll for queued input, rewrite
callback text, or manufacture the outcome being tested.

Give each evaluator an isolated scratch directory and a read-only candidate checkout. Have its startup instructions load
the candidate `skillette/SKILL.md` as the active entrypoint. Do not give it the selected skillette path or explain the
helper commands: discovery and following the instructions are part of the test. Allow artifact writes and the supplied
condition command, but prohibit repository edits, VCS, subagents, and messages to unrelated sessions.

Supply a quiet watcher that waits for a timer or an actual file condition, then writes a unique output token and exits
with the specified code. The scripted watcher may poll its data source. Choose a duration long enough that the initial
model turn can finish before the watcher exits; record timestamps to verify that this actually happened. A watcher that
finishes while the initial turn is still active does not establish idle resumption.

Ask for a durable report of arming and continuation, then an updated report after actual callback receipt. Ask the
evaluator to preserve failures instead of inventing success. Example requests:

- `skillette-codex-wake-on-exit: monitor this condition command, then report its exit code and output token.`
- `Use Codex wake-on-exit to wait for this file condition. Continue the existing flow when the watcher exits.`
- `Finish this small active task, then use Codex wake-on-exit when you next become idle.`

Vary the explicit and natural triggers, timer and file conditions, zero and nonzero exits, and literal arguments with
spaces, quotes, shell metacharacters, or newlines. Include active work before idle monitoring. For reliability claims,
run at least 20 separate fresh sessions, and require every counted session to discover the skillette and complete a real
arm/idle/callback/ack cycle. Failed pilots and queue-only results do not count. If instructions or helper behavior
change in response to a failure, freeze the new candidate and run a fresh final suite.

Inspect independent evidence rather than trusting an evaluator's final claim:

1. Its tool history reads the candidate entrypoint and the correct `SKILLETTE.md`.
2. Exactly one run is armed for the evaluator's own actual thread, with the supplied command and literal arguments.
3. The initial turn completes before watcher exit. There are no idle model turns or repeated status/wait calls.
4. A real queue submission drives the next turn. Its run ID and directory match the registered continuation.
5. The helper acknowledgment exists for that generation. The report gives the exact output token and terminal code.
6. The callback contains identity and paths, without embedding watcher output as instructions.

Record every case, failures included, and separate runtime/client failures from instruction failures. A small clean
suite supports a claim about those tested conditions; it does not prove an arbitrary failure probability, restart
recovery, or behavior in other harnesses.

## Additional boundaries

Use isolated fixtures for cancellation before exit, cancellation after enqueue, duplicate callbacks, stale generation
IDs, and foreign thread IDs. Cancel owned watchers through the helper; never signal a PID copied from an old state file.
Give fresh agents successful CLI receipts whose phases are `runner-failed`, `queue-failed`, or `cancelled`; verify they
branch on the phase, report failure or retain cancellation, and never promise a callback or blindly arm a replacement.
Confirm that a late cancelled callback cannot restart monitoring. Keep missing-queue and enqueue-failure cases distinct:
unsupported capability starts nothing, while failed delivery preserves diagnostic state without a retry loop.

Review discussion/edit requests and other-harness requests separately: they must not arm a watcher. Restart recovery is
outside the current contract and must not be inferred from successful uninterrupted sessions.

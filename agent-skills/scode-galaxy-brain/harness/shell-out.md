# Shelled-out delegates: rules common to every harness

Read this file in full before the first shell-out of a session, then the file for the harness you are about to launch.

## Launch hygiene

These rules were each learned from a real incident and apply to every shelled-out harness, whatever its flags:

- Build the prompt in its own earlier command — write it to a scratch file — and pass it to the launch as a file
  argument or `"$(cat <file>)"`. Keep heredocs out of the command that launches the delegate: at least one agent harness
  omits its own stdin redirect from the wrapper exactly when the command text contains a heredoc, handing the child a
  pipe that never closes.
- Always end the launch with an explicit `< /dev/null`. Some harnesses read stdin to EOF before starting work and block
  forever on a pipe that never closes; the hang is intermittent and looks like a slow run. The redirect holds even when
  the wrapper drops its own, and it is harmless on harnesses that never read stdin.
- Set the model and the reasoning effort explicitly on every launch, matching the inventory row you chose. Never rely on
  the user's configured default, and check the harness file for effort words the provider rejects or silently ignores.
- A zero exit status is necessary but not sufficient. Every harness exits 0 when the turn completes, including turns
  whose final message declines the task, reports a tool or permission failure it could not work around, asks a question
  nobody will answer, or gives status instead of the work. Judge the captured result against the task's explicit
  acceptance criteria. A result far shorter than the task warrants is the cheapest tell — a reason to look, not grounds
  to reject on its own. When a whole fan-out fails the same way, treat it as one broken execution path rather than N
  model failures: stop the batch and fix the path instead of escalating each delegate through it.
- Long tasks can exceed your shell tool's default timeout. Run them in the background and monitor them per the section
  below; use a foreground timeout only when it is shorter than the monitoring interval.
- Permission bypass flags (`--yolo`, `--auto`, `--dangerously-skip-permissions`) disable approval, sandboxing, and trust
  checks for the delegate. Use them only where you would accept the same for the orchestrating session.

## Monitoring long-running delegates

The stdin hang above was found only after an orchestrator waited hours on a shelled-out code review that was never going
to finish. The lesson generalizes beyond that one bug: a background delegate has no guaranteed liveness or
forward-progress signal before it exits, and "no news yet" is not evidence of progress.

- Never wait open-endedly on a shelled-out delegate, and never make a foreground call whose timeout exceeds the
  monitoring interval — a blocked foreground wait bypasses monitoring entirely. Run long delegates in the background and
  record what you need to check on and kill them later: job handle or pid, log path, output path, start time, expected
  duration, and a hard deadline. Include still-running delegates in any handoff or pre-compaction note.
- Wake up and check every running delegate at least every 30 minutes — sooner when the expected duration is shorter —
  using whatever timer, scheduled wake-up, or bounded-wait mechanism your harness offers. Failing all else, cap each
  blocking wait at 30 minutes and re-check between waits. In a fan-out, check every member at each wake-up: the batch is
  only done when its slowest member is, and one hung member silently holds the whole batch.
- Check known hang signatures first; they are cheap and decisive. Each harness file names its own (codex's stdin line
  with no version header after it, for example); when one matches, kill and relaunch with the corrected invocation,
  since more waiting cannot help.
- Otherwise weigh the evidence by what the tool shows. Harnesses that stream a transcript or event log while working
  give you something to compare against the last check's baseline; record the new observation for the next one. That is
  the transcript's only role: it is a liveness signal, never the thing you judge. The gate works from the diff, the
  checks, and the delegate's output artifact, and reading a transcript to find out what a delegate did is a sign the
  task spec failed to name an artifact (see `delegating.md`), not a monitoring technique. New output proves activity,
  not necessarily useful progress; a log that has not grown across a full interval is a reason to inspect the run, not
  by itself grounds to kill it — long reasoning stretches can be quiet. Harnesses that print only the final message tell
  you nothing while silent; each harness file says which mode it is in and how to get a stream when a long run needs
  one.
- Use the estimate and the deadline for different decisions. Crossing the expected duration triggers investigation, not
  a kill. Crossing the hard deadline means the run is over budget regardless of apparent liveness: kill it, capture the
  log, and treat the result as inconclusive.
- A hang or launch failure is an execution-path failure, not a substantive model failure — fix the path and relaunch
  once rather than escalating models over it. Make sure the kill takes down the delegate's children too, and before
  relaunching a writer in a shared tree, remove its attributable partial changes (or discard its isolated tree) so the
  retry starts from a known baseline.

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
- Kill by pid, walking descendants first (see the harness files for which process to target). Never `pkill -f codex` or
  the like: the pattern matches unrelated sessions of the same harness and the shell that launched the run.

## Resuming a stopped delegate

Writers stop twice under the checkpoint protocol in `delegating.md` and are resumed in the same session with your
answers. Resume works on every harness (verified 2026-08-30 with a planted codeword that each harness answered on the
resumed turn), and a resumed turn is mostly cache hits, so it is cheap. The rules common to all of them:

- Record the harness's session or thread id at launch; each harness file says where it comes from (a `--session-id` you
  generate, or an id the harness prints in its JSON stream). Without it there is no resume, only a relaunch that throws
  away the delegate's context. Include the id in your monitoring record and in any handoff note.
- The stopped turn exits 0, like every completed turn. Detect the stop by the sentinel on the last line of the final
  message (`AWAITING GUIDANCE` or `AWAITING REVIEW`), and confirm the file it promises exists under `.galaxy-brain/`.
  Each harness file says where the final message is on a resumed turn; it is not always the same place as on the launch.
- Write your `ANSWERS.md` or `REVIEW.md` into the delegate's `.galaxy-brain/` first, then launch the resume. The resume
  prompt is the short one from `delegating.md`; build it in a file like any other prompt, pass it the same way, and keep
  the explicit `< /dev/null`. All of the launch-hygiene rules above apply to a resume unchanged, including running it in
  the background and monitoring it — an implementation turn after `ANSWERS.md` is the long one. Pass the same model,
  effort, working-directory, and permission flags as the launch: none of the harnesses is verified to restore them from
  the session, and a resume that lands in the wrong directory edits the wrong tree with bypass flags on.
- One extra reply per checkpoint at most, per `delegating.md`; after that the delegation is abandoned, its changes
  removed, and the work rerouted or done by you.
- Distinguish two ways a run can end without a sentinel or a report. A run that never got going or stuck (the hang
  signatures above, a launch error) is an execution-path failure: kill it, clean the tree per the monitoring rules, fix
  the path, relaunch. A run that was making progress and was stopped by an environmental cause you have since verified
  and fixed — disk full, OOM, a kill you issued because a shell-tool wait expired — is a crash: resume it, once, rather
  than relaunch. The checkpoint files it already wrote let it re-orient, and same-session resume recovered two eval runs
  killed mid-implementation by a full disk. If the resumed turn dies the same way, stop resuming; treat the result as
  inconclusive, fix the environment, and relaunch from a cleaned tree. A hard-deadline kill is neither: the run is over
  budget and its result is inconclusive, as the monitoring rules say. The test for "crash" is the cause, not the shape
  of the message: a disk-full turn usually ends as a polite report that an edit failed with `No space left on device`,
  exit 0, which reads like a decline. A delegate that declined or asked a question for its own reasons is not a crash,
  and for it the prompt below is a false statement that buys another declined turn; that case is the ordinary gate
  failure, escalated per SKILL.md. When resuming after a crash, tell the delegate what happened, to check the tree, and
  which stop it is heading for — the third sentence below changes with the phase the run was in:

  ```
  Your previous turn was interrupted by <what happened, e.g. a disk-full error: writes failed with "No space left on
  device", including one of your file edits>. The cause has been fixed. Inspect the working tree (status and diff in
  this repository's version control, e.g. `git status` and `git diff`) to see what actually landed, redo any edit that
  was lost, then continue the protocol from where you were:
  <launch turn: finish `.galaxy-brain/ASSUMPTIONS.md` and stop with `AWAITING GUIDANCE` without implementing anything>
  <after ANSWERS.md: finish the implementation, run the project's checks, keep `.galaxy-brain/DECISIONS.md` current,
  and stop with `AWAITING REVIEW` before writing `REPORT.md`>
  <after REVIEW.md: finish applying the review changes, re-run the checks, and write `.galaxy-brain/REPORT.md`>.
  ```

  A resumed turn that shows no sign of its earlier context — it re-reads the task from scratch, asks what
  `.galaxy-brain/` is — is a fresh session, not a resume; kill it, clean the tree, and relaunch.

- Disk is the cause to check first when a run dies for no visible reason. Each isolated worktree of a Rust project costs
  on the order of 1.5 GB of build output, tests leave temp dirs behind, and a fan-out of writers can fill a disk
  mid-turn. Tear down gated trees promptly.

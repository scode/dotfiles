---
name: scode-harness-shellout
description: >
  Rules for launching, monitoring, resuming, and killing a delegate that runs in a foreign agent harness (`codex exec`,
  `claude -p`, `muse exec`, `opencode run`), with the verified launch command for each. Loaded by other skills at the
  moment a delegate is about to be shelled out; inert on its own, never active for a session, and not meant to be
  invoked by the user directly.
---

# scode-harness-shellout

This skill is never active. It answers when loaded and claims nothing about later work. Loading it writes no state,
claims no later spawn, and changes nothing about the session; it exists to be read by a skill that has already decided
to run a delegate through a foreign harness and now needs the mechanics. Everything below is conditional on inputs the
caller supplies, and the caller keeps every decision: which model, which harness, whether to resume, how to judge the
result, and what to do with the tree afterwards.

## What the caller supplies

The text below never chooses any of these; where it needs one, it names it.

- The launch mechanism: one of `codex exec`, `claude -p`, `muse exec`, or `opencode run`. The fifth value a caller can
  hold, `native`, means the caller's own sub agent mechanism; nothing in this skill applies to it.
- The model id and reasoning effort, both set explicitly on every launch.
- A run id, a string unique to this delegation that the caller generated, used to name every file this skill creates.
- The prompt, as a file the caller writes before the launch, the way Launch hygiene below describes.
- The working directory or tree the delegate runs in.
- An expected duration and a hard deadline.
- Whether the run will be resumed later (a writer that stops and continues), so that the session or thread id is
  arranged at launch.
- Where the deliverable goes: a named artifact file the prompt tells the delegate to write, because every harness
  captures only the final message and a deliverable emitted mid-run is stranded in the transcript.

## Read the harness file, then launch

The harness files are the only place launch commands live. That is deliberate: a launch built from memory of a previous
session, or improvised from a harness's `--help`, skips the observed-behavior notes that exist because each one cost
someone a hung or silently wrong run. Read the file for the mechanism in full the first time in a session that you are
about to launch through it, then launch.

| mechanism      | read                                                   |
| -------------- | ------------------------------------------------------ |
| `codex exec`   | `harness/codex.md`                                     |
| `claude -p`    | `harness/claude.md`                                    |
| `muse exec`    | `harness/muse.md`                                      |
| `opencode run` | `harness/opencode.md`                                  |
| `native`       | nothing; this skill has no part in a native delegation |

"`harness/`" means the `harness` directory under the directory this `SKILL.md` was loaded from, whatever path that is in
the current harness. Each file carries the launch template, the writer launch and resume commands, where the final
message lands, the effort words the provider rejects or silently ignores, and what a dead or hung run looks like. Where
a harness's process layout has been observed, the file also says which process to target so a kill takes the delegate's
children with it; the others have not been verified and say nothing.

## Launch hygiene

These rules were each learned from a real incident and apply to every shelled-out harness, whatever its flags:

- Every file you create for a delegate — prompt files, result and `-o` files, event logs, stderr logs, resume prompts —
  goes in scratch space private to this session (a fresh `mktemp -d`, or your harness's per-session scratch directory)
  and carries the run id the caller supplies in its name. Never a fixed name under `/tmp`: other orchestrators may be
  running delegates on the same machine at the same time, and this skill's `SPEC.md` requires that nothing it writes can
  collide with or be mistaken for theirs.
- Build the prompt in its own earlier command — write it to a scratch file — and pass it to the launch as a file
  argument or `"$(cat <file>)"`. Keep heredocs out of the command that launches the delegate: at least one agent harness
  omits its own stdin redirect from the wrapper exactly when the command text contains a heredoc, handing the child a
  pipe that never closes.
- Always end the launch with an explicit `< /dev/null`. Some harnesses read stdin to EOF before starting work and block
  forever on a pipe that never closes; the hang is intermittent and looks like a slow run. The redirect holds even when
  the wrapper drops its own, and it is harmless on harnesses that never read stdin.
- Set the model and the reasoning effort explicitly on every launch, matching the model and effort the caller supplies.
  Never rely on the user's configured default, and check the harness file for effort words the provider rejects or
  silently ignores.
- A zero exit status is necessary but not sufficient. Every harness exits 0 when the turn completes, including turns
  whose final message declines the task, reports a tool or permission failure it could not work around, asks a question
  nobody will answer, or gives status instead of the work. The caller judges the captured result against the task's
  explicit acceptance criteria. A result far shorter than the task warrants is the cheapest tell — a reason to look, not
  grounds to reject on its own. When a whole fan-out fails the same way, treat it as one broken execution path rather
  than N model failures: stop the batch and fix the path instead of judging each delegate separately.
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
  the transcript's only role: it is a liveness signal, never the thing you judge. The caller judges artifacts — the
  diff, the checks, the delegate's output file — not transcripts, and reading a transcript to find out what a delegate
  did is a sign the prompt failed to name an artifact, not a monitoring technique. New output proves activity, not
  necessarily useful progress; a log that has not grown across a full interval is a reason to inspect the run, not by
  itself grounds to kill it — long reasoning stretches can be quiet. Harnesses that print only the final message tell
  you nothing while silent; each harness file says which mode it is in and how to get a stream when a long run needs
  one.
- Use the estimate and the deadline for different decisions. Crossing the expected duration triggers investigation, not
  a kill. Crossing the hard deadline means the run is over budget regardless of apparent liveness: kill it, capture the
  log, and report the result to the caller as inconclusive.
- A hang or launch failure is an execution-path failure, not a substantive model failure: report it to the caller as
  such, with the signature seen, rather than as a failure of the model. The caller owns cleanup of whatever the run left
  in the tree and decides whether to relaunch; when it does, fix the path first, and make sure the kill took down the
  delegate's children too.
- Kill by pid, walking descendants first (see the harness files for which process to target). Never `pkill -f codex` or
  the like: the pattern matches unrelated sessions of the same harness and the shell that launched the run.

## Resuming a stopped delegate

A delegate that stops mid-task and is later continued with more input is resumed in the same session rather than
relaunched. Resume works on every harness (verified 2026-08-30 with a planted codeword that each harness answered on the
resumed turn), and a resumed turn is mostly cache hits, so it is cheap. Whether a run stops, what it must produce before
stopping, and what the continuation says are the caller's protocol; this skill supplies the mechanics common to every
harness:

- Record the harness's session or thread id at launch; each harness file says where it comes from (a `--session-id` you
  generate, or an id the harness prints in its JSON stream). Without it there is no resume, only a relaunch that throws
  away the delegate's context. Include the id in your monitoring record and in any handoff note.
- A stopped turn exits 0, like every completed turn; the caller detects the stop from the final message. Each harness
  file says where the final message is on a resumed turn; it is not always the same place as on the launch.
- Build the resume prompt in a file like any other prompt, pass it the same way, and keep the explicit `< /dev/null`.
  All of the launch-hygiene rules above apply to a resume unchanged, including running it in the background and
  monitoring it. Pass the same model, effort, working-directory, and permission flags as the launch: none of the
  harnesses is verified to restore them from the session, and a resume that lands in the wrong directory edits the wrong
  tree with bypass flags on.
- A resume can come back as a fresh session instead of a continuation: a resumed turn that shows no sign of its earlier
  context, re-reading the task from scratch or asking where it was told to write. Report it to the caller as such;
  classifying it and deciding what happens to the run are the caller's. A harness file adds a mechanism-specific
  signature only where one has been observed.
- Disk is the cause to check first when a run dies for no visible reason. Each isolated worktree of a Rust project costs
  on the order of 1.5 GB of build output, tests leave temp dirs behind, and a fan-out of writers can fill a disk
  mid-turn. Tear down trees promptly once the caller has gated their results and is done with them; a tree whose result
  is still ungated holds the only copy of that result.

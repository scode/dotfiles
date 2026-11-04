# Scheduling reviewer turns

Always invoke the `schedule` launcher relative to this skill's actual base directory; `/skill/schedule` below is a
placeholder. Like `evidence`, it selects a matching compiled executable when present, otherwise runs the neighboring
`schedule.ts` with Deno.

The scheduler owns three things: the queue of turns you have marked ready, the concurrency limit, and the rule that one
reviewer identity never has two live turns. It runs the launch script you wrote for each turn, records when it started
and how it exited, and parks the identity as `awaiting-parent`. It never builds launch commands, reads transcripts or
findings content, audits delivery, or decides that a turn is done. A turn that exited 0 is a turn you have not judged
yet.

Use it for every reviewer turn of a run. A turn launched outside it bypasses the one-live-turn check.

## Set up a run

```text
/skill/schedule init /run/schedule --max 19
/skill/schedule add /run/schedule g001 --findings /run/g001/findings.jsonl
```

Every path is absolute, and unknown options are refused. `--max` is the actual concurrency you can sustain; change it at
any time with `/skill/schedule cap /run/schedule 12`, and the daemon applies it on its next pass. Identity names use
letters, digits, `.`, `_`, and `-`.

Start the daemon once:

```text
/skill/schedule run /run/schedule
```

Use your harness's ordinary background execution. Where it has none, start it detached with its output in the schedule
directory, for example `nohup /skill/schedule run /run/schedule > /run/schedule/daemon.log 2>&1 &`. It refuses to start
while another daemon for the same directory is alive. Each turn runs in its own process group, so neither `schedule
stop` nor a signal to the daemon's process group touches running turns, and a daemon started later records how they
ended. If the daemon dies, start it again.

## Write a launch script

Write each turn's launch script from the shellout harness file for the reviewer's backend: the launch or resume command
with its event log, final-message file, and `< /dev/null`, then the `evidence audit` for the turn, then an exit with the
reviewer's own status, so a failed reviewer turn is not reported as the audit's success. The scheduler runs the script
as `/bin/sh <script>` with stdin from `/dev/null` and keeps its stdout and stderr in `turns/<id>-<n>.out` and `.err`
under the schedule directory. A first Codex turn learns its thread ID only from the `thread.started` event, so extract
it inside the script before auditing:

```sh
codex ... exec ... --json -C /work -o /run/g001/turn1.reply "$(cat /run/g001/turn1.prompt)" < /dev/null > /run/g001/turn1.events.jsonl
s=$?
jq -r 'select(.type=="thread.started") | .thread_id' /run/g001/turn1.events.jsonl | head -n 1 > /run/g001/thread
thread=$(cat /run/g001/thread)
rollout=$(find "$HOME/.codex/sessions" -name "*$thread.jsonl" | head -n 1)
/skill/evidence audit codex "$thread" /work /run/g001/manifest.json "$rollout" > /run/g001/turn1.audit.json
exit $s
```

Take the command lines themselves from the harness file, not from this sketch. Later turns resume with the recorded
thread ID and audit the same rollout, which resumes append to.

## Queue a turn

```text
/skill/schedule ready /run/schedule g001 --script /run/g001/turn2.sh --label synthesis --audit /run/g001/turn2.audit.json
```

`ready` refuses unless the identity is new or awaiting a parent decision, so a second turn can never be queued while one
is queued or running. Queue the next turn only after you have judged the previous one under the skill's rules. Queued
turns start in the order they became ready as soon as a slot is free, with no further call from you. `schedule cancel
/run/schedule g001` takes back a turn queued by mistake before it starts.

When an identity needs no more turns, close it as `complete` or `stopped-incomplete` with your reason:

```text
/skill/schedule finish /run/schedule g001 --state complete --reason "final discovery pass empty after audit"
```

## Wait for turns to end

```text
/skill/schedule wait /run/schedule --after 41
```

Run it in the background so the harness's completion notice wakes you. It blocks until a turn ends or the daemon exits
after journal sequence number 41, then prints those events with `last_seq` and exits 0. Pass that `last_seq` as
`--after` next time, and keep it in your current-state file. It exits 3 at once when no daemon is alive, rather than
waiting forever, and 4 when an optional `--timeout SECONDS` passes; use the timeout only where your harness has no
completion notice and you need a bounded wait. The daemon counts as alive only while its heartbeat is fresh and its pid
is alive, never by matching command lines. A wake reports only the events that existed at that moment, and more turns
may have ended since; after each wake, act on every identity `status` lists as awaiting you, not only the ones in the
wake's events.

## Read the state

```text
/skill/schedule status /run/schedule
/skill/schedule status /run/schedule --all
```

`status` prints the journal position `seq`, counts by state, `idle_capacity` (free slots), every identity awaiting a
decision, oldest first, and the running turns with their pids and output files. Each awaiting identity shows its exit
status, a compact summary of the audit report named at `ready`, and `findings_start_line` and `findings_end_line`: the
findings-log lines when the turn started and ended. Pass `findings_start_line` to `evidence events --after` to read that
turn's records. Free slots next to queued turns mean the daemon is not running or has not made its next pass yet.
`--all` adds every identity. This is the view to read after each wake-up and after a compaction, instead of
chronological notes.

An exit of `lost` means nothing of the turn's process group is left and it recorded no exit status, or it never started
within 30 seconds of being launched. Treat it as a failed turn, not an empty one. Before resuming that reviewer, confirm
that its session is not still being written (for Codex, that the rollout has stopped growing), since a process that left
the turn's group could still be running it.

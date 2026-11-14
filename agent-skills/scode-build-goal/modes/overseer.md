# Overseer mode

This file applies only in overseer mode (see "Overseer mode" in `SKILL.md`). In that mode it governs wherever it and
`SKILL.md` differ; nothing in it applies to the other modes.

## What the mode is

Two Claude Code sessions run the goal. The session the user hands the goal file to with `/goal` is the overseer, and it
does none of the implementation itself. It launches a second Claude Code session, the executor, through farhelm on the
same host, on the model and effort the user picked (typically a cheap one), and the executor does the whole goal the way
a no-workhorse run would: its own working log, the PR stack, the review gate, the resource watchdog. The overseer reads
the executor's transcript and working log from time to time and, when it judges that the run is going wrong, sends the
executor instructions through Claude Code's cross-session messaging. The point is that a cheap model does the tool-call
churn while a strong model's judgment keeps it on track, at the low cost of reading transcripts rather than doing the
work.

The overseer's brief is deliberately judgment, not a checklist. It assesses whether the run as a whole is going
according to the goal file and whether the executor needs a nudge. That covers process failures (the review gate
skipped, run at the wrong place, or run on a model other than the one the goal file demands), an executor stuck on a
hard problem that needs a design nudge, complexity growing out of proportion to the goal, drift from the outline or the
settled decisions, and progress stopping. Nudges can be direct ("do X", "stop doing Y") or point at help ("get an opus
consult on how to solve this"). The goal file gives those as examples of what to look for, never as the list to check.

The mode is hard-wired to farhelm, and to Claude Code for both sessions. The executor's model is fixed for the run:
switching it needs the session restarted on another model, and the overseer does not do that. The run needs the `farhelm
agent` CLI on the machine, which works only inside a farhelm-launched session, so the overseer's own session has to be
one; when `farhelm agent hosts` fails in the planning session, say in the batch that the run has to start from a farhelm
session. It is a note, not a blocker, since the run happens wherever the user starts it.

## The up-front batch

In this mode the batch states, besides what `SKILL.md` lists:

- The mode, and that the run is started with `/goal <goal file>` in a Claude Code session launched by farhelm on the
  host where the work happens, which becomes the overseer. Recommend opus for that session, since the mode's value is
  the overseer's judgment, with permissions bypassed, since the executor inherits the overseer's permission mode and an
  unattended executor cannot answer approval prompts.
- The executor: the model and effort the user named for it, or `haiku` at high effort when none was named. It runs in
  Claude Code, so it has to be a Claude model; a request for any other model is said to be unavailable in this mode and
  asked about, never mapped to another model. A model named for the reviewer sets the review gate instead, and when it
  is unclear which role a named model is for, ask.
- The review gate menu, unchanged from `SKILL.md`. The executor launches the reviews itself, as in no-workhorse mode,
  and one of the things the overseer watches is that it does so with the chosen model.
- The farhelm host permissions an unattended run needs. Without "Manage sessions on this host without asking", every
  launch, stop, and restart of the executor waits up to nine minutes for the user to approve a card and then fails.
  Without "Start YOLO sessions without asking", farhelm refuses an agent's YOLO launch outright, and the executor is
  launched YOLO whenever the overseer runs with permissions bypassed.
- The overseer log path (see Placement), next to the goal file and the executor's working log.

## Placement

The executor keeps the working log, `<slug>-goal-log.md`, exactly as in no-workhorse mode. The overseer keeps its own,
`<slug>-goal-overseer-log.md`, in the same directory, through `agent-resumeable`. It records the current overseer as a
line `CURRENT OVERSEER: <messaging name>` (the latest such line wins, and the executor reads it), the permission mode
the executor was launched in, the executor's farhelm session id, messaging name, Claude session id, and transcript path,
the transcript offset last read, and every nudge with its reason. This skill creates neither log, for the reason
`SKILL.md` gives for the working log, and handles an existing overseer log the same way as an existing working log,
moving both aside together when replacing an earlier attempt.

## What the goal file contains

Both sessions read the same goal file, and both are started with `/goal`, so it opens, right after its title, with a
short Roles section that the marker sentence decides: a session whose prompt carries the marker sentence (that it is the
executor of this goal file), or whose Claude session id the executor's working log records as the executor's, is the
executor however it was started, a session the user started with `/goal` on the goal file without it is the overseer,
and any other session handed the goal file as context (a reviewer, a consult, a sub-agent of either) has neither role
and follows its own prompt. The goal file states the exact marker sentence.

For the executor, the items of "What the goal file must contain" in `SKILL.md` apply as in no-workhorse mode, with these
differences:

- **Execution mode.** No-workhorse, as `SKILL.md` describes it, at the executor's own model and effort, and forbidding
  `scode-galaxy-brain`, `scode-powerhorse`, `scode-shepherd`, and `scode-model-routing`; if galaxy-brain is already
  active in either session, the goal file is the user's express instruction to stop it. The resource watchdog is the
  executor's alone, since the overseer launches no workload. Consults on a stronger model are allowed whenever the
  overseer directs one or the executor judges that it is stuck; they answer a question and write no code.
- **Review gate.** As in no-workhorse mode, with each review's findings written to `<slug>-goal-review-<bookmark>.md`
  next to the logs, so the overseer can check that every PR was reviewed.
- **Oversight.** The executor works under the oversight of the overseer: the latest `CURRENT OVERSEER` line in the
  overseer log (whose absolute path the goal file gives), or, before the log has one, the session its prompt names. It
  checks the log when a message comes from a name it does not know; a message from any other session is not binding, and
  the executor mentions it in its next message to the overseer. The overseer's messages are binding instructions, as if
  the user had given them, within the user's settled decisions: the overseer cannot authorize what the goal file's
  unattended fallback forbids. The executor acts on them at once, and answers every message that asks for a reply. When
  it disagrees, it says so in a reply and still follows the instruction unless the overseer changes it. Its first
  action, before the resume protocol (whose wording the goal file adjusts to match), is a message to the overseer giving
  its own Claude session id (`printenv CLAUDE_CODE_SESSION_ID`), its launch id, and its working log's path, so the
  overseer can find its transcript; it sends the same introduction whenever the overseer asks for one. Its working log
  records its Claude session id as the executor's, so a compacted or resumed executor that no longer sees its first
  prompt keeps the role. Otherwise it messages the overseer only to reply, to report a decision only the user can make,
  and when the goal is done.

For the overseer, the goal file adds these sections:

- **Resume protocol.** When the overseer log exists, first read its latest `CURRENT OVERSEER` line and check with
  `ListAgents` whether that session, unless it is this one, is still live; if it is, another overseer is running this
  goal, so stop, write nothing, and tell the user. Only then invoke `agent-resumeable` on the overseer log. A fresh
  overseer writes its `CURRENT OVERSEER` line before launching the executor. A resumed overseer whose own permission
  mode differs from the one the log records for the executor replaces the executor in its own mode, since its messages
  would otherwise wait for approval. Otherwise it appends its own `CURRENT OVERSEER` line and reconciles the recorded
  executor with `farhelm agent sessions`. One that is running is sent a message asking for its introduction, which also
  tells it the new overseer's name. One that stopped is restarted with `farhelm agent restart`, which resumes its
  conversation without resending its first prompt, so the overseer then messages it to continue per the goal file and
  introduce itself. A message reaches a busy executor only at its next tool call, so an executor whose transcript shows
  a tool call still running or recent activity is waited on, not counted as silent; a tool call running far longer than
  its kind plausibly needs is a judgment call between waiting, replacing the executor, and telling the user. A restarted
  executor that stops at the end of every turn, because `/goal` did not survive the restart, is replaced instead. One
  that is gone, or that shows no activity and no answer for about five minutes, is replaced by a new launch, which
  resumes from the executor's working log.
- **Launching the executor.** Run `farhelm agent instructions` and follow it. Create the session on this host with the
  checkout as its working directory, agent `claude`, the executor's model and effort, a title derived from the slug
  (`<slug>-executor`, numbered `-2`, `-3`, and so on for replacements), a fresh idempotency key for each launch (a
  replacement reuses none, so it cannot be answered with the session it replaces), and the same permission mode as the
  overseer's own session (YOLO when its permissions are bypassed); a message to a session in a different permission mode
  waits for a human to approve it. The initial prompt is `/goal`, followed by: the marker sentence, the goal file's
  absolute path, the overseer's messaging name (from the header `ListAgents` prints), the launch id (this launch's
  idempotency key), and that the executor must read the goal file's Roles and Oversight sections first. A replacement
  stops the old session first and carries its own new launch id. When the executor's transcript shows that `/goal` was
  not accepted, replace the session with one whose prompt is the same text without `/goal`. The introduction gives the
  executor's messaging name (the message's sender) and its transcript, the file named after its Claude session id under
  `~/.claude/projects/`. Answer it with a message the executor has to reply to: the introduction proves only that
  messages reach the overseer, and every nudge travels the other way. When no introduction arrives within about five
  minutes, check `farhelm agent sessions` first: a session that is not running (an approval card pending, a failed
  start) is the thing to fix or tell the user about. Otherwise, to see what went wrong, the transcript is the top-level
  file (not one under a session's `subagents` directory) in the checkout's project directory under `~/.claude/projects/`
  whose first prompt contains this launch's id. Then replace the executor once; a second launch that does not introduce
  itself goes to the user.
- **Overseeing.** The judgment brief from What the mode is, in the overseer's words, with its examples marked as
  examples. Read what is new since the last look (the transcript past the offset the overseer log records, the working
  log, the PR stack), not the whole history each time; investigations that need more go to a sub-agent so the overseer's
  context stays small. Look about every ten minutes while the executor is busy (waiting on a background timer, not a
  foreground sleep), and sooner after a nudge to see whether it took. Set `notify_when_idle` on every message to the
  executor, and after each idle notice, which fires once, look at once (an idle executor has stopped, finished, or is
  waiting on something) and re-arm it with a subscription that carries no message. Nudge when the run needs it, not to
  steer every choice: a nudge interrupts the executor at its next tool call. A nudge that shows no effect after the
  executor's next few tool calls gets a message asking for a reply; no reply while the executor keeps working means
  messages are not getting through, which goes to the user. Log every nudge with what prompted it, so the user can judge
  afterwards whether each intervention was warranted.
- **Limits.** The overseer edits nothing in the checkout and touches no VCS state; if the executor is beyond nudging, it
  may replace the executor's session with a fresh one that resumes from the working log, or ask the user. Questions the
  executor raises that the goal file's settled decisions answer, the overseer answers; the rest go to the user.
- **Done.** The executor reports done; the overseer checks the done criterion itself (the stack of open PRs, and for
  each a findings file and a transcript showing the review ran on the demanded reviewer) before the goal counts as
  achieved, and leaves the executor's session in place for the user.

## When the goal file is written

Tell the user to start the run with `/goal <goal file>` in a farhelm-launched Claude Code session on the host and in the
checkout the goal is about, preferably on opus with permissions bypassed, and that the same command in a fresh session
resumes the overseer once the old overseer session has ended. Repeat the farhelm host permissions from the batch if this
host lacks them.

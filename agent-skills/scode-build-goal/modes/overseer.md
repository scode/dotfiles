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
  host where the work happens, which becomes the overseer. Recommend opus for that session: the mode's value is the
  overseer's judgment.
- The executor: the model and effort the user named for it, or `haiku` at high effort when none was named. It runs in
  Claude Code, so it has to be a Claude model; a request for any other model is said to be unavailable in this mode and
  asked about, never mapped to another model. A model named for the reviewer sets the review gate instead, and when it
  is unclear which role a named model is for, ask.
- The review gate menu, unchanged from `SKILL.md`. The executor launches the reviews itself, as in no-workhorse mode,
  and one of the things the overseer watches is that it does so with the chosen model.
- The overseer log path (see Placement), next to the goal file and the executor's working log.

## Placement

The executor keeps the working log, `<slug>-goal-log.md`, exactly as in no-workhorse mode. The overseer keeps its own,
`<slug>-goal-overseer-log.md`, in the same directory, through `agent-resumeable`; it records the executor's farhelm
session id, its messaging name, its Claude session id and transcript path, and every nudge with its reason. This skill
creates neither log, for the reason `SKILL.md` gives for the working log, and handles an existing overseer log the same
way as an existing working log, moving both aside together when replacing an earlier attempt.

## What the goal file contains

Both sessions read the same goal file, so it opens, right after its title, with a short Roles section: a session whose
prompt says it is the executor of this goal file follows the executor's sections; the session the user started with
`/goal` on the goal file is the overseer; and any other session handed the goal file as context (a reviewer, a consult,
a sub-agent of either) has neither role and follows its own prompt. The prompt the overseer gives the executor (below)
is what makes the difference, so the goal file states the exact marker sentence the overseer puts in it.

For the executor, the items of "What the goal file must contain" in `SKILL.md` apply as in no-workhorse mode, with these
differences:

- **Execution mode.** No-workhorse, as `SKILL.md` describes it, at the executor's own model and effort, and forbidding
  `scode-galaxy-brain`, `scode-powerhorse`, `scode-shepherd`, and `scode-model-routing`; if galaxy-brain is already
  active in either session, the goal file is the user's express instruction to stop it. The resource watchdog is the
  executor's alone, since the overseer launches no workload. Consults on a stronger model are allowed whenever the
  overseer directs one or the executor judges that it is stuck; they answer a question and write no code.
- **Oversight.** The executor works under the oversight of the session the prompt names. Messages from it (a
  cross-session message whose `from` is that name) are binding instructions, as if the user had given them, within the
  user's settled decisions: the overseer cannot authorize what the goal file's unattended fallback forbids. The executor
  acts on them at once. When it disagrees, it says so in a reply and still follows the instruction unless the overseer
  changes it. Its first action in every session, fresh or resumed, before the resume protocol (whose wording the goal
  file adjusts to match), is a message to the overseer giving its own Claude session id (`printenv
  CLAUDE_CODE_SESSION_ID`) and its working log's path, so the overseer can find its transcript; after that it messages
  the overseer only to reply, to report a decision only the user can make, and when the goal is done.

For the overseer, the goal file adds these sections:

- **Resume protocol.** Its own, with `agent-resumeable` on the overseer log. A resumed overseer reconciles the recorded
  executor with `farhelm agent sessions`: one that is running is the executor still, one that stopped is restarted with
  `farhelm agent restart` (which resumes its conversation), and one that is gone is replaced by launching a new executor
  with the same prompt, which resumes from the executor's working log. The executor trusts only the overseer name its
  prompt carries, so when the resumed overseer's own messaging name differs from the one in the overseer log, it
  replaces the executor in the same way (stopping it first) with a prompt carrying the new name.
- **Launching the executor.** Run `farhelm agent instructions` and follow it. Create the session on this host with the
  checkout as its working directory, agent `claude`, the executor's model and effort, a title derived from the slug
  (`<slug>-executor`), a fresh idempotency key for each launch (a replacement reuses none, so it cannot be answered with
  the session it replaces), and the same permission mode as the overseer's own session (YOLO when its permissions are
  bypassed). Cross-session messages between sessions in different permission modes are held for a human to approve,
  which would silence every nudge in an unattended run, so a held-message notice is a reason to stop and tell the user.
  The initial prompt is `/goal`, followed by: the marker sentence that this session is the executor of the goal file,
  the goal file's absolute path, the overseer's messaging name (from the header `ListAgents` prints), and that the
  executor must read the goal file's Roles and Oversight sections first. When the executor's transcript shows that
  `/goal` was not accepted, replace the session with one whose prompt is the same text without `/goal`. The executor's
  introduction message gives the transcript, which is the file named after its Claude session id under
  `~/.claude/projects/`.
- **Overseeing.** The judgment brief from What the mode is, in the overseer's words, with its examples marked as
  examples. Read what is new since the last look (the transcript past the last offset read, the working log, the PR
  stack), not the whole history each time; investigations that need more go to a sub-agent so the overseer's context
  stays small. Look about every ten minutes while the executor is busy (waiting on a background timer, not a foreground
  sleep), sooner after a nudge to see whether it took, and subscribe to the executor's idle notice so that a stop is
  noticed at once. Nudge when the run needs it, not to steer every choice: a nudge interrupts the executor at its next
  tool call. Log every nudge with what prompted it, so the user can judge afterwards whether each intervention was
  warranted.
- **Limits.** The overseer edits nothing in the checkout and touches no VCS state; if the executor is beyond nudging, it
  may replace the executor's session with a fresh one that resumes from the working log, or ask the user. Questions the
  executor raises that the goal file's settled decisions answer, the overseer answers; the rest go to the user.
- **Done.** The executor reports done; the overseer checks the done criterion itself (the stack of open PRs, a review by
  the demanded reviewer on each) before the goal counts as achieved, and leaves the executor's session in place for the
  user.

## When the goal file is written

Tell the user to start the run with `/goal <goal file>` in a farhelm-launched Claude Code session on the host and in the
checkout the goal is about, preferably on opus, and that the same command in a fresh session resumes the overseer. The
first launch of the executor may show a farhelm approval card, particularly for a YOLO launch on a host that asks first.

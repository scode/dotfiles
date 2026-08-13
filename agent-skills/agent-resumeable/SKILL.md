---
name: agent-resumeable
description: Use only when the user explicitly invokes `$agent-resumeable PATH` or `/agent-resumeable PATH`. Puts the session under a resume-log protocol anchored at PATH; if the log exists, read it and resume the work, otherwise create it. Either way, keep logging for the rest of the session so a fresh session can pick up after an abrupt break.
---

# agent-resumeable

Sessions die abruptly — crashes, context exhaustion, safeguard triggers, network drops. This skill maintains a working
log at a user-specified path so that a fresh session can read it and continue the work with minimal loss. Invocation
means the whole session runs under this protocol: everything below applies until the session ends or the user explicitly
stops it. Finishing a task does not end it, and neither does context compaction — a summary that has gone quiet about
the protocol is a summarization artifact, not a decision anyone made.

After compaction or resume, if retained context says or implies a resume log is being kept, re-read this SKILL.md and
the log itself before further substantive work; the details of the protocol do not survive summarization reliably. When
writing a handoff or pre-compaction note, include the log's absolute path and the fact that this protocol is active.

The invocation is `$agent-resumeable <path>` (equivalently `/agent-resumeable <path>`) where `<path>` points at a
markdown file (which may not exist yet). The path is the anchor; the same path handed to a later session is how
resumption works. Resolve it to an absolute path immediately and use the absolute form from then on — a relative path
reinterpreted from a different working directory silently splits the log in two.

## On invocation

1. Check whether `<path>` exists.
2. **If it exists**: it is the resume point. Read the header, find the latest checkpoint entry (label contains
   CHECKPOINT — see Log format), and read it plus everything after it; dig into older entries only when something refers
   back to them. No checkpoint means read the whole log. A truncated final entry is expected wreckage from an abrupt
   death: preserve it and reconcile from the latest complete entry instead. Cross-check the log's claims against reality
   — VCS state, open PRs, files on disk, running processes. Reality wins over the log when they disagree. Append a
   session-start entry recording that a new session picked up, what state you verified, and any discrepancies (including
   a truncated tail). Then continue from the reconciled state: the log's final "Next:" line is the starting hypothesis,
   not an order — when verification contradicts it, derive the next action from reality and log the correction. If the
   file exists but is clearly not a log in this format, confirm with the user before adopting it; adopt by inserting the
   header at the top of the file, keeping the existing content below it, and appending a session-start entry at the end.
3. **If it does not exist**: consider a mistyped or mis-resolved path before assuming a fresh start — if the user's
   phrasing implies prior work exists, confirm before creating. Otherwise create it with a header (see format below) and
   a first entry recording the goal as you understand it — or that none exists yet — and the initial verified state.
4. In both cases, keep logging for the rest of the session.

A clear goal is not required. Invoking with no task at all, or with only a vague direction, is a first-class use: the
protocol logs state, decisions, and lessons regardless of whether a goal exists yet. Create the file right away; if
there is no goal, say so in the first entry and log what is known — context, constraints, hunches. When a goal emerges,
sharpens, or shifts later, that is itself a loggable event: write an entry for it and update the header to match. With
no goal there may be no real next action; write `Next: await user direction` rather than inventing work.

## Log format

The header states what the file is for and names the resume contract:

```md
# Resume log: <short goal description, or the session's subject if no goal has formed>

Working log for <goal, with a pointer to any plan or spec file — or the area being explored>. This file is the resume
point: a fresh session reads this, cross-checks against reality (VCS state, open PRs, files on disk), and continues.
Reality wins over this log when they disagree.
```

The header is descriptive metadata, not history: unlike entries, update it in place as the goal crystallizes or shifts,
and record the shift itself as an entry.

NOTE: This skill used to be named `scode-fable-resume`, and logs written under it start with `# Fable log:` instead.
Treat those as the same format — resume from them without asking; do not reject them as foreign files.

Entries are appended dated sections, newest last:

```md
## YYYY-MM-DD — <short milestone or checkpoint label>

- <what happened, what was decided and why, what was verified>
- Next: <the immediate next action a resumer should take>
```

Entries are terse but self-contained. The test: a fresh session with no other context must be able to resume from the
log plus repository reality alone. Do not rely on conversation context the resumer will not have. Multiple entries per
day are normal; add an HH:MM to the label when ordering matters for cross-checking against commit or CI timestamps.

A long-running goal can grow the log past what a fresh session can comfortably read, which would make the recovery
mechanism fail exactly when context is tight. The sanctioned escape is a checkpoint entry: a self-contained restatement
of the goal, current verified state, decisions still in force, lessons, and everything the "What to record" section
lists (active skills, work in flight, do-not-fix state). Put the word CHECKPOINT in the entry label — a later session
finds the checkpoint by scanning labels, so the marker must be predictable, not ad hoc. A resumer reads the latest
checkpoint and everything after it; older entries become skippable history. Write one when the log gets long or the user
asks. This does not soften append-only — past entries still never get rewritten.

## When to log

Write an entry at every point where losing the session would lose something expensive to reconstruct:

- After establishing or verifying initial state.
- When the goal emerges, sharpens, or shifts — update the header in the same edit.
- After any decision with reasoning behind it — especially deviations from a plan, rejected alternatives, and scope
  changes. Record the why; the resumer must not re-litigate or silently reverse it.
- When delegating or starting long-running background work: what was launched, where its spec/output lives, and what the
  gate is when it returns.
- When a milestone completes or ships: record durable identifiers (commit hashes, PR numbers/URLs, branch or bookmark
  names) and the overall state (e.g. the current PR stack shape).
- When you learn a lesson the resumer needs (a tool pitfall, a flaky test, a command that must run from a particular
  directory). Prefix with something scannable like NOTE or LESSON.
- Before a risky or long step, so the log reflects intent even if the step kills the session.

Log promptly — write the entry before starting the next long step, not after. Batching entries defeats the purpose: the
session may die at any moment, and an unwritten entry is lost work.

## What to record

Beyond the per-entry content above, keep these visible somewhere in the log (typically the most recent entry, restated
when they change):

- The immediate next action ("Next:"). Every entry ends with one; the final entry's Next line is the resumption point.
- Which skills or protocols are active in the session, so the resumer reloads them before continuing.
- Anything in flight: background tasks, pending CI runs, review passes not yet collected — and what to do with each when
  it lands.
- Known-broken or by-design-red state the resumer might otherwise "fix" (e.g. a stacked PR whose base check is red by
  design).

## What not to do

- Do not rewrite or reorder past entries. The log is append-only history; correct earlier statements with a new entry.
  (The header is exempt — see Log format.)
- Do not trust the log over observed reality when resuming. The log says what a past session believed; verify before
  acting on it.
- Do not pad entries with narration of routine tool use. Log state, decisions, and lessons — not a transcript.
- Do not run two sessions against the same log concurrently. The protocol assumes one writer at a time; resuming implies
  the previous session is dead. Re-read the log's tail before each append — that is how foreign entries get noticed —
  and if entries appear that you did not write, stop and ask the user.
- Do not log secrets — credentials, tokens, private keys, or sensitive command output. Logs get committed and shared.
- Do not commit the log or let it ride along in commits unless the user asks. Where it lives is the user's call; when
  the path is inside a working tree, keep it out of your commits deliberately — auto-tracking VCSs (jj snapshots new
  files on its own) will otherwise sweep it in.

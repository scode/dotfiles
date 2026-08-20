---
name: scode-build-goal
description: Use only when the user explicitly invokes `$scode-build-goal` or `/scode-build-goal`. Takes a goal file path, a log file path, and a natural-language goal; interrogates the user up front to resolve the decisions unattended work will need, then writes a self-resumeable goal file that requires scode-galaxy-brain and is meant to be passed to /goal. `scode-build-goal help` prints a usage TLDR instead.
---

# scode-build-goal

This skill turns a natural-language goal into a goal file that `/goal` (in Claude Code or Codex) can execute unattended.
The output is the goal file itself — this skill does not start the work. The goal file it writes is self-resumeable:
pointing `/goal` at it either starts the work fresh or resumes from the log file, depending on what state exists on
disk. The same goal file can be handed to session after session until the work is done.

NOTE: The whole design assumes unattended execution. The executing agent will not have the user available, so every
decision it would otherwise have to ask about must be resolved before the goal file is written. That is why the up-front
questions section below is not optional politeness — it is the mechanism that makes unattended progress possible.

## Invocation

```
$scode-build-goal <goal-file> <log-file> <goal text...>
```

(equivalently `/scode-build-goal ...`). The first filename is where the goal file gets written; the second is the
working log the executing agent will keep via the `agent-resumeable` skill. The rest is the user's natural-language
statement of what they want built.

If the entire argument string is `help`, answer in chat with a TLDR — the invocation shape, what the two files are for,
and that the output is a file to pass to `/goal` — and stop. Do not create or modify anything.

If a filename or the goal text is missing, ask rather than guessing. Resolve both paths to absolute form before writing
anything; the goal file will be read by sessions with arbitrary working directories, so relative paths inside it are a
resumption bug waiting to happen.

## Up-front questions

Before writing the goal file, close the gaps that would otherwise stall or derail an unattended run. First do enough
reading (the repo, existing docs, prior art) to answer what is answerable without the user — do not ask questions the
codebase already answers. Then ask the user about what remains, batched rather than dribbled:

- Scope boundaries: what is explicitly in, what is explicitly out, and what "done" looks like beyond the mechanical
  PR-stack criterion below.
- Design forks: places where the goal could reasonably be built more than one way — technology choices, API shape, data
  model, user-visible behavior. Ask about the forks you can foresee; for the ones you cannot, the goal file's
  decision-logging rules cover the gap.
- Constraints and tradeoffs: performance vs simplicity, compatibility requirements, anything the user would veto if they
  saw it in review.
- PR shaping: any preference about how the work should be sliced, beyond the default bite-sized-stack rules.

Keep asking until you would bet on an agent completing the goal without needing the user. Then record the answers in the
goal file as decisions already made, so the executing agent inherits them instead of re-deriving or re-litigating them.

## What the goal file must contain

Write the goal file as instructions addressed to the executing agent. It must be self-contained: the executing session
has none of this conversation's context. Include, at minimum:

- **The goal.** The user's intent, sharpened by the Q&A. State the acceptance criteria.
- **Decisions already made.** The Q&A answers, phrased as settled decisions with their reasoning. The executing agent
  must not silently reverse these.
- **Resume protocol.** The first action is to invoke the `agent-resumeable` skill with the log file's absolute path.
  Spell out the semantics even though that skill also enforces them: if the log file already exists, read it and resume
  where the previous session left off — cross-checking the log against reality (VCS state, open PRs) — rather than
  starting over. If it does not exist, this is a fresh start. This is what makes the goal file self-resumeable.
- **Galaxy-brain execution.** State explicitly that the user requires the executing agent to use `$scode-galaxy-brain`
  to achieve the entire goal. Invoke that skill immediately after setting up the resume protocol and keep it active for
  the whole run, including every delegation. Merely reading it for delegation mechanics does not satisfy this
  requirement.
- **PR discipline.** Split the work into a linear stack of reviewable PRs using the `jjstack` skill. Err on the side of
  bite-sized PRs, but do not create churn — code added in one PR and deleted in a later PR of the same stack means the
  stack should have been shaped differently. Restructure the stack instead of stacking a correction on top.
- **Review gate.** Before finishing any PR, use the active `scode-galaxy-brain` routing layer to delegate a review to
  gpt-5.6-sol running the `pre-pr-review-swarm` skill against that PR's changes, and address what it finds before moving
  on. From a Claude session the delegation seam is `codex exec`, e.g.:

  ```
  codex -c model_reasoning_effort=high exec --yolo -m gpt-5.6-sol -o <scratch-file> "<self-contained review prompt>"
  ```

  The review prompt must name the skill, the repo root, and the commit range or bookmark to review; the reviewer has no
  other context.
- **Decision logging.** Log major design decisions in the working log as they happen, with a scannable DECISION label —
  especially decisions that could reasonably have gone another way. The user will later ask for the major decisions in
  order to revisit them, so an unlogged decision is effectively a hidden one.
- **Unattended fallback.** When the agent hits a fork the goal file does not settle, it makes the call, logs it as a
  DECISION with the alternatives considered, and keeps going. Stalling to ask is the one thing it must not do.
- **Done criterion.** The goal is achieved when a linear stack of open PRs — open, not merged — collectively achieves
  the goal. Merging is the user's job.

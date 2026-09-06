---
name: scode-build-goal
description: Use only when the user explicitly invokes `$scode-build-goal` or `/scode-build-goal`. Takes a natural-language goal, optionally prefixed with an `in <dir>` clause (also `into`, `under`); picks mnemonic paths for the goal file and its working log (by default beside the checkout, out of the repo; proposes a temporary location when no good place exists), interrogates the user up front to resolve the decisions unattended work will need, then writes a self-resumeable goal file that requires scode-galaxy-brain and is meant to be passed to /goal. `scode-build-goal help` prints a usage TLDR instead.
---

# scode-build-goal

This skill turns a natural-language goal into a goal file that `/goal` (in Claude Code or Codex) can execute unattended.
The output is the goal file itself — this skill does not start the work. The goal file it writes is self-resumeable: the
user hands `/goal /path/to/foo-goal.md` (or the harness's equivalent) to a fresh session and the work starts if nothing
has happened yet, or resumes from the working log if it has. The user never has to know or say which of the two it is;
the goal file decides from what exists on disk. The same command can be repeated session after session until the work is
done.

NOTE: The whole design assumes unattended execution. The executing agent will not have the user available, so every
decision it would otherwise have to ask about must be resolved before the goal file is written. That is why the up-front
questions section below is not optional politeness — it is the mechanism that makes unattended progress possible.

## Invocation

```
$scode-build-goal <goal text...>
$scode-build-goal in /path/to/dir <goal text...>
```

(equivalently `/scode-build-goal ...`). The argument string is the user's natural-language statement of what they want
built. Two paths come out of it: the goal file, which this skill writes, and the working log, which the executing agent
creates and keeps via the `agent-resumeable` skill. Where those live is this skill's call, per Placing the files below;
the user does not have to name paths.

The second form overrides the directory only. The clause is one of the words `in`, `into`, or `under`, followed by a
path-shaped word: something that starts with `/`, `~`, or `.`, or contains a `/`, and ends at the next whitespace (a
directory with a space in its name cannot be given this way). It has to be the first thing after the skill name. When
those conditions hold, both files go in that directory, with their names still derived the way Placing the files
describes, and the override is honored even when the directory is inside a repository; the user asked. The clause says
where the files go and nothing else: it does not choose which repository the goal is about, which is always the checkout
containing the working directory, so `in ~/git/foo` from a checkout of `bar` files a goal about `bar` inside `foo`. When
the conditions do not hold — `$scode-build-goal into the weeds: untangle the config loader` — there is no clause, and
the whole argument string is goal text. Nothing inside the goal text is ever a placement instruction, however much it
sounds like one: "write the results to /srv/reports" or "keep notes in docs/" are part of the executing agent's brief,
and this skill does not mine the brief for where to put its own files. Resolve `<dir>` to an absolute path (relative to
the working directory, `~` expanded), and if it does not exist or is not a writable directory, say so and ask rather
than creating it or falling back to the defaults.

If the entire argument string is `help`, answer in chat with a TLDR — both invocation shapes, what the two files are
for, where they get put by default, and that the output is a file to pass to `/goal` — and stop. Do not create or modify
anything.

If the goal text is missing, ask rather than guessing.

## Placing the files

The point of picking the paths here is that the user should never have to invent two filenames to get an unattended run
started, and should never be handed an anonymous id as a filename either. Both files get a mnemonic name, and by default
they live outside the repository but next to it, so that the run cannot commit them by accident and a later session (or
the user, scanning a directory listing weeks later) can find them again without remembering anything.

Naming first. Derive a short kebab-case slug from the goal, the kind of thing a person would type from memory:
`build-bar-and-baz`, `farhelm-dist`, `patreon-buddy`. Lead with the project name (`farhelm-dist`, not `dist`) unless the
directory that will hold the files is dedicated to that one project; the default directory never is, since it holds the
checkout's siblings. The project name is the basename of the checkout directory (the outermost one, when checkouts are
nested), not a package name or a remote name, so two agents given the same goal in the same checkout produce the same
slug. Never a UUID, a timestamp, a session id, or anything else the user could not reproduce from the goal itself. The
goal file is `<slug>-goal.md` and the log is `<slug>-goal-log.md`; the shared prefix keeps the pair adjacent in a
listing and makes the log's role obvious.

Where they go. An `in <dir>` clause on the invocation (see Invocation) settles this outright: both files go in that
directory and the list below is skipped. Otherwise, in order of preference:

1. **The parent directory of the repository checkout.** The checkout is the one containing the working directory, and
   the repository the goal text talks about does not change this. Find it by taking `$PWD` as the shell reports it and
   walking up to the nearest directory containing `.git` or `.jj`; do not use `git rev-parse --show-toplevel` or
   `jj root` for this step, since both resolve symlinks and a checkout at `~/git/foo -> /data/repos/foo` should get its
   files in `~/git`, not `/data/repos`. When that checkout is itself nested inside another (a worktree at
   `~/git/foo/.worktrees/bar`, a vendored repo), keep walking up and use the outermost checkout's parent. A working tree
   rooted at the home directory (a dotfiles arrangement) is never counted as a checkout in this section: it neither
   makes `~/git/foo` a nested checkout nor makes `~/git` count as inside a repository, or every invocation on such a
   machine would fall through. So for a goal "add a --verbose flag" in a checkout at `~/git/foo`, the files are
   `~/git/foo-verbose-goal.md` and `~/git/foo-verbose-goal-log.md`, and for a checkout at `~/dotfiles` they are
   `~/dotfiles-verbose-goal.md` and `~/dotfiles-verbose-goal-log.md`; the home directory is a fine parent. This is the
   default whenever the work is in a repository, and it is what to do in the common case. The parent has to be writable,
   and as a sanity check `git rev-parse --show-toplevel` and `jj root` run from it must both fail or report the home
   directory. When any of that fails, fall through.
2. **A proposed temporary location, confirmed by the user.** When there is no repository, or the parent directory is
   unsuitable, propose a path for a fresh directory under the harness's per-session scratch space or the system temp
   directory, name both files there under the same mnemonic names, and present the paths to the user as a proposal.
   Create the directory only once the user accepts. Say plainly that the location is temporary and may not survive a
   reboot or a scratch cleanup; the user is the one who knows a durable place, and the proposal exists so they can name
   it with one line. Do not write anything until they answer.

Whichever case applies, including the `in <dir>` case, the chosen paths go into the up-front questions batch below, so
the user sees them before anything is written and can override with a word. In the default and `in <dir>` cases that is
a statement, not a question: "Goal file at X, log at Y unless you want them elsewhere", adding that the directory is
inside a repository when it is. In the temporary-location case it is a blocking question. There is no case where the
paths are silently decided and only discoverable by reading the goal file.

Every path the goal file mentions — the log file, the repo root, worktrees, scratch directories, anything the Q&A
settles — is written in absolute form. The goal file will be read by sessions with arbitrary working directories, so a
relative path inside it is a resumption bug waiting to happen.

NOTE: This skill writes the goal file only. Never create the log file, not even an empty one or one with a heading: the
goal file's resume protocol treats the log's existence as the signal that a previous session already started, so a log
created here would make the first `/goal` run "resume" from nothing. If a goal or log file with the chosen name already
exists, never overwrite or remove it without asking. Either refine the slug with another word from the goal (not a
counter or a date) so the new pair is distinct, or, when the existing files really are an earlier attempt at the same
goal, ask the user whether to replace them. Replacing means moving both existing files aside under a dated suffix
(`<name>.archive-YYYY-MM-DD.md`), never deleting them, and never leaving the old log in place: a leftover log would make
the new goal's first run resume from the old attempt.

Why the parent directory and not somewhere standardized: there is no standard. On 2026-09-06 each of Codex, Claude Code,
Muse Code, and OpenCode was asked twice, from checkouts under two different parent directories, to keep a goal and a log
out of the repo with no further hint. No two harnesses agreed, and no harness agreed with itself across the two runs.
From `~/git` they reached for `~/.codex/task-state/<slug>/`, `~/.local/state/agent-goals/<repo>/`,
`~/.local/share/<repo>/`, and `~/.local/state/<slug>/`; from a scratch directory Codex and OpenCode went to
`~/.codex/tasks/<slug>/` and `/tmp/opencode/`, while Claude Code chose a sibling directory of the checkout and Muse Code
chose the checkout's parent, which is this skill's convention. Two runs per harness cannot separate run-to-run variance
from sensitivity to the location, so treat the specific paths as illustrative rather than stable. Asked once each, from
`~/git`, with the out-of-repo constraint dropped, every harness put both files inside the repo, at the root or under
`docs/` or a harness-specific hidden directory. A goal file that has to survive across harnesses and sessions cannot
depend on any of that. The parent-of-checkout convention is harness-neutral, already how goal files have been kept next
to repositories by hand on the machine this was developed on, and easy to explain in one sentence.

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
- File placement: the goal and log paths chosen per Placing the files, stated so the user can override them, or put as a
  question when only a temporary location was available.

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
- **Review gate.** Before finishing any PR, use the active `scode-galaxy-brain` skill to delegate a review to
  gpt-5.6-sol running the `pre-pr-review-swarm` skill against that PR's changes, and address what it finds before moving
  on. Do not write a launch command into the goal file; the `scode-harness-shellout` skill's harness files own the
  launch mechanics, and a copy pasted into a goal file drifts away from the guards they carry. The review prompt must
  name the skill, the repo root, and the commit range or bookmark to review; the reviewer has no other context.
- **Resource watchdog.** Immediately after activating galaxy-brain and before the first delegation, the executing agent
  must start a sub agent (or the harness's background-monitor equivalent, whichever delivers notifications back to the
  orchestrating session without being polled) whose only job is to watch memory and disk for the rest of the run and
  alert the orchestrator when either is heading for exhaustion. This is mandatory, not a suggestion: unattended runs fan
  out delegates and worktrees, a Rust worktree costs on the order of 1.5 GB of build output, tests leave temp
  directories behind, and a full disk or an OOM kill has ended real runs mid-implementation with no signal to the
  orchestrator beyond a dead delegate. Spell out in the goal file what the watchdog checks and how often — free space on
  the filesystems holding the repository, any worktrees, the scratch directory, and `/tmp`, plus available memory and
  swap, using whatever the platform provides (`df`, `free` or `/proc/meminfo` on Linux, `vm_stat`/`sysctl` on macOS),
  sampled every minute or so — and the thresholds at which it alerts (a sensible default: under 10% or under 5 GB free
  on any watched filesystem, or under 10% available memory, whichever comes first, with a second alert when the number
  keeps falling). An alert is an instruction to act, not to note: the orchestrator stops launching new delegates,
  removes gated worktrees and build caches it owns, waits for or kills the delegate most likely responsible, and only
  resumes when the watchdog reports headroom. The goal file must also say that a watchdog that dies is restarted, and
  that its running state is part of every handoff note so a resumed session restarts it too.
- **Decision logging.** Log major design decisions in the working log as they happen, with a scannable DECISION label —
  especially decisions that could reasonably have gone another way. The user will later ask for the major decisions in
  order to revisit them, so an unlogged decision is effectively a hidden one.
- **Unattended fallback.** When the agent hits a fork the goal file does not settle, it makes the call, logs it as a
  DECISION with the alternatives considered, and keeps going. Stalling to ask is the one thing it must not do.
- **Done criterion.** The goal is achieved when a linear stack of open PRs — open, not merged — collectively achieves
  the goal. Merging is the user's job.

# scode-build-goal specification

NOTE: This file is binding on the skill's text. It is deliberately sparse: it records only requirements that have been
stated as such, not a description of everything the skill does. Absence of an entry means the behavior is not yet
specified, not that it is unspecified on purpose. When the skill and this file disagree, that is a bug in one of them;
fix the skill or change this file in the same change, never leave them apart.

NOTE: Nothing on the skill's own path reads this file. Its readers are agents changing the skill (see `AGENTS.md`) and
whoever is judging whether a change kept the user-facing behavior intact.

## Triggering

- The skill runs only on an explicit invocation: `$scode-build-goal` or `/scode-build-goal` followed by the argument
  string. A message that merely talks about goal files, unattended runs, or `/goal` does not trigger it.
- The argument string `help`, and nothing else, prints a usage TLDR in chat and stops. That path creates and modifies
  nothing.
- With the argument string missing or empty, the skill asks for the goal text. It never guesses a goal.

## What comes out

- The output is one file, the goal file, written as instructions to the agent that will execute it under `/goal`. The
  skill does not start the work.
- The goal file is self-resumeable from the user's point of view: handing `/goal <absolute path of the goal file>` (or
  the harness's equivalent) to any fresh session either starts the work or resumes it, decided by whether the working
  log exists on disk. The user never has to know or say which of the two applies, and repeats the same command until the
  work is done.
- The skill never creates the working log file, not even empty or with a heading. The goal file's resume protocol treats
  the log's existence as proof that a previous session already started, so a log created by this skill would turn the
  first run into a resume from nothing.
- The skill never overwrites or removes an existing goal or log file without asking. On a name collision it refines the
  slug with another word from the goal, or, when the existing files are an earlier attempt at the same goal, asks the
  user whether to replace them. Replacing moves both existing files aside under a dated suffix; it never deletes them
  and never leaves the old log in place.
- Every path written into the goal file, not only the log path, is absolute. The goal file is read by sessions with
  arbitrary working directories.

## Naming and placement

- The user does not have to name either file. The skill derives a short kebab-case slug from the goal that a person
  could reproduce from the goal itself (`build-bar-and-baz`, `farhelm-dist`). The slug leads with the project name,
  which is the basename of the checkout directory (the outermost one when checkouts are nested), unless the target
  directory is dedicated to that one project; the default directory never is. A UUID, timestamp, session id, or any
  other anonymous identifier is never a filename.
- The goal file is `<slug>-goal.md` and the log is `<slug>-goal-log.md`, always in the same directory as each other.
- The checkout that placement is relative to is the one containing the shell's working directory, found by walking
  `$PWD` up to the nearest `.git` or `.jj` without resolving symlinks. A repository the goal text mentions does not
  change it. A checkout nested inside another checkout (a worktree, a vendored repo) is replaced by the outermost one. A
  working tree rooted at the home directory is never counted as a checkout for this purpose: it neither nests the
  checkouts under it nor makes their parents count as inside a repository.
- The default directory is the parent directory of that checkout, the home directory included, provided the parent is
  writable and is not inside a VCS working tree other than one rooted at the home directory. In the default case the
  skill picks the paths itself and states them; it does not ask.
- When there is no repository, or the parent directory fails those conditions, the skill proposes a temporary location
  (per-session scratch space or a fresh temp directory), says plainly that it is temporary, and creates nothing, the
  directory included, until the user answers.
- The chosen or proposed paths are always shown to the user in the up-front question batch, before anything is written,
  so they can be overridden with one line; a directory inside a repository is called out as such. There is no path the
  user only discovers by reading the goal file.

## The `in <dir>` override

- `$scode-build-goal in <dir> <goal text>` puts both files in `<dir>` and changes nothing else: the names are still
  derived from the goal as above, and the repository the goal is about is still the checkout containing the working
  directory. The override is honored even when `<dir>` is inside a repository.
- The clause is exactly one of the words `in`, `into`, or `under`, followed by a path-shaped word (starting with `/`,
  `~`, or `.`, or containing `/`) that ends at the next whitespace, and it must be the first thing after the skill name.
  Anything else after the skill name means there is no clause and the whole argument string is goal text.
- A path mentioned anywhere inside the goal text is part of the goal and never a placement instruction, however much it
  sounds like one. The skill does not scan the goal text for where to put its own files.
- `<dir>` is resolved to an absolute path against the working directory, with `~` expanded. A `<dir>` that does not
  exist or is not a writable directory is reported and asked about. The skill neither creates it nor silently falls back
  to the default placement.

## Up-front questions

- The skill assumes the executing agent will run unattended. Every decision the run would otherwise have to ask the user
  about is settled before the goal file is written, and the answers are recorded in the goal file as decisions already
  made.
- Questions are batched, not dribbled, and only asked after the skill has read enough of the repository to answer what
  the codebase already settles.
- The batch always presents the review-gate menu, numbered, with these five options in this order and the first marked
  as the default: `pre-pr-review-swarm` on gpt-5.6-sol high; `pre-pr-review-swarm` on fable high; `pre-pr-review-swarm`
  on gpt-6-astra high; an in-harness fresh-context agent at the executing session's own model with a general
  correctness, design, and idiomatic-code charter; a fresh-context agent with that charter on fable or gpt-6-astra at
  high effort, cross-harness when the executing harness cannot reach the model natively. The user's choice is recorded
  in the goal file; the skill never picks a reviewer silently.

## What the goal file requires of the executing agent

- Invoking `agent-resumeable` on the log file's absolute path as its first action, with the resume semantics spelled
  out: an existing log means resume after cross-checking it against reality, a missing log means a fresh start.
- Using `scode-galaxy-brain` for the entire run, activated right after the resume protocol and kept active through every
  delegation. Reading it for mechanics does not satisfy this.
- A resource watchdog for memory and disk, started before the first delegation, restarted if it dies, and carried in
  every handoff note.
- A linear stack of reviewable PRs via `jjstack`, each reviewed before it is finished by a delegated fresh-context run
  of the reviewer the user chose from the menu, stated in the goal file as an explicit demand for its model, effort, and
  skill or charter. The prompt names the skill or carries the full charter, the repo root, the range to review, and the
  findings file, with no launch command copied into the goal file. For the charter options the goal file spells out the
  charter in full: general correctness, design, and idiomatic code, findings to a named file, no edits, no VCS changes.
- Major decisions logged with a scannable DECISION label, including forks the goal file did not settle, which the agent
  resolves on its own rather than stalling to ask.
- The done criterion is a linear stack of open, unmerged PRs that collectively achieve the goal. Merging is the user's.

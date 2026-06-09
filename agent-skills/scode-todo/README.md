# scode-todo

`scode-todo` is a version-controlled TODO list whose primary interaction surface is an AI agent.

The point is not that it has a bunch of advanced task-management features. It does not. The point is that the storage is
simple and customizable, and the interface is whatever agent workflow you want to put in front of it. I made it because
most todo products were opinionated in ways I did not want, while my actual needs were pretty modest.

That means the interaction can stay natural:

```text
Remind me to call the insurance company on 2026-03-10 [admin]
Remind me to replace the bike tire [errands]
T-14 is done
```

And because the data lives in a plain repo, you can also build higher-level agent flows around it, like "every morning
at 6am give me today's TODOs and the next couple of days, limited to [work,house]".

## Installation

- Install this skill however you normally install local agent skills.
- In an empty repository, run `/scode-todo init`.
- Pick a VCS preset: `git`, `graphite`, `sapling`, or `custom`.
- The init step creates `TODO.md`, `DONE.md`, `T_max.md`, `bin/pull`, `bin/push`, `bin/commit`, and tiny `AGENTS.md` /
  `CLAUDE.md` files that tell future agents to keep using `$scode-todo`.

`/scode-todo init` is the one-time setup step. After that, the repository is supposed to be self-describing enough that
future agent sessions keep using the same workflow without another install step.

If you pick `custom`, you provide the exact shell for pull, push, and commit. The commit command has to stage its own
changes and use `$message` or `${message}` where the commit message goes.

If you care about the exact wrapper commands each preset writes, see
[references/vcs-presets.md](references/vcs-presets.md).

## How It Works

The storage format is intentionally boring. Open tasks live in `TODO.md`. Completed tasks get moved to `DONE.md`.
`T_max.md` tracks the largest allocated task ID so new IDs stay unique. If a task needs longer notes, it can have a
matching `T-N.md` file, and the TODO entry gets a `+` marker so the notes file is not hidden state.

The skill gives the agent a deterministic editing workflow on top of those files. It knows how to turn natural-language
requests into timed or untimed tasks, normalize dates, keep tags sorted, move completed entries into `DONE.md`, and keep
the note marker in sync with `T-N.md` files.

It also keeps the VCS behavior repo-local instead of baking assumptions into the prompt. Before edits, the agent runs
`bin/pull`. After edits, it commits through `bin/commit`. `bin/push` is there for the cases where pushing is
appropriate. Those wrappers are the abstraction boundary, which is why the same workflow can sit on top of plain git,
Graphite, Sapling, or whatever custom commands you want.

That is really the whole idea. Keep the task store simple, keep the behavior deterministic, and let the interesting
customization happen in the agent layer. If this is useful to you, the intended move is to crib it and change the parts
you do not like.

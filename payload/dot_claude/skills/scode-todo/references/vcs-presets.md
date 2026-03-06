# VCS presets

NOTE: This is not a general VCS guide. These are the exact three-operation presets that `/scode-todo init` writes today.

## Standard git

This preset keeps the same branch-oriented pull and push flow as the original TODO repo, but the generated `bin/commit`
now stages everything before creating the commit.

- `bin/pull` detects the current branch, runs `git fetch origin "$branch"`, then `git merge --ff-only "origin/$branch"`.
- `bin/push` detects the current branch and runs `git push origin "$branch"`.
- `bin/commit` runs `git add -A` and then `git commit --message="$message"`.

This is the default preset when the repo is just plain git and you are not using a wrapper layer like Graphite or
Sapling.

Sources:

- [git-pull](https://git-scm.com/docs/git-pull)
- [git-add](https://git-scm.com/docs/git-add)
- [git-commit](https://git-scm.com/docs/git-commit)
- [git-push](https://git-scm.com/docs/git-push)

## Graphite

This preset assumes the repository already uses Graphite-tracked branches and Graphite's pull-request workflow.

- `bin/pull` runs `gt sync --all --force`.
- `bin/push` runs `gt submit --publish --no-edit`.
- `bin/commit` runs `gt modify --all --commit --message "$message"`.

Why these commands:

- `gt sync` is Graphite's sync-and-restack entry point, and `--all --force` makes it non-interactive across configured
  trunks.
- `gt submit` is the push operation that updates GitHub and Graphite PR state; `--publish --no-edit` avoids inline
  prompting.
- `gt modify --all --commit --message` stages all changes and creates a new commit instead of amending.

If the repo uses Graphite but you do not want PR-oriented pushes, use `standard git` or `custom` instead.

Sources:

- [Graphite command reference](https://graphite.com/docs/command-reference)

## Sapling

This preset assumes Sapling is the primary CLI for the repo.

- `bin/pull` runs `sl pull --rebase`.
- `bin/push` runs `sl push --rev . --to <bookmark>`.
- `bin/commit` runs `sl commit --addremove --message "$message"`.

`<bookmark>` defaults to `main` during init unless the user asks for another remote bookmark.

Why these commands:

- `sl pull` on its own only downloads commits; `--rebase` makes it act more like a sync-before-edit operation.
- `sl push --rev . --to <bookmark>` pushes the current commit to the chosen remote bookmark.
- `sl commit --addremove --message` rolls the add/remove step into the commit so the wrapper stays a single operation.

If the repo needs `--create`, a non-default destination, or some more specific flow, use `custom`.

Sources:

- [Sapling pull](https://sapling-scm.com/docs/commands/pull/)
- [Sapling push](https://sapling-scm.com/docs/commands/push/)
- [Sapling commit](https://sapling-scm.com/docs/commands/commit/)

## Custom

Ask the user for three shell commands:

1. The command for `bin/pull`
2. The command for `bin/push`
3. The command for `bin/commit`

Rules for custom commit commands:

- The command must include its own staging step. There should still be only three wrapper operations in the repo.
- The command must use `$message` or `${message}` for the commit message slot.
- Preserve the user's shell exactly. Do not try to normalize it into one of the presets after the fact.

---
name: stax
# No description — this skill is invoked directly via slash command, not by natural language matching.
---

# Stax Workflow

Use [stax](https://cesarferreira.com/stax/) (`stax`) for stacked diffs and PR management.

## Sandbox

**ALL `stax` commands MUST be run with `dangerouslyDisableSandbox: true`.** The `stax` CLI writes to `.git/` and makes
network requests to GitHub — the sandbox blocks both.

## Non-interactive mode

Stax commands that prompt for confirmation fail with "not a terminal" in Claude Code. Always pass the appropriate flag
to skip prompts:

- `stax ss --no-prompt` for submit
- `stax rs -f` for sync
- `stax merge -y` for merge
- `stax restack -y` for restack (when conflicts are predicted)

## Critical Rules

1. **NEVER create or update PRs unless the user explicitly asks in that message.** Each PR operation requires fresh
   explicit consent.
2. **ALWAYS provide an explicit branch name to `stax create`** (e.g. `stax create auth-refactor`).
3. **NEVER switch to trunk between stacked branches** — always stack on the current branch.
4. **Do not run `stax` with `--help` or exploratory commands** unless a command fails with an unexpected error.

## Starting a session

Before creating new branches or doing any stax work, **always run `stax rs -f`** to sync with remote. This cleans up
branches whose PRs were merged outside stax (e.g., via the GitHub UI) and prevents stale local state from causing
submission failures.

## Core Operations

### Create a stacked branch

```bash
stax create <branch-name>
```

Creates a new branch stacked on the current one. `stax create` only creates the branch — it does not stage or commit
anything. After creating, stage and commit your changes with `git add` + `git commit`.

If you already have staged changes and want to commit them as part of branch creation:

```bash
stax create <branch-name> -m "commit message"
```

Note: `-m` only commits already-staged changes. Unstaged or untracked files must be `git add`'ed first.

### Submit / make PRs

All changes must be committed before submitting. `stax ss` only pushes and creates/updates PRs — it does not commit.

```bash
stax ss --no-prompt
```

Submits the full stack — creates PRs for branches that don't have one, updates PRs for branches that do. Each branch
becomes its own PR, linked in a stack.

Note: `--no-prompt` creates new PRs as drafts. Before merging with `stax merge`, mark PRs ready with
`gh pr ready <number>`. Draft PRs cannot be merged.

Use `stax cascade` as an alternative that also restacks before submitting.

### Update the current branch

For small amendments to the current commit:

```bash
stax modify
```

Stages all changes and amends them into the current commit. Optionally pass `-m "new message"` to change the commit
message.

For new commits, use `git add` + `git commit` as normal, then `stax ss --no-prompt` to push and update the PR.

### Sync

```bash
stax rs -f
```

Pulls trunk, deletes merged branches. Does not rebase. Use this to clean up after merges.

To also restack (rebase) branches after syncing:

```bash
stax rs -f --restack
```

### Restack

```bash
stax restack
```

Rebases the current branch onto its parent. To restack all branches in the stack:

```bash
stax restack --all
```

Use this after a branch in the stack was force-pushed or rebased on GitHub.

### Merge a branch

```bash
stax merge -y
```

Merges PRs from the bottom of the stack up to (and including) the current branch. Default merge method is squash.

To poll until ready (CI + approval) and then merge:

```bash
stax merge --when-ready -y
```

### Merge the entire stack

```bash
stax merge --all -y
```

Merges every PR in the stack, bottom to top.

## After Submitting

After every successful `stax ss` or `stax cascade`, you **MUST** display the GitHub PR URLs from the output to the user.
This applies to both PR creation and PR updates — always show the links.

## Stack Visualization

- `stax ls` — show the stack with PR and rebase status.
- `stax ll` — show the stack with PR URLs and details.

## Commit Message Style

The first line of the commit message becomes the PR title. Keep it terse:

- "Add retry logic for flaky webhook delivery"
- "Fix: pagination cursor off-by-one"

For the body, explain _why_ the change was made when the reasoning is non-obvious. Err on the side of brevity.

## Recovery

- `stax undo` — undo the last stax operation.
- `stax redo` — reapply an undone operation.

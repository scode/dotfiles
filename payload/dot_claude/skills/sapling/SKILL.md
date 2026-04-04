---
name: sapling
description: Use only when the user explicitly invokes `/sapling` or `$sapling`.
---

# Sapling Stacked Diffs Workflow

Use [Sapling](https://sapling-scm.com/) (`sl`) for stacked diffs and PR management on local Git repositories.

Sapling runs on top of an existing `.git` repo — no `sl clone` or `.sl` repo needed. Just run `sl` commands in any git
working tree.

## Sandbox

**ALL `sl` commands MUST be run with `dangerouslyDisableSandbox: true`.** Sapling writes to `.git/` (via `.sl/` backing
store) and makes network requests to GitHub.

## Critical Rules

1. **NEVER commit or amend unless the user explicitly asks, or asks to create/update/merge a PR for uncommitted local
   changes.** Requests such as "make a PR", "submit this stack", or "merge it" implicitly authorize creating the
   necessary local commit(s) first when the working copy is dirty. Staging files (`sl add`) is fine without asking.
2. **NEVER create or update PRs unless the user explicitly asks.** Each PR operation requires fresh explicit consent.
3. **ALWAYS stack new commits on top of the current working copy parent.** Never navigate to trunk first. Even if
   changes are logically independent, the default is to extend the stack. Only start a new stack off trunk when
   explicitly asked.
4. **Do not run `sl` with `--help` or exploratory commands** unless a command fails with an unexpected error.

## Stack Visualization

Run `sl` (no arguments) or `sl smartlog` to display the commit graph. This shows your stack, the working copy position,
and commit hashes.

Use `sl ssl` (shorthand for `sl smartlog -T {ssl}`) to show the graph with GitHub PR status annotations.

## Core Operations

### New commit on top of the stack

```bash
sl add <files>
sl commit -m "commit message"
```

Sapling has no staging index — `sl commit` includes all pending changes by default. Use `sl add` for new untracked
files, then `sl commit` captures everything. To commit only specific files, pass them directly:

```bash
sl commit -m "message" <file1> <file2>
```

### Amend the current commit

```bash
sl amend
```

Folds all pending changes into the current commit. Automatically rebases descendant commits unless conflicts arise.

To also update the commit message:

```bash
sl amend -m "updated message"
```

To amend only specific files into the current commit:

```bash
sl amend <file1> <file2>
```

### Amend a commit lower in the stack

`sl amend` automatically rebases all descendant commits. When there are no conflicts, restacking is fully automatic — no
manual rebase step needed.

The typical workflow for editing a commit in the middle of the stack:

```bash
sl goto <commit-hash>
# ... make changes ...
sl amend
sl goto top
```

Alternatives that skip the navigation:

- `sl amend --to <commit-hash>` — fold pending changes into a specific commit without navigating to it first.
- `sl absorb` — automatically distribute pending changes to the correct commits in the stack based on which commit last
  touched each edited line.

Both also auto-restack descendants.

### Resolving conflicts during restacking

If `sl amend` (or `sl rebase`) hits a conflict, it stops and leaves conflict markers in the affected files. To resolve:

1. Edit the conflicted files to resolve the markers.
2. Run `sl rebase --continue` to resume restacking.

To abandon instead: `sl rebase --abort`.

### Navigate the stack

- `sl goto <commit-hash>` — jump to a specific commit
- `sl next` / `sl prev` — move up/down one commit in the stack
- `sl next <n>` / `sl prev <n>` — move up/down by n commits
- `sl goto top` — jump to the top of the stack
- `sl goto bottom` — jump to the bottom of the stack

### Rebase

```bash
sl rebase -d <destination>
```

Moves the current commit (and descendants) onto `<destination>`. Use `-s <commit>` to pick a different root.

To restack all commits in the current stack onto their latest parent versions:

```bash
sl rebase --restack
```

### Fold commits together

```bash
sl fold --from <commit-hash>
```

Combines commits from `<commit-hash>` through the current commit into one.

### Split a commit

```bash
sl split
```

Opens an interactive editor to split the current commit's changes into multiple commits.

### Pull / sync with remote

```bash
sl pull
```

Fetches commits from the remote. Does not modify local commits or the working copy.

## Creating PRs

When the user asks to make a PR or submit the stack:

- If the working copy has uncommitted changes, treat that request as implicit authorization to create the necessary
  commit first. Do not stop just because the changes have not been committed yet.
- Keep the commit message concise because its first line becomes the PR title.

```bash
sl pr submit --stack
```

This creates or updates GitHub PRs for all commits in the stack. Each commit becomes its own PR, stacked on GitHub. The
`--stack` flag includes draft ancestor commits.

To submit only the current commit (not the full stack):

```bash
sl pr submit
```

To create PRs as drafts:

```bash
sl pr submit --draft --stack
```

After submitting, display the PR URLs from the output to the user. Use `sl ssl` to see PR status.

### Update existing PRs

After amending commits or adding new commits, re-run `sl pr submit --stack` to push updates to existing PRs and create
PRs for new commits.

## Merging PRs

Sapling does not have a merge command. Use `gh` to squash-merge:

```bash
gh pr checks <number> --watch --fail-fast
gh pr merge <number> --squash
```

Wait for CI to pass before merging. `gh pr checks --watch --fail-fast` blocks until all checks finish, exiting non-zero
on failure.

**After every merge**, pull and rebase before doing anything else — including merging the next PR in the stack:

```bash
sl pull
sl rebase -d main
sl pr submit --stack
```

This must happen between each merge. For a stack of five PRs, merge the bottom one, pull + rebase + resubmit, then merge
the next one, pull + rebase + resubmit, and so on. Without the rebase, the remaining PRs stay parented on the pre-merge
commit and their GitHub diffs will include the already-merged changes.

## Undo

```bash
sl undo
```

Undoes the last Sapling operation. Use `sl redo` to reapply.

## Commit Message Style

The first line of the commit message becomes the PR title. Keep it terse and imperative.

For the body, explain _why_ the change was made when non-obvious. Err on the side of brevity.

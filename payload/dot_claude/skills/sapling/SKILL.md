---
name: sapling
description: Use when the user indicates they want to work with Sapling; after that initial signal, prefer Sapling by default for the rest of the conversation.
---

# Sapling Stacked Diffs Workflow

Use [Sapling](https://sapling-scm.com/) (`sl`) for stacked diffs and PR management on local Git repositories.

Do not invoke this skill just because the repo could support Sapling or because stacked-diff workflow might be useful.
The user needs to give some signal that they want Sapling, such as explicitly invoking `/sapling` or `$sapling`, asking
to use Sapling, referring to `sl`, or otherwise making Sapling the requested tool.

After that first signal, treat Sapling as the default VCS workflow for the rest of the conversation unless the user asks
to switch away from it.

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
3. **ALWAYS stack new commits on top of the current working copy parent while the stack is live.** Never navigate to
   trunk first just to start a new diff. Even if changes are logically independent, the default is to extend the stack.
   Only start a new stack off trunk when explicitly asked.
4. **Do not assume `sl pull` restacks anything.** It fetches remote state, but it does not rewrite local ancestry. If a
   stack has landed ancestors or GitHub shows stale parent diffs, explicitly rebase the open stack onto `main`.
5. **Always submit from the top of the stack.** After rebases or `sl amend --to ...`, the working copy may be left on a
   lower commit. Running `sl pr submit --stack` from there can leave descendant PRs stale.
6. **Do not run `sl` with `--help` or exploratory commands** unless a command fails with an unexpected error.

## Stack Visualization

Run `sl` (no arguments) or `sl smartlog` to display the commit graph. This shows your stack, the working copy position,
and commit hashes.

Use `sl ssl` (shorthand for `sl smartlog -T {ssl}`) to show the graph with GitHub PR status annotations.

Pay attention to landed ancestors. If `sl ssl` shows the bottom open commit parenting on a local commit annotated like
`[Landed as ...]`, the stack is still based on the pre-merge local commit, not the landed `main` commit. That is a sign
you need an explicit rebase before trusting GitHub's diffs.

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

When a stack has been extended on top of a local commit that later landed on `main`, `sl pull` by itself is not enough.
Use one of these explicitly:

```bash
sl pull
sl rebase --restack
```

or, if the root of the open stack is clearly known:

```bash
sl pull
sl rebase -s <bottom-open-commit> -d main
```

The second form is the safer repair when GitHub is still showing old merged changes in the bottom PR.

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

Fetches commits from the remote. It does **not** modify local commits or the working copy.

If the stack includes landed ancestors, or if GitHub is still showing changes from already-merged commits in an open PR,
follow `sl pull` with an explicit restack or rebase. Do not assume Sapling repaired the ancestry just because the remote
state is now current.

## Creating PRs

When the user asks to make a PR or submit the stack:

- If the working copy has uncommitted changes, treat that request as implicit authorization to create the necessary
  commit first. Do not stop just because the changes have not been committed yet.
- Keep the commit message concise because its first line becomes the PR title.
- First move to the top of the stack so the full descendant chain is in scope for submission.

```bash
sl goto top
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
sl goto top
sl pr submit --draft --stack
```

After submitting, display the PR URLs from the output to the user. Use `sl ssl` to see PR status.

### Update existing PRs

After amending commits, adding new commits, or rebasing the stack, re-run submission from the top:

```bash
sl goto top
sl pr submit --stack
```

If the bottom PR still shows diff from an already-merged ancestor after submit, the stack is probably parented on the
pre-merge local commit rather than the landed `main` commit. Fix the ancestry first with `sl rebase --restack` or
`sl rebase -s <bottom-open-commit> -d main`, then submit again from the top.

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
sl goto top
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

---
name: slstack
description: >
  Use only when the user explicitly invokes `slstack` and wants a stacked-diff
  workflow with Sapling in `.git` mode and clean non-overlapping GitHub PRs via
  upstream `ghstack`. After explicit invocation, treat later commit and PR work
  in the same conversation as using this workflow by default. Do not use for
  native Sapling `.sl` repos or for Sapling's overlapping
  `sl pr submit --stack` workflow.
---

# slstack

This skill is for the workflow where Sapling is used locally in Git mode, and upstream `ghstack` is used for GitHub PR
submission and landing.

The split is simple:

- Use `sl` for local stack manipulation.
- Use upstream `ghstack` for GitHub PR state.

Do not use `sl ghstack`. It is deprecated.

Do not use `sl pr submit --stack` unless the user explicitly wants overlapping PRs reviewed via ReviewStack. That is a
different workflow.

## Triggering

This skill should not trigger just because the repo contains Sapling metadata or because stacked diffs might be useful.

Only activate it when the user explicitly invokes `slstack` or otherwise directly asks to use this exact workflow.

After that first explicit invocation, assume that later commit operations and PR operations in the same conversation
should continue using this skill unless the user explicitly switches away from it.

## Preconditions

Before doing anything substantial, verify that the repo is actually in Sapling `.git` mode:

```bash
sl root --dotdir
```

Valid output ends in `.git/sl`.

If it ends in `.sl`, stop treating this as the supported `slstack` workflow. Upstream `ghstack` expects ordinary Git
commits reachable from `HEAD`. Native Sapling `.sl` mode does not give you that model cleanly.

Also verify that `ghstack` is installed:

```bash
which ghstack
```

If PR submission or landing is requested, verify `ghstack` auth is configured. In practice that usually means a working
`~/.ghstackrc` and GitHub credentials. If `ghstack submit` prompts for config, stop and tell the user what is missing.

## Core model

Here's the TLDR of the mechanics:

- Sapling manages a commit stack locally. It makes editing lower commits and restacking descendants much less horrible
  than raw `git rebase -i`.
- `ghstack` does not care about your local branch structure very much. It cares about the stack of commits reachable
  from `HEAD` and the metadata it stores in rewritten commit messages and synthetic remote branches.
- In `.git` mode, Sapling commits are ordinary Git commits. Temporary detached `HEAD` states while navigating the stack
  are expected and are not, by themselves, a problem for `ghstack`.

`ghstack` keeps PR correspondence by rewriting commit messages with trailers and by maintaining synthetic remote
branches like:

- `gh/<user>/<n>/orig`
- `gh/<user>/<n>/head`
- `gh/<user>/<n>/base`

Treat those refs as `ghstack` internals. Do not edit them directly.

## Critical rules

1. Always prefer `sl` for local navigation and stack surgery.
2. Always prefer upstream `ghstack` for PR submission and landing.
3. Never use `sl ghstack`.
4. Never use `sl pr submit --stack` for this workflow.
5. Never edit `ghstack` synthetic branches directly.
6. If you need to recover a PR's local commit, use `ghstack checkout`, not `git checkout` of the `.../head` branch.
7. Expect detached `HEAD` when moving to non-tip commits with Sapling. This is normal. Judge the state by commit
   reachability, not by branch-name anxiety.
8. After a lower PR merges, restack locally before updating the remaining PRs. Do not keep submitting a stale stack and
   hope GitHub figures it out.

## Local stack workflow

### Inspect the stack

Use:

```bash
sl
```

or:

```bash
sl smartlog
```

This is the source of truth for local stack shape.

### Create commits

Use normal Sapling commit flow:

```bash
sl commit -m "message"
```

In `.git` mode, this creates ordinary Git commits.

### Navigate the stack

Use:

```bash
sl prev
sl next
sl goto <rev>
sl goto top
sl goto bottom
```

If `git status --branch` says `HEAD (no branch)` after moving to a lower commit, that is fine. Sapling is still
operating on ordinary Git commits.

### Edit a lower commit

Typical flow:

```bash
sl goto <rev>
# edit files
sl amend
```

If descendants apply cleanly, Sapling restacks them automatically.

If there are conflicts, resolve them and continue:

```bash
sl resolve --mark <file>
sl rebase --continue
```

If the user wants to stay at the top afterward:

```bash
sl goto top
```

## PR workflow

### Submit or update the whole stack

From the top of the stack, use:

```bash
ghstack submit
```

`ghstack` defaults to stack mode and processes the commits reachable from `HEAD`, excluding commits already reachable
from the chosen base branch.

If the user wants only the latest PR URL:

```bash
ghstack submit --short
```

### Submit only part of a stack

Be deliberate here. `ghstack` has stack semantics by default.

Useful cases:

- Submit only `HEAD`:

```bash
ghstack submit --no-stack
```

- Submit a specific range:

```bash
ghstack submit <rev1>..<rev2>
```

Do not improvise range semantics from memory. If the exact range matters, check the current `ghstack` CLI and be
explicit.

### Recover a PR's editable local commit

If the user wants to resume work from an existing ghstack PR:

```bash
ghstack checkout <pr-url-or-number>
```

This checks out the `orig` ref, which is the editable commit history. Do not work directly on the synthetic `head`
branch.

### Land a stack

Use:

```bash
ghstack land <pr-url-or-number>
```

This is the intended merge path for ghstack-managed PRs. Do not assume the normal GitHub merge UI is the right tool for
these PRs.

## After a lower PR lands

This is the part people usually screw up.

When a bottom PR in the stack merges, the remaining PRs are now based on stale history. Fix the local stack first, then
resubmit.

Typical flow:

```bash
git fetch origin
sl rebase -d origin/main
ghstack submit
```

If the repo uses a different trunk branch, substitute the correct branch name.

If Sapling hits conflicts while replaying descendants, resolve them normally. This is not a ghstack-specific failure; it
is ordinary stack replay work.

## When to use raw Git

Use raw Git sparingly here.

Good uses:

- `git fetch`
- `git status`
- `git rev-parse`
- `git log`
- `git rebase` if the user explicitly wants raw Git rather than `sl rebase`

Bad uses:

- Managing the stack by hand with ad hoc branch juggling
- Editing ghstack synthetic refs
- Recovering PR state by checking out `gh/.../head`

The whole point of this workflow is to avoid that nonsense.

## Failure modes

### Repo is in `.sl` mode, not `.git` mode

Symptom:

- `sl root --dotdir` ends in `.sl`

Response:

- Stop.
- Explain that this skill is for Sapling Git mode only.
- Do not try to use upstream `ghstack` as if the local stack were ordinary Git.

### `ghstack submit` says there are no commits to process

Usually this means one of:

- `HEAD` does not contain draft commits above the base branch
- the wrong base branch is being used
- the user is not on the part of the stack they think they are

Check:

```bash
sl
git rev-parse HEAD
ghstack submit --base <expected-branch>
```

### `ghstack` prompts for configuration

Symptom:

- it asks for GitHub enterprise domain or auth details

Response:

- stop and tell the user that `~/.ghstackrc` or equivalent auth setup is missing or incomplete

### A commit is "poisoned"

This usually means someone started editing a synthetic ghstack `head` or `base` branch.

Response:

- stop editing that checkout
- recover with `ghstack checkout <pr>`
- continue from the recovered `orig` commit instead

## Default decision policy

If the user asks for a stacked workflow and does not specify otherwise:

- use Sapling `.git` mode for local commit-stack editing
- use `ghstack submit` for PR creation and update
- use `ghstack land` for merging
- use `sl rebase` and `sl amend` for restacking after lower-commit edits

If the user asks whether Sapling is still useful when `ghstack` is available, the answer is yes: `ghstack` solves the
GitHub PR representation problem, whereas Sapling solves the local stack manipulation problem. They overlap a little,
but they are not substitutes for each other in this workflow.

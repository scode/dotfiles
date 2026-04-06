---
name: jjstack
description: Use Jujutsu for a solo-developer stacked GitHub workflow where each reviewable change is its own commit, bookmark, and pull request; create and update the stack with jj, and use gh only for PR creation and PR base updates.
---

# jjstack

NOTE: This is for a solo developer who owns the whole stack and is willing to force-push bookmark updates. It is not a
team branch-management workflow.

Use `jj` as the source of truth for history. Use `gh` only because GitHub PRs are based on branch names, while `jj`
manages changes and bookmarks.

If the user explicitly asks to use `jj`, `Jujutsu`, or `$jjstack`, treat that as direction to use this workflow even in
a plain Git checkout. Do not silently fall back to a Git-only flow just because the repo has not been bootstrapped yet.
Initialize `jj` in place first unless the user explicitly says not to.

## What to optimize for

Keep one reviewable change per commit. Keep one stable bookmark per reviewable commit. Keep one PR per bookmark.

That mapping is the whole trick:

- commit: the actual change you are reviewing
- bookmark: the stable GitHub branch name that keeps pointing at the latest version of that change
- PR: created once for that bookmark, then updated by pushing the bookmark again

If you let those drift apart, the stack gets annoying fast.

## Expected user commands

This skill should treat a few natural-language requests as specific workflow intents.

An explicit request to use `jj`, `Jujutsu`, or `$jjstack` is itself a workflow instruction. If the current checkout is
not already bootstrapped for `jj`, the default behavior is to run `jj git init --git-repo .` from the repo root and
continue with the `jj` workflow. Only stop to ask instead of initializing when doing so would be risky in context or the
user explicitly asked you not to modify repo setup.

When an agent is driving this workflow from a sandboxed environment, prefer running the actual `jj` and `gh` workflow
commands outside the sandbox by default. In practice `jj` often needs to lock Git metadata such as `packed-refs`, and
`gh` often needs access to the host credential store or keyring. Do not burn time proving the sandbox is broken first if
the task is clearly "use `jj` to manage history and `gh` to manage the PR". Start outside the sandbox unless the
environment already guarantees those operations work inside it.

- "make a PR" or "create a PR" Create a new stacked PR for the current reviewable change. In practice that usually means
  making sure the current change has a stable bookmark, pushing that bookmark, then creating a GitHub PR whose base is
  the bookmark immediately below it in the stack, or `main`/`trunk()` if it is the bottom PR.
- "update the PR" Update the existing PR for the current bookmark. Do not create a new bookmark or a new PR unless the
  user explicitly asks for that. Rewrite or extend the current change, push the same bookmark again, and keep the same
  PR number.
- "rewrite the PR cleanly" Treat this as "update the PR by rewriting commits instead of adding a follow-up fixup
  commit".
- "make the next PR" Create a new reviewable commit on top of the current stack, assign it a new stable bookmark, push
  it, and create a new PR whose base is the bookmark below it.
- "insert a PR below this one" or "insert a PR into the stack" Create a new change in the middle of the stack with
  `jj new --insert-after ...` or an equivalent rebase-based flow, resolve any descendant conflicts, push the affected
  bookmarks, create the new PR, then update downstream GitHub PR bases.
- "restack the stack" Use `jj` to rewrite or rebase the local stack. If bookmark ancestry changes in a way GitHub cares
  about, update the affected PR bases afterwards with `gh pr edit --base ...`.
- "go to PR 45" or "switch to PR 45" Do not interpret the PR number itself as a local jj revision. Look up PR 45 in
  GitHub, find its head branch or bookmark name, and then move to that bookmarked change locally. In practice, that
  usually means using `gh pr view 45` to find the head ref, then using `jj new <bookmark>` or another explicit move to
  that change.

If the user says only "make a PR" and there is already a PR for the current bookmark, push back. In this workflow,
"make/create" means a new stacked PR, while "update" means revise the existing one.

## Preconditions

Before doing anything substantial:

- make sure `jj` is installed and `jj --version` works
- make sure `gh auth status` works
- make sure `jj config get user.name` and `jj config get user.email` are set
- if the user explicitly asked for `jj` and this is still a plain Git checkout, bootstrap `jj` before doing the rest
- if the agent is sandboxed, prefer unsandboxed execution for `jj` and `gh` commands that actually drive the workflow

If `jj` is missing, use the official install instructions first. On systems with a working Rust toolchain,
`cargo install --locked --bin jj jj-cli` is a reasonable generic fallback.

If `jj` `user.name` or `user.email` is missing, first try to copy the existing Git identity into repo-local `jj` config
before making commits:

```bash
jj config set --repo user.name "$(git config user.name)"
jj config set --repo user.email "$(git config user.email)"
```

If the current working-copy commit was already created with the empty identity, update that author too before
committing:

```bash
jj metaedit --update-author @
```

If Git also does not have a usable identity configured, set `jj` explicitly before making commits:

```bash
jj config set --user user.name "Your Name"
jj config set --user user.email "you@example.com"
```

If you do not do this, later commit-creating commands will use an empty identity and those commits will not be pushable
in a normal GitHub workflow. If you need repo-local identity instead of a user-global one, use `--repo` instead of
`--user`.

## Bootstrapping a repo into jj

For a fresh clone, prefer cloning with jj directly:

```bash
jj git clone --colocate <repo-url> <directory>
```

For an existing plain Git checkout, initialize jj in place from the repo root:

```bash
jj git init --git-repo .
```

When the user explicitly asked for `jj`, this initialization step is the default, not an optional suggestion. The
mistake to avoid here is noticing that the repo is plain Git and then drifting into a Git-only PR flow. If `jj` is
installed and the checkout is an ordinary Git repo, initialize `jj` and keep going.

After bootstrapping, start from `trunk()` if jj defined it. That is the safest default because it follows the imported
default branch instead of assuming a local `main` exists. If `trunk()` is not defined, inspect `jj bookmark list` and
branch from the tracked default bookmark you actually have, which is usually `main`, `main@origin`, `master`, or
`master@origin`.

Prefer a colocated repo for GitHub work. If the repo is not colocated, `gh` may need `GIT_DIR=$(jj git root)` as
described in the jj docs:

- `https://docs.jj-vcs.dev/latest/github/`
- `https://docs.jj-vcs.dev/latest/cli-reference/`

## Default rules

- Prefer `jj` commands over `git` for history manipulation.
- When the agent is sandboxed, prefer running real `jj` and `gh` workflow commands outside the sandbox by default.
- Prefer `jj bookmark set` over create/move split-brain. `set` can create or move a bookmark by name.
- Push named bookmarks explicitly with `jj git push --bookmark <name>`. Do not use `--all` unless the user clearly wants
  every local bookmark published.
- Remember that after `jj commit`, the real commit is usually `@-`. `@` is typically the new empty working-copy commit.
- Keep bookmark names stable once a PR exists. Move the bookmark to new commits; do not invent a fresh branch name for
  every revision.
- After you change stack shape, update GitHub PR bases explicitly with `gh pr edit --base ...`. GitHub will not infer jj
  ancestry changes from bookmark movement alone.
- Do not let anything else mutate the same working copy while a `jj` command is running. That includes another `jj`
  process, `git`, an editor auto-save doing broad rewrites, or another shell touching the same files. jj snapshots the
  working copy at command boundaries, so concurrent mutation is an easy way to confuse yourself.

## Starting a new stack

Start from the tracked default branch via `trunk()` unless you already know you need a different base:

```bash
jj new trunk()
```

Make the first change, then commit it:

```bash
jj commit -m "Add first change"
```

Make the second change, then commit it:

```bash
jj commit -m "Add second change"
```

At that point the stack usually looks like:

- `@` = new empty working copy
- `@-` = top reviewable commit
- `@--` = the commit below it

Create stable bookmarks on the reviewable commits, not on the empty working copy:

```bash
jj bookmark set pr/first -r @--
jj bookmark set pr/second -r @-
```

Publish only those bookmarks:

```bash
jj git push --bookmark pr/first --bookmark pr/second
```

Create PRs in order:

```bash
gh pr create --base main --head pr/first --title "Add first change" --body "..."
gh pr create --base pr/first --head pr/second --title "Add second change" --body "..."
```

The base branch of PR N should be the bookmark for PR N-1.

## Updating the top PR

If the project is okay with follow-up commits on the PR:

```bash
jj new pr/top
# edit files
jj commit -m "Address review comments"
jj bookmark set pr/top -r @-
jj git push --bookmark pr/top
```

If the project wants a clean rewritten commit instead:

```bash
jj new pr/top
# edit files
jj squash -m "Original commit title"
jj git push --bookmark pr/top
```

That second flow rewrites the bookmarked commit instead of adding a new one.

## Rewriting an older PR in the middle of the stack

Pick the bookmark for the commit you want to rewrite:

```bash
jj new pr/middle
# edit files
jj squash -m "Original middle commit title"
```

What happens next matters:

- `jj` rewrites the target commit
- descendants get rebased automatically
- descendant commits may become conflicted if they touched the same files

Do not pretend those conflicts are surprising. They are a normal part of changing lower layers in a stack.

If a descendant bookmark becomes conflicted, resolve it commit by commit:

```bash
jj new pr/descendant
# resolve conflict markers or use jj resolve
jj squash -m "Original descendant commit title"
```

Repeat until the stack is clean, then push every bookmark whose commit changed:

```bash
jj git push --bookmark pr/middle --bookmark pr/descendant
```

## Inserting a new PR into the middle of an existing stack

This is the cleanest way to add a new review unit under existing PRs:

```bash
jj new --insert-after pr/previous
# edit files
jj describe -m "Add inserted change"
jj bookmark set pr/inserted -r @
```

That inserts a new commit between `pr/previous` and its descendants, then rebases the descendants on top of the new
commit.

After that:

1. resolve any descendant conflicts
2. push the new bookmark and every descendant bookmark that moved
3. create the new PR
4. update downstream PR bases in GitHub

Example:

```bash
jj git push --bookmark pr/inserted --bookmark pr/downstream
gh pr create --base pr/previous --head pr/inserted --title "Add inserted change" --body "..."
gh pr edit <downstream-pr-number> --base pr/inserted
```

That last `gh pr edit` step is mandatory whenever GitHub's PR parent should change.

## Sanity checks

Use these before and after push operations:

```bash
jj log -r 'bookmarks() | @ | @-'
jj bookmark list
jj git push --bookmark pr/foo --dry-run
```

If you need to see what has changed since the last pushed version of a bookmark, `jj interdiff` is often the right tool:

```bash
jj interdiff --from pr/foo@origin --to pr/foo
```

## Failure modes

Stop and surface the problem instead of improvising if:

- `jj git push` reports a bookmark conflict or stale remote state and a fetch is clearly required
- `gh` auth is missing or the repo path is wrong in a non-colocated workspace
- rewriting a lower commit causes conflicts you cannot resolve confidently
- the requested GitHub PR shape does not match the bookmark ancestry anymore

## Practical notes

- `bookmark-` in jj revset syntax means "the parent of bookmark". It is useful when you only know the child bookmark and
  want the commit directly below it.
- Do not create bookmarks on empty working-copy commits by accident.
- Do not treat GitHub PR branches as the source of truth. The source of truth is the jj graph plus the bookmark names.
- When in doubt, preserve bookmark names and move them to the right commits. That is what keeps existing PRs updating
  instead of multiplying.

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
Initialize `jj` in place first unless the user explicitly says not to. This preference lasts for the rest of the
session, or until the user explicitly says to stop using jjstack.

If a `jj` or `gh` command is blocked by sandboxing, approval policy, missing escalation, or any similar execution
permission issue, stop and ask the user how to proceed. Do not downgrade to a Git-only, manual, or otherwise different
commit/PR flow to get around the block. A user who asked for jjstack asked for this workflow, not merely for "some way
to create a commit or PR".

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
  the bookmark immediately below it in the stack, or `main`/`trunk()` if it is the bottom PR. If the current bookmark
  already has a PR but the working copy contains new intended reviewable work on top of it, default to "make the next
  PR" rather than pushing back. In this workflow that is a very common meaning of "make a PR": turn the new top-of-stack
  work into its own commit, bookmark it, and open a new PR based on the existing one.
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
- "merge the PR", "merge this PR", or a bare "merge it" In a stacked workflow, "merge" means "land this change on the
  default branch", not "run `gh pr merge` on whatever PR is in focus". Those are different operations when the PR's
  GitHub base is another stack bookmark: `gh pr merge` on such a PR folds it into its parent's branch, GitHub closes it
  as "merged", and nothing reaches `main` at all. Before any merge, read the PR's `baseRefName`. If it is the default
  branch, merge it directly, then restack and retarget any child PRs that were based on the merged bookmark per "Landing
  stacked PRs safely" — a direct merge still leaves children pointing at the merged branch. If it is a stack bookmark,
  the request means landing the stack bottom-up per "Landing stacked PRs safely": merge each PR separately in stack
  order, restacking between merges, until the named PR has landed, then restack and retarget any still-open descendants
  above it. When that interpretation implies landing more than one PR, tell the user which PRs will land in what order
  and confirm before the first merge — a bare "merge it" that fans out to several irreversible merges is exactly where a
  misread intent is most expensive. Each PR lands as its own merge on the default branch. Never merge a PR into its
  parent bookmark, and never retarget a stacked PR's base to the default branch so one squash swallows its unmerged
  ancestors — either shortcut collapses review units the stack exists to keep separate, and both need an explicit user
  request. Default to squash merge unless the user explicitly asks for a different merge strategy, and do not use
  `--delete-branch` automatically for a non-top stacked PR.

If the user says only "make a PR" and there is already a PR for the current bookmark, do not push back immediately.
First inspect whether there is new intended work in the working copy that should become the next stacked PR. If yes,
create that next PR on top of the existing one. Only push back when there is already a PR for the current bookmark and
there is no meaningful new work to turn into a new stacked PR. In that narrower case, "make/create" would be ambiguous
or wrong, while "update" means revise the existing PR.

## Preconditions

Before doing anything substantial:

- make sure `jj` is installed and `jj --version` works
- make sure `gh auth status` works
- make sure `jj config get user.name` and `jj config get user.email` are set
- if the user explicitly asked for `jj` and this is still a plain Git checkout, bootstrap `jj` before doing the rest
- if the agent is sandboxed, prefer unsandboxed execution for `jj` and `gh` commands that actually drive the workflow

These are session preconditions, not a toll booth before every command. Once you have confirmed them for the current
repo in the current thread, treat them as known good until something relevant changes: a command fails in a way that
points at auth or config, the repo changes, `jj` is initialized after the check, or you switch to a different checkout.

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

After `jj git init` in an existing working tree, treat author repair as part of bootstrap, not as optional cleanup:

1. copy the Git identity into repo-local `jj` config
2. run `jj metaedit --update-author @` if the working-copy commit already exists with an empty author

If you skip that, the first commit you create from the bootstrapped working copy may inherit a broken identity.

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

Detached Git HEAD is normal in this workflow. Do not treat "not on any branch" as evidence that the checkout is broken.
The jj graph and bookmark names are the source of truth. For `gh` commands, prefer explicit PR numbers, explicit
`--head` values, and `-R owner/repo` rather than relying on Git branch inference.

## Default rules

- Prefer `jj` commands over `git` for history manipulation.
- When the agent is sandboxed, prefer running real `jj` and `gh` workflow commands outside the sandbox by default.
- Prefer `jj bookmark set` over create/move split-brain. `set` can create or move a bookmark by name.
- Do not parallelize `jj` commands in a colocated repo. This includes apparent read-only commands such as `jj status`,
  `jj log`, `jj bookmark list`, `jj diff`, and `jj config get`. jj may import or export Git refs, reset Git HEAD state,
  or create per-repo config as part of commands that look like inspection. Run all `jj` invocations one at a time, and
  do not put them in `multi_tool_use.parallel`.
- Also keep Git and repo-scoped GitHub inspection commands sequential when they may inspect the same Git metadata jj is
  managing. Examples include `git status`, `git show`, `git rev-parse`, `git branch`, and `gh pr view` from inside the
  repo. Parallelizing ordinary file reads such as `sed`, `rg`, `ls`, `nl`, and `wc` is fine as long as no `jj` or Git
  repo-state command is running at the same time.
- Treat workflow mutations as ordered steps in one shared state machine. In particular, `jj commit`, `jj bookmark set`,
  `jj git push`, `gh pr create`, and `gh pr edit` are not independent chores you can fan out. Run them one at a time.
  Re-read state between steps only when the next command depends on uncertain state; do not turn every successful
  command into a separate inspection round trip.
- Before creating a reviewable commit, inspect `jj status` and make sure unrelated working-copy junk is not about to get
  swept in by accident. If needed, commit only the intended paths with `jj commit <paths> -m ...` and leave unrelated
  files in the working copy.
- When the user asks to "make/create a PR" and the current bookmark already has an open PR, treat new intended
  working-copy changes as a request to create the next stacked PR by default. Do not force the user to say "make the
  next PR" unless there is genuine ambiguity about whether the working-copy changes are intended for a new review unit.
- Push named bookmarks explicitly with `jj git push --bookmark <name>`. Do not use `--all` unless the user clearly wants
  every local bookmark published. If the bookmark name contains `/` or you need exact matching instead of pattern
  matching, use `jj git push --bookmark 'exact:<name>'`.
- Do not pass `--allow-new` to `jj git push`. It was deprecated in jj 0.36 and removed in 0.42; on current jj it fails
  immediately with `unexpected argument '--allow-new'`. Explicitly naming the bookmark with `--bookmark`/`exact:` is
  sufficient to push a new bookmark on jj 0.36 and later. Only if `jj --version` reports 0.24–0.35 does a brand-new
  untracked bookmark need `--allow-new` (or `jj bookmark track` on 0.35).
- Never splice arbitrary PR text directly into a shell command. If a PR title or body came from the user, the model, a
  commit message, or `gh pr view --json ...`, materialize it first and pass it to `gh` through a quoted variable for the
  title and `--body-file` for the body. Do not improvise escaping for backticks, `$()`, quotes, or multi-line markdown.
- Default to squash merging GitHub PRs unless the user explicitly asks for a different strategy. This avoids
  repositories that reject merge commits, and it matches the common "one reviewable change lands as one commit on main"
  shape this workflow is usually trying to preserve.
- Before any `gh pr merge`, verify the PR's `baseRefName` is the default branch. Merging a PR whose base is another
  stack bookmark does not land it — it folds the change into the parent PR's branch and closes the PR as "merged" while
  `main` stays untouched. A merge request for such a PR means landing the stack bottom-up; see "Landing stacked PRs
  safely". Do not retarget the PR's base to the default branch just to make this check pass — that squashes its unmerged
  ancestors into one commit, which is the collapse the landing rules forbid.
- Do not delete a merged stack branch while any open GitHub PR still names that branch as its base. GitHub PRs are
  attached to base branch names, not jj ancestry, so a locally-correct jj graph does not protect child PRs from being
  closed or misrepresented if their GitHub base branch disappears too early.
- Remember that after `jj commit`, the real commit is usually `@-`. `@` is typically the new empty working-copy commit.
- That means the sequence matters. If you need `jj bookmark set <name> -r @-`, run it only after `jj commit` has
  finished. Do not start `jj commit` and `jj bookmark set -r @-` concurrently or you can race against the old graph and
  pin the bookmark to the parent change instead of the new reviewable commit.
- Keep bookmark names stable once a PR exists. Move the bookmark to new commits; do not invent a fresh branch name for
  every revision.
- After you change stack shape, update GitHub PR bases explicitly with `gh pr edit --base ...`. GitHub will not infer jj
  ancestry changes from bookmark movement alone.
- Do not let anything else mutate the same working copy while a `jj` command is running. That includes another `jj`
  process, `git`, an editor auto-save doing broad rewrites, or another shell touching the same files. jj snapshots the
  working copy at command boundaries, so concurrent mutation is an easy way to confuse yourself.

## Fast path for the common case

NOTE: This is a tool-call optimization, not permission to parallelize state mutations. `jj commit`, `jj bookmark set`,
`jj git push`, `gh pr create`, `gh pr merge`, and `gh pr edit` are still one ordered state machine. The faster path is
to run the obvious ordered sequence in one shell invocation and let normal command failures stop the sequence, instead
of spending separate tool calls re-reading state after every successful step.

For straightforward happy paths, batch ordered commands with `set -e` or `&&` in one shell invocation when the next
command does not need the model to inspect fresh output. This is still sequential execution. It just avoids paying one
tool call per command for state you already know.

Do not batch an inspection command with the mutation it is supposed to guard. If the working-copy scope is uncertain,
inspect `jj status` first and let the model decide what to include. Once the intended paths are known, use path-limited
commands such as `jj commit README.md -m ...` in the batched sequence.

When a batched sequence uses shell variables to construct bookmark names, validate the variables before creating or
pushing anything. A missing prefix can turn `"$prefix-1"` into a real remote branch named `-1`, and that is needless
cleanup work. Print the exact bookmark names you are about to push, then push those exact names.

Use this fast path when all of these are true:

- the repo is already bootstrapped for `jj` and identity config is known good
- the working copy changes are known and intentionally scoped, usually because you just made them
- you are creating new bookmarks with names you chose for this stack
- you know the base bookmark (`main`, `trunk()`, or the previous stack bookmark)
- you have no reason to suspect stale remote state, existing PRs for the same bookmarks, conflicted descendants, or
  unrelated local changes

In that case, do not burn tool calls on `jj log`, `jj bookmark list`, `jj git push --dry-run`, `gh pr view`, or
post-create PR inspection unless a command fails or the user specifically asked for that detail. Use the command output
you already have: `jj commit` tells you where `@-` landed, `jj bookmark set` tells you what it moved, `jj git push`
reports the pushed refs, and `gh pr create` prints the PR URL.

One check is exempt from that skip-the-reads advice: the pre-merge `baseRefName` verification required by the Default
rules. Never skip it before `gh pr merge`, no matter how well you think you know the stack. It can be batched into the
same shell invocation as the merge, the way the landing snippets do, so it costs nothing extra.

For a fresh two-PR stack where you just edited `README.md`, this is the intended shape:

```bash
jj status
```

If the status output only shows the intended paths:

```bash
set -e
jj commit README.md -m "Add first change"

# edit README.md again
jj commit README.md -m "Add second change"

first_bookmark=pr/first
second_bookmark=pr/second
test -n "$first_bookmark"
test -n "$second_bookmark"
printf 'publishing %s\n' "$first_bookmark" "$second_bookmark"

jj bookmark set "$first_bookmark" -r @--
jj bookmark set "$second_bookmark" -r @-
jj git push --bookmark "exact:$first_bookmark" --bookmark "exact:$second_bookmark"
```

Then create the PRs in order, using the safe title/body-file pattern below. The second PR's base is the first PR's
bookmark:

```bash
set -e
gh pr create -R owner/repo --base main --head pr/first --title "$title" --body-file "$body_file"
gh pr create -R owner/repo --base pr/first --head pr/second --title "$title" --body-file "$body_file"
```

If any command in the fast path fails, stop optimizing and switch to the diagnostic path: inspect `jj status`,
`jj log -r 'bookmarks() | @ | @-'`, `jj bookmark list`, and the relevant `gh pr view` or `gh pr list` output before
trying to repair anything.

When a batched step fails, the steps before it have already mutated state. Resume from the failed step, not from the top
of the batch. Rerunning the whole sequence would replay the earlier mutations: a second `jj commit` creates a spurious
empty commit, and a replayed `jj bookmark set` can pin the bookmark to the wrong revision now that the graph has
advanced.

## Passing PR text to gh safely

This part is easy to get wrong and the failure mode is dumb: the shell eats markdown or code spans, and you silently
publish a mangled PR title or body.

Do not do this with real text:

```bash
gh pr create ... --title "some title with `code`" --body "multi-line body ..."
```

That style is only safe for toy placeholders. It is not a real workflow for arbitrary content.

When you need to create or edit a PR with actual text, use this pattern instead:

```bash
title_file=$(mktemp)
body_file=$(mktemp)

cat >"$title_file" <<'EOF'
feat: add first change
EOF

cat >"$body_file" <<'EOF'
This explains why the change exists.

It may contain `code`, "$vars", $(subshells), single quotes, double quotes, and blank lines.
EOF

title=$(tr -d '\n' <"$title_file")
gh pr create -R owner/repo --base main --head pr/first --title "$title" --body-file "$body_file"

rm -f "$title_file" "$body_file"
```

Use the same pattern for `gh pr edit`:

```bash
title=$(tr -d '\n' <"$title_file")
gh pr edit <pr-number> -R owner/repo --title "$title" --body-file "$body_file"
```

The important parts are:

- use a single-quoted heredoc delimiter like `<<'EOF'` when writing literal text
- keep the title in a quoted variable, not inline in the command text
- pass the body with `--body-file`, not `--body`
- if you fetched existing text from GitHub and want to preserve it exactly, write it to files first and then reuse the
  same file-based flow

Do not ad-lib shell escaping here. Use the file-and-variable pattern every time.

## Starting a new stack

Start from the tracked default branch via `trunk()` unless you already know you need a different base:

```bash
jj new 'trunk()'
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

Create stable bookmarks on the reviewable commits, not on the empty working copy, then publish only those bookmarks:

```bash
set -e
first_bookmark=pr/first
second_bookmark=pr/second
test -n "$first_bookmark"
test -n "$second_bookmark"
printf 'publishing %s\n' "$first_bookmark" "$second_bookmark"
jj bookmark set "$first_bookmark" -r @--
jj bookmark set "$second_bookmark" -r @-
jj git push --bookmark "exact:$first_bookmark" --bookmark "exact:$second_bookmark"
```

Create PRs in order:

Prepare the correct `title` and `body_file` for each PR using the safe pattern above, then run:

```bash
set -e
gh pr create -R owner/repo --base main --head pr/first --title "$title" --body-file "$body_file"
gh pr create -R owner/repo --base pr/first --head pr/second --title "$title" --body-file "$body_file"
```

The base branch of PR N should be the bookmark for PR N-1.

## Updating the top PR

If the project is okay with follow-up commits on the PR:

```bash
jj new pr/top
# edit files
jj commit -m "Address review comments"
jj bookmark set pr/top -r @-
jj git push --bookmark 'exact:pr/top'
```

Those are intentionally separate commands. Do not batch them into one parallel tool call. `jj bookmark set pr/top -r @-`
is only correct after the commit has completed and the working-copy graph has advanced.

If the project wants a clean rewritten commit instead:

```bash
jj new pr/top
# edit files
jj squash -m "Original commit title"
jj git push --bookmark 'exact:pr/top'
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
jj git push --bookmark 'exact:pr/middle' --bookmark 'exact:pr/descendant'
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

Assuming `title` and `body_file` were prepared with the safe pattern above:

```bash
jj git push --bookmark 'exact:pr/inserted' --bookmark 'exact:pr/downstream'
gh pr create -R owner/repo --base pr/previous --head pr/inserted --title "$title" --body-file "$body_file"
gh pr edit -R owner/repo <downstream-pr-number> --base pr/inserted
```

That last `gh pr edit` step is mandatory whenever GitHub's PR parent should change.

## Sanity checks

Use these before and after push operations:

```bash
jj log -r 'bookmarks() | @ | @-'
jj bookmark list
jj git push --bookmark 'exact:pr/foo' --dry-run
```

If you need to see what has changed since the last pushed version of a bookmark, `jj interdiff` is often the right tool:

```bash
jj interdiff --from pr/foo@origin --to pr/foo
```

## Failure modes

Stop and surface the problem instead of improvising if:

- a `jj` or `gh` workflow command is blocked by sandboxing, approval policy, missing escalation, or another execution
  permission issue
- `jj git push` reports a bookmark conflict or stale remote state and a fetch is clearly required
- `gh` auth is missing or the repo path is wrong in a non-colocated workspace
- rewriting a lower commit causes conflicts you cannot resolve confidently
- the requested GitHub PR shape does not match the bookmark ancestry anymore
- you already ran commit/bookmark/push in parallel and are no longer sure which commit the bookmark points at. In that
  case, stop, inspect `jj log` and `jj bookmark list`, repair the bookmark target explicitly, and only then push or
  create/edit the PR.

If a `jj` or `gh` command fails with `unexpected argument` or an unknown-flag error, treat it as version drift between
the installed tool and whatever produced the flag — this skill's examples, or your own priors. Check `--help` for the
installed version and drop or replace the flag, rather than retrying the same command or inventing a workaround. Do not
assume the examples here match the installed version's CLI surface; jj in particular removes deprecated flags
(`--allow-new` is the known case).

## Landing stacked PRs safely

NOTE: Landing always proceeds bottom-up, and every individual `gh pr merge` must target a PR whose `baseRefName` is the
default branch at the moment you merge it. Never merge a PR into another stack bookmark. GitHub will happily do it — the
PR closes as "merged", the parent PR absorbs the child's changes, and nothing lands on `main` — which is exactly the
wrong-way merge this section exists to prevent. If the user asks to merge a PR that sits above unmerged parents, that is
a request to land the stack up to and including that PR — one merge per PR, in stack order, restacking between merges —
not to merge it where it currently points, and not to retarget its base to the default branch and squash the whole stack
into one commit. Each PR was made a separate review unit on purpose; landing must preserve one landed commit per PR
unless the user explicitly asks to collapse them. When more than one PR will land, tell the user which PRs will land in
what order and confirm before the first merge; that requirement applies no matter which section routed you here.
Restacking is an after-every-merge obligation, not just preparation for the next merge: when the named PR has landed and
open descendants remain above it, rebase them onto the landed result, push every descendant bookmark the rebase moved —
not just the lowest — and retarget the lowest remaining PR's base to the default branch before calling the job done. The
base guards in the snippets below are this rule in executable form; keep them. Derive the default branch once per
landing instead of hardcoding `main` — a repo whose default is `master` can even contain a stack branch literally named
`main`, which would turn a hardcoded guard into an authorization for the wrong-way merge:

```bash
default_branch=$(gh repo view -R "$repo" --json defaultBranchRef --jq .defaultBranchRef.name)
```

The `jj`-side commands below still say `main` for readability; substitute the repo's actual default branch and local
bookmark names when they differ.

NOTE: Do not use `gh pr merge --delete-branch` for a non-top stacked PR. If a child PR still has the parent bookmark as
its GitHub `baseRefName`, deleting the parent branch can cause GitHub to close the child PR before you can edit its
base. `jj` ancestry may still be correct locally, but GitHub is tracking branch names here.

When the merge request names a PR whose base is a stack bookmark and you do not already hold the stack mapping, first
walk the ancestor chain upward to the stack bottom: read the PR's `baseRefName`, find the same-repo open PR whose
`headRefName` is that branch, and repeat until a base equals the default branch. The resulting bottom-up list of
PR-number/bookmark pairs is the landing plan: it is what you show the user for confirmation before the first merge, and
each entry's bookmark is the `$parent_bookmark`/`$child_bookmark` input for the merge steps below:

```bash
set -e
default_branch=$(gh repo view -R "$repo" --json defaultBranchRef --jq .defaultBranchRef.name)
pr="$target_pr"
chain=""
while :; do
  read -r state base head <<EOF
$(gh pr view "$pr" -R "$repo" --json state,baseRefName,headRefName \
  --jq '[.state, .baseRefName, .headRefName] | @tsv')
EOF
  test "$state" = OPEN || { echo "PR #$pr is ${state:-unreadable}; refusing to plan a landing through it" >&2; exit 1; }
  case " $chain" in *" #$pr:"*) echo "base/head cycle at PR #$pr" >&2; exit 1 ;; esac
  chain="#$pr:$head $chain"
  test "$base" = "$default_branch" && break
  parents=$(gh pr list -R "$repo" --state open --head "$base" \
    --json number,isCrossRepository --jq '[.[] | select(.isCrossRepository | not) | .number] | join(" ")')
  set -- $parents
  test $# -eq 1 || { echo "expected exactly one same-repo open PR with head $base, got: '$parents'" >&2; exit 1; }
  pr=$1
done
printf 'landing order (bottom-up), PR:bookmark: %s\n' "$chain"
```

The `isCrossRepository` filter matters: `gh pr list --head` matches branch names across forks, and silently picking a
fork's PR would produce a landing plan through a stranger's branch. If the loop errors — a closed PR mid-chain, no
same-repo parent, more than one candidate parent, or a cycle — stop and show the user what you found instead of
improvising; the stack may have been partially landed, retargeted by another actor, or wired up wrong.

Before merging a stacked PR, identify downstream PRs whose base is the bookmark you are about to land:

```bash
gh pr list -R "$repo" --state open --base "$parent_bookmark" \
  --json number,title,headRefName,baseRefName
```

If you just created the whole stack in this same session, you already have the PR-to-bookmark mapping. In that case,
skip repeated `gh pr list --base ...` checks while landing the known stack. Keep the guard when you inherited the stack,
when another actor may have edited PR bases, when the local notes are incomplete, or before deleting any remote branch.
Same-session mapping lets you skip rediscovering which PR number belongs to which bookmark. It does not replace checking
the state, base, and head of the specific PR you are about to merge.

Merge the parent with the full guard. The guard is the same whether or not downstream PRs exist; the downstream answer
only decides whether branch deletion is allowed later. `--match-head-commit` protects the head SHA only — it does not
protect the base, which is what the `test` lines are for:

```bash
set -e
default_branch=$(gh repo view -R "$repo" --json defaultBranchRef --jq .defaultBranchRef.name)
read -r state base head head_sha <<EOF
$(gh pr view "$parent_pr" -R "$repo" \
  --json state,baseRefName,headRefName,headRefOid \
  --jq '[.state, .baseRefName, .headRefName, .headRefOid] | @tsv')
EOF
test "$state" = OPEN
test "$base" = "$default_branch"
test "$head" = "$parent_bookmark"
test -n "$head_sha" || { echo "parent PR metadata did not match expected state/base/head" >&2; exit 1; }
gh pr merge "$parent_pr" -R "$repo" --squash --match-head-commit "$head_sha"
test "$(gh pr view "$parent_pr" -R "$repo" --json state --jq .state)" = MERGED
```

That final `MERGED` check is not paranoia: on repos with a merge queue or auto-merge, `gh pr merge` can queue the merge
and exit zero without anything having landed. If the check fails while the PR sits queued, wait and re-read instead of
proceeding — fetching and restacking now would rebase descendants onto a stale default branch and retarget the next PR
before its parent actually landed.

Then fetch the landed state and move local `main` to the remote result:

```bash
jj git fetch --remote origin
jj bookmark set main -r main@origin
```

Restack downstream bookmarks in stack order. For a two-PR stack where the child should now target `main`, the concrete
sequence is:

```bash
set -e
jj rebase -s "$child_bookmark" -d main
jj git push --bookmark "exact:$child_bookmark"
gh pr edit "$child_pr" -R "$repo" --base main
gh pr view "$child_pr" -R "$repo" --json state,baseRefName,headRefName,headRefOid,mergeStateStatus,statusCheckRollup
```

After creating a PR, pushing a bookmark, or editing a PR base, use
`gh pr view --json
state,baseRefName,headRefName,headRefOid,mergeStateStatus,statusCheckRollup` for CI and mergeability
waits. GitHub can briefly report no checks for a just-pushed or just-retargeted PR before Actions has attached the new
runs. Treat "no checks" as pending if checks were expected; wait briefly and re-read PR metadata instead of treating it
as success. Use workflow logs only when checks fail or get stuck.

For a larger stack, repeat the same rebase, push, and `gh pr edit --base ...` process from bottom to top. The new base
is either the newly-landed branch such as `main`, or the bookmark for the nearest parent PR that is still open. If this
is not a known stack you just created, re-read GitHub state between steps instead of assuming a prior local graph
observation still describes the PRs.

Before deleting the old merged branch, query GitHub again:

```bash
gh pr list -R "$repo" --state open --base "$parent_bookmark" \
  --json number,title,headRefName,baseRefName
```

If that returns any PRs, do not delete the branch yet. It is safe to delete the old merged branch only after no open PR
uses it as `baseRefName`, or after every downstream PR base has already been moved away from it and verified open.

If deletion is desired after that guard passes, delete the remote ref explicitly:

```bash
gh api -X DELETE "repos/$repo/git/refs/heads/$parent_bookmark"
```

After every base edit, confirm the downstream PR is still open and that checks have started or are already green:

```bash
gh pr view "$child_pr" -R "$repo" --json state,baseRefName,headRefName,headRefOid,mergeStateStatus,statusCheckRollup
```

If a child PR is already closed because its base branch was deleted too early, do not assume `gh pr reopen` repaired the
stack. Verify the PR state from GitHub. If GitHub still reports `CLOSED`, the practical recovery is usually to rebase
the child bookmark onto updated `main`, push it, and open a replacement PR.

If GitHub reports `DIRTY` or `CONFLICTING` after a base edit or force-push but the local jj stack looks clean, do not
immediately invent a new branch. Wait once and re-read the PR metadata; GitHub mergeability can lag behind rewritten
refs. If it still reports dirty, verify the local diff/ancestry for the specific child, rewrite that bookmark onto the
current intended base, push that bookmark explicitly, and recheck the PR metadata before merging. This is a recovery
path for GitHub's view getting stale or confused, not a replacement for resolving real conflicts.

For the common two-PR landing case, keep the same safety constraints but avoid extra reads. If you already know the
parent/child mapping because you just created the stack, you can skip rediscovering the stack, but still verify the
specific PR you are about to merge:

```bash
set -e
default_branch=$(gh repo view -R "$repo" --json defaultBranchRef --jq .defaultBranchRef.name)
read -r state base head head_sha <<EOF
$(gh pr view "$parent_pr" -R "$repo" \
  --json state,baseRefName,headRefName,headRefOid \
  --jq '[.state, .baseRefName, .headRefName, .headRefOid] | @tsv')
EOF
test "$state" = OPEN
test "$base" = "$default_branch"
test "$head" = "$parent_bookmark"
test -n "$head_sha" || { echo "parent PR metadata did not match expected state/base/head" >&2; exit 1; }
gh pr merge "$parent_pr" -R "$repo" --squash --match-head-commit "$head_sha"
test "$(gh pr view "$parent_pr" -R "$repo" --json state --jq .state)" = MERGED

jj git fetch --remote origin
jj bookmark set main -r main@origin
jj rebase -s "$child_bookmark" -d main
jj git push --bookmark "exact:$child_bookmark"
gh pr edit "$child_pr" -R "$repo" --base "$default_branch"

read -r state base head head_sha <<EOF
$(gh pr view "$child_pr" -R "$repo" \
  --json state,baseRefName,headRefName,headRefOid \
  --jq '[.state, .baseRefName, .headRefName, .headRefOid] | @tsv')
EOF
test "$state" = OPEN
test "$base" = "$default_branch"
test "$head" = "$child_bookmark"
test -n "$head_sha" || { echo "child PR metadata did not match expected state/base/head" >&2; exit 1; }
gh pr merge "$child_pr" -R "$repo" --squash --match-head-commit "$head_sha"
test "$(gh pr view "$child_pr" -R "$repo" --json state --jq .state)" = MERGED
```

You do not need to rediscover the child PR after `gh pr edit` when the command succeeds and the next operation is an
explicit merge of that same PR. You still need to verify that the PR you are about to merge is open and has the expected
base and head. If `gh pr edit` fails, if the metadata check returns nothing, or if GitHub reports the child PR as
closed, fall back to the full inspection flow above.

The child guard in that snippet checks PR shape only. On repos where checks are expected to run, do not merge the child
in the same breath as the base edit: split the batch after `gh pr edit`, wait for checks per the "no checks means
pending" guidance above, and only then run the verify-and-merge block. The same split applies on merge-queue repos: the
`MERGED` check after the parent merge will fail while the PR sits in the queue — that is the guard working, not an
error. Wait for the queue to land it, then resume from the fetch.

For a known stack with more than two PRs, use the same loop in stack order. After each landing, rebase the remaining
local stack onto the landed base while preserving the relative order of the still-open PRs, and push every bookmark the
rebase moved — GitHub tracks branch heads, so an unpushed descendant keeps showing a stale diff against its rewritten
parent. Then edit only the next bottom PR's GitHub base to the newly landed branch; descendants should keep their bases
on their immediate parent bookmarks unless that parent changed. Do not rediscover the child PR with `gh pr list` on
every iteration when the mapping is already in hand, but do verify the state, base, and head of the specific PR before
merging it.

## After merging a PR

This section applies after the final merge of a landing, when no open stacked PRs remain above what you merged. If open
descendants remain, keep following the restack flow in "Landing stacked PRs safely" instead — the `jj rebase -s @` below
moves only the working-copy commit, and running it mid-landing strands `@` away from the stack it was sitting on.

After GitHub merges the PR, bring the local jj view back into sync before doing anything else:

```bash
jj git fetch --remote origin
jj bookmark set main -r main@origin
jj rebase -s @ -d main
```

The first command imports the new remote state. The second makes the local `main` bookmark match the merged remote
bookmark. The third moves the usually-empty working-copy commit on top of the new `main` so the checkout is coherent
again.

If the repository's default branch is not named `main`, use the correct local and remote bookmark names instead of
blindly pasting `main`.

## Practical notes

- `bookmark-` in jj revset syntax means "the parent of bookmark". It is useful when you only know the child bookmark and
  want the commit directly below it.
- Do not create bookmarks on empty working-copy commits by accident.
- Do not treat GitHub PR branches as the source of truth. The source of truth is the jj graph plus the bookmark names.
- When in doubt, preserve bookmark names and move them to the right commits. That is what keeps existing PRs updating
  instead of multiplying.

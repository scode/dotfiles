---
name: jjstack
description: Use Jujutsu for a solo-developer stacked GitHub workflow where each reviewable change is its own commit, bookmark, and pull request; create and update the stack with jj, and use gh only for PR creation (always as a draft), publishing, and PR base updates (via gh api, never gh pr edit).
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

One more default rides on that mapping: a PR is a draft from creation until the user says to publish it, or until
landing it needs a CI verdict. Repos increasingly gate CI on draft status, so that ready means "run CI", and a stack is
rewritten and re-pushed many times before anyone wants a verdict. One stack observed on 2026-09-02 produced 73 CI runs
of about ten jobs each, mostly for commits nobody looked at. Draft status is the switch that stops that; this skill sets
it and only the user, or a landing that is about to merge the PR, flips it. See "Draft and ready" under Default rules
for the full rule.

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

- "make a PR" or "create a PR" Create a new stacked PR, as a draft, for the current reviewable change. In practice that
  usually means making sure the current change has a stable bookmark, pushing that bookmark, then creating a GitHub PR
  whose base is the bookmark immediately below it in the stack, or `main`/`trunk()` if it is the bottom PR. If the
  current bookmark already has a PR but the working copy contains new intended reviewable work on top of it, default to
  "make the next PR" rather than pushing back. In this workflow that is a very common meaning of "make a PR": turn the
  new top-of-stack work into its own commit, bookmark it, and open a new PR based on the existing one.
- "update the PR" Update the existing PR for the current bookmark. Do not create a new bookmark or a new PR unless the
  user explicitly asks for that. Rewrite or extend the current change, push the same bookmark again, and keep the same
  PR number.
- "rewrite the PR cleanly" Treat this as "update the PR by rewriting commits instead of adding a follow-up fixup
  commit".
- "make the next PR" Create a new reviewable commit on top of the current stack, assign it a new stable bookmark, push
  it, and create a new PR, as a draft, whose base is the bookmark below it.
- "insert a PR below this one" or "insert a PR into the stack" Create a new change in the middle of the stack with `jj
  new --insert-after ...` or an equivalent rebase-based flow, resolve any descendant conflicts, push the inserted
  bookmark, create the new PR as a draft, retarget the immediate downstream GitHub PR, then push every rewritten
  descendant bookmark.
- "publish the PRs" or "mark the PRs ready" Run `gh pr ready <n>` for each named or implied PR, bottom-up, and report
  which PRs were marked ready. A named subset ("publish 329 and up") means exactly those PRs and no others; an
  unqualified request means the current stack. A request to publish is not a request to merge, and it does not change
  how later PRs get created: the next "make a PR" is still a draft.
- "run CI on the stack", "run CI on PR N", "kick off CI", or "make sure CI passes" Trigger one run per named or implied
  PR without changing any PR's draft state: `gh workflow run <workflow> --ref <bookmark>` on a repo whose workflow
  allows `workflow_dispatch`, and report the run URLs. This is a one-off. Marking the PR ready would also start a run,
  but it changes future behavior too: every later rebase and push of a ready PR starts CI again, which is the cost the
  draft default exists to avoid. A request for CI is not a request to publish. If no workflow can be dispatched, say so
  and ask; do not mark the PR ready to get the run.
- "unpublish", "back to draft", or "convert to draft" Run `gh pr ready --undo <n>`. Rarely needed, and only ever on
  request; the skill never converts a ready PR back to a draft on its own.
- "restack the stack" Use `jj` to rewrite or rebase the local stack. If bookmark ancestry changes in a way GitHub cares
  about, update the affected PR bases afterwards with the REST base edit from "Editing PR metadata".
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
  above it. Always say which PRs will land in what order before the first merge. Stop and wait for confirmation only
  when landing them means going wider than the user asked: a request naming one PR that turns out to sit above unmerged
  parents also lands those parents, and choosing to land work the user did not name is theirs to make. When the request
  already covers the whole set — "merge the stack", "land all of it" — the plan is an announcement, not a question, so
  state the order and proceed. Bottom-up is the only valid order, so there is nothing for the user to decide about it.
  Do not re-ask partway through a landing they already approved. Each PR lands as its own merge on the default branch.
  Never merge a PR into its parent bookmark, and never retarget a stacked PR's base to the default branch so one squash
  swallows its unmerged ancestors — either shortcut collapses review units the stack exists to keep separate, and both
  need an explicit user request. Default to squash merge unless the user explicitly asks for a different merge strategy,
  and do not use `--delete-branch` automatically for a non-top stacked PR.
- "fast path merge the stack", "fast-path merge", or "merge it, fast path" Land the whole stack with one squash commit
  on the default branch per PR without waiting for CI on any of them, using the bundled `fast-land` script. See "Fast
  path merge". A landing request that explicitly says not to wait for CI ("land the stack, don't wait on CI", "skip CI
  and merge it") is the same request. Either wording is a one-shot instruction for that single merge request. Never
  carry it forward: the next merge request in the same session, however it is phrased, gets the normal landing flow
  unless it says "fast path" or "don't wait for CI" again.

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

If `jj` is missing, use the official install instructions first. On systems with a working Rust toolchain, `cargo
install --locked --bin jj jj-cli` is a reasonable generic fallback.

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
  `jj git push`, `gh pr create`, and the REST PR edit are not independent chores you can fan out. Run them one at a
  time. Re-read state between steps only when the next command depends on uncertain state; do not turn every successful
  command into a separate inspection round trip.
- Before creating a reviewable commit, inspect `jj status` and make sure unrelated working-copy junk is not about to get
  swept in by accident. If needed, commit only the intended paths with `jj commit <paths> -m ...` and leave unrelated
  files in the working copy.
- When the user asks to "make/create a PR" and the current bookmark already has an open PR, treat new intended
  working-copy changes as a request to create the next stacked PR by default. Do not force the user to say "make the
  next PR" unless there is genuine ambiguity about whether the working-copy changes are intended for a new review unit.
- Push named bookmarks explicitly with `jj git push --bookmark <name>`. Do not use `--all` unless the user clearly wants
  every local bookmark pushed. If the bookmark name contains `/` or you need exact matching instead of pattern matching,
  use `jj git push --bookmark 'exact:<name>'`.
- Do not pass `--allow-new` to `jj git push`. It was deprecated in jj 0.36 and removed in 0.42; on current jj it fails
  immediately with `unexpected argument '--allow-new'`. Explicitly naming the bookmark with `--bookmark`/`exact:` is
  sufficient to push a new bookmark on jj 0.36 and later. Only if `jj --version` reports 0.24–0.35 does a brand-new
  untracked bookmark need `--allow-new` (or `jj bookmark track` on 0.35).
- Draft and ready. Every `gh pr create` this skill issues passes `--draft`: a new stack, the next PR on top, a PR
  inserted into the middle. There is no "small PR" exception and no per-repo detection; the rule is unconditional so
  that repos whose CI gates on draft status get the saving without the agent having to notice. A PR leaves draft only
  for two reasons: the user asked ("publish the PRs" under Expected user commands), or a landing flow is about to merge
  it and needs it ready (see "Landing stacked PRs safely" and "Fast path merge", which each mark one PR ready at the
  last responsible moment and leave the PRs above it as drafts). Never mark a PR ready on your own initiative: not
  because CI would be useful, not because a review round finished, not because you are about to hand off. "Publish" in
  this file means this flip and nothing else; pushing a bookmark is not publishing, and running CI is not publishing
  either. Only "publish" or "mark ready" means `gh pr ready`. Any request for CI, from the user or from your own wish
  for a verdict, is a one-off: run the workflow without changing the PR's state, because a ready PR keeps starting runs
  on every later rebase and push. On a repo whose workflow supports `workflow_dispatch`, that looks like `gh workflow
  run ci.yml --ref <bookmark>`; the workflow filename is the repo's, not necessarily `ci.yml`, so read
  `.github/workflows/` to find the right one. If no workflow can be dispatched, say so and ask rather than publishing
  the PR to get a run.
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
  ancestors into one commit, which is the collapse the landing rules forbid. (Retargeting after the parent has reached
  `MERGED`, as "Fast path merge" does, is a different situation: there is no unmerged ancestor left to swallow.)
- Do not delete a merged stack branch while any open GitHub PR still names that branch as its base. GitHub PRs are
  attached to base branch names, not jj ancestry, so a locally-correct jj graph does not protect child PRs from being
  closed or misrepresented if their GitHub base branch disappears too early.
- Remember that after `jj commit`, the real commit is usually `@-`. `@` is typically the new empty working-copy commit.
- That means the sequence matters. If you need `jj bookmark set <name> -r @-`, run it only after `jj commit` has
  finished. Do not start `jj commit` and `jj bookmark set -r @-` concurrently or you can race against the old graph and
  pin the bookmark to the parent change instead of the new reviewable commit.
- Keep bookmark names stable once a PR exists. Move the bookmark to new commits; do not invent a fresh branch name for
  every revision.
- After you change stack shape, update GitHub PR bases explicitly with the REST base edit from "Editing PR metadata".
  GitHub will not infer jj ancestry changes from bookmark movement alone. Never use `gh pr edit` for this; it is broken
  on the `gh` versions distros ship.
- Do not let anything else mutate the same working copy while a `jj` command is running. That includes another `jj`
  process, `git`, an editor auto-save doing broad rewrites, or another shell touching the same files. jj snapshots the
  working copy at command boundaries, so concurrent mutation is an easy way to confuse yourself.

## Fast path for the common case

NOTE: This is a tool-call optimization, not permission to parallelize state mutations. `jj commit`, `jj bookmark set`,
`jj git push`, `gh pr create`, `gh pr merge`, and the REST PR edit are still one ordered state machine. The faster path
is to run the obvious ordered sequence in one shell invocation and let normal command failures stop the sequence,
instead of spending separate tool calls re-reading state after every successful step.

For straightforward happy paths, batch ordered commands with `&&` chaining or a per-command `|| exit 1` in one shell
invocation when the next command does not need the model to inspect fresh output. This is still sequential execution. It
just avoids paying one tool call per command for state you already know.

NOTE: Never rely on `set -e` to stop a batched sequence. Agent harnesses commonly run the tool command via `eval` in a
non-final position of an `&&` list, and bash ignores errexit for any command in that position — including everything the
`eval` executes. The result is the worst failure shape: `set -e` shows as enabled in `$-` and `SHELLOPTS`, yet a failed
guard falls through and the mutation it was supposed to stop runs anyway. This fails open and has caused a real
unguarded `gh pr merge`. Give every command in a batch its own failure path (`&&` chaining or `|| exit 1`); an explicit
`exit` does propagate out of the wrapper correctly.

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
reports the pushed refs, and `gh pr create` prints the PR URL. The PR it prints is a draft, and it stays one until the
user publishes it or a landing marks it ready; do not follow the create with a `gh pr ready`.

One check is exempt from that skip-the-reads advice: the pre-merge `baseRefName` verification required by the Default
rules. Never skip it before `gh pr merge`, no matter how well you think you know the stack. It can be batched into the
same shell invocation as the merge, the way the landing snippets do, so it costs nothing extra.

For a fresh two-PR stack where you just edited `README.md`, this is the intended shape:

```bash
jj status
```

If the status output only shows the intended paths:

```bash
jj commit README.md -m "Add first change" || exit 1

# edit README.md again
jj commit README.md -m "Add second change" || exit 1

first_bookmark=pr/first
second_bookmark=pr/second
test -n "$first_bookmark" || exit 1
test -n "$second_bookmark" || exit 1
printf 'pushing %s\n' "$first_bookmark" "$second_bookmark"

jj bookmark set "$first_bookmark" -r @-- || exit 1
jj bookmark set "$second_bookmark" -r @- || exit 1
jj git push --bookmark "exact:$first_bookmark" --bookmark "exact:$second_bookmark" || exit 1
```

Then create the PRs in order, using the safe title/body-file pattern below. The second PR's base is the first PR's
bookmark:

```bash
gh pr create -R owner/repo --draft --base main --head pr/first --title "$title" --body-file "$body_file" || exit 1
gh pr create -R owner/repo --draft --base pr/first --head pr/second --title "$title" --body-file "$body_file" || exit 1
```

If any command in the fast path fails, stop optimizing and switch to the diagnostic path: inspect `jj status`, `jj log
-r 'bookmarks() | @ | @-'`, `jj bookmark list`, and the relevant `gh pr view` or `gh pr list` output before trying to
repair anything.

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
gh pr create -R owner/repo --draft --base main --head pr/first --title "$title" --body-file "$body_file"

rm -f "$title_file" "$body_file"
```

Editing an existing PR uses the same files, but not `gh pr edit`. See "Editing PR metadata" below for why; the shape is:

```bash
title=$(tr -d '\n' <"$title_file")
gh api -X PATCH "repos/owner/repo/pulls/<pr-number>" -f "title=$title" -F "body=@$body_file" --jq .number
```

The important parts are:

- use a single-quoted heredoc delimiter like `<<'EOF'` when writing literal text
- keep the title in a quoted variable, not inline in the command text
- pass the body from a file: `--body-file` for `gh pr create`, `-F "body=@$body_file"` for the REST edit. The `@path`
  form makes `gh api` read the file and send its contents as the field value, so the body never passes through shell
  parsing
- if you fetched existing text from GitHub and want to preserve it exactly, write it to files first and then reuse the
  same file-based flow

Do not ad-lib shell escaping here. Use the file-and-variable pattern every time.

## Editing PR metadata

Every edit to an existing PR's title, body, or base goes through the REST endpoint, never through `gh pr edit`:

```bash
# Retarget the base. --jq keeps the output to the one field you want to confirm.
gh api -X PATCH "repos/$repo/pulls/$pr" -f "base=$new_base" --jq .base.ref

# Reword. Title from a variable, body from a file (see "Passing PR text to gh safely").
gh api -X PATCH "repos/$repo/pulls/$pr" -f "title=$title" -F "body=@$body_file" --jq .number
```

NOTE: This is a SPEC.md requirement of the dotfiles repo this skill ships from, not a style preference. `gh pr edit` is
broken on every `gh` release before 2.82.1, with any flags: it reads the PR through GraphQL first, that query still asks
for the classic-Projects `projectCards` field GitHub has removed, and the command exits 1 before writing anything.
Distro packages sit on exactly those versions (Ubuntu 24.04 ships 2.45.0), and this skill has to work on such hosts
without asking the user to install a newer `gh`. The REST call is the same operation without the GraphQL read, and it
works on old and new `gh` alike. Do not "fix" this by upgrading `gh` or by retrying `gh pr edit`; the failure is
deterministic. The error text to recognize, should `gh pr edit` sneak in anyway, is:

```
GraphQL: Projects (classic) is being deprecated in favor of the new Projects experience, ...
(repository.pullRequest.projectCards)
```

Two things to know about the REST form. `gh api` prints the full PR object on success, so always pass `--jq` to keep the
output small, and read the field back with `gh pr view --json baseRefName` (or `title`, `body`) when a later step
depends on it, exactly as the landing flows below already do. A base change is rejected with HTTP 422 when the named
branch does not exist on GitHub, which is the failure you get if you retarget before pushing the new parent bookmark.

A base retarget can also 422 with `A pull request already exists for base '<default>' and head '<bookmark>'` when it
races GitHub's own retarget. On a repo with "automatically delete head branches" on, merging the parent deletes its
branch, and GitHub then moves every PR based on that branch to the parent's base on its own, asynchronously. Land the
PATCH inside that window and GitHub reports the collision as an error even though the PR ends up exactly where you
wanted it. (A plain no-op retarget, where the base already matches and nothing is in flight, returns 0.) So a failed
retarget is not automatically fatal: re-read `baseRefName`, and if it already equals the target, carry on. The landing
snippets below encode that as `|| test "$(gh pr view ... --jq .baseRefName)" = "$target" || exit 1`. Observed on
2026-09-13 during a fast path landing on scode/repotesting.

## Passing commit messages to jj safely

The same problem exists on the `jj` side, with an extra trap: `jj commit`, `jj squash`, and `jj describe` take `-m`, but
none of them takes `-F` or `--file`, and only `jj describe` has `--stdin`. An agent that reaches for the Git habit `jj
commit -F msg.txt` gets `unexpected argument '-F' found` (observed on jj 0.44). The inline `-m "Add first change"`
examples elsewhere in this file are fine for one-line placeholders; a real multi-paragraph message with backticks,
quotes, or `$` goes through a file.

Write the message with a single-quoted heredoc, then hand it to `-m` through command substitution. This works
identically for all three commands, so there is one pattern to remember:

```bash
msg_file=$(mktemp)
cat >"$msg_file" <<'EOF'
feat: add first change

This explains why the change exists. It may contain `code`, "$vars",
$(subshells), single quotes, double quotes, and blank lines.
EOF

jj commit README.md -m "$(cat "$msg_file")" || exit 1
rm -f "$msg_file"
```

`$(cat ...)` strips trailing newlines and nothing else, and the double quotes keep the rest intact. Use the same shape
for `jj squash -m "$(cat "$msg_file")"` when rewriting a PR's commit and `jj describe -m "$(cat "$msg_file")"` when
describing the working copy. `jj describe --stdin <"$msg_file"` is an equivalent for that one command; do not assume the
other two accept it.

The PR title and body usually come from the same text. Derive them from the message file rather than retyping: the first
line is the title, and everything after the blank line is the body:

```bash
title=$(head -n 1 "$msg_file")
body_file=$(mktemp)
tail -n +3 "$msg_file" >"$body_file"
```

Then continue with the `gh pr create` pattern above.

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

Create stable bookmarks on the reviewable commits, not on the empty working copy, then push only those bookmarks:

```bash
first_bookmark=pr/first
second_bookmark=pr/second
test -n "$first_bookmark" || exit 1
test -n "$second_bookmark" || exit 1
printf 'pushing %s\n' "$first_bookmark" "$second_bookmark"
jj bookmark set "$first_bookmark" -r @-- || exit 1
jj bookmark set "$second_bookmark" -r @- || exit 1
jj git push --bookmark "exact:$first_bookmark" --bookmark "exact:$second_bookmark" || exit 1
```

Create PRs in order:

Prepare the correct `title` and `body_file` for each PR using the safe pattern above, then run:

```bash
gh pr create -R owner/repo --draft --base main --head pr/first --title "$title" --body-file "$body_file" || exit 1
gh pr create -R owner/repo --draft --base pr/first --head pr/second --title "$title" --body-file "$body_file" || exit 1
```

The base branch of PR N should be the bookmark for PR N-1. Both PRs are drafts; leave them that way until the user asks
to publish.

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
2. push the new bookmark
3. create the new PR
4. update the immediate downstream PR's base in GitHub
5. push every descendant bookmark that moved

Example:

Assuming `title` and `body_file` were prepared with the safe pattern above:

```bash
jj git push --bookmark 'exact:pr/inserted'
gh pr create -R owner/repo --draft --base pr/previous --head pr/inserted --title "$title" --body-file "$body_file"
gh api -X PATCH "repos/owner/repo/pulls/<downstream-pr-number>" -f base=pr/inserted --jq .base.ref
jj git push --bookmark 'exact:pr/downstream'
```

The base edit step is mandatory whenever GitHub's PR parent should change, and it must happen before pushing the
rewritten downstream head. A base-dependent required check must see the final base when the push starts workflows for
the new head commit.

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
- any command fails with the `(repository.pullRequest.projectCards)` GraphQL error. That means a `gh pr edit` ran on a
  pre-2.82.1 `gh`. Nothing was written. Redo the edit with the REST call from "Editing PR metadata"; do not retry the
  same command and do not stop to upgrade `gh`.

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
unless the user explicitly asks to collapse them. Say which PRs will land in what order before the first merge, no
matter which section routed you here. Whether to wait for an answer is the narrower question decided by the "merge the
PR" bullet under Expected user commands: wait when the landing set is wider than the request, and otherwise just state
the plan and go. Restacking is an after-every-merge obligation, not just preparation for the next merge: when the named
PR has landed and open descendants remain above it, rebase them onto the landed result, retarget the lowest remaining
PR's base to the default branch, then push every descendant bookmark the rebase moved — not just the lowest, and never a
landed one (see "The push set" below). Finish the landing by deleting landed bookmarks after no open PR depends on them.
The base guards in the snippets below are this rule in executable form; keep them — including the per-guard `|| exit`
failure paths, which exist because `set -e` is silently inert under agent shell wrappers (see the errexit NOTE in "Fast
path for the common case") and a guard without its own exit fails open. Derive the default branch once per landing
instead of hardcoding `main` — a repo whose default is `master` can even contain a stack branch literally named `main`,
which would turn a hardcoded guard into an authorization for the wrong-way merge. Note that `gh repo view` takes the
repository as a positional argument, not via `-R`:

```bash
default_branch=$(gh repo view "$repo" --json defaultBranchRef --jq .defaultBranchRef.name)
```

The `jj`-side commands below still say `main` for readability; substitute the repo's actual default branch and local
bookmark names when they differ.

NOTE: Do not use `gh pr merge --delete-branch` for a non-top stacked PR. If a child PR still has the parent bookmark as
its GitHub `baseRefName`, deleting the parent branch can cause GitHub to close the child PR before you can edit its
base. `jj` ancestry may still be correct locally, but GitHub is tracking branch names here.

When the merge request names a PR whose base is a stack bookmark and you do not already hold the stack mapping, first
walk the ancestor chain upward to the stack bottom: read the PR's `baseRefName`, find the same-repo open PR whose
`headRefName` is that branch, and repeat until a base equals the default branch. The resulting bottom-up list of
PR-number/bookmark pairs is the landing plan: it is what you show the user before the first merge, and each entry's
bookmark is the `$parent_bookmark`/`$child_bookmark` input for the merge steps below. Reaching this walk at all means
the request named a PR above unmerged parents, so the chain is wider than what was asked for and the plan needs an
answer before you merge:

```bash
default_branch=$(gh repo view "$repo" --json defaultBranchRef --jq .defaultBranchRef.name) || exit 1
test -n "$default_branch" || { echo "could not resolve default branch for $repo" >&2; exit 1; }
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

Mark the parent ready, then wait for its checks. PRs are drafts by default (see "Draft and ready" under Default rules),
and on repos that gate CI on draft status the verdict the merge is about to depend on cannot exist while the PR is a
draft. So the landing flips exactly one PR at a time, immediately before waiting on that PR's checks, and leaves every
PR above it as a draft: the restack after this merge may rewrite them, and a draft rewrite is free where a ready rewrite
starts a CI run nobody will read. `gh pr ready` is idempotent on an already-ready PR (it warns and exits zero), so the
step is safe when the user published earlier. Guard before flipping: un-drafting is a visible change to the PR, and a
wrong `$parent_pr` (a mid-stack PR on an inherited stack, a closed PR) must fail here rather than after the flip:

```bash
default_branch=$(gh repo view "$repo" --json defaultBranchRef --jq .defaultBranchRef.name) || exit 1
test -n "$default_branch" || { echo "could not resolve default branch for $repo" >&2; exit 1; }
read -r state base head <<EOF
$(gh pr view "$parent_pr" -R "$repo" --json state,baseRefName,headRefName \
  --jq '[.state, .baseRefName, .headRefName] | @tsv')
EOF
test "$state" = OPEN || { echo "PR #$parent_pr is ${state:-unreadable}, not OPEN" >&2; exit 1; }
test "$base" = "$default_branch" \
  || { echo "PR #$parent_pr base is '$base', expected '$default_branch'" >&2; exit 1; }
test "$head" = "$parent_bookmark" \
  || { echo "PR #$parent_pr head is '$head', expected '$parent_bookmark'" >&2; exit 1; }
gh pr ready "$parent_pr" -R "$repo" || exit 1
gh pr view "$parent_pr" -R "$repo" --json state,baseRefName,headRefName,headRefOid,mergeStateStatus,statusCheckRollup
```

Then wait for checks per the "no checks means pending" guidance below. One caveat on that wait: a `pull_request`
workflow only fires on the ready event if the repo lists `ready_for_review` under `types:` (GitHub's default set is
opened, synchronize, and reopened). A repo that skips jobs on drafts but never added that type starts nothing when the
PR is marked ready, and since the push landed while it was a draft, no run exists at all. If no checks attach within the
usual grace period, read `.github/workflows/` for `ready_for_review`; when it is missing, trigger a run explicitly with
`gh workflow run <workflow> --ref <bookmark>` where the workflow allows `workflow_dispatch`, or re-push the bookmark to
fire `synchronize`, and note the gap to the user. Only once the checks are green, merge the parent with the full guard.
The guard is the same whether or not downstream PRs exist; the downstream answer only decides whether branch deletion is
allowed later. `--match-head-commit` protects the head SHA only — it does not protect the base, which is what the `test`
lines are for:

```bash
default_branch=$(gh repo view "$repo" --json defaultBranchRef --jq .defaultBranchRef.name) || exit 1
test -n "$default_branch" || { echo "could not resolve default branch for $repo" >&2; exit 1; }

# The guards and the merge are one retryable unit. GitHub can briefly retain
# stale branch-policy state after the preceding base edit, ready transition, or
# descendant push. Refresh every guard before retrying; never reuse a head or
# base read from the failed attempt.
# jj is the source of truth for which commit should land.
expected_sha=$(jj log -r "$parent_bookmark" --no-graph -T commit_id) || exit 1
merged=no
for attempt in 1 2 3 4 5; do
  read -r state base head head_sha mergeable merge_status <<EOF
$(gh pr view "$parent_pr" -R "$repo" \
  --json state,baseRefName,headRefName,headRefOid,mergeable,mergeStateStatus,statusCheckRollup \
  --jq '[.state, .baseRefName, .headRefName, .headRefOid, .mergeable, .mergeStateStatus] | @tsv')
EOF
  test "$state" = OPEN || { echo "PR #$parent_pr is ${state:-unreadable}, not OPEN" >&2; exit 1; }
  test "$base" = "$default_branch" \
    || { echo "PR #$parent_pr base is '$base', expected '$default_branch'" >&2; exit 1; }
  test "$head" = "$parent_bookmark" \
    || { echo "PR #$parent_pr head is '$head', expected '$parent_bookmark'" >&2; exit 1; }
  test -n "$head_sha" || { echo "no head sha for PR #$parent_pr" >&2; exit 1; }

  # GitHub's headRefOid lags a push by a second or two, and mergeable read in
  # that window describes the old head. Wait for the pushed commit first.
  if test "$head_sha" != "$expected_sha"; then
    test "$attempt" -lt 5 \
      || { echo "PR #$parent_pr head is $head_sha, local $parent_bookmark is $expected_sha" >&2; exit 1; }
    sleep 3
    continue
  fi

  # A non-CLEAN merge state is a real gate or unresolved GitHub state, not
  # evidence that the stack is blocked by the transient policy race. The
  # normal check-wait guidance above handles required checks; optional checks
  # must not be promoted to required blockers here.
  if test "$mergeable" = UNKNOWN || test "$merge_status" = UNKNOWN; then
    test "$attempt" -lt 5 || { echo "PR #$parent_pr mergeability stayed UNKNOWN" >&2; exit 1; }
    sleep 3
    continue
  fi
  test "$mergeable" = MERGEABLE && test "$merge_status" = CLEAN \
    || { echo "PR #$parent_pr mergeable=$mergeable mergeStateStatus=$merge_status" >&2; exit 1; }

  merge_err=$(gh pr merge "$parent_pr" -R "$repo" --squash --match-head-commit "$head_sha" 2>&1) \
    && { merged=yes; break; }
  case "$merge_err" in
    *"base branch policy prohibits the merge"*|*"Base branch was modified"*)
      test "$attempt" -lt 5 \
        || { echo "PR #$parent_pr still rejected after transient-policy retries: $merge_err" >&2; exit 1; }
      echo "PR #$parent_pr: GitHub policy state has not converged; refreshing and retrying ($attempt)" >&2
      sleep 3
      ;;
    *) echo "$merge_err" >&2; exit 1 ;;
  esac
done
test "$merged" = yes || exit 1
test "$(gh pr view "$parent_pr" -R "$repo" --json state --jq .state)" = MERGED \
  || { echo "PR #$parent_pr did not reach MERGED (merge queue?)" >&2; exit 1; }
```

The bounded retry above is specifically for GitHub convergence after a base edit, ready transition, or rewritten head.
The generic `base branch policy prohibits the merge` response is not enough evidence that the stack is blocked. On each
retry, re-read the state, default-branch base, head branch, exact head SHA, `mergeable`, `mergeStateStatus`, and status
contexts. Retry only while the exact guards still match, mergeability is converging, and no check is pending or failed.
This does not wait for CI, start another workflow, rebase, push, or use `--admin`. If a required check is pending or
failed, or the guards change, stop and report that actual condition. Record the first rejected attempt in your landing
notes as a transient GitHub-policy rejection; do not mark the stack blocked unless the bounded retry is exhausted or a
real guard/check failure remains. This behavior was observed during a 19-PR landing where GitHub returned that exact
policy error immediately after a base retarget and push, then reported `CLEAN`, `MERGEABLE`, and a successful required
check seconds later. See the incident note: https://gist.github.com/scode/33ba018851f1e1acfc8dde4e302a86fb

The same loop waits for GitHub's `headRefOid` to equal the commit the local bookmark points at. Right after a push,
GitHub can report the previous head for a second or two; a 32-PR landing on 2026-10-11 hit exactly that (`4fa48fa` shown
while the branch was already `7a7e9f9`). Mergeability read in that window describes the old head, so the head has to
converge before `mergeable` means anything. A mismatch that outlasts the retries is real: unpushed local work, or
someone else pushed the branch. The block assumes the bookmark exists locally, which holds for any stack this skill
built; for an inherited stack, fetch and track its bookmarks first.

For the two-PR shortcut below, use the same retryable verify-and-merge block rather than the one-shot `gh pr merge`
command. The shortcut may omit repeated stack discovery, but it must not omit the convergence retry after the child base
edit and rewritten push.

After creating a PR, pushing a bookmark, marking a PR ready, or editing a PR base, use `gh pr view --json
state,baseRefName,headRefName,headRefOid,mergeStateStatus,statusCheckRollup` for CI and mergeability waits. GitHub can
briefly report no checks for a just-pushed or just-retargeted PR before Actions has attached the new runs. Treat "no
checks" as pending if checks were expected; wait briefly and re-read PR metadata instead of treating it as success. Use
workflow logs only when checks fail or get stuck.

That final `MERGED` check is not paranoia: on repos with a merge queue or auto-merge, `gh pr merge` can queue the merge
and exit zero without anything having landed. If the check fails while the PR sits queued, wait and re-read instead of
proceeding — fetching and restacking now would rebase descendants onto a stale default branch and retarget the next PR
before its parent actually landed.

Then fetch the landed state and move the local default-branch bookmark to the remote result:

```bash
jj git fetch --remote origin || exit 1
jj bookmark set "$default_branch" -r "$default_branch@origin" || exit 1
```

Restack downstream bookmarks in stack order. For a two-PR stack where the child should now target the default branch,
the concrete sequence is:

```bash
jj rebase -s "$child_bookmark" -d "$default_branch" || exit 1
gh api -X PATCH "repos/$repo/pulls/$child_pr" -f "base=$default_branch" --jq .base.ref \
  || test "$(gh pr view "$child_pr" -R "$repo" --json baseRefName --jq .baseRefName)" = "$default_branch" \
  || exit 1
jj git push --bookmark "exact:$child_bookmark" || exit 1
gh pr ready "$child_pr" -R "$repo" || exit 1
gh pr view "$child_pr" -R "$repo" --json state,baseRefName,headRefName,headRefOid,mergeStateStatus,statusCheckRollup \
  || exit 1
```

The `gh pr ready` comes after the push on purpose: the push of the rewritten head lands while the PR is still a draft,
so a draft-gated CI does not start a run for it, and the ready event immediately afterwards starts the one run whose
verdict the next merge will wait on (subject to the `ready_for_review` trigger caveat above). Retarget before pushing
the rewritten bookmark. The push starts workflows for the new head, and a base-dependent required check such as
`require-main-base` must evaluate that head against its final base. Pushing first can attach a failing run created from
the old base to the new head. Rerunning that job does not fix it because GitHub reuses the original event payload.

If the wrong-order race has already happened, do not rerun the failed job. Set the correct base, inspect the workflow's
event triggers, and fire a fresh event that the base-dependent workflow actually handles. If it handles `pull_request:
edited`, a reversible PR body edit (the REST body edit from "Editing PR metadata") can create that evaluation; preserve
and restore the exact body with the file-based safe-text procedure above. Do not assume every pull-request workflow
handles body edits.

After creating a PR, pushing a bookmark, marking a PR ready, or editing a PR base, use `gh pr view --json
state,baseRefName,headRefName,headRefOid,mergeStateStatus,statusCheckRollup` for CI and mergeability waits. GitHub can
briefly report no checks for a just-pushed or just-retargeted PR before Actions has attached the new runs. Treat "no
checks" as pending if checks were expected; wait briefly and re-read PR metadata instead of treating it as success. Use
workflow logs only when checks fail or get stuck.

For a larger stack, repeat the same rebase, REST base edit, and push process from bottom to top. The new base is either
the newly-landed branch such as `main`, or the bookmark for the nearest parent PR that is still open. Retarget the
lowest remaining PR before pushing any rewritten descendant bookmarks; descendants keep their immediate parent bases
unless that parent changed. Mark only the lowest remaining PR ready after that push; the descendants above it stay
drafts until their own turn comes. If this is not a known stack you just created, re-read GitHub state between steps
instead of assuming a prior local graph observation still describes the PRs.

### Clean up landed stack branches

After the final merge and restack of every landing, delete each merged stack branch remotely and its matching local
bookmark unless the user explicitly asked to keep branches. This also applies when the landing stops below open
descendants: retarget and push those descendants first, then clean up every landed branch they no longer use. Preserve
the PR number, bookmark, and exact head SHA from each pre-merge guard until its cleanup finishes; if that evidence is
lost across an interruption, stop instead of guessing which current ref is safe to delete.

Before each deletion, use the exact `$head_sha` captured by the guarded pre-merge check as `$merged_head_sha`. Verify
that the PR came from this repository, was based on the default branch, reached `MERGED`, and still names the expected
bookmark and head commit. Then verify that both the remote ref and local bookmark still point to that exact commit and
that no open PR uses the bookmark as either a base or head:

```bash
read -r state base head current_pr_head is_cross_repo <<EOF
$(gh pr view "$merged_pr" -R "$repo" \
  --json state,baseRefName,headRefName,headRefOid,isCrossRepository \
  --jq '[.state, .baseRefName, .headRefName, .headRefOid, .isCrossRepository] | @tsv')
EOF
test "$state" = MERGED \
  || { echo "PR #$merged_pr is ${state:-unreadable}, not MERGED" >&2; exit 1; }
test "$base" = "$default_branch" \
  || { echo "PR #$merged_pr base is '$base', expected '$default_branch'" >&2; exit 1; }
test "$head" = "$merged_bookmark" \
  || { echo "PR #$merged_pr head is '$head', expected '$merged_bookmark'" >&2; exit 1; }
test "$current_pr_head" = "$merged_head_sha" \
  || { echo "PR #$merged_pr head changed from '$merged_head_sha' to '$current_pr_head'" >&2; exit 1; }
test "$is_cross_repo" = false \
  || { echo "PR #$merged_pr is cross-repository; refusing base-repository cleanup" >&2; exit 1; }

remote_sha=$(gh api "repos/$repo/git/ref/heads/$merged_bookmark" --jq .object.sha) \
  || { echo "could not read remote ref '$merged_bookmark'; refusing cleanup" >&2; exit 1; }
test "$remote_sha" = "$merged_head_sha" \
  || { echo "remote '$merged_bookmark' moved to '$remote_sha'" >&2; exit 1; }

local_sha=$(jj log -r "$merged_bookmark" --no-graph -T 'commit_id ++ "\n"') || exit 1
test "$local_sha" = "$merged_head_sha" \
  || { echo "local '$merged_bookmark' moved to '$local_sha'" >&2; exit 1; }

open_bases=$(gh pr list -R "$repo" --state open --base "$merged_bookmark" \
  --json number --jq 'map(.number) | join(" ")') || exit 1
test -z "$open_bases" \
  || { echo "open PRs still use '$merged_bookmark' as base: $open_bases" >&2; exit 1; }

open_heads=$(gh pr list -R "$repo" --state open --head "$merged_bookmark" \
  --limit 1000 --json number,isCrossRepository \
  --jq '[.[] | select(.isCrossRepository | not) | .number] | join(" ")') || exit 1
test -z "$open_heads" \
  || { echo "open PRs still use '$merged_bookmark' as head: $open_heads" >&2; exit 1; }
```

If either query returns a PR, do not delete the branch. Fix or finish the remaining stack relationship first; branches
whose own PRs are still open are never cleanup targets.

After the guards pass, delete both names. Delete the remote branch with a lease pinned to the landed head rather than
through GitHub's ref DELETE endpoint, which takes no expected SHA: a push to the branch between the guard's read and the
DELETE would otherwise be deleted along with it. Run this from the colocated checkout; a rejected lease means the branch
moved, so keep it and report that. The guards above read `$repo` through `gh`, while the push goes wherever `origin`
pushes, so first confirm with `git remote get-url --push --all origin` that every push URL names `$repo`; a separate
push URL or a mirror would otherwise delete a branch in a repository nothing checked:

```bash
git push origin "--force-with-lease=refs/heads/$merged_bookmark:$merged_head_sha" ":refs/heads/$merged_bookmark" \
  || exit 1
jj bookmark delete "$merged_bookmark" || exit 1
```

If repository settings already removed the remote ref, the read above returns an explicit HTTP 404. In that one case,
re-run the PR, local-target, and open-PR guards without the remote-target comparison, then delete only the local
bookmark. Treat authentication failures, rate limits, network errors, and every non-404 response as a hard stop; none of
them proves the branch is absent.

Use the PR's `MERGED` state together with its recorded default-branch base as the evidence that a squash-merged branch
landed. Its old tip is not an ancestor of the default branch, so an ancestry test would reject a correctly merged PR.

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
specific PR you are about to merge. The snippet assumes the parent was already marked ready and its checks waited on,
per the paragraph above; that wait is the reason the first merge cannot be batched with the ready step:

```bash
default_branch=$(gh repo view "$repo" --json defaultBranchRef --jq .defaultBranchRef.name) || exit 1
test -n "$default_branch" || { echo "could not resolve default branch for $repo" >&2; exit 1; }
merge_one() {
  local pr="$1" bookmark="$2" merged=no expected_sha
  expected_sha=$(jj log -r "$bookmark" --no-graph -T commit_id) || return 1
  for attempt in 1 2 3 4 5; do
    read -r state base head head_sha mergeable merge_status <<EOF
$(gh pr view "$pr" -R "$repo" \
  --json state,baseRefName,headRefName,headRefOid,mergeable,mergeStateStatus,statusCheckRollup \
  --jq '[.state, .baseRefName, .headRefName, .headRefOid, .mergeable, .mergeStateStatus] | @tsv')
EOF
    test "$state" = OPEN || { echo "PR #$pr is ${state:-unreadable}, not OPEN" >&2; return 1; }
    test "$base" = "$default_branch" \
      || { echo "PR #$pr base is '$base', expected '$default_branch'" >&2; return 1; }
    test "$head" = "$bookmark" \
      || { echo "PR #$pr head is '$head', expected '$bookmark'" >&2; return 1; }
    test -n "$head_sha" || { echo "no head sha for PR #$pr" >&2; return 1; }
    if test "$head_sha" != "$expected_sha"; then
      test "$attempt" -lt 5 || { echo "PR #$pr head is $head_sha, local $bookmark is $expected_sha" >&2; return 1; }
      sleep 3
      continue
    fi
    if test "$mergeable" = UNKNOWN || test "$merge_status" = UNKNOWN; then
      test "$attempt" -lt 5 || { echo "PR #$pr mergeability stayed UNKNOWN" >&2; return 1; }
      sleep 3
      continue
    fi
    test "$mergeable" = MERGEABLE && test "$merge_status" = CLEAN \
      || { echo "PR #$pr mergeable=$mergeable mergeStateStatus=$merge_status" >&2; return 1; }
    merge_err=$(gh pr merge "$pr" -R "$repo" --squash --match-head-commit "$head_sha" 2>&1) \
      && { merged=yes; break; }
    case "$merge_err" in
      *"base branch policy prohibits the merge"*|*"Base branch was modified"*)
        test "$attempt" -lt 5 \
          || { echo "PR #$pr still rejected after transient-policy retries: $merge_err" >&2; return 1; }
        sleep 3
        ;;
      *) echo "$merge_err" >&2; return 1 ;;
    esac
  done
  test "$merged" = yes || return 1
  test "$(gh pr view "$pr" -R "$repo" --json state --jq .state)" = MERGED \
    || { echo "PR #$pr did not reach MERGED (merge queue?)" >&2; return 1; }
}

merge_one "$parent_pr" "$parent_bookmark" || exit 1

jj git fetch --remote origin || exit 1
jj bookmark set "$default_branch" -r "$default_branch@origin" || exit 1
jj rebase -s "$child_bookmark" -d "$default_branch" || exit 1
gh api -X PATCH "repos/$repo/pulls/$child_pr" -f "base=$default_branch" --jq .base.ref \
  || test "$(gh pr view "$child_pr" -R "$repo" --json baseRefName --jq .baseRefName)" = "$default_branch" \
  || exit 1
jj git push --bookmark "exact:$child_bookmark" || exit 1
gh pr ready "$child_pr" -R "$repo" || exit 1

merge_one "$child_pr" "$child_bookmark" || exit 1
```

You do not need to rediscover the child PR after the base edit when the command succeeds and the next operation is an
explicit merge of that same PR. You still need to verify that the PR you are about to merge is open and has the expected
base and head. If the base edit fails, if the metadata check returns nothing, or if GitHub reports the child PR as
closed, fall back to the full inspection flow above.

The child guard in that snippet checks PR shape only. On repos where checks are expected to run, do not merge the child
in the same breath as the base edit: split the batch after `gh pr ready`, wait for checks per the "no checks means
pending" guidance above, and only then run the verify-and-merge block. The same split applies on merge-queue repos: the
`MERGED` check after the parent merge will fail while the PR sits in the queue — that is the guard working, not an
error. Wait for the queue to land it, then resume from the fetch.

For a known stack with more than two PRs, use the same loop in stack order. After each landing, rebase the remaining
local stack onto the landed base while preserving the relative order of the still-open PRs. Edit the next bottom PR's
GitHub base to the newly landed branch before pushing every bookmark the rebase moved — GitHub tracks branch heads, so
an unpushed descendant keeps showing a stale diff against its rewritten parent, while a pushed bottom bookmark can start
a base-dependent check against stale PR metadata. After the push, mark that next bottom PR ready and wait for its
checks; the PRs above it stay drafts. Descendants should keep their bases on their immediate parent bookmarks unless
that parent changed. Do not rediscover the child PR with `gh pr list` on every iteration when the mapping is already in
hand, but do verify the state, base, and head of the specific PR before merging it.

The push set is the bookmarks of PRs that are still open, and nothing else. A fixed list of the stack's bookmarks
written down at the start of the landing still contains the landed ones, and pushing a landed bookmark on a repo with
automatic head-branch deletion recreates the branch GitHub just deleted. That it usually does not happen is luck: `jj
git fetch` tends to have dropped those local bookmarks already. Drop each bookmark from the push set the moment its PR
reaches `MERGED`.

This flow pushes the whole remaining suffix after every merge so that the PRs above stay reviewable with correct diffs
while the landing is under way. For a long stack that is quadratic in branch updates (about 465 for 30 PRs, at roughly
25 seconds per PR in one observed landing). If the user does not want to wait for CI either, that is a fast-path
request; use "Fast path merge", which pushes only the PR about to merge.

## Fast path merge

NOTE: Only use this section when the current merge request literally says "fast path" (or "fast-path"), or explicitly
says not to wait for CI. It is a one-shot instruction scoped to that one request. It does not become a session
preference, it does not apply to the next "merge the stack", and a user who said "fast path" an hour ago has not said it
now. When the request says neither, land per "Landing stacked PRs safely" and do not offer the fast path as a shortcut.

The fast path lands the whole stack bottom-up, one squash commit on the default branch per PR, and never waits for CI.
It runs as a script, `scripts/fast-land` in this skill's directory (for Claude Code, the "Base directory for this skill"
shown when the skill loads; for Codex, `${CODEX_HOME:-$HOME/.codex}/skills/jjstack`). Run the script rather than
composing the steps by hand. It needs `deno` and Git 2.40 or later. If `deno` is missing, say so and ask whether to
install it or to land with the normal CI-gated flow instead; do not transcribe the script into shell commands.

Measured on scode/repotesting on 2026-10-11: about 10 seconds per PR that GitHub can squash as-is and about 15 seconds
per PR that needs a restack, so a 30-PR stack takes roughly 5 to 8 minutes. The CI-gated flow takes minutes per PR.

### What the script does

Each PR lands in one of two modes, and the switch from the first to the second is one-way for the rest of the landing:

- `github`: once the parent is `MERGED`, edit the PR's base to the default branch, mark it ready, and squash it as it
  is. No rebase and no push.
- `restack`: rebase the PR onto the landed default branch with jj, push only that one bookmark, wait until GitHub's
  `headRefOid` shows the pushed commit, mark it ready, and squash it.

GitHub-only retargeting stops working sooner than it looks. A squash commit is never an ancestor of the next PR's head,
so for the whole landing the merge base GitHub uses stays at the default branch's tip from before the stack. Relative to
that base, any file an earlier PR created exists on both sides with different content, an add/add conflict, and any hunk
an earlier PR changed is changed on both sides. So a PR conflicts in `github` mode whenever it edits a file or hunk that
any earlier PR in the stack created or changed, not just its parent. A stack whose PRs all append to one log file
switches to `restack` at the second PR that touches the log. A restack never needs conflict resolution in that case: the
squash commit's tree equals the parent PR's tree, so rebasing onto it reproduces the PR's own change exactly.

Before merging anything, the script simulates the whole landing with `git merge-tree` and prints the mode each PR will
use, so the plan announces where the switch to `restack` happens. If any PR would need a manual conflict resolution, the
script refuses to start. While landing, it checks every step against that simulation:

- GitHub's default branch must still be at the commit the previous squash produced, checked before each PR starts and
  again before every merge attempt, and each squash commit's tree must equal the predicted tree. Both catch a push to
  the default branch from elsewhere mid-landing. The tip check stops before the merge. The tree check is the only
  defense against a push that lands between the last tip check and GitHub executing the merge, so such a race can still
  put one unpredicted squash on the default branch before the landing stops.
- In `restack` mode the rebased commit's tree must equal the prediction before anything is pushed.
- Every merge is guarded on state `OPEN`, base equal to the default branch, head equal to the expected bookmark, and
  `--match-head-commit` pinned to the exact head. GitHub's transient "base branch policy prohibits the merge", "Base
  branch was modified", and "not mergeable" rejections are retried with every guard re-read, the default-branch tip
  included, so a retry never merges onto a base the plan did not predict.
- `gh pr ready` runs only immediately before each PR's own merge, because GitHub refuses to merge drafts. It is not a
  request for CI, and the run it may start is moot seconds later. A PR the script never reaches stays a draft.
- A PR is `MERGED` before the script moves on, because merge queues and auto-merge make `gh pr merge` exit 0 without
  landing anything.

In `restack` mode the push set is the one PR about to merge. The PRs above it keep stale GitHub heads until their own
turn, which costs nothing while nobody reviews mid-landing, and avoids the quadratic pushes of re-pushing the whole
suffix every cycle. A landed bookmark is never pushed again, so a branch GitHub auto-deleted is never recreated.

The simulation is what makes skipping CI defensible: apart from that one race, every tree that lands on the default
branch is one the plan predicted from the PRs as they were when the landing started.

After the last merge the script cleans up. Whatever sat directly on the top PR, normally the empty working copy, moves
onto the landed default branch. Each landed branch is deleted after the guards from "Clean up landed stack branches"
pass (PR `MERGED` into the default branch from this repo at exactly the landed head, no open PR using the branch as base
or head, 404 accepted as already deleted). The delete itself is a `git push` with `--force-with-lease` pinned to the
landed head, because GitHub's ref DELETE takes no expected SHA and would remove work pushed to the branch a moment
earlier. The landed local commits are abandoned so they do not linger as stray heads in `jj log`, except any that other
local work outside the stack still builds on. The working copy ends up on the landed default branch, so there is no need
to run "After merging a PR" afterwards.

Every fetch the script runs passes `--config git.abandon-unreachable-commits=false`. By default, `jj git fetch` abandons
commits that only a now-deleted remote branch kept reachable and rebases their descendants away, even when the working
copy or side work sits on top. In testing on jj 0.46 that moved the working copy onto an older default-branch commit,
and side work built on a landed commit silently lost that commit's content. Keep that in mind for manual fetches after a
landing on a repo with automatic head-branch deletion, too.

The fast path does not bypass branch protection. If `gh pr merge` is rejected because required checks or reviews are
missing, the script stops; `--admin` needs its own explicit request from the user, and "fast path" is not that request.
It is meant for repos without a merge queue or required checks.

### Running it

"The stack" means the ancestry chain between the remote default branch and the working copy, which is the only thing the
script plans. Other stacks and stray bookmarks in the checkout are not ancestors of `@` and are never included. If the
working copy is not on top of the stack, pass the top bookmark as `--top <bookmark>`, or ask which PR is the top.

Plan first. This fetches but changes nothing else:

```bash
<skill-dir>/scripts/fast-land --dir <checkout> --plan-only
```

The request covers the whole stack, so the printed plan is an announcement, not a question: tell the user the bottom-up
PR list and where the switch to `restack` happens, then land. Pass the planned PR list back as `--expect` so the run
refuses to start if the stack changed between the two calls:

```bash
<skill-dir>/scripts/fast-land --dir <checkout> --expect 175,176,177
```

Give the call a timeout of at least a minute per PR, or run it in the background and wait for it to exit. Other options:
`--repo owner/name` when `gh` cannot infer the repository from the checkout, `--remote <name>` for a remote other than
`origin`, and `--no-cleanup` when the user asked to keep the branches.

Lines starting with `JJSTACK_` are the ledger. `JJSTACK_PLAN` lists the planned PRs. `JJSTACK_LANDED` names each landed
PR with its exact landed head and squash commit. `JJSTACK_KEPT` names a branch cleanup kept and why. `JJSTACK_STOPPED`
names the PR the run stopped at and splits the stack into `landed=`, `unknown=`, and `pending=`. Report the landed PRs
and their squash commits from these lines, then stop. Do not say or imply that later merges will also use the fast path.

### When it stops

A refusal during planning means the stack is not in the shape the fast path assumes: a local bookmark that differs from
its PR's head, a broken base chain, a missing PR, a commit with no bookmark, or a PR that cannot land without resolving
conflicts. Nothing was merged. Show the user what the script reported; do not fall back to the normal landing flow on
your own.

A `JJSTACK_STOPPED` line means the PRs in `landed=` are merged and the PRs in `pending=` are not. A PR in `unknown=` had
`gh pr merge` run for it without the outcome being confirmed: the merge may have gone through with the confirming read
failing, or it may sit in a merge queue or auto-merge. Check its state with `gh pr view` and tell the user what you
found; never describe it as not merged without checking. Show the user the failure and wait for direction. Once the
cause is fixed, a general "continue" means rerunning with `--resume`:

```bash
<skill-dir>/scripts/fast-land --dir <checkout> --resume
```

`--resume` recognizes already landed commits at the bottom of the stack by matching them to the heads of merged PRs,
puts every pending PR in `restack` mode, and predicts each from the head GitHub shows. Rebasing the local commit must
reproduce that prediction, so a local edit that never reached GitHub stops the run before anything is pushed. It also
handles a run that stopped before its first merge, after restacking a PR (which moves the pending commits off the landed
ones), or after its last merge (cleanup only). It has no record of the earlier run beyond what jj and GitHub show, so
landed commits that are no longer ancestors of the working copy are not part of its cleanup; with automatic head-branch
deletion that only leaves their local commits visible in `jj log`, and without it their branches too. Resuming continues
the fast path the user already asked for, so it does not need a new fast-path request. Switching to the CI-gated flow
instead is the user's call.

## After merging a PR

This section applies after the final merge of a landing, when no open stacked PRs remain above what you merged. If open
descendants remain, keep following the restack flow in "Landing stacked PRs safely" instead — the `jj rebase -s @` below
moves only the working-copy commit, and running it mid-landing strands `@` away from the stack it was sitting on. After
GitHub merges the PR, bring the local jj view back into sync before doing anything else:

```bash
default_branch=$(gh repo view "$repo" --json defaultBranchRef --jq .defaultBranchRef.name) || exit 1
test -n "$default_branch" || { echo "could not resolve default branch for $repo" >&2; exit 1; }
jj git fetch --remote origin || exit 1
jj bookmark set "$default_branch" -r "$default_branch@origin" || exit 1
jj rebase -s @ -d "$default_branch" || exit 1
```

The first command imports the new remote state. The second makes the local default-branch bookmark match the merged
remote bookmark. The third moves the usually-empty working-copy commit on top of the merged default branch so the
checkout is coherent again.

## Practical notes

- `bookmark-` in jj revset syntax means "the parent of bookmark". It is useful when you only know the child bookmark and
  want the commit directly below it.
- Do not create bookmarks on empty working-copy commits by accident.
- Do not treat GitHub PR branches as the source of truth. The source of truth is the jj graph plus the bookmark names.
- When in doubt, preserve bookmark names and move them to the right commits. That is what keeps existing PRs updating
  instead of multiplying.

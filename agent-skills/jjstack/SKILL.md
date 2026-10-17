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
  means giving the change a stable bookmark and running `submit` (see "Building and updating the stack"), which opens a
  PR based on the bookmark below it, or on the default branch for the bottom PR. If the current bookmark already has a
  PR but the working copy contains new intended reviewable work on top of it, default to "make the next PR" rather than
  pushing back. In this workflow that is a very common meaning of "make a PR": turn the new top-of-stack work into its
  own commit, bookmark it, and submit.
- "update the PR" Update the existing PR for the current bookmark. Do not create a new bookmark or a new PR unless the
  user explicitly asks for that. Rewrite or extend the current change and `submit`, which pushes the same bookmark again
  and keeps the same PR number.
- "rewrite the PR cleanly" Treat this as "update the PR by rewriting commits instead of adding a follow-up fixup
  commit".
- "make the next PR" Create a new reviewable commit on top of the current stack, assign it a new stable bookmark, and
  `submit`.
- "insert a PR below this one" or "insert a PR into the stack" Create a new change in the middle of the stack with `jj
  new --insert-after ...`, resolve any descendant conflicts, bookmark it, and `submit`, which creates the new PR,
  retargets the PR above it, and pushes every rewritten descendant.
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
- "restack the stack" Use `jj` to rewrite or rebase the local stack, then `submit`, which fixes any PR base the new
  ancestry changed.
- "go to PR 45" or "switch to PR 45" Do not interpret the PR number itself as a local jj revision. Look up PR 45 in
  GitHub, find its head branch or bookmark name, and then move to that bookmarked change locally: `gh pr view 45 -R
  owner/repo --json headRefName`, then `jj new <bookmark>` or another explicit move to that change.
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
  on the default branch per PR without waiting for CI on any of them, using the bundled script's `fast-land` command.
  See "Fast path merge". A landing request that explicitly says not to wait for CI ("land the stack, don't wait on CI",
  "skip CI and merge it") is the same request. Either wording is a one-shot instruction for that single merge request.
  Never carry it forward: the next merge request in the same session, however it is phrased, gets the normal landing
  flow unless it says "fast path" or "don't wait for CI" again.

If the user says only "make a PR" and there is already a PR for the current bookmark, do not push back immediately.
First inspect whether there is new intended work in the working copy that should become the next stacked PR. If yes,
create that next PR on top of the existing one. Only push back when there is already a PR for the current bookmark and
there is no meaningful new work to turn into a new stacked PR. In that narrower case, "make/create" would be ambiguous
or wrong, while "update" means revise the existing PR.

## Preconditions

Confirm these once per repo and session, not before every command:

- `jj --version`, `deno --version`, and `gh auth status` work. The bundled script needs `deno` and Git 2.40 or later.
- `jj config get user.name` and `jj config get user.email` are set.

If `jj` is missing, use the official install instructions; on a system with a working Rust toolchain, `cargo install
--locked --bin jj jj-cli` is a reasonable fallback. If `deno` is missing, say so and ask whether to install it.

If the jj identity is missing, copy the Git one: read `git config user.name` and `git config user.email`, then run `jj
config set --repo user.name '<name>'` and `jj config set --repo user.email '<email>'` with those values. If the
working-copy commit already exists with the empty identity, also run `jj metaedit --update-author @`; otherwise the
first commit inherits the broken identity and will not push in a normal GitHub workflow. If Git has no identity either,
ask the user for one.

Treat the preconditions as good until a command fails in a way that points at auth or config, or you switch checkouts.

## Bootstrapping a repo into jj

For a fresh clone, prefer `jj git clone --colocate <repo-url> <directory>`. For an existing plain Git checkout, run `jj
git init --git-repo .` from the repo root. When the user explicitly asked for `jj`, initializing is the default, not a
suggestion: the mistake to avoid is noticing a plain Git checkout and drifting into a Git-only PR flow. Then start from
`trunk()` if jj defined it, or from the tracked default bookmark you actually have (usually `main@origin`).

Use a colocated repo; the script works only in one. Detached Git HEAD is normal in this workflow and not a sign the
checkout is broken: the jj graph and the bookmark names are the source of truth. For `gh` commands you run yourself,
pass explicit PR numbers and `-R owner/repo` rather than relying on Git branch inference.

## Default rules

- Prefer `jj` over `git` for history manipulation.
- When sandboxed, run the real `jj`, `gh`, and script commands outside the sandbox by default. `jj` needs to lock Git
  metadata such as `packed-refs`, and `gh` needs the host credential store. Do not spend time proving the sandbox is
  broken first.
- Never run `jj` commands in parallel in a colocated repo, including ones that look read-only (`jj status`, `jj log`,
  `jj bookmark list`, `jj diff`, `jj config get`): jj may import or export Git refs, or reset Git HEAD, as part of them.
  Keep `git`, repo-scoped `gh` commands, and the script sequential with them too. Parallel plain file reads are fine.
- Before committing, inspect `jj status` and commit only the intended paths (`jj commit <paths> ...`), leaving unrelated
  files in the working copy.
- After `jj commit`, the commit you just made is `@-`, and `@` is a new empty working copy. Run `jj bookmark set <name>
  -r @-` only after the commit has finished, never alongside it.
- Keep one bookmark per reviewable commit, and keep its name once a PR exists: move the bookmark to new commits rather
  than inventing a fresh name per revision, which would open a second PR.
- Draft and ready. Every PR this workflow creates is a draft, and `submit` always creates them that way. A PR leaves
  draft only when the user asks ("publish the PRs") or when a landing is about to merge it. Never mark a PR ready on
  your own initiative: not because CI would be useful, not because a review round finished, not because you are about to
  hand off. Repos increasingly gate CI on draft status, and a stack is rewritten and re-pushed many times before anyone
  wants a verdict; one stack observed on 2026-09-02 produced 73 CI runs of about ten jobs each, mostly for commits
  nobody looked at. A request for CI is a one-off run, not a reason to publish (see "Expected user commands").
- Never edit a PR with `gh pr edit`. On every `gh` before 2.82.1, which is what distros ship (Ubuntu 24.04 has 2.45.0),
  it fails with a `(repository.pullRequest.projectCards)` GraphQL error and writes nothing; do not retry it or stop to
  upgrade `gh`. The script edits PRs through the REST endpoint. To edit one by hand, use `gh api -X PATCH
  repos/<owner>/<repo>/pulls/<n> -f base=<branch> --jq .base.ref`, or `-f title=<title>` and `-F body=@<file>`.
- Never put PR or commit text inline in a shell command beyond a one-line message in single quotes. Write it to a file
  first (see "Writing commit messages").
- Do not let anything else mutate the working copy while a `jj` command or the script runs: no other `jj` or `git`
  process, no editor auto-save doing broad rewrites.

## Writing commit messages

`submit` takes each new PR's title and body from its commit's description: the first line is the title, and everything
after the first blank line is the body. The commit message is the PR text, so write it as such.

`jj commit`, `jj squash`, and `jj describe` take `-m`, but none of them takes `-F` or `--file` (an agent reaching for
the Git habit `jj commit -F msg.txt` gets `unexpected argument '-F' found`, observed on jj 0.44). A one-line message can
go inline in single quotes. For anything longer, write the message to a file with your file-writing tool, then set it
from stdin, which only `jj describe` reads:

```bash
jj commit <paths> -m 'wip'
jj describe -r @- --stdin < <message-file>
```

To bring an existing PR's title and body in line with a reworded commit, run `submit --update-text`.

## Building and updating the stack

Every one of these ends with the script's `submit` command, which makes GitHub match the local stack: it pushes every
bookmark that moved, opens a draft PR for every bookmark that has none, and bases each PR on the bookmark below it (the
bottom one on the default branch). Run it with the working copy on top of the stack, or pass the top bookmark as
`--top`; it refuses when the stack continues above `--top` with open PRs, since those would be left stale. Start a new
stack from the default branch with `jj new 'trunk()'`.

- New PR on top: make the change, `jj commit <paths>` with its message, `jj bookmark set <name> -r @-`, then `submit`.
- Update a PR: `jj new <bookmark>`, make the edit, `jj squash` to fold it into the bookmarked commit, then `jj new
  <top-bookmark>` to put the working copy back on top, and `submit`. jj rebases everything above the edit, and `submit`
  pushes all of it. Each PR is exactly one commit: a follow-up commit with the bookmark moved onto it leaves the
  original commit unbookmarked, and `submit` refuses that rather than let GitHub fold it in silently.
- Insert a PR into the stack: `jj new --insert-after <bookmark-below>`, make the change, describe it, `jj bookmark set
  <name> -r @`, then `jj new <top-bookmark>` and `submit`. It creates the inserted PR and retargets the PR above it.
- Restack, reorder, or drop a commit: do it with `jj rebase` or `jj abandon`, then `submit`.

Rewriting a lower commit can leave the commits above it conflicted. That is a normal part of changing lower layers, not
a surprise. `submit` refuses to push a conflicted commit, so resolve them bottom-up: `jj new <conflicted-bookmark>`,
resolve the markers (or use `jj resolve`), `jj squash`, and repeat, then `submit`.

`status` shows the same plan without acting on it. To see what changed since a bookmark was last pushed, use `jj
interdiff --from <bookmark>@origin --to <bookmark>`.

## The bundled `jjstack` script

The deterministic parts of this workflow run as a Deno script, `scripts/jjstack` in this skill's directory (for Claude
Code, the "Base directory for this skill" shown when the skill loads; for Codex,
`${CODEX_HOME:-$HOME/.codex}/skills/jjstack`). Run its commands rather than composing their steps by hand, and never
transcribe the script into shell commands. Every command takes `--dir <checkout>` and runs everything there, so it works
from any working directory; `--repo owner/name` when `gh` cannot infer the repository from the checkout; `--remote
<name>` for a remote other than `origin`; and `--top <rev>` for the top of the stack (default `@`). "The stack" is
always the ancestry chain from the remote default branch to `--top`. Before anything else, every command checks that the
remote fetches from and pushes to that repository alone, on the same host. A remote URL that uses an ssh host alias from
`~/.ssh/config` does not name the host and is refused; use the real host name in the remote URL.

- `status`: prints, for each PR bottom-up, what `submit` would do, and a `JJSTACK_STATUS` line per PR. It fetches but
  changes nothing else.
- `submit [--update-text] [--partial]`: syncs the stack to GitHub as described in "Building and updating the stack", and
  prints a `JJSTACK_SUBMITTED` line per PR naming what it did. It runs every check before its first write. Within one PR
  it retargets before pushing, because the push starts CI for the new head and a base-dependent check has to see the
  final base. It refuses a range up to `--top` that is not one chain, a non-empty commit without a bookmark (GitHub
  would fold it into the PR above), a commit with more than one bookmark, a conflicted commit or bookmark, open PRs on
  commits that build on the stack but are not below `--top` (unless `--partial`), a commit without a description when
  its PR has to be created, a bookmark that has two open PRs, and a bookmark whose earlier PR was closed or merged (it
  will not open a second PR for the same branch). A `JJSTACK_WARN` line names an open PR based on a stack bookmark whose
  own head is not in the stack.
- `fast-land`: see "Fast path merge".

### When the script stops

Every way the script can stop prints the same three lines to stderr:

```
JJSTACK_ERROR code=E_MATCH_HEAD_DIFFERS phase="plan" mutated=sync
error: PR #175 head is 4fa48fa... but local pr/foo is 7a7e9f9...; GitHub does not show the local commit
raised at: grep -n '"E_MATCH_HEAD_DIFFERS"' /path/to/scripts/jjstack
```

Each code appears exactly once in the script, so the `raised at:` grep lands on the one line that stopped the run;
shared helpers take their caller's code, so that line is the call site. `mutated=` says how far the run got: `no` means
nothing ran, `sync` means it only fetched or let jj snapshot the working copy (a fetch can move or delete local
bookmarks that track the remote, but the script itself wrote nothing), and `yes` means it had started writing to jj,
Git, or GitHub. The exit status follows: 3 is a refusal with nothing written, 4 is a stop after writes began, 2 is a
usage error, and 1 is a crash (`code=E_UNEXPECTED`) before any write, which is a bug in the script and comes with a
stack trace. A crash after writes began exits 4. Reads from GitHub are retried a couple of times on server and network
errors, so a stop on one means the failure persisted.

When the script stops, read the script around the raising line before deciding what to do. The check there, and the
docstring of the function it sits in, say which invariant failed and why it matters; the messages say what was checked
and what was found, not what to do about it. Then fix the cause and rerun the same command, or tell the user what you
found. After exit 4, work out what already changed before rerunning anything: the `JJSTACK_` ledger lines on stdout say
what completed, and `jj log` and `gh pr view` show the rest. Do not get past a refusal by doing its step by hand. Most
refusals guard against pushing, merging, or deleting something nobody reviewed, and doing the step by hand removes the
guard along with the obstacle. If the script itself looks wrong for the situation, a crash or a check that is clearly
mistaken, say so and propose a fix to the script instead of routing around it.

## Failure modes

Stop and surface the problem instead of improvising if:

- a `jj`, `gh`, or script command is blocked by sandboxing, approval policy, or a similar permission issue
- `jj git push` reports stale remote state or a bookmark conflict, and a fetch is clearly required
- rewriting a lower commit causes conflicts you cannot resolve confidently
- the GitHub PR shape no longer matches the bookmark ancestry in a way `submit` refuses to repair
- you ran commit, bookmark, and push steps in parallel and are no longer sure which commit a bookmark points at; inspect
  `jj log` and `jj bookmark list` and repair the bookmark explicitly before anything else

If a `jj` or `gh` command fails with `unexpected argument` or an unknown-flag error, treat it as version drift between
the installed tool and whatever produced the flag. Check `--help` for the installed version and drop or replace the flag
rather than retrying the same command. jj in particular removes deprecated flags: `jj git push --allow-new` was removed
in jj 0.42, and naming the bookmark with `--bookmark` is enough to push a new one.

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
failure paths, which exist because `set -e` is silently inert under agent shell wrappers and a guard without its own
exit fails open. Derive the default branch once per landing instead of hardcoding `main` — a repo whose default is
`master` can even contain a stack branch literally named `main`, which would turn a hardcoded guard into an
authorization for the wrong-way merge. Note that `gh repo view` takes the repository as a positional argument, not via
`-R`:

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
edited`, a reversible PR body edit (the REST body edit under "Default rules") can create that evaluation; preserve and
restore the exact body by saving it to a file first. Do not assume every pull-request workflow handles body edits.

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
It runs as the `fast-land` command of the bundled script (see "The bundled `jjstack` script"). If `deno` is missing, say
so and ask whether to install it or to land with the normal CI-gated flow instead; do not transcribe the script into
shell commands.

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
<skill-dir>/scripts/jjstack fast-land --dir <checkout> --plan-only
```

The request covers the whole stack, so the printed plan is an announcement, not a question: tell the user the bottom-up
PR list and where the switch to `restack` happens, then land. Pass the planned PR list back as `--expect` so the run
refuses to start if the stack changed between the two calls:

```bash
<skill-dir>/scripts/jjstack fast-land --dir <checkout> --expect 175,176,177
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
<skill-dir>/scripts/jjstack fast-land --dir <checkout> --resume
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

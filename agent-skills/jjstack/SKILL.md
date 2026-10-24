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
Set the checkout up with the bundled script's `init` command first (see "Preconditions"). This preference lasts for the
rest of the session, or until the user explicitly says to stop using jjstack.

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
not already set up for `jj`, run the script's `init`, which initializes jj in place, and continue with the `jj`
workflow. Do not stop to ask first.

When an agent is driving this workflow from a sandboxed environment, prefer running the actual `jj` and `gh` workflow
commands outside the sandbox by default. In practice `jj` often needs to lock Git metadata such as `packed-refs`, and
`gh` often needs access to the host credential store or keyring. Do not burn time proving the sandbox is broken first if
the task is clearly "use `jj` to manage history and `gh` to manage the PR". Start outside the sandbox unless the
environment already guarantees those operations work inside it.

- "make a PR" or "create a PR" Create a new stacked PR, as a draft, for the current reviewable change. In practice that
  means writing its message to a file and running the script's `new-pr` with a new bookmark name (see "Building and
  updating the stack"), which commits the change, bookmarks it, and opens a PR based on the bookmark below it, or on the
  default branch for the bottom PR. If the current bookmark already has a PR but the working copy contains new intended
  reviewable work on top of it, default to "make the next PR" rather than pushing back. In this workflow that is a very
  common meaning of "make a PR": turn the new top-of-stack work into its own PR with `new-pr`.
- "update the PR" Update the existing PR for the current bookmark. Do not create a new bookmark or a new PR unless the
  user explicitly asks for that. Make the edit and fold it into the PR's commit with the script's `amend` (see "Building
  and updating the stack"), which pushes the same bookmark again and keeps the same PR number.
- "rewrite the PR cleanly" Treat this as "update the PR by rewriting commits instead of adding a follow-up fixup
  commit".
- "make the next PR" Create a new reviewable commit on top of the current stack with a new stable bookmark and its own
  PR: `new-pr`.
- "insert a PR below this one" or "insert a PR into the stack" Create a new change in the middle of the stack with `jj
  new --insert-after ...`, resolve any descendant conflicts, bookmark it, and `submit`, which creates the new PR,
  retargets the PR above it, and pushes every rewritten descendant.
- "publish the PRs" or "mark the PRs ready" Run the script's `publish`, with `--prs <list>` (comma-separated PR numbers)
  for a named subset, and report which PRs it marked ready (its `JJSTACK_PUBLISHED` lines). A named subset ("publish 329
  and up") means exactly those PRs and no others; an unqualified request means the current stack, which is what
  `publish` without `--prs` covers. It refuses PRs GitHub does not show as they are locally (the selected ones and every
  PR below them), since publishing starts CI on draft-gated repos; run `submit` first. A request to publish is not a
  request to merge, and it does not change how later PRs get created: the next "make a PR" is still a draft.
- "run CI on the stack", "run CI on PR N", "kick off CI", or "make sure CI passes" Trigger one run per named or implied
  PR without changing any PR's draft state: run the script's `ci` (with `--prs <list>` for named PRs), and report the
  run URLs from its `JJSTACK_CI` lines. It dispatches the one workflow on the default branch that allows
  `workflow_dispatch`; when there are several, it refuses and names them, and choosing one (`--workflow <file>`) is the
  user's call. Like `publish`, it refuses PRs GitHub does not show as they are locally; run `submit` first. This is a
  one-off. Marking the PR ready would also start a run, but it changes future behavior too: every later rebase and push
  of a ready PR starts CI again, which is the cost the draft default exists to avoid. A request for CI is not a request
  to publish. If no workflow can be dispatched, say so and ask; do not mark the PR ready to get the run.
- "unpublish", "back to draft", or "convert to draft" Run `gh pr ready --undo <n>`. Rarely needed, and only ever on
  request; the skill never converts a ready PR back to a draft on its own.
- "restack the stack" Use `jj` to rewrite or rebase the local stack, then `submit`, which fixes any PR base the new
  ancestry changed.
- "rebase onto main", "sync the stack", or "I merged the bottom PR" Run the script's `sync`, which moves the stack onto
  the current default branch, cleans up PRs at the bottom that were merged outside the script, and submits the rest.
- "go to PR 45" or "switch to PR 45" Do not interpret the PR number itself as a local jj revision. Run the script's
  `goto --pr 45`, which looks up the PR's branch on GitHub and puts a new working copy on top of that bookmarked commit,
  so the next edit lands on that PR (fold it in with `amend`). It refuses a PR that is not open, a PR from a fork, and a
  branch whose local bookmark is missing or conflicted.
- "merge the PR", "merge this PR", or a bare "merge it" In a stacked workflow, "merge" means "land this change on the
  default branch", not "run `gh pr merge` on whatever PR is in focus". Those are different operations when the PR's
  GitHub base is another stack bookmark: `gh pr merge` on such a PR folds it into its parent's branch, GitHub closes it
  as "merged", and nothing reaches the default branch at all. Land with the script's `land` command (see "Landing
  stacked PRs"), passing the named PR as `--through` when PRs above it should stay open. If the named PR sits above
  unmerged parents, landing it lands them too, one merge per PR, bottom-up. Always say which PRs will land in what order
  before the first merge. Stop and wait for confirmation only when that means going wider than the user asked: a request
  naming one PR that turns out to sit above unmerged parents also lands those parents, and choosing to land work the
  user did not name is theirs to make. When the request already covers the whole set ("merge the stack", "land all of
  it"), the plan is an announcement, not a question, so state the order and proceed. Bottom-up is the only valid order,
  so there is nothing for the user to decide about it. Do not re-ask partway through a landing they already approved.
  Squash merging is the default; another merge strategy needs an explicit request, and the script does not do one.
- "fast path merge the stack", "fast-path merge", or "merge it, fast path" Land the whole stack with one squash commit
  on the default branch per PR without waiting for CI on any of them, using the bundled script's `fast-land` command.
  See "Fast path merge". A landing request that explicitly says not to wait for CI ("land the stack, don't wait on CI",
  "skip CI and merge it") is the same request. Either wording is a one-shot instruction for that single merge request.
  Never carry it forward: the next merge request in the same session, however it is phrased, gets the normal landing
  flow unless it says "fast path" or "don't wait for CI" again.
- "stop waiting for CI", "don't run more CI, just land", or similar while a CI-gated `land` is running. Switch the rest
  of that landing to the fast path: run the script's `stop` command, wait for the running `land` to exit with
  `E_STOP_REQUESTED`, then plan and land what is left with `fast-land --resume`, passing the same `--expect` and
  `--through` as the stopped landing (see "Fast path merge"). This is the fast-path request for the rest of that landing
  only. Any other change of course mid-landing (editing a PR, slipping a PR in below the next merge) starts the same
  way: `stop`, wait for the exit, make the change and `submit`, then rerun the landing with `--resume`.
- "merge anyway" or "land anyway" after a CI wait or refusal is an explicit request to skip CI for the rest of that
  landing. If `land` is still running, stop it as above. If it has stopped, plan with `fast-land --resume --plan-only`,
  then run `fast-land --resume` with the original `--expect` and `--through`, if any. Use the script's merge guards; do
  not run `gh pr merge` or retarget PRs by hand. This instruction applies to that landing only. A branch-protection
  refusal still stops the script; skipping its checks does not grant `--admin`.

If the user says only "make a PR" and there is already a PR for the current bookmark, do not push back immediately.
First inspect whether there is new intended work in the working copy that should become the next stacked PR. If yes,
create that next PR on top of the existing one. Only push back when there is already a PR for the current bookmark and
there is no meaningful new work to turn into a new stacked PR. In that narrower case, "make/create" would be ambiguous
or wrong, while "update" means revise the existing PR.

## Preconditions

Run the bundled script's `init` once per session in each checkout, before any other jjstack work there
(`<skill-dir>/scripts/jjstack init --dir <checkout>`; see "The bundled `jjstack` script"). It is the only setup call: do
not check tool versions, `gh auth status`, or the jj identity separately. In a checkout that is already set up it
changes nothing, and otherwise:

- in a plain Git checkout it runs `jj git init --colocate` at the top level, without asking first;
- when jj has no `user.name` or `user.email`, it copies Git's into the repository's jj config;
- when the working-copy commit has an empty author, it runs `jj metaedit --update-author @`, because jj refuses to push
  a commit without one;
- then it runs the checks every command shares: the repository and its default branch resolve on GitHub (which needs
  `gh` to be logged in), the remote points at that repository, and the checkout is colocated.

It prints a `JJSTACK_INIT` line saying what it changed. When it stops, the usual rules apply (see "When the script
stops"). A refusal with `E_INIT_NO_IDENTITY` means neither jj nor Git has an identity; ask the user for one. If `jj`
itself is missing, use the official install instructions; on a system with a working Rust toolchain, `cargo install
--locked --bin jj jj-cli` is a reasonable fallback. If `deno` is missing, the script cannot run at all: say so and ask
whether to install it. The script also needs Git 2.40 or later, for `git merge-tree --merge-base`; on an older Git a
landing stops at its first `merge-tree` call.

Treat the setup as good until a command fails in a way that points at auth or config, or you switch checkouts.

## Bootstrapping a repo into jj

For a fresh clone, prefer `jj git clone --colocate <repo-url> <directory>`, then `init`. For an existing plain Git
checkout, `init` does the bootstrap. The mistake to avoid is noticing a plain Git checkout and drifting into a Git-only
PR flow. Then start from `trunk()` if jj defined it, or from the tracked default bookmark you actually have (usually
`main@origin`).

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
  Keep `git`, repo-scoped `gh` commands, and the script sequential with them too. The one exception is the script's
  `stop` command, which exists to be run while a landing is in progress. Parallel plain file reads are fine.
- Before committing, inspect `jj status` and commit only the intended paths, leaving unrelated files in the working
  copy. For a new PR on top of the stack, pass those paths to `new-pr`, which commits and bookmarks in one call;
  otherwise use `jj commit <paths> ...`.
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

Write the message to a file with your file-writing tool and pass it to a script command that takes `--message-file` (see
"Building and updating the stack"); the script sets it as the commit's description itself. Give the file as an absolute
path: a relative one is read from the directory you run the script in, not from `--dir`. A message file without a title
(an empty first line) is refused.

Outside those commands, set a message with jj directly. `jj commit`, `jj squash`, and `jj describe` take `-m`, but none
of them takes `-F` or `--file` (an agent reaching for the Git habit `jj commit -F msg.txt` gets `unexpected argument
'-F' found`, observed on jj 0.44). A one-line message can go inline in single quotes. For anything longer, write the
message to a file, then set it from stdin, which only `jj describe` reads:

```bash
jj commit <paths> -m 'wip'
jj describe -r @- --stdin < <message-file>
```

To reword a PR, pass the new message to `amend --bookmark <name> --message-file <file>`, which rewords its commit and
updates the PR's title and body in the same call. Like `submit --update-text`, it resets the title and body of every PR
in the stack from its commit message, so edits made to PR text on GitHub are overwritten. To bring a PR's title and body
in line with a commit you reworded some other way, run `submit --update-text`. Rewording a commit rewrites the commits
above it too, so `submit` pushes those as well, which starts CI for them on repos that run CI for drafts.

## Building and updating the stack

Every one of these ends with the script's `submit` command (the commands below that take `--message-file` run it
themselves), which makes GitHub match the local stack: it pushes every bookmark that moved, opens a draft PR for every
bookmark that has none, and bases each PR on the bookmark below it (the bottom one on the default branch). Run it with
the working copy on top of the stack, or pass the top bookmark as `--top`; it refuses when the stack continues above
`--top` with open PRs, since those would be left stale. Start a new stack from the default branch with `jj new
'trunk()'`.

- New PR on top: make the change, write its message to a file, and run `new-pr --bookmark <name> --message-file <file>
  [paths...]`. It commits the working copy's changes (only the given paths, relative to `--dir`, when there are any)
  with that message, bookmarks the commit, and submits; changes outside the paths stay in the working copy. Put `--`
  before a path that starts with `-`. Pick a new bookmark name of letters, digits, `.`, `_`, `/`, and `-`, starting with
  a letter or digit, that no local bookmark, remote branch, or earlier PR has used. Before committing anything, it
  refuses a name that breaks those rules, nothing to commit, a path that matches no change (a path may name a file or a
  directory, which covers everything under it; for a rename, pass both the old and the new path), a working copy that
  already carries a bookmark or a conflict, a working copy that is not on top of the stack, and anything `submit` would
  refuse about the stack below. Once it prints `JJSTACK_COMMITTED`, the commit and bookmark exist: if it stops after
  that, fix the cause and run `submit`, not `new-pr` again.
- Update a PR: make the edit in the working copy and run `amend --bookmark <name> [--message-file <file>] [paths...]`.
  It folds the working copy's changes (only the given paths, when there are any) into that PR's commit, rewords the
  commit when given a message file, and submits; jj rebases everything above the edit, and the submit pushes all of it.
  Run it from the top of the stack, or, when the edit touches lines that PRs higher up also change, from the PR's own
  commit: `jj new <bookmark>`, edit, `amend --bookmark <bookmark>`, then `jj new <top-bookmark>` to put the working copy
  back on top. It refuses, before changing anything, a bookmark that is not in the stack below the working copy, an
  amend with no changes and no message, a path that matches no change, and a working copy that has conflicts or carries
  a bookmark of its own (it is then a PR's commit, which `jj edit` leaves you on; use `jj new <bookmark>` instead). Once
  it prints `JJSTACK_AMENDED`, the amend is done: if the submit after it stops, fix the cause and run `submit`, with
  `--update-text` when the amend reworded the commit. Each PR is exactly one commit: a follow-up commit with the
  bookmark moved onto it leaves the original commit unbookmarked, and `submit` refuses that rather than let GitHub fold
  it in silently.
- Insert a PR into the stack: `jj new --insert-after <bookmark-below>`, make the change, describe it, `jj bookmark set
  <name> -r @`, then `jj new <top-bookmark>` and `submit`. It creates the inserted PR and retargets the PR above it.
- Restack, reorder, or drop a commit: do it with `jj rebase` or `jj abandon`, then `submit`.
- Catch up with the default branch: `sync`. When the bottom PRs were merged outside the script (in the web UI, say), it
  recognizes them by their exact heads, moves the rest of the stack and the working copy onto the default branch,
  deletes the merged branches and drops their local commits, and submits, which retargets the new bottom PR to the
  default branch. When nothing merged but the default branch moved, it rebases the stack onto it and submits. It prints
  a `JJSTACK_SYNCED` line saying which PRs it found landed and whether it rebased. It refuses, before changing anything,
  a stack that continues above `--top` with open PRs, and uncommitted work in the working copy under the default `--top
  @`; pass the top PR's bookmark as `--top` to sync the stack and carry that work along on top. A rebase that leaves
  conflicts stops in the submit; resolve them bottom-up and run `submit`. A PR merged with a merge commit rather than a
  squash is not recognized as landed; the rest of the stack is still synced, but that PR's branch and bookmark are left
  for you to delete.

Rewriting a lower commit can leave the commits above it conflicted. That is a normal part of changing lower layers, not
a surprise. `submit` refuses to push a conflicted commit, and an `amend` that conflicts the amended commit or those
above it stops the same way after printing `JJSTACK_AMENDED` (the amend itself is done), so resolve them bottom-up: `jj
new <conflicted-bookmark>`, resolve the markers (or use `jj resolve`), `jj squash`, and repeat, then `submit`.

`status` shows the same plan without acting on it. To see what changed since a bookmark was last pushed, use `jj
interdiff --from <bookmark>@origin --to <bookmark>`.

## The bundled `jjstack` script

The deterministic parts of this workflow run as a Deno script, `scripts/jjstack` in this skill's directory (for Claude
Code, the "Base directory for this skill" shown when the skill loads; for Codex,
`${CODEX_HOME:-$HOME/.codex}/skills/jjstack`). Run its commands rather than composing their steps by hand, and never
transcribe the script into shell commands. Every command takes `--dir <checkout>` and runs everything there, so it works
from any working directory; `--repo owner/name` when `gh` cannot infer the repository from the checkout; `--remote
<name>` for a remote other than `origin`; and `--top <rev>` for the top of the stack (default `@`). "The stack" is
always the ancestry chain from the remote default branch to `--top`. Before it reads or changes the stack, every command
checks that the remote fetches from and pushes to that repository alone, on the same host. A remote URL that uses an ssh
host alias from `~/.ssh/config` does not name the host and is refused; use the real host name in the remote URL.

- `init`: sets the checkout up for the other commands and runs their shared checks; see "Preconditions".
- `status`: prints, for each PR bottom-up, what `submit` would do, and a `JJSTACK_STATUS` line per PR. It fetches but
  changes nothing else.
- `new-pr --bookmark <name> --message-file <file> [paths...]`: commits working-copy changes as the next PR, bookmarks
  the commit, and submits; see "Building and updating the stack".
- `amend --bookmark <name> [--message-file <file>] [paths...]`: folds working-copy changes into an existing PR's commit,
  optionally rewords it, and submits; see "Building and updating the stack".
- `sync`: brings the stack along after PRs merged outside the script or the default branch moved, and submits; see
  "Building and updating the stack".
- `ci [--prs <list>] [--workflow <file>]`: dispatches one run of the repo's `workflow_dispatch` workflow per PR, or per
  listed PR, without changing draft state, and prints a `JJSTACK_CI` line per PR with the run's URL. Before dispatching
  anything it refuses a listed PR not in the stack, PRs GitHub does not show as they are locally (up to the topmost
  selected one), a default branch with no dispatchable workflow or several (unless `--workflow` names one), a workflow
  file that is not on the default branch yet (GitHub cannot find it there), and a PR whose own commit lacks that
  workflow (GitHub runs the branch's copy; `sync` brings an older stack up to date); see "Expected user commands".
- `goto --pr <n>`: puts a new working copy on top of PR n's commit and prints a `JJSTACK_GOTO` line with the commit and
  the head GitHub shows for the PR (they differ when the bookmark has unpushed work); see "Expected user commands".
- `publish [--prs <list>]`: marks the stack's PRs, or the listed ones, ready for review, bottom-up, and prints a
  `JJSTACK_PUBLISHED` line per PR (`action=ready` or `action=already-ready`). Before marking any PR ready, it refuses a
  listed PR that is not in the stack, and any PR, up to the topmost listed one, whose head or base on GitHub is not the
  local one or that has no PR yet. Run from the middle of a stack, it stops on the PRs above; pass the top bookmark as
  `--top` there.
- `submit [--update-text] [--partial]`: syncs the stack to GitHub as described in "Building and updating the stack", and
  prints a `JJSTACK_SUBMITTED` line per PR naming what it did. It runs every check before its first write. Within one PR
  it retargets before pushing, because the push starts CI for the new head and a base-dependent check has to see the
  final base. It refuses a range up to `--top` that is not one chain, a non-empty commit without a bookmark (GitHub
  would fold it into the PR above), a commit with more than one bookmark, a conflicted commit or bookmark, open PRs on
  commits that build on the stack but are not below `--top` (unless `--partial`), a commit without a description when
  its PR has to be created, a bookmark that has two open PRs, and a bookmark whose earlier PR was closed or merged (it
  will not open a second PR for the same branch). A `JJSTACK_WARN` line names an open PR based on a stack bookmark whose
  own head is not in the stack.
- `land [--through <PR-or-bookmark>] [--expect <PRs>] [--plan-only] [--resume] [--no-cleanup] [--check-timeout
  <minutes>] [--allow-no-checks] [--ignore-check <names>] [--accept-skipped-check <PR:workflow/job,...>]`: CI-gated
  landing; see "Landing stacked PRs".
- `fast-land [--through <PR-or-bookmark>] [--expect <PRs>] [--plan-only] [--resume] [--no-cleanup]`: landing without CI;
  see "Fast path merge".
- `stop`: asks a running `land` or `fast-land` in the same checkout to stop at its next safe point, which is before it
  starts the next PR or between polls while it waits on checks, never inside a merge. The landing exits with
  `E_STOP_REQUESTED` and a `JJSTACK_STOPPED` line. Use it rather than killing the process, which stops it wherever it is
  and reports `E_KILLED`.

### The run log

Every command except `stop` writes a timestamped log of its run to a file of its own under `.jj/jjstack-logs/` in the
checkout (an `init` that stops before the checkout is set up writes it to a temporary file, named on the `log:` line)
and prints the path as `JJSTACK_LOG path=...`, the first line of its output. The log opens with the process id and
arguments and, on success, ends with `finished ok`. In between it has every command the run executes, with its call-site
code, exit status, and duration, the output of every command that failed or wrote something, and the ledger lines.
Writes and fetches also get a line when they start, and any other command one once it has run for 30 seconds, so a tail
shows a slow step in flight. Repeated identical status polls (the check wait, head convergence, mergeability) are logged
once and then as a count with the time span they cover, and an unchanged wait gets a heartbeat line every five minutes.
Old logs are pruned to the 20 newest, but never one modified in the last hour.

Nothing requires reading it. It is there for checking on a long run while it goes, and for explaining a stop that the
report and the ledger lines do not. When the user asks how a running landing is going, read the last 20 or so lines
(`tail -n 20 <path>`) and answer from them instead of waiting for the run to finish. A newest line more than about six
minutes old, with no command shown as started or still running, suggests the run is stuck; check the process before
saying so.

### When the script stops

Every way the script can stop prints the same lines to stderr:

```
JJSTACK_ERROR code=E_MATCH_HEAD_DIFFERS phase="plan" mutated=sync
error: PR #175 head is 4fa48fa... but local pr/foo is 7a7e9f9...; GitHub does not show the local commit
raised at: grep -n '"E_MATCH_HEAD_DIFFERS"' /path/to/scripts/jjstack
log: /path/to/checkout/.jj/jjstack-logs/2026-10-17T03-55-57-266Z-land.log (every command this run executed; ...)
```

Each code appears exactly once in the script, so the `raised at:` grep lands on the one line that stopped the run;
shared helpers take their caller's code, so that line is the call site. `mutated=` says how far the run got: `no` means
nothing ran, `sync` means it only fetched or let jj snapshot the working copy (a fetch can move or delete local
bookmarks that track the remote, but the script itself wrote nothing), and `yes` means it had started writing to jj,
Git, or GitHub. The exit status follows: 3 is a refusal with nothing written, 4 is a stop after writes began, 2 is a
usage error, and 1 is a crash (`code=E_UNEXPECTED`) before any write, which is a bug in the script and comes with a
stack trace. A crash after writes began exits 4. Reads from GitHub are retried up to five times, backing off, on server,
rate-limit, and network errors, so a stop on one means the failure persisted.

When the script stops, read the script around the raising line before deciding what to do. The check there, and the
docstring of the function it sits in, say which invariant failed and why it matters; the messages say what was checked
and what was found, not what to do about it. Then fix the cause and rerun the same command, or tell the user what you
found. After exit 4, work out what already changed before rerunning anything: the `JJSTACK_` ledger lines on stdout say
what completed, and `jj log` and `gh pr view` show the rest. When those do not explain what happened, read the file the
`log:` line names: every command the run executed, in order, with its call-site code, exit status, and duration, and the
output of every command that failed or wrote something (see "The run log"). Do not get past a refusal by doing its step
by hand. Most refusals guard against pushing, merging, or deleting something nobody reviewed, and doing the step by hand
removes the guard along with the obstacle. If the script itself looks wrong for the situation, a crash or a check that
is clearly mistaken, say so and propose a fix to the script instead of routing around it.

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

## Landing stacked PRs

"Merge" in a stacked workflow means landing on the default branch: one squash commit per PR, bottom-up, each its own
merge. The "merge the PR" bullet under "Expected user commands" says what a request covers and when to wait for
confirmation. The script's `land` command does the landing:

```bash
<skill-dir>/scripts/jjstack land --dir <checkout> --plan-only [--through <PR-or-bookmark>]
<skill-dir>/scripts/jjstack land --dir <checkout> --expect 175,176 [--through <PR-or-bookmark>]
```

Plan first, and tell the user the bottom-up list of PRs that will land and the PRs above them that will only be
restacked. Then pass the planned list back as `--expect`, so the run refuses if the stack changed between the two calls.
`--through` lands up to and including that PR and leaves the PRs above it open. The stack has to be in sync with GitHub
first: `land` refuses when a PR's head is not the local commit, so run `submit` when `status` shows anything to do. Give
the call a timeout of at least the repo's CI time per PR, or run it in the background and wait for it to exit; its run
log shows the progress meanwhile.

Before the first `land` call, check the target repo's CI instructions, workflow triggers in `.github/workflows/`, and
any other automatic PR checks. If those sources show that no automatic PR verdict is expected after retarget, push, or
ready (for example, CI is documented as manual-only and workflows have only `workflow_dispatch`), pass
`--allow-no-checks` on that first call and tell the user why. A manually dispatched run is a separate decision; the flag
does not dispatch one. If check behavior is uncertain or a check may attach after the ready event, leave the flag off
and use the ordinary wait. One empty check rollup is not evidence that a repo has no automatic PR checks.

For each PR, bottom-up, `land`:

1. retargets it to the default branch once its parent has landed, before any push, so CI sees the final base;
2. rebases it onto the landed default branch when its parent is not already part of it, refusing on conflicts;
3. proves the rebased commit is still the reviewed change, byte for byte: replaying the head GitHub showed at planning
   time onto the new parent must give exactly the local tree, so a local edit (made during a long CI wait, say) is a
   refusal, not a merge;
4. pushes only that PR and waits until GitHub shows the pushed head. The PRs above it are rewritten locally but pushed
   only on their own turn, or for those above `--through` in the restack at the end, so mid-landing they show a stale
   diff on GitHub. Pushing them on every cycle would cost a CI run per open PR per landed PR on repos that run CI for
   drafts;
5. marks it ready, the one PR it un-drafts at that moment, because GitHub will not merge a draft and draft-gated CI
   would not run on one;
6. waits for its checks on that exact head, refusing on a failed check, on a timeout (`--check-timeout <minutes>`,
   default 30), and on no CI verdict after two minutes (with `--allow-no-checks`, no verdict proceeds at once); and
7. squash-merges it, subject "<PR title> (#N)", guarded on state `OPEN`, base, head ref, `--match-head-commit`, and the
   check verdict re-read before every attempt, retrying only GitHub's known transient rejections, then confirms the PR
   reached `MERGED`, since merge queues and auto-merge make `gh pr merge` exit 0 without landing anything.

Then it restacks the PRs above `--through` onto the landed result with `submit` and cleans up (see "After a landing").
It refuses up front when open PRs build on the stack above `--top`, since those would be left on a rewritten base; pass
the real top as `--top` and use `--through` to land less.

The check wait is stricter than reading the rollup once. It keeps only the newest run of each Actions job, so a re-run
flake or a superseded draft-time run does not block forever; checks from other apps are never collapsed, because the
rollup cannot tell two apps' same-named checks apart, so a superseded app run can block. It treats a check suite that is
still running as pending, including a queued Actions suite with no jobs yet, because a job with `needs:` gets no check
run until its dependencies finish and a workflow queued behind a concurrency group has none at all. A pass counts only
once 30 seconds have gone by since the latest event that can attach new runs (the landing's own retarget, push, or ready
call, the newest check suite on the head, or the PR leaving draft), since workflows attach at different times; checks
that ran longer than that pass on the first read. A run that ended cancelled counts as pending for two minutes, because
GitHub cancels runs a newer event supersedes and the replacement can take a while to attach; a cancellation nothing
replaced by then is a failure. Every poll and retry backs off from one second to at most one read a minute.

Every check blocks by default: the script cannot tell a required check from an optional one on a repo without branch
protection. When a check fails that the user says is optional, rerun with `--ignore-check <name>` (comma-separated; a
job name, a status context, or `workflow/job`), and say you did. Do not decide a check is optional yourself.

"No CI verdict" covers a PR with no checks or one whose checks were all skipped. When `land` stops with
`E_LAND_NO_CHECKS`, recheck the repo's CI instructions, workflow triggers, and other automatic PR checks. If the repo
really runs no automatic PR checks, resume with `--allow-no-checks` and say why. With it, a PR with no checks at all
merges as soon as it is ready; attached failed or pending checks still block. Otherwise tell the user what the workflows
do; triggering a run (the script's `ci`, where a workflow allows `workflow_dispatch`) is their call.

A check skipped in an Actions run from before the PR left draft is ambiguous. It may be intentionally inapplicable to
the diff, or it may have skipped required validation because the PR was a draft. `land` waits two minutes for a
replacement, then stops with `E_LAND_UNCLASSIFIED_SKIP`; its message does not claim the cause. Inspect the workflow
version at the reviewed head, including the job's `if:` and any upstream change detector, and compare it with this PR's
diff. If that proves the job was intentionally inapplicable independent of draft state, resume with
`--accept-skipped-check <PR:workflow/job>` and report the evidence. Use comma-separated entries for several jobs or PRs.
Each entry applies only to that exact Actions job on that PR, and only while it is `SKIPPED`; a failure or pending run
still blocks, and an all-skipped rollup remains no verdict. Do not use the flag for a job whose condition is unclear or
draft-gated. `--ignore-check` and `--allow-no-checks` have broader effects and are not substitutes for this declaration.

Lines starting with `JJSTACK_` are the ledger. `JJSTACK_PLAN` lists what will land and what will be restacked.
`JJSTACK_LANDED` names each landed PR with its exact landed head and squash commit. `JJSTACK_SUBMITTED` lines report the
restack of the PRs above. `JJSTACK_KEPT` names a branch cleanup kept and why. `JJSTACK_STOPPED` names the PR a run
stopped at and splits the stack into `landed=`, `unknown=`, and `pending=`. Report the landed PRs and their squash
commits from these lines.

When a landing stops, the PRs in `landed=` are merged and those in `pending=` are not. A PR in `unknown=` had `gh pr
merge` run for it without the outcome being confirmed: the merge may have gone through with the confirming read failing,
or it may sit in a merge queue. Check it with `gh pr view` and tell the user what you found; never describe it as not
merged without checking. Show the user the failure and wait for direction. Once the cause is fixed (a failing check
fixed with a new commit and `submit`, say), a general "continue" means rerunning the same command, `--expect` and
`--through` included, with `--resume`. A failure caused by the PR text, such as a missing tag a check looks for, is
fixed by rewording the commit with `amend --bookmark <bookmark> --message-file <file>`; the reworded commit is a new
head, so its checks run again. The PR the run stopped at stays ready rather than draft, since `land` had already marked
it ready to get its checks.

Resuming recognizes the landed PRs at the bottom of the stack by matching local commits to the heads of merged PRs. A
stopped run has usually rebased the next PR off the one it just landed, which takes the landed one out of the stack, so
`--expect` may also start with PRs the plan no longer shows: each is accepted only if GitHub shows it merged into the
default branch, and cleanup then covers its branch and stray local commit too. A `--through` that names a landed PR
means only the restack and cleanup are left to do. A PR the stopped run had rebased locally but not pushed goes through
the same content proof, so an unreviewed local edit stops the run instead of landing. Without `--expect`, resuming only
knows what jj and GitHub show, and landed commits that left the stack are not part of its cleanup.

`E_MERGE_TIP_MOVED` or `E_FAST_TIP_MOVED` means the default branch moved outside the landing, usually someone else's
merge. The guard stopped before merging onto a base the plan had not seen. Confirm with `gh pr view` that a PR listed in
`unknown=` did not merge, then rerun the same command with `--resume`; the new plan starts from the moved branch.

Two shortcuts are never part of a landing without an explicit request from the user, because each collapses review units
the stack exists to keep separate: merging a PR into its parent bookmark (GitHub closes it as merged, the parent absorbs
it, and nothing reaches the default branch), and retargeting a stacked PR to the default branch so one squash swallows
its unmerged parents. The script does neither. It also never passes `--admin`: if branch protection rejects a merge for
missing checks or reviews, it stops, and bypassing that needs its own explicit request.

### Repairing by hand

These come up only when something already went wrong outside the script:

- Never use `gh pr merge --delete-branch` on a stacked PR that is not the top one: GitHub can close a child PR whose
  base branch disappears before its base is edited.
- If a child PR was closed because its base branch was deleted, `gh pr reopen` may not bring it back; check its state.
  The usual recovery is to rebase its bookmark onto the default branch, then `submit`, which opens a replacement PR only
  after you rename the bookmark (it refuses to reopen a branch whose PR was closed).
- A CI run that evaluated a head against a stale base cannot be fixed by rerunning it, because GitHub reuses the
  original event payload. Fire a fresh event the workflow handles instead: a push, or, if it handles `pull_request:
  edited`, a reversible body edit through the REST call under "Default rules".
- GitHub can report `DIRTY` or `CONFLICTING` for a moment after a base edit or force-push. Wait once and re-read before
  treating it as a real conflict.

### After a landing

After the last merge, `land` and `fast-land` clean up unless `--no-cleanup` was given (pass it when the user asked to
keep the branches). When nothing is left above the landing, whatever sat directly on the top PR, normally the empty
working copy, moves onto the landed default branch, so the checkout is coherent without any manual sync. Each landed
branch is deleted only if its PR is `MERGED` into the default branch from this repo at exactly the landed head and no
open PR still uses the branch as base or head; a 404 counts as already deleted. The delete itself is a `git push` with
`--force-with-lease` pinned to the landed head, because GitHub's ref DELETE takes no expected SHA and would remove work
pushed to the branch a moment earlier. The landed local commits are abandoned so they do not linger as stray heads in
`jj log`, except any that other local work outside the stack still builds on.

Every fetch the script runs passes `--config git.abandon-unreachable-commits=false`. By default, `jj git fetch` abandons
commits that only a now-deleted remote branch kept reachable and rebases their descendants away, even when the working
copy or side work sits on top. On jj 0.46 that moved the working copy onto an older default-branch commit, and side work
built on a landed commit silently lost that commit's content. Pass the same option to manual fetches after a landing on
a repo that deletes merged branches automatically.

If a PR landed outside the script (the user merged it in the web UI, say), bring the rest of the stack along with `sync`
(see "Building and updating the stack"). It also handles a stack whose PRs all landed that way: it moves the working
copy onto the landed result and has nothing left to submit.

## Fast path merge

NOTE: Only use this section when the current merge request literally says "fast path" (or "fast-path"), explicitly says
not to wait for CI, or says "merge anyway" in response to a CI wait or refusal. It is a one-shot instruction scoped to
that landing. It does not become a session preference, it does not apply to the next "merge the stack", and a user who
said "fast path" an hour ago has not said it now. When the request says none of these, land with `land` and do not offer
the fast path as a shortcut.

The fast path lands the planned stack prefix bottom-up, one squash commit on the default branch per PR, and never waits
for CI. With no `--through` it lands the whole stack; with `--through` it leaves the PRs above open and restacks them
onto the landed result. It is the script's `fast-land` command. Measured on scode/repotesting on 2026-10-11: about 10
seconds per PR that GitHub can squash as-is and about 15 seconds per PR that needs a restack, so a 30-PR stack takes
roughly 5 to 8 minutes.

Each PR lands in one of two modes, and the switch from the first to the second is one-way for the rest of the landing.
In `github` mode the PR is retargeted once its parent is `MERGED` and squashed as it is, with no rebase and no push. In
`restack` mode it is rebased onto the landed default branch, only its own bookmark is pushed, and it is squashed once
GitHub shows the pushed head. GitHub-only retargeting stops working sooner than it looks: a squash commit is never an
ancestor of the next PR's head, so the merge base GitHub uses stays at the default branch's tip from before the stack,
and a PR conflicts in `github` mode whenever it edits a file or hunk that any earlier PR in the stack created or
changed. A restack never needs conflict resolution in that case, because the squash commit's tree equals the parent PR's
tree.

Before merging anything, `fast-land` simulates the requested landing with `git merge-tree`, prints the mode each PR will
use, and refuses to start if any PR would need a manual conflict resolution. While landing it checks every step against
the simulation: the default branch must still be where the previous squash left it before each PR and each merge
attempt, a restacked commit's tree must equal the prediction before it is pushed, and each squash must produce the
predicted tree. That is what makes skipping CI defensible: apart from a push that races the merge itself, which stops
the landing right after one unpredicted squash, every tree that lands is one the plan predicted. Merges use the same
guard and transient-rejection retry as `land`, and a PR is marked ready only immediately before its own merge, so a PR
the run never reaches stays a draft.

```bash
<skill-dir>/scripts/jjstack fast-land --dir <checkout> --plan-only
<skill-dir>/scripts/jjstack fast-land --dir <checkout> --expect 175,176,177
<skill-dir>/scripts/jjstack fast-land --dir <checkout> --resume
<skill-dir>/scripts/jjstack fast-land --dir <checkout> --through 175 --expect 175 --resume
```

Tell the user the bottom-up PR list that will land, the PRs that will stay open, and where the switch to `restack`
happens, then land with `--expect` and the planned `--through` boundary. The printed plan is an announcement when the
request already covers those PRs. Give the call a timeout of at least a minute per PR. The ledger is the same as for
`land`. Report the landed PRs and their squash commits, then stop, and do not say or imply that later merges will also
use the fast path.

A refusal during planning means the stack is not in the shape the fast path assumes; nothing was merged. Show the user
what the script reported, and do not fall back to `land` on your own. A stop partway is handled as for `land`, except
that resuming puts every pending PR in `restack` mode and predicts each from the head GitHub shows, so a local edit that
never reached GitHub stops the run before anything is pushed. Resuming continues the fast path the user already asked
for and needs no new fast-path request; switching to the CI-gated `land` instead is the user's call. The same `--resume`
is how a stopped `land` continues without CI: the stopped run left the PRs above its last merge rebased locally but not
pushed, which a fresh `fast-land` refuses as a stack GitHub does not show.

## Practical notes

- `bookmark-` in jj revset syntax means "the parent of bookmark". It is useful when you only know the child bookmark and
  want the commit directly below it.
- Do not create bookmarks on empty working-copy commits by accident.
- Do not treat GitHub PR branches as the source of truth. The source of truth is the jj graph plus the bookmark names.
- When in doubt, preserve bookmark names and move them to the right commits. That is what keeps existing PRs updating
  instead of multiplying.

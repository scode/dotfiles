# jjstack accelerator skill interface plan

NOTE: This is a planning note, not a committed interface. It is scoped to how an accelerator tool should interact with
the `jjstack` skill. It intentionally does not cover project setup, packaging, language choice, or how to build the
tool.

## Context

The current `jjstack` skill works, but it asks the agent to keep too much state in prose. The hard part is not that `jj`
or `gh` are slow. The hard part is that a stack is a small state machine, and the agent has to keep re-deriving the
mapping between commits, bookmarks, GitHub PRs, bases, head SHAs, mergeability, and local conflicts.

The goal of the accelerator should be narrow: make the normal jjstack flows boring enough that the skill can spend its
attention on review-unit judgment, PR text, and conflict resolution. The tool should own the repetitive mechanics and
return compact state summaries that are hard for an agent to misread.

This note is based on the existing `lore/20260527-jjstack-accelerator.md` sketch plus a real run in
`/home/scode/git/repotesting` on June 20, 2026. That run created real GitHub PRs:

- single PR: `scode/repotesting#41`
- five-PR stack: `#42` through `#46`
- inserted middle PR: `#47`

The experiment intentionally exercised the normal single-PR flow, a bottom-to-top stack creation flow, top and middle PR
rewrites, descendant conflict resolution, descendant republishing, and GitHub base retargeting.

## What the experiment changed about the plan

The previous sketch was directionally right: the useful boundary is not a giant "do everything" command, but a few
commands that collapse high-friction decision points. The real stack run made a few requirements sharper.

First, the tool needs first-class stack state, not just PR state. In the five-PR run, rewriting layer 2 changed layers
3, 4, and 5. The useful output was not "push succeeded"; it was "these descendant bookmarks moved, these PRs therefore
need a push or refresh, and this is the first conflicted descendant to resolve."

Second, GitHub mergeability can be stale in ways that are not visible from local jj state. After force-pushing rewritten
descendants, GitHub reported later PRs as `CONFLICTING` even though local ancestry was linear and the diffs were clean.
Editing the PR base to the same branch name refreshed GitHub's view and moved the PRs back to mergeable. The tool needs
a `sync-github` operation that treats "base name is already correct" as insufficient; sometimes the useful action is a
same-base refresh.

Third, insertion is a distinct operation. Adding `stack-3a` between `stack-3` and `stack-4` moved downstream commit IDs,
created a new PR, and required retargeting `stack-4` from `stack-3` to `stack-3a`. The tool should not make the skill
manually infer that from raw `jj log` and raw PR metadata.

Fourth, command compatibility matters. `jj log --graph` is wrong on the tested `jj 0.42.0`; graph output is already the
default. Accelerator commands should hide these version-specific footguns from the skill.

## Intended division of labor

The skill should remain the user-facing workflow brain. It should decide what the user is asking for, choose whether a
change is a new review unit or a rewrite of an existing one, write commit messages and PR text, decide when to stop and
ask about ambiguous user intent, and resolve real source conflicts with codebase context.

The accelerator should be the deterministic state-machine helper. It should run `jj` and `gh` in the right order,
validate bookmark names before they can hit the remote, preserve the mapping between bookmarks and PRs, publish exact
bookmarks, detect moved descendants, classify conflicts, retarget PR bases, and refresh GitHub mergeability when the
local stack and GitHub disagree.

The skill should call the accelerator for state and mechanics, then explain the result to the user. The accelerator
should not try to generate prose answers, choose product policy, or hide intent behind one opaque command.

## Command set

### `jjstack preflight`

This should answer: "Can this repo safely run the jjstack workflow?"

It should run the setup checks that are currently scattered through the skill:

- `jj --version`
- `gh auth status`
- jj repo detection, including whether this is a colocated Git repo
- GitHub repository detection
- default branch/trunk detection
- `jj config get user.name`
- `jj config get user.email`
- `jj status`

The output should classify the repo as `ready`, `needs_bootstrap`, `needs_identity`, `dirty_working_copy`, or `blocked`.
When blocked, it should include the next exact command or the reason no command is safe.

### `jjstack status`

This should be the normal "lay of land" command. It should return the stack as a table and as JSON.

For each stack entry, include:

- order from bottom to top
- local change ID and commit ID
- bookmark name
- remote bookmark commit ID, when present
- whether the local bookmark has moved relative to the remote bookmark
- PR number and URL, when present
- GitHub base branch
- expected base branch from local ancestry
- GitHub head SHA
- mergeability state
- check-rollup state
- conflict state

The human output should end with one recommended next action, not a wall of raw logs. Examples:

```text
state: ready_to_create_pr
repo: scode/repotesting
base: main
head: exp/jjaccel-20260620164107/single
review_commit: qzvut...
next: jjstack create-pr --head exp/jjaccel-20260620164107/single --base main
```

```text
state: needs_github_sync
reason: github_reports_conflicting_but_local_ancestry_is_clean
pr: 45
head: exp/jjaccel-20260620164107/stack-4
base: exp/jjaccel-20260620164107/stack-3a
next: jjstack sync-github --pr 45
```

### `jjstack create-pr`

This should cover the common single-PR case.

Inputs:

- head bookmark
- base branch or bookmark
- optional revision, defaulting to the current review commit
- optional path list and commit message when the tool is also asked to create the review commit
- title file
- body file

Behavior:

- validate the bookmark name before mutating anything
- commit only explicit paths when paths are provided
- set the bookmark only after the target revision is known
- push `exact:<bookmark>`
- create the PR with `gh pr create --base <base> --head <bookmark> --title "$title" --body-file <body-file>`
- return PR number, URL, head bookmark, base, and head SHA

The skill should use this when the user asks for a normal single PR and there is no existing PR for the chosen bookmark.

### `jjstack create-stack`

This should cover the normal "publish a stack of PRs" case.

Inputs:

- ordered stack file, bottom to top
- bottom base branch, usually `main` or `trunk()`
- per-entry bookmark, revision, title file, and body file

Behavior:

- verify the ordered local ancestry matches the requested stack
- validate every bookmark name up front
- push exact bookmarks
- create PRs bottom-to-top
- use the previous bookmark as the next PR's base
- stop on the first failure and report which PRs/bookmarks already exist

The output should make partial progress explicit. A failed third PR creation should not leave the skill guessing whether
the first two PRs were created.

### `jjstack update-pr`

This should update an existing PR without changing the bookmark name.

Inputs:

- bookmark or PR number
- mode: `rewrite` or `followup`
- optional path list and commit message

Behavior:

- resolve PR number to head bookmark when needed
- move the working copy to the bookmarked change
- in `rewrite` mode, squash the current edits into the bookmarked change
- in `followup` mode, create a descendant commit and move the same bookmark to it
- report every descendant bookmark that moved as a result
- push the updated bookmark and affected descendants when requested

The skill should still decide whether `rewrite` or `followup` is appropriate. The accelerator should make either path
mechanically safe.

### `jjstack resolve-next`

This should make descendant conflict handling explicit.

Inputs:

- optional starting bookmark

Behavior:

- find the first conflicted descendant in stack order
- report affected bookmarks and paths
- print or run the correct `jj new <change>` step
- after the user or agent resolves the files, provide the exact squash/push continuation

The experiment showed that resolving the first conflicted descendant can clear later descendants when their patches then
apply cleanly. The tool should surface that, but it should not pretend conflicts are resolved until `jj status` and the
stack state are clean.

### `jjstack insert-after`

This should cover inserting a new review unit into an existing stack.

Inputs:

- previous bookmark
- new bookmark
- title file
- body file
- optional path list and commit message

Behavior:

- create the inserted change after the previous bookmark
- rebase descendants through normal jj mechanics
- detect moved and conflicted descendants
- push the inserted bookmark and moved descendants
- create the inserted PR with base set to the previous bookmark
- retarget the immediate downstream PR to the inserted bookmark
- report downstream PRs that do not need a base-name change but may still need a mergeability refresh

The skill should use this when the user asks to split or insert work below an existing PR.

### `jjstack sync-github`

This should reconcile local stack truth with GitHub PR metadata.

Inputs:

- optional stack range, PR number, or head bookmark
- optional `--refresh-mergeability` flag

Behavior:

- compare local bookmark ancestry with GitHub PR bases
- edit PR bases that are actually wrong
- when local ancestry is clean but GitHub reports stale conflict state, run a same-base `gh pr edit --base <base>`
  refresh
- re-read mergeability and check state after the edit
- return before/after states

This command is important because the skill should not have to remember that a no-op-looking `gh pr edit` can be the
right recovery action.

### `jjstack goto-pr`

This should move the local jj working copy to the PR's head bookmark.

Inputs:

- PR number

Behavior:

- read the PR head branch from GitHub
- verify that the head branch maps to a local or fetchable bookmark
- run the jj move/new command needed to work on that change
- report the bookmark and current stack position

The skill should use this for user requests like "go to PR 45" instead of treating the PR number as a jj revision.

## Output contract

Every command should support human output and JSON output. The human output should be concise enough to paste into an
agent transcript. The JSON output should be stable enough for a future skill or wrapper to parse without scraping text.

Human output should always include:

- `state`
- `repo`
- relevant bookmark and PR identifiers
- what changed, if the command mutated state
- one `next` recommendation
- one `reason` when blocked or uncertain

JSON output should use the same vocabulary everywhere:

- `ready`
- `needs_bootstrap`
- `needs_identity`
- `dirty_working_copy`
- `ready_to_create_pr`
- `ready_to_create_stack`
- `ready_to_update_pr`
- `needs_conflict_resolution`
- `needs_push`
- `needs_github_sync`
- `checks_pending`
- `checks_failed`
- `mergeable`
- `blocked`

For stack entries, JSON should include enough information for the skill to avoid another discovery round:

```json
{
  "index": 4,
  "bookmark": "exp/jjaccel-20260620164107/stack-4",
  "local_commit": "abc123",
  "remote_commit": "def456",
  "local_remote_relation": "moved",
  "pr": {
    "number": 45,
    "url": "https://github.com/scode/repotesting/pull/45",
    "base": "exp/jjaccel-20260620164107/stack-3a",
    "expected_base": "exp/jjaccel-20260620164107/stack-3a",
    "head_sha": "abc123",
    "mergeability": "mergeable",
    "checks": "pending"
  },
  "conflict": null
}
```

The tool should explicitly say when no GitHub base update is needed. That is different from failing to check. In the
experiment, one downstream PR kept the same base branch name even though that base branch moved to a new commit.

## Version contract

The skill and tool should assume they can be installed separately. That means the skill must not silently use whatever
`jjstack` binary happens to be on `PATH` and hope the interface still means the same thing.

The tool should expose an interface version that is separate from the tool's release version. The interface version
should only change when the contract between skill and tool changes: command names, required arguments, JSON schema,
state vocabulary, error shape, or semantics that affect what the skill may safely do next.

Every skill-owned invocation should pass an exact expected interface version:

```bash
jjstack --expect-version 1 status --json
```

If the installed tool does not implement precisely that interface version, it should reject before doing any workflow
mutation. The failure should be boring and machine-readable:

```text
state: blocked
reason: interface_version_mismatch
expected_version: 1
actual_version: 2
next: stop_and_report_version_mismatch
```

The skill should stop immediately on this failure. It should tell the user that the installed accelerator does not match
the skill's expected interface version and should not continue by falling back to manual jj/gh commands unless the user
explicitly asks it to bypass the accelerator. A version mismatch means the skill can no longer trust the tool's command
or output contract.

There should also be a cheap read-only version command for diagnosis:

```bash
jjstack version --json
```

That command should not require `--expect-version`, should not inspect or mutate repository state, and should return the
tool release version plus supported interface versions.

## Error contract

When the tool fails, it should give the skill enough information to decide whether to retry, ask the user, repair state,
or stop. A non-zero exit code alone is not enough.

Every expected failure should return structured error output with:

- stable `reason`
- human `message`
- `operation`
- `phase`
- whether any mutation happened before the failure
- affected bookmarks, PRs, revisions, and paths
- command that failed, with secrets redacted
- exit code
- stderr/stdout snippets when useful
- local state summary after the failure when it can be collected safely
- exact `next` recommendation
- whether retry is safe

For example:

```json
{
  "state": "blocked",
  "reason": "push_rejected_stale_remote_bookmark",
  "message": "Remote bookmark moved after local state was read.",
  "operation": "create-stack",
  "phase": "push",
  "mutated": true,
  "safe_to_retry": false,
  "bookmarks": [
    {
      "name": "exp/example/stack-3",
      "local_commit": "abc123",
      "remote_commit": "def456"
    }
  ],
  "prs": [],
  "failed_command": {
    "argv": ["jj", "git", "push", "--bookmark", "exact:exp/example/stack-3"],
    "exit_code": 1
  },
  "stderr_excerpt": "Refusing to push because the remote bookmark has moved.",
  "next": "run jjstack status --json and decide whether to fetch/rebase or stop for user input"
}
```

The tool should distinguish failures that look similar from the outside but require different agent behavior:

- dirty working copy before mutation
- local conflicts after restacking
- stale remote bookmark
- GitHub authentication failure
- GitHub base mismatch
- GitHub mergeability lag
- checks failed
- checks missing but likely still being attached
- interface version mismatch
- unsupported workflow
- internal tool bug

For internal bugs, the tool should still print as much context as it safely can. The skill needs enough detail to write
a useful trace rather than a useless "the tool crashed" note.

## What the tool should run

The accelerator should run jj and GitHub operations sequentially when they touch repo state:

- `jj status`
- `jj log` without `--graph`
- `jj bookmark list`
- `jj commit`
- `jj describe`
- `jj squash`
- `jj new`
- `jj new --insert-after`
- `jj bookmark set`
- `jj git push --bookmark exact:<name>`
- `gh pr create`
- `gh pr edit --base`
- `gh pr view --json ...`

It can parallelize GitHub metadata reads after local jj state has been collected and no jj process is running. It should
not parallelize jj commands in a colocated repo.

It should never run:

- `jj log --graph` on modern jj
- `jj git push --all` for ordinary stack work
- `jj git push --allow-new` on current jj
- `gh pr create --body <inline markdown>`
- shell-expanded PR titles or bodies
- bookmark pushes built from unchecked empty variables

## Skill usage pattern

The skill should treat the accelerator as a trusted mechanic, not as the source of user intent.

For a normal single PR:

1. The skill scopes the change and writes the PR title/body.
2. The skill calls `jjstack preflight` or `jjstack status`.
3. The skill calls `jjstack create-pr`.
4. The skill reports the PR URL and any follow-up state.

For a stack:

1. The skill decides the review units and commit messages.
2. The skill creates or identifies the local review commits.
3. The skill writes an ordered stack file.
4. The skill calls `jjstack create-stack`.
5. The skill reports the PR URLs bottom-to-top.

For review feedback on a stack:

1. The skill calls `jjstack goto-pr` or `jjstack status` to locate the target.
2. The skill applies the requested code or prose change.
3. The skill calls `jjstack update-pr --mode rewrite` or `--mode followup`.
4. If the tool reports conflicts, the skill resolves the first conflicted descendant and calls `jjstack resolve-next`
   until the stack is clean.
5. The skill calls `jjstack sync-github` to retarget bases and refresh stale mergeability.
6. The skill reports which PRs changed and which ones are waiting on checks or review.

## Feedback traces

The skill should have a built-in feedback mechanism for cases where the accelerator was not good enough. This includes
tool bugs, missing functionality for a reasonable jjstack workflow, output that did not give the agent enough
information to choose the next action, and behavior that forced the skill back into manual `jj`/`gh` spelunking.

When that happens, the skill should write a trace under `~/tmp` if and only if `~/tmp` already exists. It should not
create `~/tmp` just to write the trace. The filename should be specific enough to identify the failure later:

```text
~/tmp/jjstack-trace-<slug>.md
```

The slug should be short and descriptive, such as `github-stale-conflict-refresh`, `missing-insert-after-output`, or
`version-mismatch`. It should not include spaces or shell-sensitive characters.

Err on the side of too much detail in the trace. The point is to hand the file to a future agent and let it improve the
skill, the accelerator, or both without having to rediscover what happened.

The trace should include:

- timestamp
- repository path and GitHub repository
- skill version or commit, if known
- expected accelerator interface version
- actual accelerator version output, when available
- user request that triggered the workflow
- intended workflow
- accelerator command invocations
- full accelerator JSON output
- relevant human output
- failed command, exit code, stdout, and stderr, with secrets redacted
- `jj status`
- relevant `jj log` output without `--graph`
- relevant `jj bookmark list` output
- relevant `gh pr view --json ...` output
- stack map before and after the failure, if known
- what the skill expected the accelerator to provide
- what was missing, wrong, or ambiguous
- what manual fallback the skill used, if any
- PR URLs and bookmark names involved
- a short "suggested improvement" section written from the skill's point of view

If `~/tmp` does not exist, the skill should not write the file anywhere else by default. In that case, it should mention
in the final response that it would have written a jjstack feedback trace, but skipped it because `~/tmp` was absent.

## Non-goals

Do not build a `jjstack do-everything` command first. The useful unit is one workflow boundary with a precise state
classification. Keeping those boundaries visible lets the skill explain what happened and recover from partial progress.

Do not bake repo-specific policy into the accelerator. Labels, changelog conventions, review templates, merge policy,
and whether to rewrite or add follow-up commits belong in the skill or the project-specific agent instructions.

Do not make the accelerator resolve source conflicts automatically. It can identify the first conflicted descendant,
move the working copy there, and say what continuation command is needed. The actual code resolution needs repository
context.

Do not treat GitHub as authoritative about local stack shape. jj owns the local history. GitHub owns PR metadata. The
accelerator's job is to compare them and make the mismatch explicit.

## Open questions

The plan still needs design work around how stack files should be shaped, whether command names should be verbs
(`create-stack`) or nouns (`stack create`), and how much mutation should be allowed by default versus requiring
`--execute`. Those are interface questions, not blockers for the core requirement: the skill needs a compact tool that
turns stack mechanics into explicit states and exact next actions.

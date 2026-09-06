# jjstack accelerator sketch

NOTE: This is a historical design note, not a committed interface. It records what the jjstack experiments suggested at
the time, and it is expected to go stale.

## What the experiments showed

The expensive part of the jjstack workflow was not the raw runtime of `jj` or `gh`. The expensive part was the agent
having to rediscover and re-interpret a small amount of state over and over:

- Is this checkout bootstrapped for jj?
- Is identity configured well enough to make pushable commits?
- What is the current working-copy commit, parent commit, trunk, and bookmark layout?
- Which bookmark maps to which GitHub PR?
- Did the base edit take effect?
- Are checks missing because they are still being created, or because no checks will run?
- Is GitHub's `DIRTY` mergeability state real, stale, or just lagging after a force-push?

The experiments also found a different class of failures: shell-level batching is useful, but it makes dumb mistakes
more expensive. An unset variable in a generated bookmark name can create real remote branches. A metadata check that
prints output but does not gate the next command can look safer than it is. A command that waits for checks can report
"no checks" during the exact window where GitHub has not attached new check runs yet.

The acceleration target is therefore not "run every command in parallel". jj and GitHub state mutations still have to be
ordered. The target is to collapse each decision point into one tool call that returns the state the agent actually
needs, in a shape that is hard to misread.

## Shape of the tool

The tool should be a small `jjstack` command available on `PATH`. It should assume the caller is an agent that needs a
compact state summary before taking exactly one next useful action.

The command should have two output modes:

- human-readable text for the transcript
- JSON for future callers that can parse it reliably

The human output should still be structured. Do not dump raw `jj log`, raw PR JSON, and raw check JSON as separate
blobs. The whole point is to do the boring interpretation in code.

The common output shape should be:

```text
state: ready_to_publish
repo: scode/dotfiles
trunk: main
working_copy: clean
current_commit: @
review_commit: @-
bookmark: pr/example
pr: none
risk: none
next: jjstack publish --bookmark pr/example --rev @-
```

For failures or uncertainty:

```text
state: blocked
reason: github_mergeability_dirty
pr: 40
head: speedtest/foo-5
base: main
local_ancestry: clean
checks: none_attached_yet
next: jjstack recheck-pr --pr 40
fallback: jjstack rewrite-bookmark --bookmark speedtest/foo-5 --base main
```

## Commands worth building first

### `jjstack lay-of-land`

This is the main discovery accelerator. It should answer the question: "What is true right now, and what is the next
safe workflow action?"

It should collect, sequentially where jj/Git metadata requires it:

- repo root and whether the checkout is a colocated jj repo
- default GitHub repo and default branch
- `jj` version and basic repo health
- `gh` auth/repo availability
- jj `user.name` and `user.email`
- working-copy status, including changed paths
- current `@`, parent `@-`, trunk, local `main`, and `main@origin`
- bookmarks on `@`, `@-`, ancestors, and descendants that look stack-related
- open PRs whose head refs match local bookmarks
- PR state/base/head/head SHA/mergeability/check rollup for those PRs

It should not run arbitrary Git commands concurrently with jj repo-state commands in a colocated checkout. It can
parallelize GitHub API calls after it has determined the set of PRs and refs to inspect.

It should classify the result into a small set of states:

- `needs_bootstrap`
- `needs_identity`
- `dirty_working_copy`
- `ready_to_commit`
- `ready_to_publish`
- `ready_to_create_pr`
- `ready_to_update_pr`
- `ready_to_merge`
- `needs_retarget`
- `checks_pending`
- `checks_failed`
- `github_mergeability_lagging`
- `blocked`

The command should prefer explicit "next" recommendations over broad summaries. The agent can still decide differently,
but it should not have to infer the obvious next step from six blocks of output.

### `jjstack publish`

This should handle the common "commit/bookmark/push" boundary, where shell mistakes have been costly.

Inputs:

- one or more `(bookmark, rev)` pairs
- optional exact paths for commit creation
- optional commit message

Behavior:

- validate every bookmark name before mutation
- reject empty components, leading `-` components, whitespace, glob-like surprises, and duplicate bookmark names
- print the exact bookmark names before pushing
- commit only the requested paths when paths are provided
- set bookmarks only after the commit or target rev is known
- push exact bookmarks, never `--all`
- return pushed bookmark names and target commit IDs

This command is not just a wrapper around `jj git push`. Its value is that it makes the dangerous string construction
and sequencing boring.

### `jjstack create-prs`

This should create PRs from an explicit stack map:

```text
main <- pr/one <- pr/two <- pr/three
```

Inputs:

- repository
- ordered list of bookmark names
- base branch for the bottom PR
- title/body files per PR, or a structured file containing all PR metadata

Behavior:

- verify each bookmark exists locally and remotely
- verify no open PR already uses the intended head unless the caller asked to update
- create PRs bottom-to-top
- return PR number, head, base, URL, and head SHA for each PR

It should not solve project-specific policy such as changelog labels or body markers. That belongs elsewhere.

### `jjstack pr-state`

This is the replacement for ad hoc `gh pr checks --watch` usage.

Inputs:

- PR number or head bookmark
- expected base
- expected head
- optionally expected head SHA

Behavior:

- read `state`, `baseRefName`, `headRefName`, `headRefOid`, `mergeStateStatus`, `mergeable`, and `statusCheckRollup`
- verify expected base/head/head SHA when provided
- classify checks as `pending`, `success`, `failed`, `missing_but_expected`, or `not_configured`
- treat "no checks" shortly after a push/base edit as pending when checks were previously observed for the PR or repo
- distinguish "GitHub has not attached checks yet" from "the repo appears to have no checks"

This command should be conservative. If it cannot tell whether missing checks are okay, it should say so.

### `jjstack merge-step`

This should perform one safe merge step, not the whole stack by default.

Inputs:

- PR number
- expected head bookmark
- expected base
- optional list of downstream bookmarks and PRs

Behavior:

- verify PR state/base/head/head SHA
- verify checks and mergeability are acceptable
- squash merge with `--match-head-commit`
- fetch origin
- move local `main` or the configured trunk bookmark to the remote result
- if downstream information was provided, rebase the remaining local stack while preserving the relative order of the
  still-open PRs
- push exact downstream bookmarks that moved
- edit only the next bottom PR's GitHub base
- return the next PR that needs checks/mergeability evaluation

The command should not delete remote branches unless it has checked that no open PR still uses the branch as a base.

### `jjstack recover-mergeability`

This is for the observed "GitHub says `DIRTY`, local jj says clean" case.

Inputs:

- PR number
- expected head bookmark
- expected base

Behavior:

- wait once and re-read PR metadata
- verify local ancestry and diff against the intended base
- if local state is clean but GitHub remains dirty, rewrite only that bookmark onto the current intended base
- push that bookmark explicitly
- re-read PR metadata and report whether GitHub now sees the PR as mergeable

This should be a recovery command, not part of the happy path.

## What not to put in the tool

Do not hide all of jjstack behind a single `jjstack do-it-all` command at first. The workflow benefits from a small
number of explicit boundaries:

- discover state
- publish refs
- create PRs
- wait/check PR state
- merge one step
- recover a specific failure

That keeps the agent in control of intent while removing the noisy mechanics. It also makes failures easier to explain.

Do not embed repo-specific policy in the jjstack tool. For example, a repository may require `changelog: skip` in PR
bodies. The accelerator can report that checks failed and surface the log. It should not assume every repo wants the
same changelog convention.

Do not rely on `gh pr checks --watch` as the primary check primitive. The experiments suggest that newly pushed or
retargeted PRs can briefly report no checks. The tool should observe PR metadata and check rollups together, with a
short "checks expected but not attached yet" state.

## Why this is likely worth doing

The current skill can be made safer, but it is still a prose program interpreted by an agent. The agent has to remember
which commands are safe to batch, which output needs inspection, which GitHub states are transient, and which refs are
dangerous to delete. That is a lot of control flow to keep in natural language.

A small accelerator can turn the known hard parts into deterministic checks:

- validate generated bookmark names before they can hit the remote
- classify GitHub state in one place
- keep the PR/bookmark/base mapping explicit
- make expected next actions obvious
- reduce tool calls without pretending state mutations are parallel

The best first version is `jjstack lay-of-land` plus `jjstack pr-state`. Those two commands attack the highest-friction
parts without taking over mutation. After that, `jjstack publish` and `jjstack merge-step` are the likely biggest wins.

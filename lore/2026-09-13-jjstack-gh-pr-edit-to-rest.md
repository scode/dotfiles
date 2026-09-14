# jjstack: switching PR edits from `gh pr edit` to the REST API

NOTE: Historical decision record, written 2026-09-13 about a failure observed the same day. It explains why the jjstack
skill edits PR title, body, and base through `gh api` rather than `gh pr edit`, and what the switch was tested against.
The current rule lives in SPEC.md and in the skill itself; this entry is the backstory. Not maintained.

## What broke

While landing a two-PR stack in scode/saltybox with the jjstack skill, the retarget step after the parent merged failed:

```
$ gh pr edit 252 -R scode/saltybox --base main
GraphQL: Projects (classic) is being deprecated in favor of the new Projects experience,
see: https://github.blog/changelog/2024-05-23-sunset-notice-projects-classic/.
(repository.pullRequest.projectCards)
exit=1
```

Every form of `gh pr edit` failed the same way: `--body-file`, `--title`, `--base`, even a no-op `--base main` on a PR
already based on main. The PR was never modified. Everything else the skill uses (`gh pr create`, `gh pr view`,
`gh pr ready`, `gh pr merge`, `gh pr list`, `gh api`) worked in the same session on the same PR.

The failure sat in the worst possible place. The parent PR had already merged, and the child was still based on the
merged branch, so the landing stopped half done. The skill's mandatory retarget-before-push ordering could not be
honored at all.

## Why

GitHub removed the classic Projects `projectCards` field from its GraphQL API after the 2024 sunset notice. `gh pr edit`
reads the PR through GraphQL before writing anything, and on older builds that query asks for `projectCards`
unconditionally, whether or not any project flag was passed. The read fails, so no write ever happens. The command is
dead on those builds regardless of arguments.

The machine was running `gh` 2.45.0, the Ubuntu 24.04 (noble) package from `noble-updates/universe`. Nothing installed
depended on it at the apt level; it was just the version apt had. Upstream `gh` was at 2.100.0 by then. The fix landed
in 2.82.1 (cli/cli PR 11987, tracked in issues 11983 and 11986); comments on those issues report 2.82.0 and every
earlier release still failing, and Debian bookworm's 2.23.0 and Alpine's 2.72.0 hitting the same wall. The upstream
maintainers' answer to each report was "upgrade".

So this is a "distro ships an old tool" problem, in the sense that a current `gh` does not have it. It is also a
"GitHub broke old clients server-side" problem, in the sense that the same binary worked one day and not the next, with
no local change. The second framing matters for the decision below.

## The decision

Two fixes were on the table: upgrade `gh` on this machine (Homebrew was available, and GitHub's own apt repo is the
other option), or make the skill stop depending on `gh pr edit`.

I chose to do the second regardless of the first. The skill runs on whatever host the session is on, and any Ubuntu
24.04 box with the stock package hits this. Telling the agent "upgrade gh" mid-landing is a host setup decision the
skill is not in a position to make, and a skill that only works after the user has fixed their tooling is a skill that
fails at the least convenient moment. The REST endpoint (`PATCH /repos/{owner}/{repo}/pulls/{number}`) is the same
operation without the GraphQL read; it works on old and new `gh` alike, it is one line, and the skill already used
`gh api` for branch deletion, so it introduced no new tool. The user's instruction was to write this up as a SPEC.md
requirement: the skill must work with a `gh` older than the fixed version, and it uses REST rather than asking for an
upgrade.

The alternative of keeping `gh pr edit` as the primary path with a REST fallback was rejected. The failure is
deterministic on the affected versions, so the primary path would be a known-dead step sitting in the middle of the
landing sequence, and the fallback would have to teach every agent to recognize the error text. Replacing the call
outright is simpler and has no failure mode to document beyond the REST call's own.

The shape that was verified on saltybox PR 252 before the skill changed:

```bash
gh api -X PATCH "repos/$repo/pulls/$pr" -F "body=@$body_file"
gh api -X PATCH "repos/$repo/pulls/$pr" -f "base=$default_branch" --jq .base.ref
```

The `-F "body=@file"` form preserves the skill's no-shell-escaping rule for PR text: `gh api` reads the file itself and
sends the contents as the field value.

## What was tested

After the rewrite, four Sonnet subagents ran the skill against scode/repotesting, a repository that exists for exactly
this kind of throwaway PR traffic. Each got a fresh clone, its own file names and bookmark prefix, and a task that
exercised one of the edit paths:

- Normal landing of a two-PR stack (PRs 162 and 164). Both merged as separate squash commits; the child was retargeted
  to main via REST between merges; cleanup coped with GitHub having auto-deleted the branches.
- Rewording the middle PR of a three-PR stack (PR 170) with a body containing backticks, `$(date)`, `$HOME`, both quote
  kinds, and a blank line. Title and body verified byte-for-byte on GitHub afterwards.
- Inserting a PR into the middle of a two-PR stack (PRs 163, 165, 167). Bases verified as main, A, B; the retarget ran
  before the push of the rewritten descendant, as the skill requires.
- Fast path merge of a three-PR stack (PRs 166, 169, 171). All merged; the mergeable poll after each retarget took about
  a second.

None of the four ran `gh pr edit`. All four repaired jj identity after `jj git clone --colocate`, which the skill
already covers, and noted that the precondition check for identity is per-repo rather than per-session; that is a
pre-existing wrinkle, not something this change introduced.

## A wrinkle the evals found

The fast path eval hit HTTP 422 on its first retarget:

```
Validation Failed: A pull request already exists for base branch 'main' and head branch 'eval/fast-20260913-2'
```

repotesting has "automatically delete head branches" on. When PR 166 merged, GitHub deleted its branch and then, on
its own and asynchronously, moved PR 169 (based on that branch) to main. The skill's explicit PATCH landed inside that
window, and GitHub reported the collision as a uniqueness error even though the PR ended up exactly where the skill
wanted it. The agent re-read `baseRefName`, saw main, and carried on, which was the right call.

I checked that a plain no-op retarget (base already main, nothing in flight) returns 0, so the race is the only trigger.
The landing snippets now fall through to a `baseRefName` read on a failed retarget and continue when it already matches
the target. Whether `gh pr edit` on a fixed `gh` would have shown the same race is unknown; it goes through GraphQL's
`updatePullRequest`, which may or may not apply the same uniqueness check. It did not come up before because on this
machine `gh pr edit` never got as far as a write.

## Loose ends

The two evals that were told not to merge left six draft PRs open in repotesting (163, 165, 167, 168, 170, 172),
alongside older eval PRs already sitting there. That repository is meant to accumulate such things.

The `gh` on this machine is still 2.45.0. Upgrading it is a separate decision and would not change the skill's
behavior either way.

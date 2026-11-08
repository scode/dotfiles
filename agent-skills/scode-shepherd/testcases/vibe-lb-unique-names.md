# Test case: globally unique account names in vibe-lb

A real goal, run once on 2026-11-07, kept so that it can become an eval. It is smaller than the patreon-buddy case: one
identity rule (an account name picks out one account across both providers) carried through storage, the management API,
the CLI and a TypeScript client, with a schema upgrade and a breaking API change. It took about 1h41m end to end and
about $9 at list prices (see the baseline below).

NOTE: Nothing here is an eval yet. It records the starting point, the ask, and what happened, so that an eval can be
written later. Unlike the patreon-buddy case, the goal was planned in Claude Code, not Codex, and the user stepped in
once mid-run (see "What went wrong"); an eval has to decide whether to replay that intervention.

## Starting point

- Repository: `scode/vibe-lb` (private), checkout at `main` `4c2cfe1ac36cf612039a617c8aa5725987462622` ("feat: control
  claude session affinity in the shell (#276)", 2026-11-07). The `TODO.md` item "Make account names globally unique
  across providers" (added 2026-11-01) is the goal.
- Skills: dotfiles `main` at `27afb8058e202acc99070a1f699d2838ccfa6562`, which has the fixes from the patreon-buddy run
  (#505 through #509) and not the one this run led to (#511).
- Planner: an interactive Claude Code session on `claude-opus-5-5`, in the repository checkout. It was a long-lived
  session with weeks of unrelated history, which made planning more expensive than it needed to be.
- Supervisor: interactive Codex on `gpt-6.1-sol` high, in a separate clean clone of the same repository on `main`.
- Tooling on the host: stable Rust, Node 24 or later, and dprint 0.63.0 (the version the repository's CI pins; the
  supervisor installed it privately under the run directory because the shared one was newer). The run needs no live
  credentials and must not touch the installed service or its database: every test uses fresh temporary state.

## The ask

The original conversation started as a question and went through one design question before planning. This is the same
ask, cleaned up, in the order to give it:

1. In the planning session, in the checkout:

   ```text
   There should be a TODO item about making account names the primary key instead of (provider, account name). What
   are the open questions?
   ```

   The session finds the `TODO.md` item and its two open questions: what happens to installations that already have the
   same name under both providers, and whether account-identifying requests drop `provider` or keep it as a check.

2. Then ask it to lay out the second question's options (drop `provider`; keep it required as a consistency check; keep
   it optional and validated), and choose dropping it.

3. Then:

   ```text
   $scode-build-goal shepherd address the TODO item. We're going with dropping provider. Don't handle existing accounts
   with duplicate names: a database that has them fails at startup, which is fine, since the only installation has
   already renamed its duplicates.
   ```

   Give these answers to the up-front questions:

   - Schema version: bump to v12 if it has any benefit at all; do nothing worse just to avoid a bump. (The planner
     recommended no bump; the user overruled it.)
   - Everything else as proposed: reviewer `gpt-6.1-sol` high, worker `gpt-6.1-terra` high, the eight repository checks.

4. Then, in a Codex session on `gpt-6.1-sol` high, start the run on the goal file it wrote:

   ```text
   $scode-shepherd <goal file>
   ```

## What a good result delivers

From the goal file's acceptance criteria, in short:

- No two configured accounts share a name, whatever their providers. Login and rename refuse a name any other account
  holds, inside the write transaction, and the database enforces it with a unique index on credential names.
- The state database moves to schema v12. A database whose credentials already hold one name under two providers fails
  to open; there is no migration or rename tooling.
- Six management requests (priority, the priority batch, enable, delete, rename, refresh) and three CLI commands
  (`accounts set`, `delete`, `rename`) identify the account by name alone. A body that still sends `provider` is
  rejected like any other unknown field, which in this codebase is 422 `invalid_request`. Login, routing and the
  provider-in-path endpoints keep `provider`.
- The shell sends the new request shapes and keeps working.
- Specs, API docs, error tables, CLI help, `API_REVISIONS.md`, the `TODO.md` tick and the repository work log all match.
- All PRs are draft, stacked, and unmerged; the API and shell PRs must land together.

## Baseline: what happened on 2026-11-07

The run finished with three approved draft PRs in 1h16m from `$scode-shepherd` to the supervisor's final report.
Planning the goal before that took 24 minutes, and the supervisor's own planning took 4.

| PR   | Scope                                          | Worker sessions | Review rounds | Escalations | Head commit                                |
| ---- | ---------------------------------------------- | --------------- | ------------- | ----------- | ------------------------------------------ |
| #278 | schema v12, unique index, transactional guards | 1               | 1             | 0           | `345f82f0658345165c3d21da730b80783c590c59` |
| #279 | name-only API requests and CLI commands        | 3               | 1             | 1           | `9b2993a104ea02c5b86e20ca4d9ef6681f34193c` |
| #280 | shell request bodies                           | 2               | 2             | 0           | `00a5ad4a5e9147866feabf2f02011e05ae0289b8` |

The terra worker finished every item; nothing was switched to sol. Every role together used 53.3M input tokens (1.56M of
them uncached) and 155k output, about $9.09 at list prices on 2026-11-07. No request exceeded the 272K long-context
threshold. By role:

| Role                                                         | Model    | Cost      |
| ------------------------------------------------------------ | -------- | --------- |
| Planning the goal (Claude Code session and its review agent) | opus 5.5 | $3.49     |
| Supervisor, planning the stack (thread and sub agents)       | sol      | $0.91     |
| Supervisor during the run (thread and sub agents)            | sol      | $0.87     |
| Workers                                                      | terra    | $2.55     |
| Worker sub agents (commit-message readers)                   | terra    | $0.14     |
| Reviewers                                                    | sol      | $1.13     |
| **Total**                                                    |          | **$9.09** |

What went wrong, and what each led to:

- The planner's outline said a body still sending `provider` would get 400 `invalid_request`, but the existing decoder
  returns Axum's 422 for unknown fields, as `docs/api.md` and the tests say. The planning review did not catch it. The
  first worker on #279 reported replan over the contradiction, correctly. The supervisor then read the planner's 400 as
  the user's requirement and changed the item to map every 422 decode error to 400 on the six routes, a consumer-visible
  change nobody asked for. The user forwarded an outside observation that acceptance criterion 3 only says "rejected
  like any other unknown field"; the supervisor stopped the runner mid-session, corrected the plan and the item's
  guidance, and the third session delivered 422. No skill change was made for this: it was the supervisor's judgment,
  misled by a wrong factual claim under a heading that also carried user requirements. An eval can check whether a
  supervisor keeps planner claims and user requirements apart without the intervention.
- `SKILL.md` said workers never write in the run directory, which is false (each writes its report there). The
  supervisor turned that into a rule for every worker that contradicts the runner's own report instruction. The terra
  workers wrote their reports anyway. Fixed in #511.
- Sol's first review of #280 found a real bug: renaming an account removed a stale cached card with the new name only
  within the same provider, leaving two cards with one name. The next worker session fixed it and round 2 approved.
- One minor finding stayed in #279 (array-form test fixtures that still include `provider`), because approval needs only
  zero blocking findings.
- `shepherd status` misses the tokens of a worker session that was killed mid-turn, since it reads usage from the
  session's last completed turn. The tally above counts it from the rollout.

An eval built on this case can compare against those numbers: escalations, worker sessions per item, review rounds,
whether the supervisor resolves the 400/422 contradiction toward the user's criterion without help, wall time, and cost.

## Where the original artifacts were

On the host that ran it, and not guaranteed to survive: the Claude Code planning session (in the vibe-lb checkout's
project history, 2026-11-07 06:05 to 06:29 Pacific), the Codex supervisor session
`01a1b66a-213a-7720-8149-7fbfefd2751c`, the goal file `~/git/vibe-lb-claude-unique-account-names-goal.md`, and the run
directory `~/git/vibe-lb-claude-unique-account-names-goal-shepherd/` (plan, journal, planning reports, and every
session's prompt and events). A readable copy of the plan is at
<https://snippets.scode.org/s/vibe-lb-unique-names-shepherd-plan-2026-11-07/>. The full token and cost tally, session by
session, is at <https://snippets.scode.org/s/vibe-lb-unique-names-shepherd-cost-2026-11-07/>.

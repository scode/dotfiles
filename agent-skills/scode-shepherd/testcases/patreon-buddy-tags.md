# Test case: tag subscriptions in patreon-buddy

A real goal, run once on 2026-11-06, kept so that it can become an eval. It is non-trivial without being hard: a typed
identity change that crosses every layer of a Rust app, a stored-state format change that must preserve existing data, a
new Patreon request shape, and UI work with a headless browser check. It is not cheap to run: about two hours and tens
of millions of worker input tokens (see the baseline below).

NOTE: Nothing here is an eval yet. It records the starting point, the ask, and what happened, so that an eval can be
written later.

## Starting point

- Repository: `scode/patreon-buddy` (private), checkout at `main` `851d5a9d8722d32ffaf3e9fcf534f2edf2a3227e`
  ("feat(macos): show build details in about (#143)", 2026-11-03).
- Skills: dotfiles `main` at `72b7ecc64cf57bacd1b88f87466666c4a804d6dd`, which has scode-build-goal's shepherd mode and
  scode-shepherd's goal-file support, and none of the fixes this run led to (dotfiles #505 through #509).
- Supervisor: interactive Codex on `gpt-6.1-sol` high, in the checkout.
- An installed, logged-in patreon-buddy session on the host, whose account is an active member of at least one creator
  that uses post tags. Planning used it for read-only feasibility probes (Patreon refused plain HTTP clients with 403;
  the app's Safari TLS emulation worked). The run itself does not need it, and its tests use synthetic data only.

## The ask

The original conversation wandered: an assessment question, a follow-up widening it to filters in general, a detour
about getting the auth token to the agent for probes, then the request to plan it. This is the same ask, cleaned up, in
the order to give it:

1. Ask for the assessment, so the session reads the code and probes Patreon before planning:

   ```text
   Assess how easily we can support subscribing to a Patreon creator's tags in addition to collections, e.g.
   https://www.patreon.com/cw/CREATOR/posts?filters%5Btag%5D=SOME+TAG. Filters in general might be worth having, but
   tags are the case that matters. Use the installed patreon-buddy session for read-only probes; don't print the token
   or change any inbox state.
   ```

   Use a real creator and tag the account can see; one whose tag spans more than one page of 20 posts exercises
   pagination. The original example's tag had 49 posts over three pages.

2. Then:

   ```text
   $scode-build-goal shepherd add support for this
   ```

   Give these answers to the up-front questions (they were the recommended options):

   - Scope: tags first, with an extensible internal subscription model, a "Subscriptions" screen, and the existing
     collection CLI syntax preserved; other filters explicitly unsupported for now.
   - Existing data: preserve current collection subscriptions and completion history through the state-format change,
     which needs a reader for the existing version-2 state.
   - Reviewer: `gpt-6.1-sol` high. The worker stays the default, `gpt-6.1-terra` high.

3. Then start the run on the goal file it wrote:

   ```text
   $scode-shepherd <goal file>
   ```

The original also asked, between steps 2 and 3, for a summary of the product decisions; it changed nothing.

## What a good result delivers

From the goal file's acceptance criteria, in short:

- A pasted creator posts URL with exactly one `filters[tag]` follows that tag. Exact decoded tag text is the identity,
  scoped to the creator's campaign, and creators with same-named tags stay distinct. Unsupported, duplicate, and empty
  filters are errors.
- Tags and collections share one inbox, sync, completion history, and every add mode. A post seen through two
  subscriptions has one done state.
- An empty tag is a valid, caught-up subscription; a missing creator membership is an error.
- Existing version-2 state converts without losing any collection, status, metadata, or completion.
- The management screen is called Subscriptions; the existing `collections` and `--collection` CLI syntax still works.
- Headless browser coverage follows a tag through its lifecycle, without live credentials.
- All PRs are draft, stacked, and unmerged.

## Baseline: what happened on 2026-11-06

The run finished with four approved draft PRs, all approved in their first review round, in 1h54m from `$scode-shepherd`
to the supervisor's final report:

| PR   | Scope                                                             | Worker sessions | Escalations | Head commit                                |
| ---- | ----------------------------------------------------------------- | --------------- | ----------- | ------------------------------------------ |
| #144 | accept terminal `links: null`; cache creator metadata             | 1               | 0           | `e340a2023bb1a42beab79bd8f3427949f7d04157` |
| #145 | typed subscription identity, version-2 conversion, rename         | 3               | 2           | `73ffba8c31c9864312f0a0f13d61be9c2529081d` |
| #146 | tag subscriptions end to end                                      | 2               | 1           | `95bb8139d9082bd255751d33237e36415252b605` |
| #147 | keep creator names visible beside long tags at 50 columns (fixup) | 1               | 0           | `f5c0bb348dffe0acc8de958d8b55f564af2958b3` |

Every role together, from the assessment through the final report and including every sub agent, used 81.6M input tokens
(1.87M of them uncached) and 208k output, about $11.97 at OpenAI's standard list prices on 2026-11-06 (sol $2.00 input,
$0.10 cached, $10.00 output per million; terra $1.00, $0.05, $5.00), including the long-context rate on 23 worker
requests over 272K tokens. By role:

| Role                                                                  | Model | Cost       |
| --------------------------------------------------------------------- | ----- | ---------- |
| Planning (assessment, scode-build-goal, its sub agent)                | sol   | $1.05      |
| Supervisor during the run                                             | sol   | $1.46      |
| Supervisor sub agents (stack boundaries, diagnosis, audits)           | sol   | $0.82      |
| Workers                                                               | terra | $2.53      |
| Workers (items 3 and 4, after the switch)                             | sol   | $3.75      |
| Worker sub agents (self-commissioned reviews, commit-message readers) | both  | $0.64      |
| Reviewers                                                             | sol   | $0.92      |
| Long-context surcharge on worker requests (estimated per request)     | both  | $0.79      |
| **Total**                                                             |       | **$11.97** |

What went wrong, and the dotfiles PR each led to:

- The `gpt-6.1-terra` worker gave up twice on broad items after a minute or two of reading, reporting blocked with no
  obstacle beyond the item's size (#505). The supervisor switched the last two items to `gpt-6.1-sol`. The plan had
  described the cross-layer identity change only as constraints, not as types and signatures (#508).
- The repository's `require-main-base` workflow fails every PR not based on `main`, on purpose. Nothing in the goal or
  plan said so, and the worker on #145 reported replan over it (#507).
- A worker appended its own DECISION to the supervisor's journal (#506).
- Workers on #145 and #147 had a sol reviewer check their diff before reporting done, duplicating the runner's review
  (#509).
- The supervisor's own final audit found the narrow-width gap that three item reviews had passed, and added #147.

An eval built on this case can compare against those numbers: escalations, worker sessions per item, whether the cheap
worker finishes every item, wall time, and tokens.

## Where the original artifacts were

On the host that ran it, and not guaranteed to survive: the Codex session `01a1b3f7-5993-7e22-8a56-0fedf325927a`, the
goal file `~/git/patreon-buddy-1-tags-goal.md`, and the run directory `~/git/patreon-buddy-1-tags-goal-shepherd/` (plan,
journal, and every session's prompt and events). A readable copy of the plan is at
<https://snippets.scode.org/s/patreon-tags-shepherd-plan-2026-11-06/>. The full token and cost tally, rollout by
rollout, is at <https://snippets.scode.org/s/patreon-tags-shepherd-cost-2026-11-06/>.

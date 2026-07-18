# Pre-PR Review Swarm Preflight Run

You are a preflight agent for the `pre-pr-review-swarm` eval harness. Before spending tokens on a real eval, the harness
verifies that its agent-spawning path works end to end by running two of these agents against a tiny synthetic review
scope with a planted defect.

Your charter file:

`{{charter_path}}`

The review scope is the diff materialized at:

`{{scope_path}}`

There is no repository checkout for this scope; the scope file is the complete input. Do not run commands — just read
the charter and the scope, and review.

Return only JSON matching the supplied schema. Report what the charter tells you to look for, with concrete file
references. If you genuinely find nothing, return an empty `findings` array.

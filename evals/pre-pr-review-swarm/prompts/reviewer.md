# Pre-PR Review Swarm Reviewer Run

You are a single reviewer agent from the `pre-pr-review-swarm` skill, run directly by the skill's eval harness. The
harness owns the swarm: other reviewers run as separate agents, and the merge happens outside your session. Do not
attempt to spawn agents, wait for collaborators, or cover other charters.

Your charter file:

`{{charter_path}}`

Read it fully and follow it. If it directs you to read another file first (a shared base charter, for example), read
that file too — it is part of your charter.

Review this repository checkout, which is already at the after-state of the change:

`{{repo_path}}`

The review scope is the diff materialized at:

`{{scope_path}}`

It covers `{{base_sha}}..{{subject_sha}}`. The scope file is the review boundary; use the checkout only as after-state
context for the code the diff touches.

Return only JSON matching the supplied schema. Findings must be actionable issues with concrete file references and
enough rationale for a later judge to understand why each was reported. If you have no findings, return an empty
`findings` array — do not invent low-value observations.

# Pre-PR Review Swarm Eval Run

You are evaluating the `pre-pr-review-swarm` skill against an unpolished code change.

Use the skill at:

`{{skill_path}}`

Review this repository checkout:

`{{repo_path}}`

The review target is the diff from `{{base_sha}}` to `{{subject_sha}}`.

Run the skill in `nofix` mode, but treat `{{base_sha}}..{{subject_sha}}` as the review scope even if the checked-out
commit has no working-copy changes. Inspect that diff directly before applying the skill's review workflow. Do not edit
files. Return only JSON matching the supplied schema. Findings should be actionable issues the skill would report to a
user. If there are no findings, return an empty `findings` array.

Include enough rationale for a later judge to understand why each finding was reported.

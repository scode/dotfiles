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

For each finding, set `reviewers` to the reviewer charter names that surfaced it. Preserve attribution through the
skill's merge/deduplication step: a finding kept after merging same-location reports from several reviewers lists every
contributing reviewer, not just the one whose wording survived. This attribution is what lets a later analysis measure
which reviewers earn their cost, so do not drop it during aggregation.

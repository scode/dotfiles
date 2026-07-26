# Pre-PR Review Swarm Eval

The user explicitly invokes the candidate `pre-pr-review-swarm` skill in `nofix` mode. Read the candidate skill at:

`{{skill_path}}`

Follow that skill as the coordinator. Do not substitute an installed copy, flatten the charters into your own context,
or review the change by yourself.

The eval harness has already fixed the review boundary so baseline and candidate runs see the same change. Use this
scope file instead of selecting or materializing another scope:

`{{scope_path}}`

The selected scope label is `{{scope_label}}`. The repository checkout at `{{repo_path}}` is aligned with the scope's
after-state. Use it only as the skill permits for after-state context. Do not edit the checkout.

Return only JSON matching the supplied schema. `reviewer_execution` must account for every charter the skill considered:
use `completed` for a reviewer that ran and `skipped` when the skill deliberately excluded it. Record how many passes
each completed reviewer used, with the initial agent turn counted as one and each same-agent continuation counted as one
more. An internal sweep within a turn is not another pass. Use the reviewer names defined by the candidate skill. Every
finding must name the reviewer or reviewers that surfaced it.

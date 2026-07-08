---
name: scode-commit-msg-reviewer
description: >
  Use only when the user explicitly invokes scode-commit-msg-reviewer by name, asks for commit messages to be reviewed
  by a fresh-context reviewer, or their standing instructions say to always use this skill. Do not trigger merely
  because a commit or PR is being created. Gates commit messages, PR titles/descriptions, and equivalent VCS prose
  through a fresh-context reviewer subagent; after the first signal, applies to all commit-message-like content for the
  rest of the conversation. Also use when the user says "scode-commit-msg-reviewer feedback: ..." to record feedback
  about how this skill performed.
---

# Scode Commit Message Reviewer

Every commit message, PR title, PR description, or equivalent VCS prose (jj/Sapling change descriptions, stacked-PR
bodies) you produce must be approved by a fresh-context reviewer subagent before you use it. The reviewer's charter is
`reviewer.md` next to this skill file; keep this file lean and put review judgment there.

Do not run this skill just because a commit or PR is being created and the skill happens to be available. It activates
only on an explicit signal: the user invokes it by name, asks for this kind of review, or has standing instructions
saying to always use it. Once triggered, it applies to every commit-message-like text you produce for the rest of the
conversation — the user does not re-invoke it per commit. Skip the review only when the user dictated the exact final
text themselves.

## Review loop

1. Draft the message yourself, following the target repo's conventions.
2. Materialize two temp files: the candidate message, and the diff the message describes.
3. Spawn a subagent with fresh context. Its prompt contains only: an instruction to read `reviewer.md` (give the
   absolute path) and follow it, the two temp file paths, the repo root, paths to the repo's own convention files
   (CLAUDE.md, AGENTS.md, CONTRIBUTING, or similar) if any exist, and — when the user gave explicit instructions about
   the message's content or structure — those instructions, verbatim or faithfully paraphrased. The user's brief is part
   of the message's spec; your reasoning, the change history, and earlier review rounds are not, and must stay out — a
   neutral first read is the entire point of this skill. When the candidate intentionally preserves wording from a
   description the user edited by hand (a PR body, a diff summary), say so in the reviewer prompt and identify the
   preserved wording — the reviewer must treat it as part of the user's spec, not as candidate text to judge.
4. The reviewer returns a verdict of approve or needs-revision, with findings tagged blocking or nitpick.
5. On blocking findings, revise and repeat from step 3 with a **new** subagent, instructing it to report only blocking
   findings — round one already surfaced nitpicks, and later fresh reviewers would otherwise keep producing new ones.
   Never continue a previous reviewer, even if the environment supports it: a reviewer that has seen an earlier draft or
   its own past complaints is no longer neutral. Nitpicks are yours to accept or reject and never force another round on
   their own.
6. Hard cap of three reviewer spawns per message. Fresh reviewers do not converge on their own — each round can raise
   complaints the last one did not — so the cap is what terminates the loop. If blocking findings survive round three,
   do not commit; show the user the final candidate and the unresolved findings and let them decide.

The user's explicit instructions outrank review findings. Never apply a finding — blocking or not — that contradicts
something the user asked for; keep the user's version and mention the conflict to them instead of silently siding with
either.

If the environment cannot spawn subagents at all, read `reviewer.md` and self-review against it, and tell the user
explicitly that the fresh-context review could not run.

## Feedback capture

When the user says "scode-commit-msg-reviewer feedback: ..." (or clearly signals feedback about how this skill
performed), read `feedback.md` next to this skill file and follow it.

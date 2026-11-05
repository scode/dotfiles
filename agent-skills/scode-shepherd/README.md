# scode-shepherd

Maintainer notes. Agents using the skill read `SKILL.md`; agents changing it read `AGENTS.md` and `SPEC.md`.

NOTE: This is a first version. It has run only on throwaway tasks: the local smoke run in `AGENTS.md`, and one two-PR
stack on `scode/repotesting` (2026-11-04, gpt-6.1-terra medium worker, gpt-6.1-sol high reviewer) where each PR passed
on its first session and first review, so the fix-round and escalation paths have not run for real yet. Nobody knows yet
whether a cheap worker under this runner produces acceptable stacks on real goals, or how often it needs the supervisor.

## Why it exists

scode-powerhorse runs strong delegates in parallel to finish sooner, and scode-galaxy-brain hands small units to cheap
models while the session stays in the loop for every unit. Neither covers the case this one is for: a whole goal on a
cheap model, with the strong model out of the loop unless something goes wrong. The pitch is that cheap models do the
expensive tool-call churn, and strong-model tokens go to planning and review rather than to watching.

## Ideas not in v1

These were discussed when the skill was designed and left out to keep the first version small:

- Concurrent review: let the worker start the next item while the reviewer looks at the last one, and feed findings back
  as fixups at item boundaries. Worth it only if blocking review works.
- Cheap-model triage of worker logs and periodic transcript audits, as an escalation tier between the runner's rules and
  the supervisor.
- Claude Code workers and supervisors. The hook channel was verified on Claude Code too (see `SPEC.md`); what is missing
  is a wake mechanism for a Claude Code supervisor and a Claude worker launch in the runner.
- Review receipts bound to patch content (`git patch-id`), so that a rebase that leaves a PR's own diff unchanged keeps
  its approval. With blocking review and a linear stack this does not come up yet.

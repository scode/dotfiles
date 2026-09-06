# Instructions for agents changing this skill

## SPEC.md

The skill must conform to `SPEC.md` next to this file. Read it before changing any file in this directory. If the change
you are making, or the text you find, disagrees with `SPEC.md`, treat that as a bug: fix the skill, or update `SPEC.md`
explicitly in the same change with the reason. Never leave the two apart, and never satisfy a spec requirement by
narrowing what the requirement says.

This skill loads no other skill at invocation time, so `SPEC.md` carries no `Dependencies:` line and `SKILL.md` carries
no dependency stanza; `tests/skill_deps.rs` at the repository root treats it as outside the layered contract. The skills
`SKILL.md` names (`agent-resumeable`, `scode-galaxy-brain`, `jjstack`, `pre-pr-review-swarm`, `scode-harness-shellout`)
are requirements the goal file places on the executing agent, not things this skill loads. If a change makes this skill
load one of them itself, it becomes a layered skill: add the `Dependencies:` line as the first non-heading line of
`SPEC.md` (the test only recognizes it there), the stanza in `SKILL.md`, and the entry in the test's `LAYERED_SKILLS`
list, all in the same change.

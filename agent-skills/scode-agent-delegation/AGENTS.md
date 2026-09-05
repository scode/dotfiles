# Instructions for agents changing this skill

## SPEC.md

The skill must conform to `SPEC.md` next to this file. Read it before changing any file in this directory. If the change
you are making, or the text you find, disagrees with `SPEC.md`, treat that as a bug: fix the skill, or update `SPEC.md`
explicitly in the same change with the reason. Never leave the two apart, and never satisfy a spec requirement by
narrowing what the requirement says.

`SPEC.md` opens with a `Dependencies:` line that `tests/skill_deps.rs` at the repository root parses, and `SKILL.md`
carries one marked stanza per dependency (between `<!-- dependency: <name> -->` and `<!-- /dependency -->`) whose
wording the same test checks against its canonical template. Change the stanza only by changing the template in the
test, and then in every skill that carries one.

## The Codex path in the stanza

The stanza reads a dependency on Codex from `${CODEX_HOME:-$HOME/.codex}/skills/<name>/SKILL.md`, because Codex has no
mid-turn skill loader and that is the root the Codex 0.152 binary uses (the bundled `skill-installer` skill and
`codex-rs/skills` both say so). Codex's public docs already describe a `.agents/skills` root. When Codex changes its
skills root, or the installer starts writing somewhere else for Codex, re-verify the stanza's path with a live
`codex exec` run in an isolated `CODEX_HOME` and update the template in `tests/skill_deps.rs` and every stanza together.

## Reference audit

Before presenting a change as done, search every file in this directory for the words `galaxy-brain`, `routing`,
`inventory`, `profile`, and `escalat`. Each hit is either a fact the text names as something the caller supplies or
decides, or a leak of a consumer's vocabulary into a skill that must not know its consumers; rewrite the leak. The
verdict `misclassified` is the one place a work profile may be named, since the verdict exists to tell the caller its
profile was wrong; "profile" as a measurement, in the task-spec rule about performance work, is this skill's own
vocabulary and stays.

## Evaluating changes

Skill text is consumed by agents that have none of your conversation context, so your own reading of the new wording
proves nothing about how it lands cold. After changing any file here, eval the change with a fresh-context agent before
presenting the work as done: install the skill into an isolated home, and ask a cold agent each question below with the
full situation stated, through a temporary caller prompt that supplies what a consumer would (the unit, the model and
mechanism, the tree) and then asks the question. Judge the answer against the expected one, not against whether it
quotes the new text. Do not point the agent at a sidecar directly; whether `SKILL.md` sends it there is part of what the
eval checks.

The list is run by the PR that creates this skill and re-run whenever the verdict table, the classification in
`checkpoints.md`, or the gate's steps change. Every situation is a writer delegation under the checkpoint protocol
unless it says otherwise.

| Situation                                                                                                     | Expected answer                                                                                                                         |
| ------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| a small writer task                                                                                           | the addendum verbatim (with the run directory substituted), run directory `<tree>/.agent-delegation/<id>/`, stop at `AWAITING GUIDANCE` |
| the launch turn produced `REPORT.md` without stopping; `ASSUMPTIONS.md` exists; every replacement would be OK | the combined-prompt resume, no verdict                                                                                                  |
| the same with `ASSUMPTIONS.md` missing                                                                        | gate the diff now; `substantive failure` with "no assumptions file" as a failure                                                        |
| the same where a replacement invalidates the design                                                           | `substantive failure, structural`                                                                                                       |
| a delegate died of a full disk, cause verified and fixed                                                      | one resume with the crash prompt                                                                                                        |
| a second identical death                                                                                      | `inconclusive`, reason `crash recurred`                                                                                                 |
| a run killed at its hard deadline                                                                             | `inconclusive`, reason `over budget`; no relaunch by this skill                                                                         |
| a third stop at the same checkpoint                                                                           | `misclassified`, with the no-paste rule                                                                                                 |
| a lost thread id                                                                                              | `unresumable`                                                                                                                           |
| a resumed turn that asks what its run directory is                                                            | `unresumable`                                                                                                                           |
| a delegate replied with a question instead of working                                                         | `substantive failure, structural`, not resumed                                                                                          |
| the gate finds the user's request needs a decision only the user can make                                     | `blocked on user`                                                                                                                       |
| the gate finds the spec dropped something the user asked for; the diff is correct against the spec            | `spec defect`                                                                                                                           |
| a diff that is wrong but well-specified                                                                       | `substantive failure, fixable`                                                                                                          |
| a diff that is structurally wrong                                                                             | `substantive failure, structural`                                                                                                       |
| a review deliverable (read-only)                                                                              | a named artifact file in the spec; no checkpoint addendum                                                                               |

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

## Evaluating changes

After changing any file in this directory that an agent reads (SKILL.md, feedback.md, the specs it points at), eval the
change with a fresh-context sub agent before presenting the work as done. Skill text is consumed by agents that have
none of your conversation context, so your own reading of the new wording proves nothing about how it lands cold —
misrouting has repeatedly been caught only by asking a clean agent.

How to run an eval:

1. Spawn a sub agent with no prior context (a general-purpose Agent tool task works).
2. Tell it only where the relevant skill definitions live — this SKILL.md, plus any other skill involved in the scenario
   — and give it a realistic user question whose answer the change should affect. For routing changes, "if I asked you
   to use scode-galaxy-brain to do X, which model would you use?" works well. Do not point it at `feedback.md` or at
   `scode-model-routing`, `scode-agent-delegation`, or `scode-harness-shellout` directly: SKILL.md is supposed to send
   it there when and only when the scenario calls for them, and whether it went is part of what the eval checks. For a
   change that touches a launch, ask for the exact launch command it would run, which it can only produce by having
   loaded the delegation skill, then the shell-out skill and its harness file; for a routing or delegation change, that
   skill's own `AGENTS.md` carries the fixed question list, asked through this skill. The verdict-to-action table is
   this skill's own to eval: for each verdict, ask what the session does next and check it against the table.
3. Judge whether the answer reflects the intended behavior, not merely whether it quotes the new text. If it doesn't,
   revise the wording and re-run until it does.

Every change to a stanza or to what `SKILL.md` says about a dependency also runs the dependency checks: a temporary
consumer whose stanza names a dependency that exists nowhere, one whose name is wrong, and one whose needed sidecar is
missing, each of which must stop and name the skill and the path or tool; and a positive load whose base directory is
the installed one.

The fixed activation and composition questions, run whenever "Staying active", "Composing with other skills", or the
load-when text changes, each asked of a cold agent with the full situation stated (a Claude Code session on fable-5
high, no preference, no config file, all four CLIs on `PATH`, unless the situation says otherwise):

| Situation                                                                                               | Expected answer                                                                                                                                                                                   |
| ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| the task galaxy-brain was invoked for is done; the next message does not mention galaxy-brain           | routing continues; any delegation in the new task is announced with its model and effort                                                                                                          |
| a compaction summary omits the skill but mentions a running `codex exec` writer stopped at a checkpoint | galaxy-brain is assumed active and the assumption is stated; the resume proceeds from the recorded session id and run directory, and no skill is re-read on that account alone                    |
| an invocation that limited scope up front ("for this one thing"); that thing is done                    | no activation for the next task                                                                                                                                                                   |
| the swarm is invoked while galaxy-brain is active, no preference                                        | correctness, security, spec-compliance, and test-quality reviewers on fable-5 high native; the rest on sonnet-5 high native; the choice announced; no checkpoint addendum or gate applied to them |

When a scenario depends on the model routing config file, `~/.scode-model-routing.md`, run it in an isolated home; never
create or edit that file in the real home for an eval — it belongs to the user.

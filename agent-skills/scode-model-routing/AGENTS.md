# Instructions for agents changing this skill

## SPEC.md

The skill must conform to `SPEC.md` next to this file. Read it before changing any file in this directory. If the change
you are making, or the text you find, disagrees with `SPEC.md`, treat that as a bug: fix the skill, or update `SPEC.md`
explicitly in the same change with the reason. Never leave the two apart, and never satisfy a spec requirement by
narrowing what the requirement says.

`SPEC.md` opens with a `Dependencies:` line that `tests/skill_deps.rs` at the repository root parses. This skill depends
on nothing, and nothing in its text may point a reader at another skill.

## Reference audit

Before presenting a change as done, search every file in this directory for the words `galaxy-brain`, `delegation`,
`harness/`, `gate`, `checkpoint`, and `run directory`. Each hit is either a request input the text names as something
the caller supplies, or a leak of a consumer's vocabulary into a skill that must not know its consumers; rewrite the
leak.

## Evaluating changes

Skill text is consumed by agents that have none of your conversation context, so your own reading of the new wording
proves nothing about how it lands cold; misrouting has repeatedly been caught only by asking a clean agent. After
changing any file here, eval the change with a fresh-context agent before presenting the work as done: install the skill
into an isolated home, and ask a cold agent each question below with the full situation stated, either directly ("load
`scode-model-routing` and answer") or through a temporary caller prompt that supplies the request the way a consumer
would. Judge the answer against the expected one, not against whether it quotes the new text. Do not point the agent at
a sidecar directly; whether `SKILL.md` sends it there is part of what the eval checks.

Unless a question says otherwise, the situation is: a Claude Code session on fable-5 high (`sota`), no provider
preference, no config file, all four CLIs on `PATH`, bypass and billing acceptable, own decomposition, first attempt.
The list is run by the PR that creates this skill and re-run whenever the profile table, the precedence list, the
request or answer shape, or the inventory changes.

| Situation                                                                                    | Expected answer                                                                       |
| -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| clear-spec writer, short, input not large                                                    | gpt-5.6-luna medium via `codex exec`, cross-family yes (workhorse default)            |
| clear-spec writer, tiny                                                                      | `orchestrator`                                                                        |
| read-only scan of one subsystem, input not large                                             | haiku-4.5 high native                                                                 |
| read-only whole-repo scan, input large                                                       | gpt-5.6-terra medium via `codex exec`                                                 |
| design decision, own decomposition                                                           | `orchestrator`                                                                        |
| design decision from a sonnet-5 orchestrator (not `sota`)                                    | fable-5 high native (delegate up)                                                     |
| UI styling from a Codex session on gpt-5.6-sol high under `prefer-gpt`                       | opus-5 high via `claude -p`, diverged from preference yes (visual)                    |
| routine authored under `prefer-muse`                                                         | muse-spark-1.3-contributor medium via `muse exec`, escalation high                    |
| critical review under `prefer-glm`                                                           | fable-5 high native, diverged from preference yes (no suitable model)                 |
| critical review under `prefer-glm` from a Codex session on gpt-5.6-sol high                  | gpt-5.6-sol high native, diverged from preference yes                                 |
| config file removes fable-5; critical review                                                 | gpt-5.6-sol high via `codex exec`, cross-family yes (the only remaining `sota` model) |
| config file removes sonnet-5 medium; clear-spec writer under `prefer-claude`                 | sonnet-5 high native                                                                  |
| no `codex` on `PATH`; clear-spec writer, short                                               | sonnet-5 medium native, reason says the gpt path is unavailable                       |
| a swarm-defined design role (process-defined spawn)                                          | `inherit`                                                                             |
| a swarm-defined mechanical-review reviewer under `prefer-gpt` (process-defined, native only) | sonnet-5 high native, diverged from preference yes (mechanism fixed)                  |
| user demands opus-5 for a mechanical task                                                    | opus-5 high native, reason attributes the demand                                      |
| critical review with independent perspective requested                                       | gpt-5.6-sol high via `codex exec`, cross-family yes (independence)                    |
| clear-spec writer after one `substantive failure` on gpt-5.6-luna medium                     | gpt-5.6-terra medium via `codex exec` (not sonnet)                                    |
| the same after `execution-path failure`                                                      | gpt-5.6-luna medium via `codex exec` again                                            |
| the same after `substantive failure (lost context)`                                          | gpt-5.6-terra medium via `codex exec`, reason says no attempt consumed                |
| clear-spec writer, bypass unacceptable                                                       | sonnet-5 medium native, reason says bypass                                            |

When a scenario depends on the model routing config file, the isolated home is where it goes; never create or edit
`~/.scode-model-routing.md` in the real home for an eval, it belongs to the user.

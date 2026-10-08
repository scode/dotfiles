# Instructions for agents changing this skill

## SPEC.md

The skill must conform to `SPEC.md` next to this file. Read it before changing any file in this directory. If the change
you are making, or the text you find, disagrees with `SPEC.md`, treat that as a bug: fix the skill, or update `SPEC.md`
explicitly in the same change with the reason. Never leave the two apart, and never satisfy a spec requirement by
narrowing what the requirement says.

`SPEC.md` opens with a `Dependencies:` line that `tests/skill_deps.rs` at the repository root parses, and `SKILL.md`
carries one marked stanza per dependency whose wording the same test checks against its canonical template. Change the
stanza only by changing the template in the test, and then in every skill that carries one.

## Leave galaxy-brain alone

This skill exists so that scode-galaxy-brain does not have to change. A fix that seems to need an edit to galaxy-brain
or scode-model-routing belongs in this skill instead. A change to `scode-agent-delegation` affects galaxy-brain too, so
make it only when it is additive (galaxy-brain's flows behave exactly as before when they do not use it), and run that
skill's own eval table.

## Evaluating changes

After changing any file in this directory that an agent reads, eval the change with a fresh-context sub agent before
presenting the work as done. Skill text is consumed by agents that have none of your conversation context, so your own
reading of the new wording proves nothing about how it lands cold.

Spawn a sub agent with no prior context, tell it only where this `SKILL.md` lives, and ask the question with the full
situation stated. Do not point it at `scode-agent-delegation` or its sidecars: `SKILL.md` is supposed to send it there
when the scenario needs them, and whether it went is part of what the eval checks. Judge whether the answer reflects the
intended behavior, not whether it quotes the text. Unless a situation says otherwise, the session is Claude Code on
fable high, the skill was invoked for a goal in a git repository, no worker model was named, and no goal file sets a
width.

| Situation                                                                                                                | Expected answer                                                                                                                                                                                                            |
| ------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| the goal splits into three independent subsystems plus a one-line config change                                          | two subsystems launched as chunks at fable high, each in its own tree named by run id; the third waits for a slot; the config change done locally; announced                                                               |
| the goal is one subsystem with nothing to overlap                                                                        | the supervisor does it itself                                                                                                                                                                                              |
| the goal has two independent subsystems, but the user said only one may run at a time (width one)                        | one chunk at a time, still in its own tree, not the main tree                                                                                                                                                              |
| a chunk needs a repo-wide search before its spec can be written                                                          | the supervisor searches itself; no helper agent                                                                                                                                                                            |
| the user said "use gpt-6.1 sol high for the powerhorses"                                                                 | chunks launched through the delegation skill with `codex exec`, gpt-6.1-sol, effort high, resumable launch with the thread id recorded; not native fable                                                                   |
| galaxy-brain was active when the user invoked this skill                                                                 | galaxy-brain is stopped and the user told so; scode-model-routing is not loaded                                                                                                                                            |
| a goal file demands `pre-pr-review-swarm` on gpt-6.1-sol high as the per-PR reviewer                                     | the supervisor picks `codex exec` and launches the swarm at gpt-6.1-sol high through the delegation skill, not at fable; no checkpoint addendum; not counted as a writer chunk; other chunks keep running while it reviews |
| `~/.scode-model-routing.md` says gpt models are unavailable and the user asked for gpt-6.1-sol workers                   | the user is asked; no substitute model                                                                                                                                                                                     |
| a chunk delegate stopped at `AWAITING REVIEW`; the diff has a bug that no `DECISIONS.md` entry covers                    | the full gate now; the bug goes into `REVIEW.md` as an `Also:` item; the complete change set is saved; the chunk table marks the second stop; the delegate is resumed                                                      |
| the same stop, but the diff shows the spec dropped something the user asked for                                          | `spec defect`, not an `Also:` item                                                                                                                                                                                         |
| while a chunk runs, the supervisor wants to amend the stack commit that chunk's tree is based on (jj)                    | it waits, or the chunk was on a detached worktree or clone; no rewrite of a running chunk's base and no commands in the chunk's tree                                                                                       |
| the gate returns `substantive failure, fixable` for a chunk                                                              | one fixup delegation at fable high over the same tree, fresh run id; after that, takeover or replanning                                                                                                                    |
| the gate returns `substantive failure, structural` with reason `lost context`                                            | remove the delegate's changes, split the chunk along a real boundary, then relaunch                                                                                                                                        |
| a fresh session resumes the run; a chunk's native handle no longer resolves; the chunk table marks it at the second stop | take the work over: re-run checks, apply outstanding review items, gate the current change set, write a recovery note; do not discard it                                                                                   |
| a delegate's turn ended with a rate-limit error, for the second time                                                     | wait for the limit to clear and resume again; not a decline, and not `crash recurred` cleanup                                                                                                                              |
| a compaction summary omits the skill but mentions a chunk table with two chunks at the first stop                        | the skill is assumed active and the assumption stated; the table is reconciled before anything is launched                                                                                                                 |
| two chunks are in flight and both are waiting at a stop                                                                  | no new launch until the supervisor has served them                                                                                                                                                                         |
| the goal file names a working log; where does the chunk table go                                                         | its own file next to the log (the path the goal file gives), edited in place; the log records phase changes                                                                                                                |
| invocation was "use scode-powerhorse for this one feature", the feature is done, and the next request needs a delegate   | the skill is no longer active                                                                                                                                                                                              |
| the user asks to use galaxy-brain for the next task                                                                      | this skill stops; galaxy-brain governs from then on                                                                                                                                                                        |
| a fresh session resumes a run whose recorded worker selection is gpt-6.1-sol high, and `codex` is not on `PATH`          | the user is asked and the launches that need it pause; no switch to the session's own model                                                                                                                                |
| the user said "chunks on fable" with no effort, from a sonnet medium session                                             | the worker selection is fable at medium, recorded in the run state                                                                                                                                                         |

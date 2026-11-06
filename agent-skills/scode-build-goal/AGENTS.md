# Instructions for agents changing this skill

## SPEC.md

The skill must conform to `SPEC.md` next to this file. Read it before changing any file in this directory. If the change
you are making, or the text you find, disagrees with `SPEC.md`, treat that as a bug: fix the skill, or update `SPEC.md`
explicitly in the same change with the reason. Never leave the two apart, and never satisfy a spec requirement by
narrowing what the requirement says.

This skill loads no other skill at invocation time, so `SPEC.md` carries no `Dependencies:` line and `SKILL.md` carries
no dependency stanza; `tests/skill_deps.rs` at the repository root treats it as outside the layered contract. The skills
`SKILL.md` names (`agent-resumeable`, `scode-galaxy-brain`, `scode-powerhorse`, `scode-shepherd`, `jjstack`,
`pre-pr-review-swarm`, `scode-harness-shellout`) are requirements the goal file places on the executing agent, not
things this skill loads. `modes/shepherd.md` is this skill's own file, read only in shepherd mode, not another skill. If
a change makes this skill load one of them itself, it becomes a layered skill: add the `Dependencies:` line as the first
non-heading line of `SPEC.md` (the test only recognizes it there), the stanza in `SKILL.md`, and the entry in the test's
`LAYERED_SKILLS` list, all in the same change.

## Evaluating skill text

After changing `SKILL.md` or a file under `modes/`, eval it with fresh-context sub agents before calling the change
done: tell each only where `SKILL.md` lives, state the situation in full, ask for the up-front batch it would send and
an outline of the goal file it would write, and judge the answer against the expectation. Unless a situation says
otherwise, the session is interactive in a Git checkout at `~/git/foo` with a GitHub remote, and no models were named.
The first rows are regression checks: shepherd mode must not change what the other modes do.

| Situation                                                             | Expected answer                                                                                                                                                                                                                  |
| --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `$scode-build-goal add a --verbose flag`                              | no-workhorse stated; menu with opus high as the default; goal file and log beside the checkout; `modes/shepherd.md` not read; the goal file has the resume protocol and the watchdog; started with `/goal`                       |
| `$scode-build-goal powerhorse add a --verbose flag`                   | powerhorse with its worker, width, and PR-sizing defaults; menu with opus high as the default; log and chunk table paths; `modes/shepherd.md` not read                                                                           |
| `$scode-build-goal galaxy-brain add a --verbose flag`                 | galaxy brain stated; `modes/shepherd.md` not read; otherwise as before                                                                                                                                                           |
| `$scode-build-goal fix the scode-shepherd runner's lock`              | the mention is the goal's subject, not a switch: no-workhorse, `modes/shepherd.md` not read                                                                                                                                      |
| `$scode-build-goal shepherd the 2.0 migration through CI`             | `shepherd` is the goal's verb: goal text, no-workhorse (or asks); `modes/shepherd.md` not read                                                                                                                                   |
| `$scode-build-goal teach scode-shepherd to run Claude workers`        | a goal about scode-shepherd: no-workhorse, `modes/shepherd.md` not read                                                                                                                                                          |
| `$scode-build-goal shepherd runner: add a --dry-run flag`             | `shepherd` is a noun in the goal: goal text, no-workhorse (or asks)                                                                                                                                                              |
| `$scode-build-goal help`                                              | the TLDR names all four modes, and that a shepherd-mode goal file is started with `$scode-shepherd`; nothing written                                                                                                             |
| `$scode-build-goal shepherd add a --verbose flag`                     | reads `modes/shepherd.md`; batch states shepherd mode, worker `gpt-6.1-terra` high, how the run is started, the menu with opus unavailable and sol high the default, the run directory `~/git/foo-verbose-goal-shepherd`; no log |
| the same, goal file outline                                           | opens with the shepherd-mode sentence; execution mode requires `$scode-shepherd`, names the run directory and worker, forbids the other orchestrators; no resume protocol, no watchdog; decision logging to the run's journal    |
| the same, after writing                                               | tells the user `$scode-shepherd <goal file>` and the `shepherd drive` command; the run directory is not created                                                                                                                  |
| `$scode-build-goal shepherd ...`, the user answers "review with opus" | says opus cannot run in shepherd mode and asks; does not map it to another model                                                                                                                                                 |
| `$scode-build-goal shepherd powerhorse add a --verbose flag`          | two modes requested: asks which                                                                                                                                                                                                  |

For an end-to-end check of the shepherd mode, have one sub agent write a real goal file for a small goal into a scratch
directory, then run scode-shepherd's own goal-file evals (in `../scode-shepherd/AGENTS.md`) against that file instead of
their sample.

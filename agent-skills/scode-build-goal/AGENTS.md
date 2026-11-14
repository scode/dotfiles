# Instructions for agents changing this skill

## SPEC.md

The skill must conform to `SPEC.md` next to this file. Read it before changing any file in this directory. If the change
you are making, or the text you find, disagrees with `SPEC.md`, treat that as a bug: fix the skill, or update `SPEC.md`
explicitly in the same change with the reason. Never leave the two apart, and never satisfy a spec requirement by
narrowing what the requirement says.

This skill loads no other skill at invocation time, so `SPEC.md` carries no `Dependencies:` line and `SKILL.md` carries
no dependency stanza; `tests/skill_deps.rs` at the repository root treats it as outside the layered contract. The skills
`SKILL.md` names (`agent-resumeable`, `scode-galaxy-brain`, `scode-powerhorse`, `scode-shepherd`, `jjstack`,
`pre-pr-review-swarm`, `scode-harness-shellout`, and `skillette` for its `codex-wake-on-exit` entry) are requirements
the goal file places on the executing agent, not things this skill loads. `modes/shepherd.md` and `modes/overseer.md`
are this skill's own files, each read only in its mode, not other skills, and `scripts/watchdog` is this skill's own
script, which goal files point the executing agent at. If a change makes this skill load one of them itself, it becomes
a layered skill: add the `Dependencies:` line as the first non-heading line of `SPEC.md` (the test only recognizes it
there), the stanza in `SKILL.md`, and the entry in the test's `LAYERED_SKILLS` list, all in the same change.

## The watchdog script

`scripts/watchdog` follows the `skillette-scripts` conventions. After changing it, run `deno fmt --line-width=120
--ext=ts`, `deno check`, and `deno test -A` on it; CI runs the tests too. A change to what it prints, when, or how it
exits changes what goal files tell executors to rely on, so check it against the Resource watchdog item in `SKILL.md`
and do a real run under the Monitor tool: a synthetic alert, a killed sampler, a sampler stalled with `--stall-after`
caught by its observer within two intervals of the missed sample, and quiet samples printing nothing. For the Codex
shape, also check that `observe --exit-on-event` exits with status 10 and prints the sampler's first new event while the
sampler keeps writing.

## Evaluating skill text

After changing `SKILL.md` or a file under `modes/`, eval it with fresh-context sub agents before calling the change
done: tell each only where `SKILL.md` lives, state the situation in full, ask for the up-front batch it would send and
an outline of the goal file it would write, and judge the answer against the expectation. Unless a situation says
otherwise, the session is interactive in a Git checkout at `~/git/foo` with a GitHub remote, and no models were named.
The first rows are regression checks: shepherd and overseer mode must not change what the other modes do.

| Situation                                                                      | Expected answer                                                                                                                                                                                                                                                                                                                                              |
| ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `$scode-build-goal add a --verbose flag`                                       | no-workhorse stated; menu with opus high as the default; goal file and log beside the checkout; neither mode file read; the goal file has the resume protocol and the watchdog (the shipped script, Monitor in Claude Code, wake-on-exit in Codex, no per-minute polling); started with `/goal`                                                              |
| `$scode-build-goal powerhorse add a --verbose flag`                            | powerhorse with its worker, width, and PR-sizing defaults; menu with opus high as the default; log and chunk table paths; `modes/shepherd.md` not read                                                                                                                                                                                                       |
| `$scode-build-goal galaxy-brain add a --verbose flag`                          | galaxy brain stated; `modes/shepherd.md` not read; otherwise as before                                                                                                                                                                                                                                                                                       |
| `$scode-build-goal fix the scode-shepherd runner's lock`                       | the mention is the goal's subject, not a switch: no-workhorse, `modes/shepherd.md` not read                                                                                                                                                                                                                                                                  |
| `$scode-build-goal shepherd the 2.0 migration through CI`                      | `shepherd` is the goal's verb: goal text, no-workhorse (or asks); `modes/shepherd.md` not read                                                                                                                                                                                                                                                               |
| `$scode-build-goal teach scode-shepherd to run Claude workers`                 | a goal about scode-shepherd: no-workhorse, `modes/shepherd.md` not read                                                                                                                                                                                                                                                                                      |
| `$scode-build-goal shepherd runner: add a --dry-run flag`                      | `shepherd` is a noun in the goal: goal text, no-workhorse (or asks)                                                                                                                                                                                                                                                                                          |
| `$scode-build-goal help`                                                       | the TLDR names all five modes, that a shepherd-mode goal file is started with `$scode-shepherd`, and that an overseer-mode `/goal` session launches and oversees an executor; nothing written                                                                                                                                                                |
| `$scode-build-goal shepherd add a --verbose flag`                              | reads `modes/shepherd.md`; batch states shepherd mode, worker `gpt-6.1-terra` high, how the run is started, the menu with opus unavailable and sol high the default, the run directory `~/git/foo-verbose-goal-shepherd`; no log                                                                                                                             |
| the same, goal file outline                                                    | opens with the shepherd-mode sentence; execution mode requires `$scode-shepherd`, names the run directory and worker, forbids the other orchestrators; no resume protocol, no watchdog; decision logging to the run's journal                                                                                                                                |
| the same, after writing                                                        | tells the user `$scode-shepherd <goal file>` and the `shepherd drive` command; the run directory is not created                                                                                                                                                                                                                                              |
| `$scode-build-goal shepherd ...`, the user answers "review with opus"          | says opus cannot run in shepherd mode and asks; does not map it to another model                                                                                                                                                                                                                                                                             |
| `$scode-build-goal shepherd powerhorse add a --verbose flag`                   | two modes requested: asks which                                                                                                                                                                                                                                                                                                                              |
| `$scode-build-goal overseer add a --verbose flag`                              | reads `modes/overseer.md`, not `modes/shepherd.md`; batch states overseer mode, executor `haiku` high, the unchanged menu with opus high the default, started with `/goal` in a farhelm Claude Code session, the farhelm host permissions an unattended run needs, the log and the overseer log `~/git/foo-verbose-goal-overseer-log.md`                     |
| the same, goal file outline                                                    | opens with Roles decided by the marker sentence; executor sections as no-workhorse plus Oversight (binding overseer messages, introduction message first); overseer sections: its resume protocol with the live-overseer check, launching via farhelm with matching permission mode and a `/goal` prompt, overseeing by judgment with examples, limits, done |
| `$scode-build-goal overseer ...`, the user answers "executor on gpt-6.1-terra" | says the executor must be a Claude model in this mode and asks; does not map it to another model                                                                                                                                                                                                                                                             |
| `$scode-build-goal add an overseer role to the job scheduler`                  | `overseer` is part of the goal: no-workhorse (or asks); `modes/overseer.md` not read                                                                                                                                                                                                                                                                         |
| `$scode-build-goal overseer shepherd add a --verbose flag`                     | two modes requested: asks which                                                                                                                                                                                                                                                                                                                              |

For an end-to-end check of the shepherd mode, have one sub agent write a real goal file for a small goal into a scratch
directory, then run scode-shepherd's own goal-file evals (in `../scode-shepherd/AGENTS.md`) against that file instead of
their sample.

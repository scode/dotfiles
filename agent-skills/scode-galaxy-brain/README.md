# scode-galaxy-brain

> [!WARNING]
> **This skill bakes in assumptions that do not generalize.** In particular, the shell-out skill at the bottom of its
> stack prescribes running delegate harnesses with all permission checks disabled — `codex --yolo`, `muse exec --yolo`,
> `opencode run --agent build --auto`, and `claude --dangerously-skip-permissions` — so that delegates can work
> unattended. Do not use it in any environment where that is not acceptable: shared machines, checkouts with secrets or
> production credentials in reach, or anywhere you would not already grant the orchestrating session the same
> unrestricted access.

An agent skill for a session running a frontier model. Invoke it with `Use scode-galaxy-brain to <goal>`. The current
session keeps planning, judgment, quality gating, and all commits and PRs, and delegates suitable pieces of the work to
cheaper models — Claude, GPT via Codex, Meta's Muse, or Z.ai's GLM via OpenCode — picking the model by the kind of work
rather than by nominal price, checking the actual output, and escalating when the first route was not enough.

Add `prefer-gpt`, `prefer-claude`, `prefer-muse`, or `prefer-glm` to steer spend to one provider, and
`~/.scode-model-routing.md` (the model routing config file, read by `scode-model-routing`) to declare which models exist
in your environment. Say `galaxy brain feedback: ...` to record a report about how the skill performed.

## Checkpoints for delegates that write code

A task description that reads as complete still leaves decisions to whoever implements it, and cheap models do not
notice which of those decisions matter. Told "ask if unsure", they never ask; told to list their assumptions, they list
the important one and then pick the wrong side of it anyway. So every delegate that edits code stops for review before
it writes code and again before it finishes, and the orchestrating session does the judging. The basis is a 32-run eval
on four cheap models, where the checkpoint took a feature from wrong in 7 of 8 runs to right in 4 of 4, at roughly 1.5k
tokens of orchestrator reading per run:
[Treeward Guidance-Protocol Eval](https://claude.ai/code/artifact/ef01172f-812c-4c8f-9742-68ebe1a8a0f1).

The protocol itself — the stops, the files, what counts as a finished run, how a stopped delegate is resumed — belongs
to the `scode-agent-delegation` skill (its `checkpoints.md`), not to this one; galaxy-brain only supplies the answers at
each stop and acts on the verdict that comes back.

## Layout

Galaxy-brain is the top of a small stack of skills, and [SKILL.md](SKILL.md) is what an agent loads up front: the
workflow, how to stay active for a session, what goes into a routing request, what the orchestrator supplies to a
delegation, and what it does with each verdict. The three skills below it are inert until something loads them, and
SKILL.md loads each by name at the moment it becomes relevant:

- `scode-model-routing` answers which model, effort, and launch mechanism a unit of work should run on; the inventory,
  the work-profile table, provider preference, and the model routing config file live there.
- `scode-agent-delegation` owns everything between that answer and the orchestrator's verdict: the task spec, the run id
  and run directory, the checkpoint protocol, the gate, and the verdict vocabulary.
- `scode-harness-shellout`, loaded by the delegation skill, owns the launch commands and the observed-behavior notes for
  `codex exec`, `claude -p`, `muse exec`, and `opencode run`.

The one procedure file next to SKILL.md, `feedback.md`, is read when the user gives feedback. The reasoning behind the
rules is archived under [lore/](lore/) and is not maintained.

# scode-galaxy-brain

> [!WARNING]
> **This skill bakes in assumptions that do not generalize.** In particular, it directly prescribes running delegate
> harnesses with all permission checks disabled — `codex --yolo`, `muse exec --yolo`, and
> `claude --dangerously-skip-permissions` — so that delegates can work unattended. Do not use it in any environment
> where that is not acceptable: shared machines, checkouts with secrets or production credentials in reach, or anywhere
> you would not already grant the orchestrating session the same unrestricted access.

An agent skill for a session running a frontier model. Invoke it with `Use scode-galaxy-brain to <goal>`. The current
session keeps planning, judgment, quality gating, and all commits and PRs, and delegates suitable pieces of the work to
cheaper models — Claude, GPT via Codex, or Meta's Muse — picking the model by the kind of work rather than by nominal
price, checking the actual output, and escalating when the first route was not enough.

Add `prefer-gpt`, `prefer-claude`, or `prefer-muse` to steer spend to one provider, and `~/.scode-galaxy-brainrc.md` to
declare which models exist in your environment. Say `galaxy brain feedback: ...` to record a report about how the skill
performed.

The full routing rules live in [SKILL.md](SKILL.md). The reasoning behind them is archived under [lore/](lore/) and is
not maintained.

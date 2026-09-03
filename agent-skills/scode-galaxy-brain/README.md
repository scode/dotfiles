# scode-galaxy-brain

> [!WARNING]
> **This skill bakes in assumptions that do not generalize.** In particular, it directly prescribes running delegate
> harnesses with all permission checks disabled — `codex --yolo`, `muse exec --yolo`,
> `opencode run --agent build --auto`, and `claude --dangerously-skip-permissions` — so that delegates can work
> unattended. Do not use it in any environment where that is not acceptable: shared machines, checkouts with secrets or
> production credentials in reach, or anywhere you would not already grant the orchestrating session the same
> unrestricted access.

An agent skill for a session running a frontier model. Invoke it with `Use scode-galaxy-brain to <goal>`. The current
session keeps planning, judgment, quality gating, and all commits and PRs, and delegates suitable pieces of the work to
cheaper models — Claude, GPT via Codex, Meta's Muse, or Z.ai's GLM via OpenCode — picking the model by the kind of work
rather than by nominal price, checking the actual output, and escalating when the first route was not enough.

Add `prefer-gpt`, `prefer-claude`, `prefer-muse`, or `prefer-glm` to steer spend to one provider, and
`~/.scode-galaxy-brainrc.md` to declare which models exist in your environment. Say `galaxy brain feedback: ...` to
record a report about how the skill performed.

## Checkpoints for delegates that write code

A task description that reads as complete still leaves decisions to whoever implements it, and cheap models do not
notice which of those decisions matter. Told "ask if unsure", they never ask; told to list their assumptions, they list
the important one and then pick the wrong side of it anyway. So every delegate that edits code stops twice, and the
orchestrating session does the judging.

Before writing any code, the delegate writes a numbered list of every interpretation it is about to make and stops. The
orchestrator marks each item OK or replaces it with a one-line decision, and resumes the same delegate. While
implementing, the delegate logs each further decision as it makes it; when the checks pass it stops again, the
orchestrator reviews that log the same way, and the delegate finishes. The files involved (`ASSUMPTIONS.md`,
`ANSWERS.md`, `DECISIONS.md`, `REVIEW.md`, `REPORT.md`) live in a `.galaxy-brain/` directory that is hidden from version
control and moved aside once the result is accepted.

This is unconditional and not tuned to task size: a delegation too small to be worth two resumes is work the
orchestrator does itself. Read-only delegates (reviews, scans) are not affected.

The basis is a 32-run eval on four cheap models, where the checkpoint took a feature from wrong in 7 of 8 runs to right
in 4 of 4, at roughly 1.5k tokens of orchestrator reading per run:
[Treeward Guidance-Protocol Eval](https://claude.ai/code/artifact/ef01172f-812c-4c8f-9742-68ebe1a8a0f1). The details of
the protocol are in [delegating.md](delegating.md).

## Layout

The routing rules live in [SKILL.md](SKILL.md), which is what an agent loads up front. The procedure files next to it —
`delegating.md`, `rc-file.md`, `feedback.md` — are read on demand when a session actually delegates, finds an rc file,
or gets feedback, so that a session which never does those things never pays for that text. Launching a delegate in a
foreign harness (`codex exec`, `claude -p`, `muse exec`, `opencode run`) is the job of a separate skill,
`scode-harness-shellout`, which SKILL.md loads by name the first time a session shells out; that skill owns the launch
commands and the observed-behavior notes for each harness, and is inert until something loads it. The reasoning behind
the rules is archived under [lore/](lore/) and is not maintained.

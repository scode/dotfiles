---
name: skillette
description: Use when the user invokes `skillette`, `$skillette`, or any `skillette-<name>` trigger.
---

# skillette

On a trigger match, read `<name>/SKILLETTE.md` in this skill's base directory, where `<name>` is the row's first trigger
minus its `skillette-` prefix. The whole message is the request. The base directory is the directory containing this
`SKILL.md`; if that path is no longer in context, reload `skillette` through your harness's skill mechanism (the Skill
tool on Claude Code, the `skill` tool on OpenCode, the `read_skill` tool on Muse Code; Codex has no loader and the base
directory is `${CODEX_HOME:-$HOME/.codex}/skills/skillette`). If the loader fails or the file is unreadable, stop and
name the path or tool; do not continue from memory or from a search elsewhere. An explicit `skillette-` word wins over
any phrase match; if two rows still plausibly match, ask which; if a `skillette-` word matches no row, say so. Bare
`skillette` with no trigger just confirms the skill is loaded.

| Triggers         | Natural-language triggers                                           |
| ---------------- | ------------------------------------------------------------------- |
| skillette-change | add, remove, or change a skillette, or change skillette itself      |
| skillette-hackmd | the user mentions HackMD or hackmd.io                               |
| skillette-lore   | the whole word "lore" meaning repo history or a ./lore dir          |
| skillette-ntfy   | a request to send via `ntfy` or `$ntfy`, including after other work |

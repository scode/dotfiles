# Evaluating changes to this skill

NOTE: This is not part of the skill. Nothing in `SKILL.md` or `reviewer.md` refers to it, and an agent running the skill
should never read it. It exists so that a change to the reader's charter or the loop can be checked against a known set
of commits without hunting for them again.

## Commits

| PR   | commit    | title                                                                |
| ---- | --------- | -------------------------------------------------------------------- |
| #300 | `93b6697` | feat: make modernize ask whether a repo is public or private         |
| #301 | `bddc7ba` | feat: prefer tailscale ssh for tailnet workers in scode-ssh-delegate |
| #302 | `2d30f74` | feat: add a one-shot fast path merge to jjstack                      |
| #303 | `63630ed` | feat: add muse as an opt-in delegation family to scode-galaxy-brain  |
| #304 | `1c7f1b4` | fix: stop telling muse delegates to run with write policy disabled   |
| #305 | `51ee3e1` | feat: warn up front that scode-galaxy-brain skips permission checks  |

## Procedure

One drafting run per commit per version of the skill. Each run is a fresh agent given the commit's diff as a file
(`git show` output with the message stripped) and not the hash, told not to consult git history, and told to write the
commit message as if it had just authored the change, using `scode-commit-msg-reviewer` by name and running its loop
with its own fresh reader subagents until the loop says done or reaches its cap. Have it write the final message, the
number of readers spawned, and every round's candidate and reader report to a file.

`~/.claude/skills` and `~/.codex/skills` symlink into this repository's working copy, so a version of the skill is put
under test by placing its `SKILL.md` and `reviewer.md` in the working copy. Runs for one version can go concurrently;
versions go one after another; do not run `jj` while an uncommitted version is swapped in, and restore the committed
state afterward.

Compare the final messages against the real ones and against each other, and read the round logs. Change one thing per
version.

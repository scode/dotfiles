# Instructions for agents changing this skill

## SPEC.md

The skill must conform to `SPEC.md` next to this file. Read it before changing any file in this directory. If the change
you are making, or the text you find, disagrees with `SPEC.md`, treat that as a bug: fix the skill, or update `SPEC.md`
explicitly in the same change with the reason. Never leave the two apart, and never satisfy a spec requirement by
narrowing what the requirement says.

`SPEC.md` opens with a `Dependencies:` line that `tests/skill_deps.rs` at the repository root parses. This skill depends
on nothing, and nothing in its text may point a reader at another skill; the test enforces the line, and the reference
audit below enforces the text.

## Reference audit

Before presenting a change as done, search every file in this directory for the words `galaxy-brain`, `routing`,
`inventory`, `profile`, `checkpoint`, and `gate`. Each hit is either a local use this skill owns (its own `harness/`
directory, its own `SKILL.md`) or a leak of a consumer's vocabulary into a skill that must not know its consumers;
rewrite the leak so the sentence names an input the caller supplies instead.

## Evaluating changes

Skill text is consumed by agents that have none of your conversation context, so your own reading of the new wording
proves nothing about how it lands cold. After changing any file here, eval the change with a fresh-context agent before
presenting the work as done: install the skill into an isolated home, point the cold agent at the skill by name only
(never at a sidecar directly; whether `SKILL.md` sends it to the right harness file is part of what the eval checks),
and judge the answer against the expected answers below.

The fixed question list for this skill, run by the PR that creates it and re-run whenever a harness file, the read-when
table, or the list of caller-supplied inputs changes:

- The exact launch line for each of the four harnesses, given a model, an effort, a prompt file, and a result path.
  Expected: the template from that harness file with the caller's values substituted, `< /dev/null` kept, and the effort
  passed explicitly.
- The exact writer launch and resume lines for each of the four, given a session or thread id. Expected: the id's source
  per harness (`--session-id "$(uuidgen)"` for claude and muse; `thread_id` from the `--json` stream for codex;
  `sessionID` from the first event for opencode), the same flags and working directory or workspace on the resume, and
  where the final message lands on the resumed turn.
- The wake-up interval and the kill rule. Expected: at least every 30 minutes, sooner for short runs; kill by pid
  walking descendants first; never `pkill -f`.
- A codex log stuck at `Reading additional input from stdin...` with no version header. Expected: the startup hang; kill
  and relaunch with the `< /dev/null` redirect, not wait.
- "What does loading this skill make the session do?" Expected: nothing; it is inert until a caller supplies the inputs
  listed in `SKILL.md`.

When changing the description or direct-request instructions, also evaluate discovery from the skill catalog without
naming the skill in the request. These cases preserve the distinction between recognizing a delegation request and
requiring a shell-out:

- "Start an astra agent to review this" in a Claude session without native GPT support. Expected: load this skill and
  read its codex harness file before constructing a `codex exec` launch.
- "Have an opus agent investigate" in a Codex session without native Claude support. Expected: load this skill and read
  its claude harness file before constructing a `claude -p` launch.
- A named-model request supported by the native tool, including required effort and resume behavior. Expected: native
  delegation; discovering this skill must not force a shell-out.
- A named-model request with an explicitly requested external CLI, despite native support. Expected: use that CLI and
  its harness file, subject to availability and caller constraints.
- A caller that supplies an external launch mechanism, despite native support. Expected: preserve the caller's mechanism
  and read its harness file; direct-request defaults must not override an existing delegation decision.
- "What is astra?" and a task containing an incidental model name. Expected: no agent launch.

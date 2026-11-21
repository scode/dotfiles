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

## Adding a harness

A new harness file is not enough on its own. When it is a vendor's own harness for its models (a grok CLI for grok,
say), also add that family to rule 2 of Choosing the external mechanism in `SKILL.md`, and to the matching requirement
in `SPEC.md`. Until then the family is served by the default like every other model, which is intended: a harness gets
preference only once its launch mechanics are written down and verified here.

## Evaluating changes

Skill text is consumed by agents that have none of your conversation context, so your own reading of the new wording
proves nothing about how it lands cold. After changing any file here, eval the change with a fresh-context agent before
presenting the work as done: install the skill into an isolated home, point the cold agent at the skill by name only
(never at a sidecar directly; whether `SKILL.md` sends it to the right harness file is part of what the eval checks),
and judge the answer against the expected answers below.

The fixed question list for this skill, run by the PR that creates it and re-run whenever a harness file, the read-when
table, the external mechanism rule, or the list of caller-supplied inputs changes:

- The exact launch line for each of the six harnesses, given a model, an effort, a prompt file, and a result path.
  Expected: the template from that harness file with the caller's values substituted, `< /dev/null` kept, and the effort
  passed explicitly (for `agy`, flags before `-p="$(cat <prompt-file>)"`, not `agy -p --model ...`).
- The exact writer launch and resume lines for each of the six, given a session or thread id. Expected: the id's source
  per harness (`--session-id "$(uuidgen)"` for claude and muse; `thread_id` from the `--json` stream for codex;
  `conversation_id` from the `--output-format stream-json` `init` event or `--output-format json` for agy; `sessionID`
  from the first event for opencode; the `id` of the first `--mode json` event for omp), the same flags and working
  directory or workspace on the resume, and where the final message lands on the resumed turn.
- The wake-up interval and the kill rule. Expected: at least every 30 minutes, sooner for short runs; kill by pid
  walking descendants first; never `pkill -f`.
- A codex run without `--json` whose log is stuck at `Reading additional input from stdin...` with no version header.
  Expected: the startup hang; kill and relaunch with the `< /dev/null` redirect, not wait.
- A codex `--json` run two minutes in whose stderr holds only `Reading additional input from stdin...` while its stdout
  event log has `thread.started` and keeps growing, and another whose event log is still empty after a minute. Expected:
  the first is healthy (under `--json` a healthy run prints no version header and nothing else need follow that line);
  the second is the startup hang, killed and relaunched with the `< /dev/null` redirect.
- A codex `resume` turn whose stderr has no `Reading additional input from stdin...` line, and a fresh `--json` launch
  that already ended in `< /dev/null` but has no `thread.started` after a minute. Expected: the first is normal (a
  resume reads stdin only for a `-` prompt); the second is not the stdin hang, so read its stderr for the real error
  rather than relaunching it unchanged.
- An omp run whose stdout stays empty while stderr repeats `Still starting after Ns — phase: readPipedInput` with N
  counting up (after the `Reading prompt from piped stdin...` notice). Expected: the startup hang; kill and relaunch
  with the `< /dev/null` redirect, not wait. The repeating lines are not liveness.
- An agy `--conversation <id>` resume that exits 0 while stderr prints `warning: conversation "<id>" not found` and the
  returned `conversation_id` differs from `<id>`. Expected: a fresh session, not a resume; report it to the caller as
  such.
- How to inspect delivered tool results from an `agy -p --output-format stream-json` run. Expected: retain every turn's
  stream separately; match `init`'s `conversation_id` and `init.cwd`; pair `step_type: "tool"` `ACTIVE` and terminal
  updates by `step_index` and tool identity; require a subsequent `agent_response` `DONE` to establish model delivery;
  parse `run_command`'s wrapper before `Command output:\n` for exit/signal status and the 10,240-byte middle-truncation
  notice; do not treat `read_file`'s size summary or `step_type: "subagent"` as delivered file text.
- "What does loading this skill make the session do?" Expected: nothing; it is inert until a caller supplies the inputs
  listed in `SKILL.md`.

When changing the description, the direct-request instructions, or the external mechanism rule, also evaluate discovery
from the skill catalog without naming the skill in the request. These cases preserve the distinction between recognizing
a delegation request and requiring a shell-out:

- "Start an astra agent to review this" in a Claude session without native GPT support. Expected: load this skill and
  read its codex harness file before constructing a `codex exec` launch.
- "Have an opus agent investigate" in a Codex session without native Claude support. Expected: load this skill and read
  its claude harness file before constructing a `claude -p` launch.
- "Start a neon agent to review this" (or "have a neon high agent investigate") in a session without native Gemini
  support. Expected: load this skill, pick `agy -p` as Gemini's own harness, read `harness/agy.md`, and construct an
  `agy` launch using `--model gemini-4-neon --effort <level>` and `-p="$(cat <prompt-file>)"` (never `--model neon` and
  never `agy -p --model ...`).
- A "neon" request when `agy` is missing from `PATH`. Expected: `omp -p` on `openrouter`, not a refusal.
- A request naming Antigravity or `agy` explicitly ("use antigravity to run a sonnet agent"). Expected: load this skill
  and read `harness/agy.md` before constructing an `agy` launch, checking `agy models` for the non-Gemini model.
- A request that names omp as the mechanism, such as "use omp to run a mercury agent". Expected: load this skill and
  read its omp harness file before constructing an `omp -p` launch; when several mercury models exist on `openrouter`,
  ask which one.
- A GLM or grok model, not supported natively, with no harness named. Expected: `omp -p` on the `openrouter` provider
  (`openrouter/z-ai/glm-5.3-flash` for GLM), with no question about providers.
- The same GLM request with a standing user preference for omp's `opencode-zen` provider. Expected: `omp -p` with
  `opencode-zen/glm-5.3-flash`.
- A GPT model when `codex` is missing from `PATH`. Expected: `omp -p` on `openrouter`, not a refusal.
- "Run the glm agent through opencode" with no opencode credential for the provider. Expected: report that opencode
  cannot serve it; no fallthrough to omp.
- The same GLM request with a stated preference for opencode. Expected: `opencode run`, once `opencode providers list`
  shows a credential for its provider.
- The same GLM request with `omp` missing from `PATH` and an opencode credential present. Expected: `opencode run`.
- A caller that supplies `external` for gpt-6.2-sol high. Expected: `codex exec`, not omp.
- A caller that supplies `external` for a model no installed CLI serves. Expected: report it unreachable, naming what
  each candidate lacked; no substitute model.
- A named-model request supported by the native tool, including required effort and resume behavior. Expected: native
  delegation; discovering this skill must not force a shell-out.
- A named-model request with an explicitly requested external CLI, despite native support. Expected: use that CLI and
  its harness file, subject to availability and caller constraints.
- A caller that supplies an external launch mechanism, despite native support. Expected: preserve the caller's mechanism
  and read its harness file; direct-request defaults must not override an existing delegation decision.
- "What is astra?" and a task containing an incidental model name. Expected: no agent launch.

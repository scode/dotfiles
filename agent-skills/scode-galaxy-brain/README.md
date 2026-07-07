# scode-galaxy-brain

TLDR: You invoke this in a session running an expensive frontier model ("Use scode-galaxy-brain to <goal>"). That
session stays in charge — planning, judging quality, managing commits and PRs — while farming out the grunt work (log
scanning, code search, clear-spec implementation, tedious churn) to cheaper models. The goal is cost-effective quality,
not speed.

Things to know before using it:

- It needs both the `claude` and `codex` CLIs installed and authenticated. Cross-vendor delegation shells out to
  whichever CLI is foreign to the current session; same-vendor delegation uses the session's native sub agents.
- Delegates run with all permission checks bypassed (`codex --yolo`, `claude --dangerously-skip-permissions`). Only use
  this in an environment where you would be comfortable running the orchestrating session itself with permissions
  bypassed.
- Delegates edit the shared working tree directly. Read-only tasks may fan out in parallel; writing tasks run one at a
  time because the skill deliberately ships without worktree or other write-concurrency tooling.
- The orchestrator owns all version control. Delegates never commit, branch, push, or open PRs, so your VCS workflow
  preferences (stacking tools, commit conventions) stay with the session that knows them.
- The orchestrator gates every delegated result: it inspects the diff and re-runs checks itself, fixes small defects
  directly, sends substantive defects back for one fixup round, and after that escalates to a smarter (more expensive)
  model or does the work itself — without asking. Expect it to spend more when cheap output doesn't meet the bar.
- Model routing comes from a hardcoded ratings table (cost/intelligence/taste) in SKILL.md. If your cost situation or
  model lineup changes, edit the table; the routing rules derive from the numbers.
- Adding `prefer-gpt` or `prefer-claude` to the invocation steers virtually all delegation to that provider — meant for
  when your subscription sizes differ, not for model-quality reasons. The agent only diverges from the preference with a
  strong reason (and says so). Default is no preference.
- If some models aren't available in your environment, say so in `~/.scode-galaxy-brainrc.md` — plain natural language
  like "fable-5 is not available, do not use". The agent reads it before routing and treats it as authoritative over the
  built-in table, so you never need to edit the skill per environment. When several state of the art models are
  available, the one your session is already running on is preferred.
- The rc file can also hold a full replacement model table: copy the table out of SKILL.md, edit rows and scores (same
  columns and scales), and it supersedes the built-in one — models you leave out are treated as unavailable. Ask the
  agent to "populate my galaxy-brain rc with the default table" and it will seed the file for you to edit.
- Saying "galaxy brain feedback: <what went wrong or should improve>" mid-session makes the agent pause and append a
  self-contained problem report to `~/.local/state/scode-galaxy-brain/feedback.md` (it announces the exact path). The
  entries are written to be handed to an agent in this repo to improve the skill. Review before forwarding — the agent
  avoids obvious private information but does not scrub aggressively.

Credit: inspired by [t3dotgg](https://github.com/t3dotgg); the initial model performance table and some of the wording
were borrowed from his setup.

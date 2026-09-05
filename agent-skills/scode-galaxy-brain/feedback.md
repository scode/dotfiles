# Feedback capture

Read this file when the user says "galaxy brain feedback: ..." or otherwise clearly gives feedback on how this skill
performed.

When the user says "galaxy brain feedback: ..." (or clearly signals feedback about how this skill performed), pause
whatever you are doing and record the feedback before resuming. The record exists so the skill's author can later hand
it to an agent working in the skill's source repository and ask for improvements — write it with that reader in mind.

Append (never overwrite) a markdown entry to `$XDG_STATE_HOME/scode-galaxy-brain/feedback.md`, defaulting to
`~/.local/state/scode-galaxy-brain/feedback.md` when `XDG_STATE_HOME` is unset. Create the directory if needed. Compose
the whole entry first and append it in one write (one `>>` redirect of a finished file or heredoc, not several) so that
another session appending at the same moment cannot interleave with it — this file is the one piece of skill state that
is shared by design, and `SPEC.md` requires that sharing never corrupt anything. After writing, tell the user explicitly
which file you appended to.

Each entry should be self-contained — the future reader has no access to this session:

- A `## <date> — <short title>` heading.
- Which layer the feedback concerns, so the report can be handed to the right skill's repository: `scode-model-routing`
  for a model, effort, or launch-mechanism choice, `scode-agent-delegation` for the task spec, checkpoints, or gate,
  `scode-harness-shellout` for the chosen mechanism's command, flags, monitoring, resume, or kill behavior, or
  `scode-galaxy-brain` itself for activation, decomposition, concurrency, or what was done with a verdict. Name more
  than one when the problem crossed a boundary.
- The user's feedback, verbatim or near-verbatim.
- What you were doing when the problem occurred: the task, which model and delegation path was involved, the actual
  commands or prompts where relevant, and what went wrong (exact errors beat paraphrases).
- Your own analysis if you have one: root cause, and what change to the skill instructions would have prevented the
  problem. Mark speculation as such.

Avoid including private information (credentials, personal data), but do not sacrifice clarity of the problem
description to scrub aggressively — the user reviews the file before forwarding it anywhere.

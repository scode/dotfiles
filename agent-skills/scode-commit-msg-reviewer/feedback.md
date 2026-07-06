# Feedback capture

The user gave feedback about how this skill performed. Pause whatever you are doing and record it as described here
before resuming. The record exists so the skill's author can hand it to an agent working in the skill's source
repository and ask for improvements — write it for that reader, who has no access to this session.

Append (never overwrite) a markdown entry to `$XDG_STATE_HOME/scode-commit-msg-reviewer/feedback.md`, defaulting to
`~/.local/state/scode-commit-msg-reviewer/feedback.md` when `XDG_STATE_HOME` is unset. Create the directory if needed.
Each entry contains:

- A `## <date> — <short title>` heading.
- The user's feedback, verbatim or near-verbatim.
- What actually happened: the candidate message(s), what the reviewer said each round, and what you did with it.
- Root-cause analysis: which instruction in `SKILL.md` or `reviewer.md` allowed the bad outcome, or what missing
  instruction would have prevented it. Mark speculation as such.
- A concrete suggested change — exact replacement wording or a literal diff against the file it targets — always
  accompanied by the problem it fixes and the intent behind the change, so the improving agent can adapt it rather than
  apply it blindly.

After writing, show the user the entry (or a faithful summary of it) and tell them which file you appended to.

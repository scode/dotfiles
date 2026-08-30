# scode-galaxy-brain specification

NOTE: This file is binding on the skill's text. It is deliberately sparse: it records only requirements that have been
stated as such, not a description of everything the skill does. Absence of an entry means the behavior is not yet
specified, not that it is unspecified on purpose. When the skill and this file disagree, that is a bug in one of them;
fix the skill or change this file in the same change, never leave them apart.

## Requirements

- Multiple concurrent uses of scode-galaxy-brain by different orchestrators must not conflict through any state the
  skill itself maintains. Two or more orchestrating sessions may be running the skill at the same time — on the same
  machine, against the same repository, possibly in the same working tree — and every on-disk artifact the skill
  introduces for its own purposes (checkpoint files, scratch copies, prompt files, result and log files, trees it
  creates for isolated delegates) must be private to the orchestrator that created it. No orchestrator may remove,
  overwrite, or reinterpret another orchestrator's artifacts, and the skill's own state must never be the reason two
  sessions interfere. This requirement is about the skill's state only. It does not ask the skill to make the work being
  done through it safe to run concurrently: whether two sessions can edit the same working tree at once is a property of
  that work and of the user's setup, not something the skill is responsible for detecting or preventing.
- The skill is meant to work on modern Linux and macOS. Commands, paths, and tools it prescribes must be available on
  both; nothing may rely on one without an equivalent for the other. No other platform is of concern, and the skill's
  text need not accommodate one.

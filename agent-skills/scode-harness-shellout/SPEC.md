# scode-harness-shellout specification

Dependencies: none

NOTE: This file is binding on the skill's text. It is deliberately sparse: it records only requirements that have been
stated as such, not a description of everything the skill does. Absence of an entry means the behavior is not yet
specified, not that it is unspecified on purpose. When the skill and this file disagree, that is a bug in one of them;
fix the skill or change this file in the same change, never leave them apart.

## Requirements

- The skill depends on no other skill, and its text refers to no other skill: no pointer upward or sideways into a
  consumer ("escalate per the caller's rules", "see the gate", a named skill or one of its files). Where the text needs
  a fact only a caller has — the model and effort, the run id, the tree, the deadline, whether the run will be resumed —
  it names that fact as an input the caller supplies and stops there.
- Loading the skill is side-effect free. Invoking it activates nothing for the session, writes nothing, and claims
  nothing about later spawns; its `SKILL.md` says so in its first paragraph and its description says it is loaded by
  other skills and inert alone. No harness-level switch turns off description-based selection: Codex's
  `allow_implicit_invocation: false` in `agents/openai.yaml` is also read by Muse Code, which then refuses to load the
  skill for the model at all (verified on Muse Code 1.0.2), and a skill that other skills load by name cannot carry it.
  The guarantee is inertness: an unsolicited load does nothing.
- `SKILL.md` is the public surface and is kept lean: what a caller needs on every load stays in it, and anything a
  caller does not need on every load lives in a sidecar read on demand. `SKILL.md` names the trigger for each sidecar,
  and that table is keyed on the five launch-mechanism strings a caller can hold: `codex exec`, `claude -p`,
  `muse exec`, and `opencode run` each name one file under `harness/`, and `native` names nothing, because this skill
  has no part in a native delegation.
- Launch commands live only in the harness files. `SKILL.md` carries no launch line, so that every launch goes through
  the file that carries the observed-behavior notes for that harness.
- Multiple concurrent orchestrators must not conflict through anything this skill puts on disk. Every scratch file,
  prompt, result file, event log, and stderr log it prescribes is named by the run id the caller supplies, in scratch
  space private to the session; nothing uses a fixed path.
- The skill is meant to work on modern Linux and macOS. Commands, paths, and tools it prescribes must be available on
  both; nothing may rely on one without an equivalent for the other. No other platform is of concern, and the skill's
  text need not accommodate one.

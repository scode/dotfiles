# scode-galaxy-brain specification

Dependencies: scode-harness-shellout

NOTE: This file is binding on the skill's text. It is deliberately sparse: it records only requirements that have been
stated as such, not a description of everything the skill does. Absence of an entry means the behavior is not yet
specified, not that it is unspecified on purpose. When the skill and this file disagree, that is a bug in one of them;
fix the skill or change this file in the same change, never leave them apart.

## Requirements

- The skill's dependency set is the `Dependencies:` line above, and the dependency graph between skills is one-way: this
  skill may refer to a skill it depends on, and a skill it depends on never refers back to it. A dependency's text that
  points a reader here is a bug in the dependency; this skill's text that reaches into a dependency by any path other
  than the one below is a bug here.
- A dependency is reached by loading it by name through the harness's own skill mechanism, and the dependency's base
  directory is whatever that mechanism reports; everything inside the dependency is read relative to that directory.
  This skill never assumes where a dependency is installed relative to itself, never uses `../<name>/`, and never
  searches skills roots. On Codex, which has no mid-turn loader, the dependency's `SKILL.md` is read from the root the
  Codex binary uses, `$CODEX_HOME/skills/<name>/` (verified against Codex 0.152; a same-named project-local skill is not
  honored on that path). The loading text is the marked stanza in `SKILL.md`, one per dependency, whose wording
  `tests/skill_deps.rs` at the repository root checks against the canonical template.
- A dependency is either fully loaded or the skill stops. Fully loaded means the loader returned a result that is not
  marked truncated, for the skill whose frontmatter `name` matches what was asked, with a base directory, and every
  sidecar the current step needs is readable under that directory. Anything else — unknown skill, denied tool, truncated
  result, wrong name, missing sidecar, a harness whose loader reports no base directory — is a stop: tell the user which
  skill is missing or unloadable and at what path or tool, and do not continue from memory, from a copy, from a search
  for the file elsewhere, or from a similar skill.
- A same-named project-local skill can shadow the installed dependency on every harness whose loader honors
  project-local skills (all but Codex's file-read path). That is accepted: the name check proves identity, not revision,
  and project-local overrides are how these skills get developed.
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

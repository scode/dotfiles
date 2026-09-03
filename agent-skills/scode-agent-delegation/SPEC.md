# scode-agent-delegation specification

Dependencies: scode-harness-shellout

NOTE: This file is binding on the skill's text. It is deliberately sparse: it records only requirements that have been
stated as such, not a description of everything the skill does. Absence of an entry means the behavior is not yet
specified, not that it is unspecified on purpose. When the skill and this file disagree, that is a bug in one of them;
fix the skill or change this file in the same change, never leave them apart.

## Requirements

- The skill's dependency set is the `Dependencies:` line above, and the dependency graph between skills is one-way: this
  skill may refer to `scode-harness-shellout`, and never to a skill that consumes it. No text here points a reader
  upward into a consumer ("escalate per the caller's rules", a named orchestration skill or one of its files); where the
  text needs a fact only a caller has — the tree, whether isolation is allowed, the recorded base, the model and
  mechanism, what to do with a verdict — it names that fact as an input the caller supplies and stops there.
- The dependency is reached by loading it by name through the harness's own skill mechanism, and its base directory is
  whatever that mechanism reports; everything inside it is read relative to that directory. This skill never assumes
  where the dependency is installed relative to itself, never uses `../<name>/`, and never searches skills roots. On
  Codex, which has no mid-turn loader, the dependency's `SKILL.md` is read from the root the Codex binary uses,
  `$CODEX_HOME/skills/<name>/` (verified against Codex 0.152). The loading text is the marked stanza in `SKILL.md`,
  whose wording `tests/skill_deps.rs` at the repository root checks against the canonical template.
- The dependency is either fully loaded or the skill stops: a loader result that is not marked truncated, for the skill
  whose name matches what was asked, with a base directory, and every sidecar the current step needs readable under it.
  Anything else is a stop that names the missing skill and the path or tool; no inline copy, no similar skill, no search
  elsewhere, no launch from memory. The dependency is loaded only when a delegate runs on a foreign harness; a native
  delegation never loads it.
- A same-named project-local `scode-harness-shellout` can shadow the installed one on every harness whose loader honors
  project-local skills (all but Codex's file-read path). That is accepted: the name check proves identity, not revision,
  and project-local overrides are how these skills get developed.
- Loading this skill is side-effect free. Invoking it activates nothing for the session, writes nothing, and claims
  nothing about later spawns; its `SKILL.md` says so in its first paragraph and its description says it is loaded by
  other skills and inert alone. No harness-level switch turns off description-based selection (Codex's
  `allow_implicit_invocation: false` makes Muse Code refuse to load the skill for the model at all, verified on Muse
  Code 1.0.2); the guarantee is inertness.
- `SKILL.md` is the public surface and is kept lean, measured as what a session loads before its first delegation: the
  task-spec rules, the three-step start, when the dependency is loaded, the resumability rule, the crash classification
  in summary, and the verdict vocabulary stay in `SKILL.md`; the checkpoint protocol, the gate procedure, and isolated
  integration live in sidecars read on demand, each with its trigger named in `SKILL.md`.
- Multiple concurrent orchestrators must not conflict through anything this skill puts on disk. This skill generates the
  run id as the first step of every delegation, creates and owns `<tree>/.agent-delegation/<run-id>/`, names every
  artifact it prescribes by that id, never removes or reinterprets a run directory it did not create, and moves its own
  run directory to the session's private scratch space only once the caller reports having acted on the verdict.
- The gate returns exactly one of the verdicts listed in `SKILL.md` — `accepted`, `accepted with local fixes`,
  `spec defect`, `substantive failure, fixable`, `substantive failure, structural`, `execution-path failure`,
  `misclassified`, `inconclusive`, `unresumable`, `blocked on user` — with the payload the table names, and never a
  verdict outside that list. What the caller does with a verdict is the caller's; this skill never escalates, reroutes,
  relaunches, or removes changes from the tree.
- The skill is meant to work on modern Linux and macOS. Commands, paths, and tools it prescribes must be available on
  both; nothing may rely on one without an equivalent for the other. No other platform is of concern, and the skill's
  text need not accommodate one.

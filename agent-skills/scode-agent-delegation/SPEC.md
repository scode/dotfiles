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
- The dependency uses the current harness's authorized skill loader or resource resolver, or, when no dedicated loader
  exists, the exact `SKILL.md` location its catalog or instructions supplies. On Codex, when no location is supplied,
  use `${CODEX_HOME:-$HOME/.codex}/skills/<name>/SKILL.md` (the root verified for Codex 0.152). Known interfaces are
  examples, not an allowlist of harnesses. Missing filesystem metadata or an unfamiliar harness alone must not block
  loading or trigger a permission question; actual tool permissions remain binding. Never guess installation paths or
  URI schemes, assume a sibling dependency directory, or search other skill roots.
- Identify the exact requested skill from its returned frontmatter, or the loader's reported identity when frontmatter
  is not exposed. Missing or conflicting identity fails loading. Read the skill in full and all sidecars the current
  step needs, using the resolver for that same skill, its reported base, or the directory of its supplied `SKILL.md`
  path. A filesystem base is unnecessary when a resolver addresses required resources or no sidecars are needed.
- Truncated or elided output is incomplete delivery, not a terminal failure by itself. Recover omitted content through
  the tool's continuation, range reads, or full-output artifact tied to the same resource or result before acting.
  Unknown skills, denied access, invalid identity, unresolved required resources, or content that cannot be fully
  retrieved stop the affected operation and name the skill and failing path, URI, or tool. No permission bypass,
  remembered instructions, alternate copy, or similar skill may replace a failed load.
- The loading text is the marked stanza in `SKILL.md`, checked against the canonical template in `tests/skill_deps.rs`.
  On omp, `read` resolves `skill://<name>` and `skill://<name>/<relative-path>`; these satisfy the same capability-based
  contract as other authorized loaders. The dependency is loaded only for foreign-harness delegation, never for native
  delegation.
- A same-named project-local skill may shadow the installed dependency when the harness selects it. This is accepted:
  the name check proves identity, not revision. Codex's fixed-path fallback does not discover project-local overrides.
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
- The gate returns exactly one of the verdicts listed in `SKILL.md` — `accepted`, `accepted with local fixes`, `spec
  defect`, `substantive failure, fixable`, `substantive failure, structural`, `execution-path failure`, `misclassified`,
  `inconclusive`, `unresumable`, `blocked on user` — with the payload the table names, and never a verdict outside that
  list. What the caller does with a verdict is the caller's; this skill never escalates, reroutes, relaunches, or
  removes changes from the tree.
- The skill is meant to work on modern Linux and macOS. Commands, paths, and tools it prescribes must be available on
  both; nothing may rely on one without an equivalent for the other. No other platform is of concern, and the skill's
  text need not accommodate one.
- Native delegation uses available automatic completion delivery or a blocking completion call before falling back to
  bounded waits. Waits respect tool limits and the caller's responsiveness, resource checks, and deadlines; they avoid
  repeated empty status turns and batch checks across outstanding delegates. Prefer monitoring without model turns; a
  small-context watcher is allowed when it can notify the caller without being polled and is expected to reduce total
  cost, including launch overhead and polling usage. Notification support is not assumed across harnesses. Wait expiry
  alone is not delegation failure.

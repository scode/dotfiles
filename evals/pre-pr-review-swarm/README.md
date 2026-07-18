# pre-pr-review-swarm evals

NOTE: These commands spend model tokens. They are not part of CI and should be run only when there is a reason to
compare skill behavior.

NOTE: Cases should point at repositories and refs you are willing to let Codex read. The harness uses a read-only Codex
sandbox, but it still runs the agent in the target checkout and should not be treated as a hard isolation boundary for
host secrets.

NOTE: Sandboxing is TEMPORARILY DISABLED and eval runs are restricted to hand-curated cases until the sandboxing
situation is resolved. Codex's Linux sandbox wraps agent commands in bubblewrap, which fails on hosts that restrict
unprivileged user namespaces (Ubuntu's `apparmor_restrict_unprivileged_userns`) unless a profiled system bwrap is
installed. The failure is silent — every agent command exits 1 and the agent falls back to web/MCP lookups, invalidating
the run. Until that is fixed, `eval run` passes `--dangerously-bypass-approvals-and-sandbox` and refuses `mined` cases,
since running an unsandboxed agent against unvetted third-party code is not acceptable.

The eval harness runs `pre-pr-review-swarm` against unpolished code refs. It does not need the historical polished PR.
The useful signal is whether a candidate skill or model finds the same judge-approved issues as a baseline run.

## Basic workflow

```bash
cargo xtask eval run --skill pre-pr-review-swarm --case treeward-swapped-fifo --model gpt-5.4 --label baseline
cargo xtask eval baseline --run eval-runs/pre-pr-review-swarm/<run-id>
cargo xtask eval run --skill pre-pr-review-swarm --case treeward-swapped-fifo --model gpt-5.4 --label candidate
cargo xtask eval compare --baseline eval-runs/pre-pr-review-swarm/<baseline-id> --candidate eval-runs/pre-pr-review-swarm/<candidate-id>
cargo xtask eval synthesize --comparison eval-runs/pre-pr-review-swarm/<comparison-id>/comparison.json
```

By default, `run` repeats each case three times. Outputs go under `eval-runs/`, and cloned target repositories go under
`eval-worktrees/`; both directories are ignored by git.

Use `--skill-path <path>` to evaluate a local skill directory other than the working tree default. Use
`--skill-ref <git-ref>` when you want the harness to export the skill from a specific commit, branch, or tag.

Use `--reviewer <name>` to restrict a run to a single reviewer charter (for example `--reviewer test-quality`). The full
swarm costs one agent per reviewer charter per repeat, which is wasted spend when only one charter changed. The name
must match a spawnable `reviewers/<name>.md` charter in the resolved skill — shared base charters and condition-gated
charters that do not apply to the case are rejected. The restriction is recorded in `run.json`, and `compare` refuses to
compare a restricted run against a run with a different (or no) restriction — a single reviewer's findings and a full
panel's findings measure different things.

Use `--effort <minimal|low|medium|high>` to set the reasoning effort of the agents. This is the only way to control
effort: the harness runs codex with `--ignore-user-config`, which strips the config file where `model_reasoning_effort`
would normally live, so without the flag every agent runs at codex's built-in default. On `run` the effort applies to
the subject agents and is recorded in `run.json`; like the reviewer restriction, `compare` refuses to mix runs with
different efforts. On `compare` and `synthesize` the flag sets the judge/matcher/synthesis agents' effort independently
of what the compared runs used.

## How does it work?

`run` prepares the target repository under `eval-worktrees/repos/`, resolves the case's subject ref, and uses either the
explicit `base_ref` or the subject commit's first parent as the review base. The harness then owns the swarm fan-out
itself: it materializes the review scope once into `scope.diff`, discovers the spawnable reviewer panel from the
resolved skill's `reviewers/` directory (shared base charters are skipped, and `spec-compliance` runs only when the
target checkout has a `SPEC.md`), and runs one `codex exec` per reviewer charter per repeat, a few reviewers at a time,
each seeing only its own charter plus the shared scope. The command prints the new run directory. Inside it, `run.json`
records the resolved SHAs, model, skill source, label, and repeat count; each `repeat-N/` directory contains the merged
`findings.json`, per-reviewer findings and transcripts under `reviewers/`, and `execution.json`.

NOTE: an earlier harness design asked a single codex session to run the whole skill and trusted it to spawn reviewer
subagents. Observed transcripts showed it never spawned anything — collab waits on zero threads — and silently reviewed
solo with every charter loaded, which is exactly the degraded single-context mode the swarm exists to avoid, and it was
undetectable from the findings alone. The harness-side fan-out replaces trust with ground truth: `execution.json`
records which reviewer agents actually ran and how many findings each returned, any reviewer failure aborts the run, and
finding ids are namespaced by reviewer with the `reviewers` attribution stamped by the harness after each agent returns.
An agent can neither fabricate nor drop attribution, and a "swarm" that did not actually fan out can no longer
masquerade as a completed run.

Before any real spend, every `run` executes a preflight: two concurrent agents review a tiny synthetic scope with a
planted defect (a parity check claiming to test evenness), at the exact model and effort the run will use. Each agent
must return a finding referencing the planted file; an agent failure or a miss aborts the run before the first repeat.
The verdict is recorded in `<run-dir>/preflight/preflight.json` and echoed on stdout. When assessing whether a run's
agent execution actually happened as planned, that record plus each repeat's `execution.json` is the evidence to check —
do not infer health from a plausible-looking findings list alone.

The stamped `reviewers` array is what lets a single full-panel run answer per-reviewer questions offline — which
charters contribute unique findings, and which only duplicate their siblings — without paying for one restricted run per
reviewer. The comparison flow does not consume it. Codex-facing agents get `reviewer-findings.schema.json`, which has no
attribution field at all; the stored `findings.json` follows `findings.schema.json`, whose `reviewers` field the harness
reads as optional so runs recorded before the field existed still parse.

`baseline` does not rerun the model. It writes `baseline.json` into an existing run directory so later comparisons know
that run is the reference point. The command prints the path to that marker file.

`compare` reads the baseline and candidate run directories, asks judge agents to classify findings as `good`,
`incorrect`, or `indeterminate`, and asks a matcher to line up equivalent baseline and candidate findings. It writes a
new comparison directory under `eval-runs/pre-pr-review-swarm/` and prints the path to `comparison.json`. That file
contains matched findings, likely regressions, and notes about findings that appeared in only some repeats.

`synthesize` reads a `comparison.json` and asks Codex for concrete skill-change suggestions for the likely regressions.
It writes `suggestions.json` next to the comparison, plus a raw synthesis transcript for debugging.

## Cases

Cases live in `cases.toml`. Each case points at a public repository and an unpolished subject ref. If `base_ref` is
omitted, the harness resolves the subject ref to a commit and uses its first parent as the base. Merge commits require
an explicit base.

Refs that resolve neither as an origin branch nor locally are fetched from origin by name, so a case may pin an exact
commit SHA even when that commit is reachable only from GitHub's `refs/pull/N/head` (GitHub serves fetch-by-SHA for
those). Pin both `subject_ref` and `base_ref` as SHAs for such cases, and avoid commits that were force-pushed away —
upstream eventually garbage-collects them.

`curation` records how a case was produced: `"hand"` for cases individually curated by a human, `"mined"` for cases
mass-mined from upstream open source PRs by an agent pipeline with only shortlist-level human review. The harness treats
both identically; the flag preserves provenance (and is copied into each run's `run.json`).

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
must match a `reviewers/<name>.md` file in the resolved skill. The restriction is recorded in `run.json`, and `compare`
refuses to compare a restricted run against a run with a different (or no) restriction — a single reviewer's findings
and a full panel's findings measure different things.

Use `--effort <minimal|low|medium|high>` to set the reasoning effort of the agents. This is the only way to control
effort: the harness runs codex with `--ignore-user-config`, which strips the config file where `model_reasoning_effort`
would normally live, so without the flag every agent runs at codex's built-in default. On `run` the effort applies to
the subject agents and is recorded in `run.json`; like the reviewer restriction, `compare` refuses to mix runs with
different efforts. On `compare` and `synthesize` the flag sets the judge/matcher/synthesis agents' effort independently
of what the compared runs used.

## How does it work?

`run` prepares the target repository under `eval-worktrees/repos/`, resolves the case's subject ref, and uses either the
explicit `base_ref` or the subject commit's first parent as the review base. It then runs `codex exec` once per repeat
in read-only mode, with user config and execpolicy rules ignored, pointing it at the selected skill version and asking
for structured findings. The command prints the new run directory. Inside it, `run.json` records the resolved SHAs,
model, skill source, label, and repeat count. Each `repeat-N/` directory contains `findings.json` and the raw
`transcript.jsonl` from Codex.

Each finding carries a `reviewers` array naming the charters that surfaced it, preserved through the skill's own
merge/dedup step. The comparison flow does not consume it; it exists so a single full-panel run can answer per-reviewer
questions offline — which charters contribute unique findings, and which only duplicate their siblings — without paying
for one restricted run per reviewer. The schema requires the field on new runs (OpenAI strict output schemas reject
optional properties, so optionality cannot live there), while the harness reads it as optional so runs recorded before
the field existed still parse.

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

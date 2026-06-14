# pre-pr-review-swarm evals

NOTE: These commands spend model tokens. They are not part of CI and should be run only when there is a reason to
compare skill behavior.

NOTE: Cases should point at repositories and refs you are willing to let Codex read. The harness uses a read-only Codex
sandbox, but it still runs the agent in the target checkout and should not be treated as a hard isolation boundary for
host secrets.

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

## How does it work?

`run` prepares the target repository under `eval-worktrees/repos/`, resolves the case's subject ref, and uses either the
explicit `base_ref` or the subject commit's first parent as the review base. It then runs `codex exec` once per repeat
in read-only mode, with user config and execpolicy rules ignored, pointing it at the selected skill version and asking
for structured findings. The command prints the new run directory. Inside it, `run.json` records the resolved SHAs,
model, skill source, label, and repeat count. Each `repeat-N/` directory contains `findings.json` and the raw
`transcript.jsonl` from Codex.

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

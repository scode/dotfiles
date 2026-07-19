# pre-pr-review-swarm evals

NOTE: These commands spend model tokens. They are not part of CI and should be run only when there is a reason to
compare skill behavior.

NOTE: Cases should point at repositories and refs you are willing to let the agent read. Runs currently execute the
agent unsandboxed in the target checkout (see the next NOTE), so the checkout should not be treated as a hard isolation
boundary for host secrets, on either backend.

NOTE: Sandboxing is TEMPORARILY DISABLED and eval runs are restricted to hand-curated cases until the sandboxing
situation is resolved. On the codex backend this is because codex's Linux sandbox wraps agent commands in bubblewrap,
which fails on hosts that restrict unprivileged user namespaces (Ubuntu's `apparmor_restrict_unprivileged_userns`)
unless a profiled system bwrap is installed; the failure is silent — every agent command exits 1 and the agent falls
back to web/MCP lookups, invalidating the run. Until that is fixed, `eval run` runs both backends unsandboxed — codex
with `--dangerously-bypass-approvals-and-sandbox`, claude with `--dangerously-skip-permissions` — and refuses `mined`
cases, since running an unsandboxed agent against unvetted third-party code is not acceptable.

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

Use `--backend <codex|claude>` to choose the agent CLI (default `codex`). The backend is recorded in `run.json`. On
`run` it selects the subject agents; on `compare` and `synthesize` it selects the judge/matcher/synthesis agents,
independent of what the compared runs used — with one coupling: when `--model` is omitted, the default model is the
candidate run's model, so the selected backend must match the backend that produced it (pass `--model` explicitly to
judge on a different backend). Both backends run with user config and instruction files ignored — codex via
`--ignore-user-config --ignore-rules`, claude via `--safe-mode` — so the user's `CLAUDE.md`/config and the target repo's
own instruction files cannot contaminate the eval. (Claude uses `--safe-mode`, not `--bare`: `--bare` additionally
forces API-key-only auth and would break subscription-authenticated hosts.)

Use `--effort` to set the reasoning effort of the agents. The accepted vocabulary depends on the backend: codex accepts
`minimal|low|medium|high`, claude accepts `low|medium|high|xhigh|max`. This is the only way to control effort — both
backends ignore user config, which strips the place effort would normally be configured, so without the flag every agent
runs at that backend's built-in default. On `run` the effort applies to the subject agents and is recorded in
`run.json`; like the reviewer restriction, `compare` refuses to mix runs with different efforts. When the two runs used
different backends, `compare` allows the comparison but requires both to have pinned an explicit `--effort`, because an
unset effort is each vendor's own default and codex "high" and claude "high" are not the same operating point — a
default is not comparable across backends. `comparison.json` records both runs' backends. On `compare` and `synthesize`
the flag sets the judge/matcher/synthesis agents' effort independently of what the compared runs used.

## How does it work?

`run` prepares the target repository under `eval-worktrees/repos/`, resolves the case's subject ref, and uses either the
explicit `base_ref` or the subject commit's first parent as the review base. The harness then owns the swarm fan-out
itself: it materializes the review scope once into `scope.diff`, discovers the spawnable reviewer panel from the
resolved skill's `reviewers/` directory (shared base charters are skipped, and `spec-compliance` runs only when the
target checkout has a `SPEC.md`), and runs one agent per reviewer charter per repeat, a few reviewers at a time, each
seeing only its own charter plus the shared scope. The command prints the new run directory. Inside it, `run.json`
records the resolved SHAs, model, backend, skill source, label, and repeat count; each `repeat-N/` directory contains
the merged `findings.json`, per-reviewer findings and transcripts under `reviewers/`, and `execution.json`.

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
The verdict is recorded in `<run-dir>/preflight/preflight.json` and echoed on stdout.

After the repeats complete, `run` digests the on-disk evidence into `<run-dir>/verification.json`: per reviewer and
repeat, the output tokens the agent reported, a count of the actions it took, and anomalies for the shapes that indicate
no real work happened. The digest is backend-aware and reads each backend's own transcript shapes: for codex, output
tokens from the final `turn.completed` event and a count of completed command executions; for claude, output tokens from
the final `result` event and a count of tool calls (every `tool_use` except the enforced `StructuredOutput`, so
file-access tools like Read/Grep count as work). Because the two backends count different things, the action count is
comparable only within a backend. Anomalies flag each backend's native "did nothing" signatures — a missing transcript,
no terminal completion event, zero or missing output tokens, and, on claude, a result event that reported an error or a
success result carrying no structured output. Anomalies do not abort the run — judging their severity is the launching
agent's job — but the digest status and every anomaly are printed, followed by the inspection contract: the launching
agent must always read the digest and spot-check at least one reviewer transcript per repeat before reporting or
trusting the run's results. Do not infer health from a plausible-looking findings list alone; the solo-swarm incident
this design replaced produced exactly that.

The stamped `reviewers` array is what lets a single full-panel run answer per-reviewer questions offline — which
charters contribute unique findings, and which only duplicate their siblings — without paying for one restricted run per
reviewer. The comparison flow does not consume it. Reviewer agents get `reviewer-findings.schema.json` (enforced by
codex's `--output-schema` and claude's `--json-schema`), which has no attribution field at all; the stored
`findings.json` follows `findings.schema.json`, whose `reviewers` field the harness reads as optional so runs recorded
before the field existed still parse.

`baseline` does not rerun the model. It writes `baseline.json` into an existing run directory so later comparisons know
that run is the reference point. The command prints the path to that marker file.

`compare` reads the baseline and candidate run directories, asks judge agents to classify findings as `good`,
`incorrect`, or `indeterminate`, and asks a matcher to line up equivalent baseline and candidate findings. It writes a
new comparison directory under `eval-runs/pre-pr-review-swarm/` and prints the path to `comparison.json`. That file
contains matched findings, likely regressions, and notes about findings that appeared in only some repeats.

`synthesize` reads a `comparison.json` and asks an agent for concrete skill-change suggestions for the likely
regressions. It writes `suggestions.json` next to the comparison, plus a raw synthesis transcript for debugging.

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

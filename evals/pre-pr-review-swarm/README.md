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
swarm's findings measure different things. Restricted runs intentionally bypass `SKILL.md`: they measure the selected
charter in isolation, using the harness only to provide a fixed scope and structured output.

Use `--backend <codex|claude>` to choose the agent CLI (default `codex`). The backend is recorded in `run.json`. On
`run` it selects the subject agents; on `compare` and `synthesize` it selects the judge/matcher/synthesis agents,
independent of what the compared runs used — with one coupling: when `--model` is omitted, the default model is the
candidate run's model, so the selected backend must match the backend that produced it (pass `--model` explicitly to
judge on a different backend). Both backends run with user config and instruction files ignored — codex via
`--ignore-user-config --ignore-rules`, claude via `--safe-mode` — so the user's `CLAUDE.md`/config and the target repo's
own instruction files cannot contaminate the eval. (Claude uses `--safe-mode`, not `--bare`: `--bare` additionally
forces API-key-only auth and would break subscription-authenticated hosts.)

Use `--effort` to set the reasoning effort of the agents. The accepted vocabulary depends on the backend: codex accepts
`none|minimal|low|medium|high|xhigh`, while claude accepts `low|medium|high|xhigh|max`. A particular model may support
only a subset (Luna accepts `none` but not `minimal`); preflight catches an incompatible pair before reviewer spend.
This is the only way to control effort — both backends ignore user config, which strips the place effort would normally
be configured, so without the flag every agent runs at that backend's built-in default. On `run` the effort applies to
the subject agents and is recorded in `run.json`; like the reviewer restriction, `compare` refuses to mix runs with
different efforts. When the two runs used different backends, `compare` allows the comparison but requires both to have
pinned an explicit `--effort`, because an unset effort is each vendor's own default and codex "high" and claude "high"
are not the same operating point — a default is not comparable across backends. `comparison.json` records both runs'
backends. On `compare` and `synthesize` the flag sets the judge/matcher/synthesis agents' effort independently of what
the compared runs used.

## How does it work?

`run` prepares the target repository under `eval-worktrees/repos/`, resolves the case's subject ref, and uses either the
explicit `base_ref` or the subject commit's first parent as the review base. The harness materializes that exact
boundary once into `scope.diff`; scope selection is deliberately fixed outside the model so baseline and candidate runs
remain comparable.

Without `--reviewer`, the harness invokes the resolved candidate `SKILL.md` as the coordinator. The skill under test
owns panel selection, native subagent spawning, continuation, merge/deduplication, and reporting. The eval wrapper adds
only the fixed scope and a JSON artifact adapter. On Codex it also raises the documented concurrent-agent cap to the
candidate's resolved charter count because user configuration is deliberately ignored. That capacity is derived from the
charter directory; the harness does not encode reviewer names or orchestration rules. This is the full-swarm path.

With `--reviewer`, the harness runs that charter directly. The skill coordinator is intentionally absent, so a charter
change can be measured without paying for unrelated agents. The harness stamps reviewer attribution on this path because
there is no coordinator to do it.

NOTE: the first coordinator-owned harness trusted the final answer and was invalid. Its transcripts showed a collab wait
on zero threads: the coordinator had loaded every charter and reviewed alone. The current full-swarm path audits the
native collaboration events in the coordinator transcript and requires the number of distinct spawned agents to match
the completed reviewers in `execution.json`. Reported continuation passes must likewise have enough native same-agent
follow-up events. A schema-valid answer with zero agents, a partial panel reported as complete, or invented extra passes
aborts the run.

The command prints the new run directory. `run.json` records the resolved SHAs, model, backend, skill source, execution
mode, label, and repeat count. A full-swarm `repeat-N/` contains `findings.json`, the raw `swarm-result.json`,
`transcript.jsonl`, and `execution.json`. A restricted repeat keeps its per-reviewer findings and transcript under
`reviewers/`. Artifacts created before execution modes were recorded read back as `legacy_panel`; `compare` refuses to
mix those harness-fan-out runs with current full-swarm runs.

Before any real spend, every `run` executes a preflight: two concurrent agents review a tiny synthetic scope with a
planted defect (a parity check claiming to test evenness), at the exact model and effort the run will use. Each agent
must return a finding referencing the planted file; an agent failure or a miss aborts the run before the first repeat.
The verdict is recorded in `<run-dir>/preflight/preflight.json` and echoed on stdout.

After the repeats complete, `run` digests the on-disk evidence into `<run-dir>/verification.json`. Full-swarm runs
record the coordinator's output tokens, actions, spawned-agent count, and same-agent follow-up count; restricted runs
record those values for the reviewer itself. The digest is backend-aware: codex reports `turn.completed`, command
executions, and collaboration thread ids, while claude reports `result`, tool calls, `Agent`, and `SendMessage` events.
Action counts are comparable only within one backend.

Missing completion events, zero or missing output tokens, and backend-native error shapes become verification anomalies.
They do not abort the run, because the launching agent must judge their severity. Failed collaboration accounting does
abort: findings from a coordinator that did not run the reported reviewer agents are not eval evidence. Always read the
digest and spot-check `repeat-N/transcript.jsonl` for a full swarm or the selected reviewer transcript for a restricted
run before trusting the findings.

The `reviewers` array is what lets a single full-swarm run answer per-reviewer questions offline — which charters
contribute unique findings, and which only duplicate their siblings — without paying for one restricted run per
reviewer. The coordinator reports attribution on full-swarm runs; the harness stamps it on restricted runs. The
comparison flow does not consume it. `findings.json` follows `findings.schema.json`, whose `reviewers` field the harness
reads as optional so older runs still parse.

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

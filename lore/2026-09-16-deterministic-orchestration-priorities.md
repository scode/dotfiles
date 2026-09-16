# Deterministic orchestration priorities for lower token cost

Historical artifact, written September 16, 2026, after the completed farhelm token audit. Records estimates and tooling
recommendations from that discussion, not measured savings or an implementation plan that has been accepted. Not maintained.

## Context

The [preceding audit](2026-09-16-delegation-token-usage-audit.md) examined an unattended Codex session using
`scode-build-goal`, `scode-galaxy-brain`, and their delegation skills. The session implemented ten farhelm fixes and
published an RC. Its parent and delegates consumed about 335M cached input tokens over five hours. The main Astra
orchestrator alone consumed 144M, with a median input context of about 151K tokens.

Polling accounted for a meaningful minority of usage, but investigation, validation, integration, bookkeeping, and
follow-up implementation consumed more. The user asked whether reviving [agent-vcs](https://github.com/scode/agent-vcs)
would help, then asked which deterministic orchestration tools offered the greatest token-cost value.

The central recommendation was to reduce how often the large Astra context participates in routine execution. Fewer
shell commands are useful only when they also remove model turns, bulky output, or recovery work. Deterministic tooling
should execute explicit decisions and return verified results; semantic judgments remain with the agent.

## Estimated value by area

This ranking reflects expected scope of benefit, not implementation order. The categories overlap, and their observed
token counts cannot be added into a savings forecast. No controlled comparison was run.

| Priority | Area | Evidence and expected value |
| --- | --- | --- |
| 1 | Validation runner | The parent did substantial hands-on build/test execution and result collection. A runner could address both polling and validation overhead; that category's total cost was not isolated. |
| 2 | Delegate lifecycle and checkpoints | Machine-readable state could reduce artifact discovery, bookkeeping, and repeated parent interventions around every worker. Its savings were not separately quantified. |
| 3 | Shared process/resource watcher | Identifiable wait/status responses carried 37.7M cached tokens. Not all are avoidable, but this is the clearest bounded opportunity. |
| 4 | VCS/PR transactions | About 31M cached tokens occurred in parent responses containing VCS/PR commands, including useful diff inspection and mixed work. Mechanical sequences and workflow errors are plausible targets. |
| 5 | Resume state and evidence management | Seventeen compactions consumed 3.9M uncached input tokens. Smaller results and compact durable state could reduce context growth and rebuilding; the benefit is indirect. |

## Validation runner

Give the runner an explicit test plan and revision. It prepares the execution environment, runs authorized checks,
retains logs, waits without involving a model, and returns a compact result identifying passed and failed checks,
diagnostics, and evidence paths.

For this session, useful responsibilities would have been:

- Detecting the test recorder's Git-metadata requirement before choosing a validation workspace.
- Collecting complete compiler results rather than repeatedly returning partial output to the agent.
- Enforcing concurrency, timeouts, and process cleanup.
- Recording exactly which revision and test selection each result covers.
- Returning bounded diagnostics while retaining complete logs for inspection on demand.

Test selection and failure interpretation should initially remain with the agent. Execution and evidence have clearer
deterministic contracts. In particular, do not claim that results remain valid across arbitrary source changes without
an explicit, justified invalidation rule.

## Delegate lifecycle and checkpoint management

The parent should receive a checkpoint event containing task identity, workspace, assumptions or decisions requiring
attention, changed paths, and verification status. Discovering those facts should not require several model/tool
round trips.

A deterministic coordinator can maintain run directories, lifecycle transitions, ownership, deadlines, artifact
requirements, and the resume ledger. It can reject a ready-for-review submission missing required artifacts or checks.
The model still decides whether assumptions are acceptable and whether the implementation is correct.

This has broad reach because it applies to every worker. It will not automatically eliminate the 52.5M cached tokens
spent by four follow-up repair workers: much of that work addressed semantic deficiencies that machinery cannot judge
from artifact presence or process exit status.

## Shared process and resource watcher

A common mechanism for completion, resource alerts, deadlines, and cancellation could serve validation, delegates, and
release monitoring. It should return control to the model for completion, failure, or a decision, rather than for each
unchanged status sample. Notification delivery and failure behavior need verification in each harness; native support
cannot be assumed from the harness name alone. Retain bounded fallback waits when delivery is unavailable.

The final release watch is a useful regression scenario. The parent launched `gh run watch`, then independently
alternated sleeps with GitHub and resource queries. Thirty sleep responses plus thirty status responses carried 4.67M
cached tokens. A watcher should preserve required monitoring without reproducing that sequence of model wakeups.

A small-context model watcher remains an option when its verified delivery mechanism and total cost make it cheaper
than waking the parent. A process watcher is preferable when the work is fully mechanical.

## Where agent-vcs fits

The project's intended boundary is suitable: the caller chooses scope, prose, refs, stack shape, and authorization;
the binary executes a predefined sequence and reports verified state and partial effects. The assessment inspected its
README, specification, skill, and selected implementation paths. It did not run the binary or establish its reliability.

A broad scan of the farhelm trace found 222 parent responses containing VCS/PR commands, carrying 30.98M cached input
tokens, about 9.2% of the target tree. A narrower set of 65 shell-only responses involving VCS carried 9.95M. These
sets include necessary inspection and are not estimates of removable overhead. The session left its PR stack
unmerged, so it did not exercise stack landing, where the helper could have greater value.

The planning estimate was a few percent of total tokens saved initially, with potentially greater dollar impact because
these operations ran on Astra. That is a hypothesis to test, not a demonstrated saving or a reason to expect the tool
to remove most of the session's cost.

The strongest benefits would be fewer model decision points for commit/bookmark/push/PR sequences, fewer recovery
detours, and executable checks for base, head SHA, mutation ordering, and partial failure. The trace contained a concrete
example of avoidable rediscovery: an invalid `jj describe --file` invocation followed by help inspection and a correction
to `--stdin`.

At the time of inspection, the jjstack adapter had drifted from the current jjstack skill: it used `gh pr edit`, omitted
`--draft` during PR creation, and pushed rewritten heads before retargeting the next PR. Those paths needed alignment
before testing it on real stacks. Its event types also retained complete child stdout/stderr; feeding every event back
to the model could undermine the token benefit.

The proposed revival was deliberately narrow: align and test the jjstack happy path first, exercising publish, update,
and landing in a disposable GitHub repository, including failures after successful mutations. Compare against the
current already-batched shell workflow, measuring model responses, input tokens, output volume, and recovery
interventions. Expansion would depend on that comparison.

## Resume state, output contracts, and implementation order

More frequent compaction is not an obviously cheap solution: the audited summarization responses consumed substantial
uncached input, and subsequent skill/state reloads cost more. Compact durable state and smaller tool results target the
context growth that leads to this work, rather than merely causing it to happen sooner.

Across all tools, the proposed output contract was: compact verified summary to the model, detailed evidence on disk,
and explicit partial effects on failure. Recovery guidance should be loaded when recovery is needed. A wrapper that
still makes the parent load extensive instructions, inspect every step, and consume all child output will miss much of
the intended benefit.

The suggested implementation order was shared watcher, then validation runner, then delegate state management, with
agent-vcs as a bounded independent project alongside them. Validation offered the largest promising scope; the watcher
offered the cheapest demonstrable starting point. Independent correctness reviews should remain: they caught real
defects, and this analysis did not identify them as a major avoidable expense.

# The model inventory and the calibration behind the profile table

Read this file when calibrating the profile table or adding a model, never when answering a request: `SKILL.md` answers
routing requests on its own, and nothing here changes an answer.

## Inventory

Each row is a model at one configured reasoning effort. The family determines the launch mechanism (see Launch mechanism
in `SKILL.md`); `sota` marks models trusted with critical review and the orchestrator role. Availability and user
overrides may remove or replace these defaults; see Local availability in `SKILL.md`.

| model                             | family | sota |
| --------------------------------- | ------ | ---- |
| gpt-5.6-luna medium               | gpt    |      |
| gpt-5.6-luna high                 | gpt    |      |
| gpt-5.6-terra medium              | gpt    |      |
| gpt-5.6-sol low                   | gpt    |      |
| gpt-5.6-sol medium                | gpt    |      |
| gpt-5.6-sol high                  | gpt    |      |
| gpt-6-astra high                  | gpt    | yes  |
| haiku-4.5 high                    | claude |      |
| sonnet-5 low                      | claude |      |
| sonnet-5 medium                   | claude |      |
| sonnet-5 high                     | claude |      |
| opus-5 high                       | claude |      |
| fable-5 high                      | claude | yes  |
| muse-spark-1.3-contributor low    | muse   |      |
| muse-spark-1.3-contributor medium | muse   |      |
| muse-spark-1.3-contributor high   | muse   |      |
| muse-spark-1.3-contributor xhigh  | muse   |      |
| glm-5.3-flash low                 | glm    |      |
| glm-5.3-flash high                | glm    |      |
| glm-5.3-flash max                 | glm    |      |

## Why the profile table looks the way it does

These assignments are defaults, not claims that every task in a profile is equivalent. Test quality is critical because
weak tests are how correctness defects survive review. Reviews route above similarly sized implementation work because
the reviewer is the gate: a missed finding may have no later backstop. Some profiles intentionally share routes today;
keeping their semantics separate lets later calibration change one without conflating different failure costs. A second
cross-family SOTA perspective may be worth its overhead for high-risk critical review. Orchestration is not a delegation
profile; planning, decomposition, quality gating, and VCS ownership remain with the orchestrating SOTA session.

## The mid tier, and what the eval behind the luna default measured

Within implementation work, the mid tier — terra and sol as delegates rather than sol-high-as-reviewer — earns its cost
in two situations. Context economy: work that has to be reasoned across more input than the orchestrator can afford to
spend its own context on (a change threaded through dozens of files, a diagnosis that means reading a large subsystem)
goes to terra, because luna cannot hold it and the orchestrator should not have to; that is the center of the complex
implementation profile, and why its route starts at terra rather than sol. And the escalation rung: when a workhorse
fails substantively, terra is the cheap next step before sol and before the orchestrator takes the work over. Difficult
debugging and ambiguity that survives decomposition also live in the complex implementation profile — its route carries
sol for exactly the case where the delegate's own judgment turns out to matter mid-task. What the eval behind these
routes (https://claude.ai/code/artifact/43a3d4f1-fd32-41df-84bc-d62d6fb1f248) actually showed is narrower than "the mid
tier is useless": in 36 runs every model passed every hidden test, so the tasks separated prices, not failure rates, and
the expensive models' visible advantages were soft — documentation quality, benchmark discipline — which a reviewed
assumptions list and a gate that reads the diff cover. The honest conclusion is that nothing there justified paying
mid-tier prices for well-specified work, not that no task ever will. An orchestrator reaching for sol or opus to
implement something well-specified usually has an unsettled design, and the fix is to settle it; the routine authored
and mechanical review rows are unchanged by this reasoning — taste-dependent prose and review were not what the eval
measured.

Luna is the workhorse on purpose, not as a compromise. Across six treeward features and a planted bug, every model from
luna medium up to sonnet passed every hidden test on the first attempt; the gate rejected three results for hidden
work-done regressions, none of them luna's, making luna medium the cheapest clean record — $0.64 for the six features
against $5.57 for terra and $21.73 for sonnet (https://claude.ai/code/artifact/43a3d4f1-fd32-41df-84bc-d62d6fb1f248).
Luna has two demonstrated weaknesses. Judgment on open questions is the first, and the caller's checkpoint protocol
moves that judgment to the orchestrator before any code exists: in the guidance eval, the checkpoint arm went 8 for 8
across the four cheap models — luna medium and high among them — on a feature the same models had gotten right once in
eight runs without it. It does not move mid-implementation judgment anywhere, which is why the caller's gate still reads
the diff in full. The second weakness is long context, covered by the exception in `SKILL.md`. What the workhorse needs
is a clear spec and a reviewed assumptions list, and those are the orchestrator's to supply. Expect luna's decision log
to be short or empty — it does not experience decisions as decisions — and a short or empty log is not evidence that the
work was simple or that nothing was decided.

## Evidence behind the rules in SKILL.md

`SKILL.md` states the rules; this is what each rests on, so that calibration has one place to update.

- **The GPT `sota` mark.** gpt-6-astra high carries the family's `sota` mark and gpt-5.6-sol high does not, as of
  2026-09-06. That is the user's judgment on which GPT model is the current flagship, not a measurement: astra is the
  model the user's own Codex sessions run on, and the mark follows the model trusted to orchestrate. Sol keeps every
  non-critical placement it had, so the change moves only the critical-review route, the design delegate-up target for a
  non-`sota` GPT session, and the `endpoint trusted` flag on routes that end at sol high. Astra has no placement below
  critical review because nothing has calibrated it as a delegate; give it one when real use says so.
- **The opt-in families.** The muse placements rest on vendor-reported benchmarks rather than calibrated use. The glm
  family's model ran as the `ox-alpha` stealth preview on OpenRouter and OpenCode; its evidence base is one clean
  clear-spec smoke run plus vendor benchmarks, and its per-token price is roughly an order of magnitude below the other
  families.
- **The visual carve-out.** Real-world feedback on GPT-5.6 consistently rates sol below the Claude models on visual
  design taste even while its coding reputation holds up.
- **The workhorse-writer default.** The cross-family cost the native-path bias exists to weigh is small and
  characterized for writers: the shell-out path to codex (launch, resume, and monitoring) is exercised end to end (two
  items there remain explicitly unverified), and the price gap to the same-family writer alternative is about 30× for no
  measured difference in first-attempt correctness (the eval above). The default rests on one eval in one small
  repository; if a follow-up on a larger, messier codebase finds luna's literalism surviving the caller's checkpoint
  protocol, the workhorse-writer paragraph in `SKILL.md` is what to revisit.

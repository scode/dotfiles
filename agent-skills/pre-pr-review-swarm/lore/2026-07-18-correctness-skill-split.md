# 2026-07-18 — Splitting correctness and security into lens reviewers

NOTE: This is a point-in-time record of the reasoning behind the lens split, written the day the change was made. It
will not be updated as the skill evolves.

## Where this started

The original question was about cost, not structure: which reviewer charters actually need an expensive model? The
assessment was that correctness and security need top-tier models (open-ended hypothesis generation, multi-step path
tracing, expensive misses), while the advisory charters (idiomaticity, slop checklists, docs accuracy) are largely
prescriptive pattern-matching that cheap models handle well — their false positives get filtered by the coordinator, and
their false negatives are cheap. That led to a second question: is one-agent-per-charter still the right shape at all,
given that the fan-out design predates several model generations?

The answer: the original motivation (older models satisficing when given eight objectives at once) has partly aged out,
but the structural benefits have not. Independent agents are independent samples from the model's distribution of
"things noticed"; each gets its own output and investigation budget; fresh contexts avoid anchoring; and narrow focus
matters most for the cheap models the advisory charters should move to. So the fan-out stays, and the interesting
question became where extra spend buys the most recall.

## Lenses, not samples

For correctness and security, more coverage per run was worth buying. Two options: N identical copies of the broad
charter, or N differently-focused variants. Identical copies are highly correlated — the same salient bugs get found by
every copy, and the subtle miss is usually missed for a systematic reason (the prompt never directed attention there),
so resampling rarely recovers it. A different charter shifts the distribution instead of resampling it. Identical
samples do enable majority-voting for precision, but the coordinator already owns precision; recall is the scarce
resource. Hence lenses: correctness split into data-flow, state-lifecycle, and edge-inputs; security into input-trust
and secrets-env.

## Lenses are additive, and why the wording matters

The known failure mode of splitting is the ownership gap: bugs that fall between exclusive territories. The existing
charters already pay a tax policing boundaries between top-level reviewers, and we did not want that inside correctness.
So every lens reviewer reads the full base charter and is a complete reviewer; the lens is a mandatory second deep pass,
not a persona. Two wording choices matter more than they look:

- "After your normal full-charter pass, make a second, deeper pass focused on X" — a depth directive primes less
  off-lens suppression than "you are the X reviewer", which filters perception from the start.
- "Assume you are the only reviewer who will notice it" — targets the real failure mode, which is not formal scope
  exclusion but the model inferring a division of labor and not digging outside its lane.

## The generalists

Even additive lenses tilt attention, so a bug matching no lens (internally-consistent-but-wrong logic, authz checking
the wrong thing) gets only baseline attention from everyone. Each split charter therefore also runs one lensless
generalist. It doubles as the tuning diagnostic: a generalist that keeps surfacing findings the lenses missed points at
a lens worth naming; one that only duplicates marks a reclaimable slot. The lens set is explicitly meant to be tuned
over time, not treated as fixed.

## Measurement

The eval harness was extended in the same stack so the tuning loop has data: per-finding reviewer attribution stamped by
the harness (ground truth, not agent self-report), reasoning-effort control, harness-side fan-out (an earlier harness
design trusted one codex session to spawn the swarm; transcripts showed it reviewed solo and never spawned anything),
and a preflight that proves the agent-execution path before real spend.

First A/B on `treeward-swapped-fifo` at gpt-5.6-sol high, three repeats per arm: lens skill 12/12 judge-good findings vs
old skill 7/7, zero regressions, five net-new findings. The reliability effect was the clearest signal — the old single
correctness reviewer found the core issue in one of three repeats, while at least one correctness lens found it in all
three, a different lens each time. The correctness generalist contributed nothing unique on this case, but one case with
a single lens-shaped issue cluster is exactly where a generalist should duplicate, so that slot's verdict needs more
cases. Calibration warning: this is one case, one run per arm — directional evidence, not proof.

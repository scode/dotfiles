# Finding assessment

Assess after all four arm reports are sealed. Do not send the reference findings, other arms' output, or parent
hypotheses back to a reviewer. This is an unblinded parent assessment; reading old findings only after completion
prevents reviewer contamination but does not make the judge blinded.

## Unit and validity

The comparison unit is an actionable defect at an independently editable location. Different descriptions of the same
mechanism, consequence, and repair count once for coverage. Distinct bugs in the same region remain distinct. If one
reviewer bundles independently editable defects, preserve its original text and map each component explicitly rather
than silently expanding or collapsing its score.

For every raw and reported finding, record one of these assessments:

- Accepted: the pinned source and its applicable contract establish the defect and a justified concrete action.
- Rejected: a material premise is false, the issue is outside the change, or the finding demands behavior the applicable
  contract does not require.
- Optional: a defensible improvement or design preference without an established defect.
- Uncertain: a material premise cannot be resolved through the allowed source inspection. Do not score this as accepted
  recall.

Trace the after-state and relevant before-state before claiming regression. Respect intentional staging or unwired code:
new public library behavior may still be reviewable, but do not invent a shipped CLI path or require a later planned
feature to exist now. Report residual ambiguity explicitly.

## Criticality

- High: a concrete supported path to serious security failure, data loss/corruption, or sustained service failure; the
  consequence and reachability must both be established.
- Moderate: a real behavioral, recovery, compatibility, or resource defect with meaningful but narrower consequences.
  Explain preconditions and whether the affected path is shipped, public-but-unwired, or test-only.
- Low: a localized minor defect or misleading statement whose demonstrated consequence is limited. Do not promote it
  because a reviewer used security language.

For the optional aggregate "material" count, include accepted high and moderate defects only. Keep all low findings in
the inventory so this threshold does not hide them. Preserve the earlier evaluation's classification separately if it
used a different definition.

## Coverage views

Report both reviewer discovery and the final accepted arm output. A reviewer can discover a defect that its coordinator
incorrectly rejects or alters; those are different failure modes. Track restater disputes, confidence changes, rejection
reasons, same-location merges, and the pass where the issue first appeared.

Compare against two explicit incomplete denominators: accepted issues in this run's union, and the source-verified
applicable reference issues from the earlier case assessment. Identify reference issues missed by every new arm and new
issues absent from the old assessment. Do not call either denominator absolute recall.

For each unique finding or material miss, provide its mechanism, applicable source evidence, exposure/preconditions,
recommended action, criticality, discovering configurations and passes, and coordinator disposition. Findings found only
by one configuration deserve individual explanation even when low severity.

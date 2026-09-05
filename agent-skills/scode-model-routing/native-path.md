# Expected size and the strength of the native-path bias

Read this file the first time a request's expected size is in doubt. `SKILL.md` states the bias itself (prefer the
orchestrator's family when models are roughly equally suitable, and prefer a native path); this file is the size anchor
for how strongly it applies.

Apply the bias according to expected task size. These are judgment anchors, not hard thresholds:

| expected size | practical meaning                                                     | native-path bias                                                                   |
| ------------- | --------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| tiny          | Setup and review may cost as much as doing the task directly.         | Very strong. Usually do it directly or delegate natively.                          |
| short         | One bounded operation with little context or expected iteration.      | Strong. Cross harnesses only for a meaningful capability or total-cost advantage.  |
| medium        | A substantive task where model work dominates fixed startup overhead. | Moderate. Consider likely retries and review burden alongside startup overhead.    |
| large         | Extended work where cross-harness startup is a minor part of the run. | Weak. Choose the route most likely to finish cleanly at lower expected total cost. |

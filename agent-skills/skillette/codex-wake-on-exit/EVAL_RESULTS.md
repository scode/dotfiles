# Codex wake-on-exit evaluation results

On 2026-10-13, **20 fresh agents completed verified arm/idle/callback/ack cycles** against the frozen candidate. The
agents ran as separate ordinary connected Codex sessions on gpt-6.1-sol, high reasoning effort, with model fallback
disabled. Runtime: Codex CLI 0.162.1, Deno 2.9.7, Linux x86_64.

All twenty final sessions passed. Watchers lasted seven minutes, or eight minutes for active-work cases. Earlier
experiments are excluded from this final count: one earlier watcher finished before its agent's initial turn ended, so
later fixtures allowed more time to establish actual idle resumption. A late instruction review also found that a
successful CLI exit could return a failure/cancellation phase without promising a wake. That wording was corrected, then
the candidate was frozen again and these twenty fresh sessions ran. No mechanism failure was observed in the final
suite.

## Counted cases

Each agent selected the correct skillette from the loaded candidate entrypoint, used the helper for its own thread,
durably registered its continuation, ended its first turn before watcher completion, received the real fixed callback,
acknowledged the matching generation, and reported the actual terminal result and exact output token. The client
supplied only the initial task; the server started callback turns while the client stayed connected. There were no
observed idle polling calls, manufactured callbacks, premature output reads, or shortened watcher lifetimes.

“Idle seconds” is the interval from the first turn ending to the fixture publishing output immediately before exit. All
counted sessions have exactly two paired turns. Code 7 is an expected watcher failure preserved by the callback, rather
than a delivery failure.

| Case | Trigger        | Features                                        | Exit code | Idle seconds | Result |
| ---- | -------------- | ----------------------------------------------- | --------- | ------------ | ------ |
| 01   | explicit       | timer                                           | 0         | 386.557      | pass   |
| 02   | natural phrase | file condition                                  | 0         | 394.663      | pass   |
| 03   | explicit       | timer                                           | 7         | 390.456      | pass   |
| 04   | natural phrase | file condition, literal argv                    | 0         | 376.719      | pass   |
| 05   | explicit       | timer, active work first                        | 0         | 455.970      | pass   |
| 06   | natural phrase | file condition                                  | 7         | 386.999      | pass   |
| 07   | explicit       | timer                                           | 0         | 385.373      | pass   |
| 08   | natural phrase | file condition, literal argv                    | 0         | 413.856      | pass   |
| 09   | explicit       | timer                                           | 7         | 376.677      | pass   |
| 10   | natural phrase | file condition, active work first               | 0         | 429.173      | pass   |
| 11   | explicit       | timer                                           | 0         | 393.140      | pass   |
| 12   | natural phrase | file condition, literal argv                    | 7         | 415.717      | pass   |
| 13   | explicit       | timer                                           | 0         | 375.397      | pass   |
| 14   | natural phrase | file condition                                  | 0         | 385.303      | pass   |
| 15   | explicit       | timer, active work first                        | 7         | 445.429      | pass   |
| 16   | natural phrase | file condition, literal argv                    | 0         | 404.833      | pass   |
| 17   | explicit       | timer                                           | 0         | 386.560      | pass   |
| 18   | natural phrase | file condition                                  | 7         | 387.053      | pass   |
| 19   | explicit       | timer                                           | 0         | 391.975      | pass   |
| 20   | natural phrase | file condition, literal argv, active work first | 0         | 461.977      | pass   |

Four cases finish active work before arming; five preserve spaces, quotes, literal shell metacharacters, and an embedded
newline. Argument preservation was checked against the full manifest and observed output. The final candidate did not
change during this suite.

## Review and regression evidence

A separate gpt-6.1-sol high adversarial review found two descendant-lifecycle defects in the initial helper. Both were
fixed before freezing: cancellation now retains process-group ownership through escalation, and watcher exit is separate
from inherited-pipe EOF. The follow-up review found no remaining source blockers and independently passed all ten
embedded helper tests. Those tests include both real multiprocess regressions, bounded escaped-pipe capture, literal
argv and nonzero completion, cancellation, generation/ownership checks, duplicate acknowledgments, unsupported queue
capability, and enqueue rejection without retries.

A separate agent audited every final history, corroborating the automated artifact checks with command order, durable
registration, actual callback input, acknowledgment before output reads, active-work order, any model-written launcher,
and absence of observed polling or evidence fabrication. Three separate fresh agents also interpreted zero-exit-code
receipts for runner-failed, queue-failed, and cancelled correctly, without promising callbacks or substituting polling.
These phase fixtures, preliminary cases, the earlier 21-session candidate suite, and native-subagent/one-shot-client
probes are excluded from the twenty real final receipt cases.

The audit found one nonblocking reporting error: case 01 attributed the fixture's stderr sentence to stdout in its
prose. The raw stdout/stderr, actual terminal result, exact token, and lifecycle checks are correct. That caveat is
retained here; the result does not claim perfect stream attribution in every agent-authored report.

Frozen helper SHA-256: 2bc990de88be8381792dd534d1649786734d4607cab1f54794d9dad171c4076a. Frozen selected-instruction
SHA-256: 8c010be39a1fc424712944ee7d61c583809d724eb6479489efc7e035dea0e07e. The candidate hashes matched before and after
evaluation; each captured instruction read matched the frozen content.

## Scope

This establishes instructed selection from the loaded candidate entrypoint and actual receipt in the tested connected
Linux runtime. It does not establish automatic installation discovery, a general failure probability, macOS behavior,
restart recovery, or operation in other harnesses. File-condition cases use actual filesystem notifications with a timer
creating the condition marker. Raw session histories and host-specific identifiers are retained in the private
evaluation artifacts rather than published here. The procedure for future runs is in [EVALS.md](EVALS.md).

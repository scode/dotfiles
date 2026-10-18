# Evaluating persistent-review

Use fresh contexts and an isolated copy of the installed skill and its dependency. Supply the skill catalog entry and an
ordinary user task; enter through `SKILL.md` rather than supplying the harness sidecar or the expected answer. Keep all
generated inputs, prompts, results and transcripts in private scratch space. Product files stay read-only.

## Real reviewer exercises

Exercise both Codex and Claude. Name the reviewer model and effort, charter, mandatory scope and result destination. Use
stable text whose required ranges are easy to identify. A large numbered sequence with a consistency charter works
without needing planted defects or a finding quota.

Constrain the first reviewer turn to one read with a deliberately small output allowance, or a Read large enough to
trigger clipping. Let it return its current findings before it has consumed the whole scope. Subsequent turns can page
through the input. Preserve every turn's evidence. Check that the orchestrator continues while making progress without
inventing a deadline or asking for a time budget, including when loading the shellout dependency.

Inspect the outcome independently:

- Which skill did the orchestrator load, and which mechanism did it choose from the actual observation capabilities?
- Do the reviewer identity and actual initial/resume events establish at least two turns in one session?
- What text was delivered on each read, rather than merely emitted by the command? Include the parent's audit of that
  text and metadata; check its own output for truncation too.
- Did missing ranges cause concrete continuations even when the first result had no findings? Does the union of
  inspected delivered ranges cover the mandatory scope, with the right version and any diff sides?
- Did a later turn synthesize interactions after the missing material arrived, and did the parent audit it afterwards?
- Does the completion record match the evidence, preserving findings and naming unresolved material on a stop?

The shellout harness files describe version-specific evidence sources. Probe current local representations before
crediting them: a Codex raw command event can contain more than its delivered result, and Claude Read can declare
clipping in structured metadata without a literal marker in its text. Success on one backend does not establish the
other's semantics. Keep verified observations separate from a child's claims.

## Discovery and failure cases

In independent cold scenarios, check explicit invocation, the phrase "persistent adversarial review," ordinary review,
and discussion about changing the skill. The latter two must not silently launch this protocol. Exercise an unsupported
reviewer backend and both observable and unobservable native delegation.

Use controlled transcript/result fixtures for absent evidence, an empty claimed-complete result backed only by partial
reads, a caller-supplied time limit with retained findings, and two repair turns with new findings but no new delivered
content. Also check that elapsed time alone does not stop a progressing review when the caller supplied no time limit.
Require concrete next actions and honest completion status. A completed second-turn repair followed by synthesis must
not require an otherwise unnecessary third turn.

Scenarios may be combined; do not spend a separate live run on each assertion. The real exercises prove the operational
path, while fixtures cover stopping cases without waiting for a stalled run. Validate helper code behavior if any is
added, and run the repository checks. Matching instruction text is not a behavioral test.

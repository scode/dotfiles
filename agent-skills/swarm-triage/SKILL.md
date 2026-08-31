---
name: swarm-triage
description: Use only when the user explicitly invokes `swarm-triage` by name. Drives the human triage of a pre-pr-review-swarm run — bringing chosen findings into a working document the user names, iterating on them there, and recording the user's verdict on every finding (accepted, rejected, skipped) with their own words on why — so the swarm's output can later be judged against what a person actually did with it.
---

# Swarm Triage

## What this is for

A `pre-pr-review-swarm` run ends with a list of findings and a run log recording them. What happens next is a human
activity: the user reads the findings, brings some of them into a separate document, works on those — re-verifies,
rephrases, merges, drops — and ends up with the ones they will act on, whether that means posting review feedback,
changing code, or something else. Without this skill the outcome of that work is thrown away, so nothing ever says which
findings were worth producing. This skill keeps the flow exactly as it is and adds one thing: a record of the user's
verdict on every finding, in the user's own words, joined to the run log by finding identifier.

The record is the product. Everything else here exists so that recording it costs the user nothing beyond the
conversation they were already having. Later, records from many runs get read together to answer questions like "which
reviewer categories produce findings that are accepted", "would the fix-mode buckets have applied things the user
rejected", or "what proportion of findings never even get looked at" — and to find whatever patterns nobody thought to
ask about. Anything that makes the record less truthful defeats it. That includes assuming anything about why the user
chose `nofix` or fix mode, whose code is under review, or what they will do with the accepted findings: all of those
vary from run to run and none is the skill's business.

## Inputs

- **The run log.** A file the swarm writes to `~/.local/state/pre-pr-review-swarm/runs/<run name>.md`, where the run
  name looks like `20260829-0412-62d866d-9c2e` and the swarm's session output ends with `Swarm run: <run name>` and
  `Run log: <path>`. The user normally invokes this skill with the run name (`swarm-triage 20260829-0412-62d866d-9c2e`);
  a path works too. Read the log in full at the start. It holds every finding verbatim as the user saw it, with the
  identifiers this skill keys on. Identifiers are per run, so the log must be the one for the findings the user is
  looking at: if the swarm said `Run log: not written (...)`, stop and say so rather than falling back to an older log
  whose identifiers will not match. If the user names no run and no `Run log:` line is at hand, list the recent logs
  whose header names the current repository root — the name's commit id and time usually make the right one obvious —
  and confirm the choice with the user before proceeding. If there is no log, stop: without it there is nothing to join
  to.
- **The working document, if any.** Where the user does the triage. This skill has no opinion about what it is, and must
  not acquire one: a Claude artifact, a Google Doc, an internal review tool the agent can only reach by handing text to
  the user to paste, a scratch file — whatever the user says ("working doc: Claude artifact", "working doc: the internal
  tool, I'll paste"). Everything the skill does to the document reduces to three operations — create it, put findings
  into it, read it back — and when the environment cannot do one directly, the fallback is to give the user the text and
  let them do it, never to substitute a document form of your own. The document is also optional from the skill's point
  of view: a user who triages elsewhere and simply narrates verdicts ("F3 in, F4 is wrong, skip the rest") needs no
  document at all, and the skill must not ask for one. Ask where things go only when the user asks you to put something
  somewhere and has not said where. When another skill or workflow already produces the document, this skill sits beside
  it and only needs to know which findings went in and what became of them.

Whether the skill can see the document matters for the record, so notice it early. A document the agent writes to and
reads back gives direct evidence of what the user did; an opaque one gives none, and silence in the conversation then
means "not visible", not "not engaged". "Recording verdicts" says what follows from that.

## The flow

1. Read the run log. List the finding identifiers and titles back briefly so the user can refer to them.
2. Bring a finding over when the user explicitly asks for it ("bring over F3, F7, SEC-PATH") or accepts it, even if they
   never use the words "bring over." Copy each one verbatim from the run log — header and body — into the working
   document, or hand the text to the user when the document is theirs to edit. Do not shorten, tighten, or reorder at
   this step; the restated body is the thing the user chose, and edits come later at their direction.
3. Iterate as directed: re-verify a claim against the code, rephrase for the audience, merge two findings into one item,
   reframe, drop. Do what is asked; each of these is the user forming a verdict, and the verdict is theirs.
4. The user may stop the item-by-item discussion and give a blanket instruction for the untouched findings, such as "fix
   everything else I didn't mention if you agree with it." Queue that as follow-up work for after triage is finalized;
   it does not start the fixes immediately. It also does not select those findings for the working document. Never bring
   over a finding covered only by a blanket instruction: it belongs in the document only if the user accepted it
   individually or explicitly asked to bring it over.
5. When the user says the triage is complete (`finalize triage`, or plainly that they are done), resolve anything still
   uncertain in one batch of questions, write the triage record, and print a one-line summary. What happens to the
   accepted findings afterwards — a review posted by another tool, code changed by hand, a note to someone — is not this
   skill's job; if the user queued follow-up work, return to it only after the record is finalized. If the user wants
   the final text handed somewhere, hand it over without assuming which of those it is.

Writing the record at finalize is the norm. If the session looks like it may end early — a long triage, a user who says
they will continue later — write an in-progress record with what is known so far (see "The record") and complete it on
finalize.

## Recording verdicts from the conversation

The record's three states are `accepted`, `rejected`, and `skipped`. They are not labels the user is expected to say;
they are the skill's reading of what the user did. Understanding what each one means matters more than any rule for
assigning it, because most verdicts are delivered as ordinary conversation, and the skill has to recognize them as they
go by.

- **Accepted** means the user, having looked, decided the finding was worth carrying into the outcome of the triage —
  whatever that outcome is. The finding may have been reworded past recognition, merged with another, or narrowed to one
  of its claims; what makes it accepted is that the user engaged with its substance and chose to keep it. "Bring this
  one over and tighten the wording" is an acceptance. "Yeah, I'll fix that" is an acceptance, even though nothing gets
  posted anywhere.
- **Rejected** means the user looked and decided against it: it is wrong, or a misreading, or right but not worth
  raising here, or contrary to a decision they have already made. The defining feature is a judgment after engaging.
  "Nah, that's a misread of the lock order" is a rejection. "Drop it, not worth the noise" is a rejection too, and the
  reason is exactly the kind of signal the record wants.
- **Skipped** means no judgment was formed. The user sifted past it: never brought it over, never commented, or said
  something like "I'm not going to look at the rest." A skipped finding may be correct, and the record must not pretend
  otherwise — skipped is the honest state for "this was not worth my time in the context of this review", and it is
  expected to be the most common state. It carries its own signal: a category whose findings are always skipped is
  costing tokens for nothing, whichever way they would have gone.

The line between rejected and skipped is engagement, not tone. A one-word "no" after reading a finding is a rejection;
silence is a skip. A finding brought into the working document and then dropped after some discussion is rejected,
because the user looked. A finding brought over in a batch and never mentioned again is provisionally skipped — being
placed in the document by name is not by itself evidence the user read it — and goes into the finalize batch of
questions rather than being guessed at. A blanket instruction for untouched findings does not establish a verdict on any
of them either; unless the user later engages with an individual finding, it remains skipped when the agent could see
the triage.

When the conversation is ambiguous, prefer the state that claims less: skipped over rejected, rejected over accepted. A
wrong `accepted` fabricates a success; a wrong `rejected` fabricates a judgment; a wrong `skipped` only loses one data
point. Do not interrupt a working session to classify every remark; collect the uncertain ones and ask at finalize, in
one batch.

That preference has one important limit: `skipped` is a claim that the user did not engage, and it is only honest when
the agent could have seen the engagement. When the triage happened inside a document the agent cannot read — the user
worked in their internal tool and came back with "done" — the agent has no idea which findings were handled. Silence is
then lack of visibility, and defaulting everything to `skipped` would fabricate thirty verdicts at once. In that
situation ask, at finalize, for the identifier-level outcome of what the user handled, in one batch ("which of these did
you take, which did you throw out, and is everything else untouched?"); default to `skipped` only the findings the user
confirms they did not look at. If the user does not want to go through them, leave those findings pending in an
in-progress record rather than inventing states for them.

## The judgment field

Beside each state, record the user's own words on why, verbatim — trim for length if you must, never rephrase, and never
translate into a category. "Real serious bug, would have shipped." "Worth it for style." "Meh, technically right."
"Wrong, that path can't be reached." These are the highest-value bytes in the record: the states say what happened, the
judgment says how much it mattered, and the vocabulary for that is deliberately not fixed in advance. Later analysis can
cluster them; this skill must not pre-cluster them.

If the user accepts or rejects something without saying why, ask once, briefly, at a natural pause — not mid-thought. An
empty judgment is acceptable; an invented one is not. Never write a judgment the user did not express.

## Things the user adds themselves

Sometimes the user adds feedback of their own that no reviewer produced. Record it under a `user-added` heading with the
text and nothing else — no invented identifier, no reviewer category, no confidence tag, no state. It is not a swarm
finding and the record must not dress it as one. If tracking it is not useful, leaving it out entirely is also fine;
what is not fine is forcing it into the shape of a finding it never was.

The same principle governs every other edge. A finding merged into another keeps its own state and gains a
`merged into: <id>` line — the state follows whatever happened to the surviving item, since two duplicates can be merged
in order to accept one or in order to drop both. An item split in two stays one finding with a note. Anything that does
not fit gets a plain-language note rather than a fabricated field.

## The record

Write the record next to the run log, with the trailing `.md` replaced by `.triage.md`, so the two are found together
and joined by the identifiers the run log already carries. Markdown, one block per finding:

```
# Triage of <run log filename>

working document: <what the user said it was, or "none">
finalized: <UTC timestamp>, or "in progress"
pending: <identifiers not yet resolved, only while in progress>

### F3 / SEC-PATH
state: accepted
judgment: real serious bug, would have shipped
final: <the text as it was actually used, verbatim, when known>

### F4 / TEST-RNG-FAILURE
state: rejected
judgment: wrong, that RNG path is unreachable in prod

### F5 / SIMP-DUP-DETERMINISM
state: accepted
merged into: F3

### F6 / DOC-CAP-PROTECTION
state: skipped

### user-added
<text>
```

`final` is worth capturing when it is genuinely available, because the distance between the restated finding and what
was actually used is itself a signal about how the swarm phrases things. It is only ever text the user pasted back or
text the agent itself wrote into a document it can read; the verbatim run-log copy handed over in step 2 is never
`final`, and writing it there would be a reconstruction. Omit the line rather than reconstruct. When several findings
were merged, the merged text goes on the surviving identifier.

A finalized record contains every finding in the run log exactly once, each with one of the three states. An in-progress
record contains only the verdicts the conversation already supports plus the `pending:` list of everything else; it
never assigns a state merely to complete the file. Finalizing, or re-finalizing after a reopened triage, rewrites the
whole file — never append, since appending is how a finding ends up in the record twice.

## What this skill must not do

- Assume anything about why the user chose `nofix` or fix mode, whose code is under review, what the working document
  is, or what the accepted findings are for. All of these vary from run to run and none of them is the skill's business.
- Editorialize the record. No summaries of what the user "really meant", no severity scores the user did not give, no
  reclassifying a skip as a rejection because the finding looked wrong to the agent.
- Push back on verdicts. The agent may re-verify a claim when asked to and report what it found; the verdict is still
  the user's, and if they reject a finding the agent believes is correct, the record says `rejected` with their words.

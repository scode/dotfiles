---
name: scode-commit-msg-reviewer
description: >
  Use only when the user explicitly invokes scode-commit-msg-reviewer by name, asks for commit messages to be checked
  by a fresh-context reader, or their standing instructions say to always use this skill. Do not trigger merely
  because a commit or PR is being created. Puts commit messages, PR titles/descriptions, and equivalent VCS prose in
  front of a cold reader who has not seen the change, and rewrites until that reader understands them; after the first
  signal, applies to all commit-message-like content for the rest of the conversation. Also use when the user says
  "scode-commit-msg-reviewer feedback: ..." to record feedback about how this skill performed.
---

# Scode Commit Message Reviewer

Every commit message, PR title, PR description, or equivalent VCS prose (jj/Sapling change descriptions, stacked-PR
bodies) you produce goes in front of a cold reader before you use it: a fresh subagent that sees the message and nothing
else, and reports back what it understood and what it could not follow. You then rewrite until the reader's
understanding matches what you meant. The reader's charter is `reviewer.md` next to this skill file.

Do not run this skill just because a commit or PR is being created and the skill happens to be available. It activates
only on an explicit signal: the user invokes it by name, asks for this kind of check, or has standing instructions
saying to always use it. Once triggered, it applies to every commit-message-like text you produce for the rest of the
conversation — the user does not re-invoke it per commit.

It applies to messages the agent generated, with or without light guidance from the user. When it is clear from context
that the user already has a precise message — they wrote it themselves, pasted one in, or are tweaking wording they
authored — do nothing: no reader, no rewrite, not even a suggestion. The loop exists to make agent-written text
understandable to a cold reader; a message the user wrote is already what they mean, and running it through the loop
would replace their words with the agent's.

## Why a cold reader, and why you do the rewriting

The message's only real audience is someone with no context: reading a log or a PR list, deciding whether to open the
change, or git-blaming their way here years later to find out why. A message can be accurate in every clause and still
fail that reader, because the author knows what "the new helper" and "the second branch" refer to and never notices that
the reader does not.

A reviewer who has read the diff cannot catch this either — they know what the references mean too, so the references
look fine. The only party that reliably notices a gap in the message is someone who does not have the knowledge to fill
it. So the reader is kept ignorant on purpose: no diff, no repo, no conversation, just the message. Its job is not to
judge but to report what the words conveyed, in the form of a paraphrase and a list of what it could not follow.

The rewriting stays with you because you are the only one who knows what the message was supposed to mean. A cold reader
can tell you where the words fell short; it cannot tell you what should have been there. Rewrite from scratch rather
than patching: a fresh draft aimed at the gaps beats an edited one that keeps the old skeleton.

## Loop

1. Draft the message, following the target repo's conventions and the drafting rules below.
2. Materialize the temp files: the candidate message, the diff it describes, and — when the user gave any explicit
   instructions about the message's content, structure, length, or wording, at any point before now — an instructions
   file holding those instructions verbatim (or faithfully paraphrased when verbatim is impractical), including any
   wording the user wrote by hand that the candidate preserves. Write the instructions file once and reuse it every
   round: it is what keeps the user's brief intact across rounds and across context compaction, and it is what you
   reread before each rewrite so the rewrite does not drift from what was asked.
3. Spawn a subagent with fresh context. Its prompt contains only: an instruction to read `reviewer.md` (absolute path)
   and follow it, the candidate file path, the diff file path (which the charter tells it not to open until its second
   phase), the instructions file path if there is one, and the repo root and convention file paths if any. Nothing else:
   not your reasoning, not the change history, not earlier rounds. The instructions file tells the reader which wording
   is the user's, so it reports on it rather than the author being asked to change it.
4. Read the report. Compare the paraphrase with what you meant, slot by slot. If the reader could not say what was wrong
   or wanted before the change, the message has no problem statement, and no amount of detail below it will substitute.
   Where the paraphrase is vague, hedged, wrong, or missing the point, the message did not carry that point — that is
   the finding, whether or not the reader listed anything under "could not follow". The "could not follow" list is a
   diagnosis, not a to-do list: it tells you where the reader lost the thread, not which sentences to insert. "Did not
   need" is the opposite signal and just as real: sentences the reader could drop without losing anything are length the
   message charges every future reader for; cut them unless they carry a why the reader missed. "Against the change"
   names claims to correct. "Conventions" names mechanical fixes.
5. Decide whether the message is done. It is when the paraphrase matches your intent, nothing is contradicted, and every
   remaining "could not follow" item is something the message is right not to explain. The reader reports from total
   ignorance, which is what makes its report trustworthy, but the message's real audience is not totally ignorant: it
   knows what the project is and what its main tools are called. A reader who cannot tell whether `jjstack` is a command
   or a skill, in a repository whose commits are mostly about jjstack, has reported a true fact about the words that is
   not a defect in the message. That call is yours, and it is the judgment the reader is deliberately not making; make
   it honestly, against the repository's actual readers, not against your own familiarity. Otherwise rewrite for that
   reader, and rewrite means start over: begin from the problem statement, in plain words, and tell it through — what
   was wrong, what this does about it, why this way. Reread the instructions file first, if there is one. Do not answer
   the reader's items one by one. A draft that answers a list reads like a list of answers to questions the next reader
   never saw asked, which is exactly the mid-text feel this loop exists to remove; if your new draft could have been
   produced by inserting sentences into the old one, it is not a rewrite. Then go back to step 3 with a **new**
   subagent. Never continue a previous reader: one that has seen an earlier draft is no longer cold. Fresh readers keep
   finding new things, so expect diminishing returns after the first rewrite and do not chase a report that has stopped
   changing what you would write.
6. Hard cap of three readers per message. If the third reader still cannot follow it, stop, show the user the candidate
   and the last report, and let them decide.

The user's explicit instructions outrank anything a reader reports. Never remove or rephrase something the user asked
for because a reader did not follow it; keep the user's version and mention the tension to them instead.

If the environment cannot spawn subagents at all, tell the user the cold read could not run, then do the best substitute
available: reread the message pretending you have not seen the diff, and rewrite anything you would not have understood.

## Drafting rules

These are the things a good message does and the traps a bad one falls into. Apply them when drafting and when
rewriting; the cold reader will catch some of them indirectly, but it is cheaper not to write them in the first place.

The reader model: cold, no code knowledge, reads the message before the diff. Plain language, with jargon only where a
plainer word would lose real meaning. Every reference introduced in the message itself. Nothing that presumes the reader
is looking at the patch.

The title is very concise and describes the user-visible effect of the change, at the altitude of someone scanning a
list of titles: the kind of change and roughly where it lives, not an inventory of its contents. Naming the component
the change lives in — a skill, a subsystem, a command — is orientation and belongs there; a type name or an internal
codename that only someone who has read the code can parse does not. A bug fix that required heavy refactoring is still
a fix. If the repo mandates a format such as Conventional Commits, get the type right.

The body carries the _why_: motivation, constraints, tradeoffs, rejected alternatives, surprising omissions, follow-up
risks — the context a future reader cannot recover from the diff. Order it big picture to detail: the purpose or problem
first, mechanics and side points later. Length is fine when the material is useful; filler because a body field exists
is worse than nothing, and an empty body is often the right one.

Traps, each a reason to rewrite:

- **Diff narration.** Listing or paraphrasing what changed. The diff shows that; the message should say why. A short map
  of a genuinely large change is different from an inventory.
- **Manufactured motivation.** Dressing routine work in an invented rationale. "Libraries were out of date" is not a
  problem being solved; a dependency bump says so plainly or says nothing. Problem/Solution headings only when there is
  real explanatory context.
- **Softened bug framing.** A fix should say directly what was broken, at the level the affected reader cares about,
  before any mechanism. Be suspicious of openings that narrate call flow or evidence instead of naming the failure.
- **Justification before the decision.** Arguing for a choice the reader has not yet been told exists. Name the
  decision, then defend it.
- **Buried purpose.** Hundreds of characters before the reader knows why the change matters.
- **Validation and attribution boilerplate.** "Tests run: …" lines, tool-credit trailers and badges — out unless the
  repo's own rules require them. An honest caveat about how much to trust a section is not a badge.
- **Marketing register.** Enthusiasm, buzzwords, "robust", exclamation marks.
- **Vague or overlong title.** "fix bug", "update code", or a first line that runs on.

## Feedback capture

When the user says "scode-commit-msg-reviewer feedback: ..." (or clearly signals feedback about how this skill
performed), read `feedback.md` next to this skill file and follow it.

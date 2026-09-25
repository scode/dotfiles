# skillette-cr-triage

Walk the user through a set of code review findings one at a time and record an outcome for each in a queue file that
survives across sessions. This is triage only: deciding that a finding needs a code fix queues the fix, it does not make
it. Do not edit code or specs during triage. If the user asks outright to fix something now, record the outcome first,
then treat the fix as the separate request it is.

Finding bugs is half the point. The other half is steering the project toward a state where agent reviewers stop raising
the same invalid findings over and over. A lot of findings are wrong only because the reviewer did not know a constraint
("this UI does not scale to ten million hosts" when the project is scoped to a few hundred; "this prints command-line
input unescaped, security issue" when the project does not consider that a bug). The durable answer to such a finding is
a SPEC change that states the constraint, so the next reviewer reads it and does not raise the finding again. Keep that
in mind when assessing findings and proposing outcomes.

## Outcomes

Every finding starts `pending` and ends with exactly one of these:

- `spec`: change `SPEC.md` or `SPEC_impl.md`. Typically the finding is wrong because the reviewer lacked a constraint,
  and the spec should state it. Record what the spec should say, in the user's words where they gave any.
- `fix`: a legitimate finding; queue a code fix.
- `spec+fix`: both, for a finding that is partly right and partly a missing constraint, or a real bug whose fix also
  deserves a stated rule.
- `discard`: drop it. This carries no opinion about whether the finding is right; it only means it is not worth
  discussing. Do not ask the user to justify a discard.
- `other`: anything else, recorded as the user's own natural-language description.

## Findings and the queue

The user may say where the findings are (a file, a PR, a review tool's output, a pasted list) or point at an existing
queue, in which case go to "Resume". Otherwise the findings are whatever a code review produced earlier in this session.
If that is unclear, or the session has no findings, ask which findings, or which queue to resume. If the user does not
have a queue's URL at hand, their gists described `cr-triage:` are the candidates.

Settle the questions before assessing, so the user is not left waiting through a long assessment to answer them. If the
user has not said where the queue goes, ask, offering a secret GitHub gist as the default (readable by anyone with the
link, which matters for private code or security findings) and taking anything else the user names. If the findings
exist only in this session, not stored anywhere that will outlast it, ask in the same question whether to save a copy
next to the queue, recommending yes: the queue's `Sources:` references mean nothing once the session holding the
originals is gone. Write that copy before assessing, since a long assessment risks compaction.

Never copy secret values (keys, tokens, passwords) into either file; keep their location. If any read or write of the
queue or the findings copy fails, stop and ask the user how to proceed rather than working around it.

For a gist, the queue is `TRIAGE.md` and the findings copy is `FINDINGS.md`, with local copies in a temporary directory
outside the repository. A gist holding `FINDINGS.md` but no `TRIAGE.md` is an interrupted build; build the queue there.
`<id>` is the last path segment of the gist URL. Writes go through `gh api` because `gh gist edit` is editor-oriented,
and `--silent` keeps the whole gist from being echoed back after every decision.

```sh
gh gist create --desc "cr-triage: <short subject>" "$dir/FINDINGS.md"      # or TRIAGE.md, if not saving findings
gh gist view <id> --raw --filename TRIAGE.md > "$dir/TRIAGE.md"             # read
gh api --method PATCH gists/<id> --silent \
  -F "files[TRIAGE.md][content]=@$dir/TRIAGE.md"                            # write, or add the file
```

## Build the queue

Keep every finding. The only reduction is merging clear exact duplicates (same defect, same place, same cause) into one
entry that lists all their sources. Related findings, or the same pattern in different places, stay separate and
cross-referenced, and adjacent where the buckets and ordering allow.

Assess every finding for correctness against the code and record the assessment in its entry. Hold a high bar before
calling a finding invalid; someone produced it for a reason, and "I could not immediately see it" is not a refutation.

Sort into three priority buckets by what the finding claims would happen if it were right, not by your assessment:

- P1: security, data loss, and critical correctness issues, with "critical" as the project defines it.
- P2: serious UX or behavior bugs.
- P3: minor issues, very rare edge cases, and general code style or taste.

A P1-class claim stays in P1 even when it applies only in rare cases; rarity alone does not move security, data loss, or
critical correctness to P3. A "security" finding you believe is invalid stays in P1, where it gets attention as a `spec`
candidate. Within a bucket, put the most severe and most certain findings first.

## The queue file

The file must be readable and editable by a person, and must hold enough of every finding that a fresh session can
triage it without the original review output. Use Markdown in this shape by default:

```markdown
# Code review triage: <short subject>

- Findings: <where they came from, precise enough to recognize them again; e.g. review of PR 123 at commit abc1234>
- Next: T8 (ID for the next new entry, not a walk cursor)

## P1: security, data loss, critical correctness

### T1. <one-line title>

- Sources: <where in the findings this came from; all of them when merged>
- Location: <path:line, or the area of the code>
- Finding: <the finding itself, verbatim or faithfully condensed; keep the reviewer's argument>
- Assessment: <valid, doubtful, or likely invalid, with the reason>
- Outcome: pending
- Notes: <the user's reasoning; for spec, what the spec should say; for fix, anything that constrains the fix>

## P2: serious UX and behavior bugs

## P3: minor, rare, and style
```

Number entries `T1`, `T2`, ... in queue order and never renumber or reuse them, so the user can refer to them across
sessions; `Next:` holds the number the next new entry gets. Write the file before starting the walk, then tell the user
where it is.

## Walk the queue

Go through entries in queue order, one at a time. For each, give a short summary of the finding, your assessment, and a
proposed outcome with a sentence of reasoning, then let the user decide. Take their answer in whatever form it comes;
"fix", "spec it, we never support more than 500 hosts", and "discard the rest of P3" are all answers. An entry the user
skips stays `pending`.

Write each decision to the file as soon as it is made; the file is what makes a resume possible. Re-read it first if the
user says they edited it.

At the end of the queue, offer to revisit any entries still `pending`.

## Resume

Read the queue. If the user also named findings, confirm its `Findings:` line matches them. Say how many entries are
decided and how many remain, and continue from the first `pending` entry in queue order. If the user brings new findings
to an existing queue, ask whether to append them or start a new queue. When appending, extend the `Findings:` line and
ask the save-a-copy question for any that exist only in this session.

## Finish

When the walk ends, whether or not entries are still `pending`, give the user the counts per outcome and the list of
entries queued for `fix`, `spec`, and `spec+fix`.

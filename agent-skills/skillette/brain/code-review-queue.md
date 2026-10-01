# Code review queues

Read this when the user asks to put code review findings into the brain's code review queue for a project, or to read,
drain, or otherwise work with such a queue. Everything in [SKILLETTE.md](SKILLETTE.md) still applies: snapshot reuse,
operation worktrees, mechanical commits, publication and its verification.

NOTE: A queue is not a review record. It is the set of open findings for one project and nothing else. It does not
record which reviews ran, when, with what scope, what they did not find, or why a queue is small or empty. That is
deliberate. The user runs AI code review when it is convenient to spend the inference, parks whatever it finds here, and
drains the queue into the project later, through triage, fixes, or spec changes made in the project itself. Nothing is
committed to the project when findings are queued, which is why the queue lives in the brain and works for any
repository.

## Where a queue lives

A project's queue is the directory `code-review/<host>/<owner>/<repo>/` inside the `personal` brain, or inside another
brain only when the user names one. `<host>` names the forge so that other forges can be added later without renaming
anything. Only `gh` (GitHub) is defined so far; ask before queueing findings for a project hosted anywhere else.

Map the user's project name to a path like this: `scode/dotfiles` is `code-review/gh/scode/dotfiles/`, and a bare
`dotfiles` means the same, because a bare name defaults to the `scode` owner. Use any other owner exactly as given. Keep
the owner and repository segments as GitHub spells them, lowercased. Ask when the project is ambiguous instead of
guessing, since findings filed under the wrong project are easy to miss later.

The directory holds `INDEX.md` and one Markdown file per finding, nothing else. `BRAIN.md` gets one row per project,
linking to the queue's `INDEX.md`, with a description such as "Code review queue for scode/dotfiles: open findings
awaiting triage." Keep that description free of counts so it does not change on every ingest. The queue's own index is
part of the queue, not a second brain-wide index.

## INDEX.md

```markdown
# Code review queue: scode/dotfiles

Project context: <one short paragraph on what security, data loss, and critical correctness mean for this project>

## P1: security, data loss, critical correctness

- [`symlinked-home-deletes-payload.md`](symlinked-home-deletes-payload.md) — the installer follows a symlinked `$HOME`
  and deletes files outside the managed tree.

## P2: serious UX or behavior bugs

## P3: minor, rare edge cases, style
```

The index must list exactly the finding files in the directory, each once, under the bucket it belongs to. Each line
names the file and summarizes the problem in one line, in terms specific enough to recognize a duplicate without opening
the file. Order within a bucket carries no meaning; the queue is an unordered set. There are no sequence numbers and no
"next ID": the file name is the finding's identity. Write the project context when the queue is created, from what the
project is and does, and keep the user's edits to it.

The bucket recorded in the index is authoritative. Do not repeat it inside finding files, where it would drift.

## Buckets

The buckets match those of the `cr-triage` skillette, so a queue can feed that flow without re-sorting:

- P1: security, data loss, and critical correctness issues, with "critical" as the project defines it.
- P2: serious UX or behavior bugs.
- P3: minor issues, very rare edge cases, and general code style or taste.

Bucket by what the finding claims would happen if it were right, not by how likely it is to be right. Rarity does not
move a security, data loss, or critical correctness claim out of P1. When a finding sits between two buckets, use the
higher one. When a merged duplicate's sources disagree, the higher bucket wins.

## Finding files

Name each file mnemonically after the problem, in kebab-case (`stale-cache-on-rename.md`), never after a number, date,
or review run. If the name is taken by a different finding, choose a more specific name.

```markdown
# <short title>

## TLDR

<what goes wrong for the user of the project, in plain terms>

## Details

<agent-facing: file paths with line numbers against the reviewed commit, what the code does wrong, how to verify it, and
what a fix looks like when known; mention related queue files by name>

## Sources

### <tool or reviewer> · <original finding ID, if any>

Reviewed commit: <full hash, or the abbreviated hash the review gave>

Confidence as filed: <the reviewer's own tag, if any>

<the finding's verbatim text from the review output, plus any verbatim reviewer items or decision text that were merged
into it>
```

TLDR and Details are condensed and are what a reader uses to triage. Sources keeps the original wording, so that nothing
the reviewer argued is lost to condensation. Every finding has at least one source. A source records where the finding
came from, not a review run: there is no run metadata beyond what a reader needs to find the code the finding is about.

## Putting findings into a queue

The findings are whatever the user points at: review output from earlier in the session, a file, a PR, a gist, a pasted
list. If that is unclear, ask.

Keep every finding the review reported. Do not assess validity, re-read the code to confirm claims, or drop findings
that look wrong; that is the job of whoever drains the queue, and doing it here would spend inference the user meant to
spend later. Record the reviewer's confidence as filed. Drop only material that is not a finding: accounting lines,
panel descriptions, summaries of what was not found, praise.

Deduplicate against the target project's queue only, never across projects. Read `INDEX.md`, and open the existing files
whose summaries or locations make them candidates. Merge a new finding into an existing file only when all four of these
are clearly the same: the defect, the location (allowing for line drift between commits; match by symbol rather than
line number), the cause, and the consequence. Two reviewers describing one bug in different words is a duplicate; the
same mistake repeated in two places, or two bugs in one function, is not. When in doubt, it is not a duplicate: create a
new file and mention the near-duplicate by name in its Details. A spurious duplicate costs the user a moment during
draining, while a wrong merge silently loses a finding.

A merge appends a source block to the existing file. Edit its TLDR and Details only to add a fact the new source
contributes; never rewrite the existing condensation to match the new source. Raise its bucket if the new source claims
a more severe consequence. Findings that duplicate each other within the incoming batch merge the same way.

Never copy secret values (keys, tokens, passwords) into a queue. Keep their location.

Commit an ingest as one change: new files, merged files, index, and the `BRAIN.md` row for a new queue. Before
committing, check that the index and the directory list the same files. Report to the user how many findings were added
as new files, how many merged into existing ones, and how many were kept separate despite a near-duplicate.

## Reading and draining

Reading a queue starts from `INDEX.md`; open finding files only as needed. Listing a project's P1 findings needs nothing
but the index.

Draining is removal. When the user says a finding is handled, dismissed, or moved into the project, delete its file and
its index line in one commit. If only part of it was handled, narrow the file to what remains and update its index line.
Do not keep closed findings, tombstones, or a record of what was drained; durable decisions belong in the project
itself, for example in its specification. When the last finding goes, delete the queue's directory and its `BRAIN.md`
row in the same commit rather than leaving an empty queue.

## Concurrent writers

Two writers ingesting into one queue usually touch the same `INDEX.md`. After a rejected push, the normal reconciliation
applies: keep both sides' index lines and files. If both writers created a file under the same name, compare them; if
they are the same finding, merge the sources into one file, and otherwise give the newer one a more specific name. Do
not run a fresh deduplication pass across the other writer's additions as part of reconciliation.

# skillette-change

Capture a requested change to the `skillette` skill, or to any skillette in it, without derailing the current session.
"Change" covers adding a skillette, removing one, editing one, and editing `skillette` itself (the table, `SKILL.md`,
`SPEC.md`). The user says what they want, you record it where it can be acted on, you report one line, and you go back
to what you were doing. The whole message carrying the trigger is the request; if there is no request around the
trigger, ask what the user wants changed. A request about some other skill is out of scope: say so in a line and do not
file anything.

The behavior contract is `SPEC.md`, next to the top-level `SKILL.md`. You read it only on the local-edit path, from the
checkout you are about to edit; the issue path needs nothing beyond this file.

## Step 1: pick local edit or GitHub issue

Check whether the current directory or any of its ancestors contains `agent-skills/skillette/SKILL.md`. This is a file
check rather than a `git` query on purpose; some of the user's dotfiles checkouts are jj-only workspaces where `git`
commands fail, and walking up from the current directory works from any subdirectory without asking a VCS for the root.
The ancestor where the probe hits is the checkout root; paths in Step 2a are relative to it.

If the probe finds the file, you are in a dotfiles checkout and the user may want the change made right here. Ask
whether to edit locally or file a GitHub issue, unless the conversation already answers that: the session is already
editing this skill, or the user said which they want. When you do ask and the request is an addition, put the trigger
proposal from the next section in the same question, so that choosing the issue path does not cost a second round trip.

If the probe finds nothing, or you cannot tell where you are, file an issue. Do not dig further, and do not try to
locate a dotfiles checkout elsewhere. The issue path is always acceptable; the local path is only an optimization for
when you are already in the right repo.

## For additions, settle name and triggers

This applies on both paths whenever the request adds a skillette or changes a row's triggers. Removals, edits to an
existing skillette that leave its row alone, and edits to `skillette` itself skip it.

Two things must be settled: the skillette's kebab-case name, which becomes the explicit trigger `skillette-<name>` the
user will type, and whether natural-language triggers are wanted and, if so, what phrase. If the user already gave a
name, said "explicit trigger only", or gave the phrase, that part is answered; do not ask again. Otherwise ask, but ask
well: propose the obvious name and the obvious phrase rather than posing bare questions, and say so when explicit-only
cannot work (a skillette meant to fire on your own actions, such as before a commit, has no moment for the user to type
a trigger). On the issue path this rides in the one batched question described under Privacy, never as a separate round
trip.

Harness neutrality is not a question. Skillettes are harness-neutral by default; record an exception only when the user
asked for one.

## Step 2a: local edit

Read `agent-skills/skillette/SPEC.md` under the checkout root (not the installed copy, which may be a different
checkout), then make the edits under `agent-skills/skillette/` there. Adding a skillette means a new
`<name>/SKILLETTE.md` and one table row; removing means deleting both; changing means editing files. Nothing in
`src/main.rs` changes. If the edit changes user-facing behavior, update `SPEC.md` in the same edit.

Stop when the files are edited. Do not commit or open a PR on your own; the user and the surrounding session decide
that.

## Step 2b: GitHub issue

The reader of the issue is a cold agent in a fresh session with no memory of this conversation. Your job is to write
down enough that it would design roughly the change you would have designed right now. Optimize for not losing context.

### Template

The template is a skeleton, not a form. Fill what you know, drop sections that would be empty, and add anything that
does not fit. Extra context is the point.

```
Title: skillette: <imperative summary, e.g. "add a foo skillette">
Label: skillette-change

## Request
What to add, remove, or change, in the user's own terms. Quote the user where their wording carries intent, after
checking the quote against the privacy rules below.

## Context
Why the user wanted this and what in the session prompted it: the situation the skillette should handle, what went
wrong or was tedious without it, what the user seemed to be optimizing for. This section is where most of the value is.
Describe it in generic terms; the privacy rules constrain identifiers, not substance.

## Notes for the implementing agent
Read agent-skills/skillette/SPEC.md first. Keep it harness-neutral unless stated otherwise.
Name: <name>. Natural-language triggers: <the phrase | none, explicit trigger only>.
Anything else the cold agent needs: sketches of the SKILLETTE.md content you had in mind, edge cases the user
mentioned, related skillettes or skills to look at, things the user explicitly did not want.

## Done when
One or two lines of acceptance, if the user stated any.
```

### Privacy and security

The repository is public. Concrete identifiers from the session do not go into the issue unless the user approves them
for this issue. That covers, non-exhaustively: paths under the home directory, hostnames, employer, client, project, or
people names, email addresses and handles, URLs and ticket IDs, credentials or anything shaped like one, and pasted tool
output, error text, or code from the session. Treat the current repository as private unless you already know it is
public. Generalize or omit these by default; the situation and the reasoning can still be described fully in generic
terms.

Approval is per item and per issue: "include the repo name in this one" is approval, while a general remark such as
"repo names are fine" or "stop asking me about privacy" is a request to change this skillette's policy, which you record
in the issue as a request and do not act on for the current filing. Even an approved identifier goes in only when
leaving it out would lose something the cold agent needs; most requests are about policy or behavior and need none.

Before filing, reread the title and body. The items to ask about are the identifiers where generalizing or omitting
would lose substance the cold agent needs; everything else is generalized silently. If there are any such items, or if
the name and trigger question is still open, ask one question that lists all of it: each item with include, generalize,
or omit, plus the name and trigger proposal if needed. One question total, not one per item. Whatever the answer leaves
unaddressed defaults to omit, and any new identifier the answer itself introduces is generalized without asking again.
That is the only reason to hold the issue before creation; with nothing open, file immediately. Do not look for an
existing issue on the same subject; duplicates are cheap.

### Filing

Write the body to a temporary file outside the working copy (so it cannot end up in someone's next commit) and create
the issue with something like

```
gh issue create --repo scode/dotfiles --label skillette-change --title "<title>" --body-file <file>
```

If the label does not exist yet, create it with `gh label create skillette-change --repo scode/dotfiles` and retry. If
creating the label fails (for example, no write access to the repo), file without `--label` and mention the missing
label in your one-line report.

If `gh` is missing, not authenticated, or fails for any other reason (no network, a sandbox that blocks egress, wrong
account, insufficient token scope), do not investigate and do not try another route. If your harness offers a one-time
approval or escalation for the command, request it once. Otherwise ask the user whether they want to fix `gh` now and
have you retry, or whether you should print the finished issue text for them to file by hand. Do what they pick.

## Step 3: report and resume

Say in one line what happened: the issue URL, or the files you edited. On the issue path, include how many identifiers
you generalized or omitted, so the user knows whether the public text is worth a glance. Then return to whatever the
session was doing before the trigger.

# skillette specification

This file defines the user-facing behavior of the `skillette` skill and the contract every skillette inside it must
follow. Agents changing this skill or any skillette must conform to it. If the files and this spec disagree, that is a
bug, or the spec must be updated in the same change.

NOTE: This is not the skill itself and nothing on the trigger path reads it. Its only readers are the `change`
skillette, on its local-edit path, and the cold agent that picks up a `change` issue. If some other skillette or flow
finds itself needing this file, that is a sign the design has drifted.

## What a skillette is

A skillette is a small skill-like behavior that is too minor to justify a full skill of its own. The whole point is that
adding one costs almost nothing: a directory with one markdown file, plus one row in a table. There is no registration
step in the installer, because the entire `skillette` directory is installed as a single symlink per harness.

## Why the top-level file is tiny

`SKILL.md` is designed to be resident in context for a whole session. A user either invokes `skillette` explicitly or
keeps a line in their personal agent instructions saying to load it at the start of every session. Every line in
`SKILL.md` is therefore paid for on every turn of every session, so it carries nothing but the frontmatter, one short
paragraph on how to resolve a trigger to a file, and the table. Everything else lives behind the table in the
per-skillette directories.

## Layout

```
skillette/
  SKILL.md              frontmatter, the resolution paragraph, and the trigger table; nothing else
  SPEC.md               this file
  <name>/
    SKILLETTE.md        the skillette's instructions, read only on a trigger match
    ...                 optional supporting files (further .md for progressive disclosure, scripts, references)
```

Skillette names are kebab-case. The directory name is the skillette name.

## The table

The table has exactly two columns.

The first column, "Triggers", is a space-separated list of explicit trigger words. Every one of them is kebab-case and
starts with `skillette-`, so that no explicit trigger can collide with ordinary prose or with another skill's
vocabulary. The first trigger in the list is always `skillette-<name>`, and that is how an agent maps a row to its
directory: strip the `skillette-` prefix from the first trigger and read `<name>/SKILLETTE.md`. Further triggers in the
same cell are optional aliases.

The second column, "Natural-language triggers", is either empty or a short phrase describing the situations in which the
skillette applies without an explicit trigger: something the user says, or something the agent is about to do, such as
committing. Empty means the skillette fires only on its explicit triggers. When present, the phrase is a trigger
description, not a summary of what the skillette does; keep it to the words that make the activation decision, because
the whole column is paid for on every turn.

## Triggers

Every skillette has at least the explicit trigger `skillette-<name>`. A user who types it bare or with a leading `$`
gets the skillette. Whether a leading `/` works depends on the harness, which owns that namespace and may swallow an
unknown slash command before the agent sees it, so this spec makes no promise about it.

The whole message containing a trigger is the request; the trigger only marks which skillette acts on it, since users
drop triggers mid-sentence as often as at the start. A trigger with no request around it makes the skillette ask what
the user wants.

An explicit trigger wins outright over another row's phrase match, so a `change` request that mentions some other
skillette by topic does not cost a round trip. If two rows still plausibly match, by phrase against phrase or by two
explicit triggers, the agent asks the user which one they meant rather than guessing. A `skillette-` word that matches
no row is reported as such, not mapped to the nearest one. Bare `skillette` or `$skillette` with no trigger loads the
skill and confirms that in a line; nothing else happens until a trigger arrives.

Natural-language triggers are optional and are a decision made when the skillette is added. The agent adding a skillette
must ask the user whether natural-language triggers are wanted, but only when the user has not already made that clear.
A user who says "explicit trigger only" or who hands over the trigger phrase has answered; do not ask again. When the
agent does ask, it proposes the obvious phrase rather than posing a bare yes/no, and it points out when explicit-only
cannot work (a skillette meant to fire on the agent's own actions, such as before a commit, has no moment for the user
to type a trigger). The reason this is a deliberate decision rather than a default is that agents left to their own
devices pile up trigger words and descriptions until the table stops being cheap.

## Locating the skillette file

`SKILL.md` is resident for the whole session, and a trigger routinely fires long after the harness's original load
message has been compacted away. Two things cooperate to recover from that, and both are load-bearing.

The frontmatter description is what re-summons the skill when `SKILL.md` itself has fallen out of context. It survives
compaction because it lives in the harness's skills listing, not in the conversation, so it must keep naming
`skillette-<name>` triggers; an agent that sees `$skillette-change` after compaction has nothing else to tell it which
skill owns that word. Trimming the description to just `skillette` would silently break recovery on every harness.

NOTE: Only explicit triggers survive that way on their own. A natural-language phrase or an agent-action condition lives
in the table, and the table is gone once `SKILL.md` has been compacted out; nothing in the skills listing mentions
commits or whatever else the phrase names. The mitigation is a standing instruction in the user's always-loaded agent
instructions that says to load `skillette` at session start and to load it again after every compaction. The
instructions file is part of the system prompt and so is still present after compaction, unlike the conversation; the
shipped `agent-instructions/AGENTS.md` carries such a line. This only works to the extent the agent notices compaction
and honors the instruction, so a skillette that relies on its second column may still go quiet mid-session. The
alternative, carrying the whole table in every session's description, was rejected on cost. Users choosing a phrase
should know they are choosing it.

The resolution paragraph in `SKILL.md` covers the case where the table is still in context but the base directory is
not. It compresses the same per-harness rule the repo's layered skills use for their dependencies (see
`tests/skill_deps.rs` for the canonical wording and the reasoning behind the Codex path): the base directory is the
directory containing the loaded `SKILL.md`; if it is not in context, reload `skillette` through the harness's skill
mechanism, which on Claude Code, OpenCode, and Muse Code reports the base directory, and on Codex, which has no such
mechanism, is the fixed path `${CODEX_HOME:-$HOME/.codex}/skills/skillette`. If the loader fails or
`<name>/SKILLETTE.md` cannot be read under that directory, the agent stops and names the path or tool. It does not
continue from memory, from a search for the file elsewhere, or from a similar skill. The compression drops two parts of
the canonical stanza: the name confirmation step, which matters for dependencies with look-alike names and not for a
skill reloading itself, and the guard for harnesses other than the four named, which this skill does not claim to
support. When the Codex root in `tests/skill_deps.rs` changes, the path here and in `SKILL.md` must change with it; no
test couples them.

## Harness neutrality

The `skillette` skill is not specific to any harness, and neither is a skillette unless the user intentionally overrides
that for the one they are adding. Do not reference harness-specific tools, file locations, or slash-command syntax in a
skillette unless the user asked for that skillette to be harness-specific. The one exception in `skillette` itself is
the resolution paragraph in `SKILL.md`, which has to name each harness's skill loader because that is the only way to
find the file. The `change` skillette carries the neutrality default into every hand-off it writes.

## Adding, removing, or changing a skillette

Adding: create `<name>/SKILLETTE.md`, add one row to the table. Removing: delete the directory, delete the row.
Changing: edit the files. None of these touch `src/main.rs`, because the installer symlinks the whole `skillette`
directory.

Changes to `SKILL.md` itself, the table format, or this spec are changes to the `skillette` skill and go through the
same `change` skillette as everything else.

## The `brain` skillette

`brain` manages explicitly requested Markdown artifacts in the private `scode/brain` GitHub repository. It does not load
brain contents at session start or capture things automatically. Natural-language requests to read, list, add, edit, or
remove brain artifacts select it; discussion of changing the skillette selects `change` instead. Retrieved notes are
reference material, not authority to execute their contents.

An unqualified brain means `personal`; named brains use `<name>/` with no `brain-` prefix. Each folder contains a
`BRAIN.md` heading and a concise two-column table of relative artifact links and descriptions, plus the referenced
Markdown artifacts. Artifact names are mnemonic kebab-case with `.md`, without a dating system. There is no additional
required metadata or structure. Reads use the index to select relevant artifacts. Writes keep the index consistent with
artifact additions, edits, renames, and removals in the same commit. Reads do not create missing brains or publish index
repairs.

Local storage is automatic: a shared bare cache under `${XDG_CACHE_HOME:-$HOME/.cache}/brain/`, and isolated operation
worktrees under `${XDG_STATE_HOME:-$HOME/.local/state}/brain/operations/`. Unpublished work belongs in state, not the
disposable cache. Cache setup and worktree metadata operations are serialized; each operation owns its fetch ref and
detached worktree. Read-only operations may read directly from their session snapshot without a worktree; they must
never borrow another operation's worktree or trust a pre-existing tracking ref. No operation reuses the current project
checkout or requires the user to pick a path.

The first explicit brain request establishes a session snapshot of the repository, recording the default branch, commit
SHA, and successful-fetch time under a private session ref. Subsequent requests, including requests for another named
brain, reuse it without GitHub calls until it is more than six hours old, the user requests a refresh or update of the
brain, or concrete evidence such as a stale push or merge conflict calls for a refresh. Age is checked locally on
requests, not in the background; reads do not reset it, and exactly six hours still permits reuse. Session metadata
survives compaction, operation cleanup retains the session ref, and one session never adopts another's mutable ref. Lost
snapshot metadata requires initialization on the next request. Failed refreshes preserve the previous timestamp and are
reported, with retained content identified as stale.

Writes may start from that snapshot without a pre-write fetch. Publication and its verification still contact GitHub,
and successful fetches advance the session snapshot to verified remote state, never to unpublished local work. A request
to edit an artifact is not by itself a refresh request. An already-satisfied request is answered from the snapshot
without a reconfirmation fetch or empty commit, and is not presented as a new check of current GitHub state.

A write succeeds only after a normal push publishes the artifact and index commit to the default branch and a fetch
verifies remote acceptance. A rejected stale push requires fetching, rebasing, and semantic reconciliation, including
preserving independent index additions and handling filename collisions. Force pushes are forbidden. Incompatible edits
requiring user judgment, persistent contention, and network or permission failures retain pending work and are reported
as not stored. An empty repository is initialized on `main`; a competing initialization is reconciled against the
winning history. Cleanup is limited to the completed operation. Later requests surface relevant pending operations
without automatically publishing them.

`brain/EVALS.md` records manual use flows, edge cases, and expected outcomes. Maintainers keep it current when behavior
changes, guided by `brain/AGENTS.md` and its `CLAUDE.md` symlink. Evals run only when requested, defaulting to a cheap,
fast available model without a hardcoded model name. These maintenance files are not part of normal brain use.

## The `ntfy` skillette

`ntfy` sends one notification to an explicitly supplied topic on `https://ntfy.sh`. Its short forms are
`ntfy <topic> <message>` and `$ntfy <topic> <message>` at the beginning of a send request; quoted examples and
discussion do not send anything. Embedded instructions such as "do the work, then ntfy <topic> that you're done" also
trigger it. Its explicit trigger is `skillette-ntfy`, with the same arguments. The short forms live in the table's
natural-language column so the explicit-trigger naming convention stays intact.

The topic must be explicitly supplied and match `[-_A-Za-z0-9]{1,64}` in full. In command-style requests it is the first
argument, and the remaining message is literal text, including newlines and shell metacharacters. In natural-language
requests the agent composes the requested meaning, preserving exact wording when supplied, and honors send conditions.
Completion messages wait for successful completion; a blocked task does not authorize a false success or an unrequested
failure notification. Missing topics or unclear message intent require clarification; invalid topics are rejected. There
is no default topic or reuse from previous requests, but a pending conditional send retains its explicitly named topic.
No real user topic appears in the shipped instructions or examples.

Publishing uses curl, assumed installed; if unavailable, abort without installing it or switching clients. Publishing
defaults to `high` priority. Overrides require a separate explicit instruction, not option-like text inside the message.
Reject messages over 4,096 UTF-8 bytes before publishing to prevent automatic attachment conversion. One invocation
makes one publish attempt, without automatic retries, splitting, or truncation. Success means confirmed server
acceptance, not confirmed phone delivery. Authentication errors are reported without searching for credentials. The
skillette neither manages subscriptions nor enables ongoing notifications.

## The `change` skillette

`change` exists so that "oh, by the way, skillette should also do X" can be said mid-session, in any project, without
derailing what the session was doing. Its trigger is `skillette-change` and it covers adding, removing, and changing
skillettes as well as changing the `skillette` skill itself.

The flow, in order:

1. Decide where the change is made. The probe is a file check: does the current directory or any ancestor contain
   `agent-skills/skillette/SKILL.md`? It is deliberately not a VCS query, because the user's dotfiles checkouts include
   jj-only workspaces where `git` commands fail. If the probe finds the file, the session is in a dotfiles checkout and
   the agent asks whether to edit locally or file a GitHub issue, unless the conversation already answers that (the
   session is already editing this skill, or the user said which they want). For an addition, the name and trigger
   proposal rides in that same question. If the probe finds nothing, or the agent cannot tell where it is, file an issue
   without investigating further; the issue flow is always correct. A request about some other skill is out of scope and
   is declined in a line.
2. Local path: read this spec from the checkout the probe found, make the edits under `agent-skills/skillette/` there,
   and stop. An edit that changes user-facing behavior updates this spec in the same edit. Whether a commit or PR
   follows is up to the user and the surrounding session.
3. Issue path: write a hand-off for a cold agent and open a GitHub issue on `scode/dotfiles` labeled `skillette-change`.
   The hand-off carries everything a later agent needs to produce roughly the change the current agent would have made
   right then, minus anything private. Create the issue immediately unless a question is open, and there is at most one
   question, asked once: it batches the name and natural-language trigger decision for an addition or a trigger change
   (when the user has not already settled it) together with every privacy item worth asking about. Whatever the answer
   leaves unaddressed defaults to omit.
4. Report the issue URL in one line, with a count of identifiers generalized or omitted, and go back to whatever the
   session was doing. The agent does not check for an existing issue on the same subject; duplicates are cheap and
   deduplicating costs a round trip on every use. If `gh` fails for any reason, the agent requests its harness's
   one-time approval if there is one, and otherwise asks the user to choose between fixing `gh` and receiving the issue
   text to file by hand; it never investigates or routes around the failure. If the label cannot be created, the issue
   is filed without it and the report says so.

The repo is public. The hand-off must not contain personal or privacy-sensitive information. Concrete identifiers from
the session (paths under the home directory, hostnames, employer, client, project, or people names, email addresses and
handles, URLs and ticket IDs, credentials or anything shaped like one, and pasted tool output, error text, or code from
the session) are generalized or omitted by default. That list is not exhaustive. The current repository is treated as
private unless the agent already knows it is public. The check covers the title as well as the body. The agent asks only
about identifiers whose generalization or omission would lose substance the cold agent needs; everything else is
generalized silently. It asks once, listing every such item in that one question with the choices include, generalize,
or omit for each. One question, however many items. Approval is per item and per issue: a general remark that some class
of identifier is fine, or that the agent asks too much, is itself a change request to this skillette and is recorded as
one rather than applied to the current filing. An approved identifier still goes in only when leaving it out would lose
substance.

The hand-off template is in `change/SKILLETTE.md`. It is deliberately high level. Agents are expected, not merely
allowed, to add whatever context does not fit the template; the failure mode this skillette is optimized against is lost
context, not an untidy issue. The privacy rule constrains identifiers, not substance: the situation, the reasoning, and
the intended behavior can and should be described in full in generic terms.

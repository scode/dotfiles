# skillette-lore

Repo lore is a convention for keeping historical material inside a repository without letting it leak into current work.
This file is the concept and the canonical texts; it is what a session needs to know when the word comes up. The two
actions that change a repository live in `setup.md` and `add.md` next to this file, and are read only when asked for.

"Lore" here means a repository's `./lore` directory and nothing else. When the surrounding request plainly uses the word
another way (a game's or story's world, folklore, "lorem ipsum", a wiki page called lore), this skillette does not
apply; carry on with the request and do not mention it.

Triggering alone implies no action. The user said "lore", so you now know what it means here, and you continue with
whatever the request actually was. Two phrasings ask for something to happen:

- "set up lore", "install lore", "adopt the lore convention", "normalize lore", or similar: read `setup.md` and follow
  it. It installs the convention, or checks an existing `./lore` against it and normalizes what deviates.
- "add a lore entry", "record this in lore", "write this up as lore", "put this in lore", or similar: read `add.md` and
  follow it. It creates one new entry.

When a phrasing could mean either ("add lore" on its own, for instance), ask which. A bare `skillette-lore` with no
request around it gets the same question, as the skillette spec requires. Everything else, including a question about
what lore is or a passing mention of an entry, is context only.

## The convention

A `./lore` directory at the repository root holds historical artifacts: design notes, experiment write-ups, eval
results, decision records, and the like, each written into lore at a point in time and then frozen. Nothing in the
project's current behavior reads or depends on it. Agents doing normal work stay out of it, and routine maintenance
(formatting, refactors, renames, link fixing, documentation cleanup) never touches it. An entry changes only when the
user intentionally asks for that entry to change. Set-up is the one sanctioned exception: it renames entries and adds
missing entry-point files, and even it never edits an entry's contents.

Layout:

```
lore/
  AGENTS.md                         canonical text below
  CLAUDE.md -> AGENTS.md            symlink
  YYYY-MM-DD-short-name.md          single-file entry
  YYYY-MM-DD-short-name/            directory entry, for material that spans several files
    README.md                       entry point: what the entry is, when it went into lore, where to start
    ...
```

An entry is any top-level file or directory under `lore/` other than `AGENTS.md` and `CLAUDE.md`. Its name starts with
the date it was written into lore in `YYYY-MM-DD` form, then a short kebab-case name. When that date is not known (an
old note adopted into lore with no record of when it was written), the prefix is `0000-00-00-`; a date is never invented
from file times, VCS history, or contents. A single document is a file. Several related files (a report plus its data,
prompts, or transcripts) are a directory whose `README.md` says what the entry is, when it went into lore, and which
file to read first. There is no index of entries; the directory listing is the index.

The top of a single-file entry carries the same orientation as a `README.md`: a title, then a line or two saying what
the entry is and, when different from the filename date, the period it describes. For example:

```
# Roomba mapping experiment

NOTE: Historical artifact, written 2026-09-06 about runs made in August 2026. Records what worked and what did not; not
maintained.
```

`CLAUDE.md` inside `./lore` is a symlink to `AGENTS.md`, mirroring the common top-level layout. That is a deliberate,
user-chosen exception to the skillette rule against harness-specific file names. Where the repository cannot carry a
symlink, an identical regular file is acceptable, and set-up reports it rather than replacing it.

Any tool that rewrites or checks Markdown, wherever it runs (a formatter, a linter, a pre-commit hook, a CI job),
excludes `lore/`. Without that, the first tool upgrade that changes formatting forces an edit to frozen entries.

## The two canonical texts

The repository's top-level agent instructions carry this statement under a `## Lore` heading. It is verbatim in wording;
line wrapping follows whatever the file's formatter wants, and a check compares the words, not the line breaks:

```
The ./lore directory contains historical information about the project. Only use it for historical digging or when
requested. Current project behavior does not use or depend on it, and routine maintenance does not touch it.
```

`lore/AGENTS.md` is this text, written without the surrounding fence, with the same wording rule:

```
# Lore directory

Everything in this directory is a historical artifact: notes, experiments, evaluations, and decisions written down at a
point in time and frozen there. Nothing in the project's current behavior reads or depends on any of it.

Do not read these files during normal work. They are for historical archeology: digging into why something was done,
what was tried, or what an old experiment showed, either because the user asked or because the task is specifically
about the past.

Do not touch these files during routine maintenance. Formatting passes, refactors, renames, link fixes, dependency
bumps, and documentation cleanup all skip this directory, and a stale or broken entry stays stale and broken. An entry
changes only when the user intentionally asks for that entry to change.

## Adding an entry

Every entry is named for the date it was written into lore, then a short kebab-case name:

- `YYYY-MM-DD-short-name.md` for a single document.
- `YYYY-MM-DD-short-name/` for material that spans several files (a report plus its data, prompts, or transcripts).
  The directory holds a `README.md` that says what the entry is, when it went into lore, and which file to read first.

Use `0000-00-00-` as the date when it is genuinely unknown, for instance an old note adopted into lore; never invent one
from file times or history.

The top of a single-file entry carries the same orientation: a title, then a line or two saying what this is and, when
different from the filename date, the period it describes. Never overwrite an existing entry; if the name is taken,
pick a more specific one. Do not edit other entries when adding one. There is no index to maintain.
```

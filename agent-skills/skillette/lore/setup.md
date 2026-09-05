# Set up lore

Read `SKILLETTE.md` in this directory first if it is not in context; the convention and both canonical texts are there
and this file does not repeat them. This procedure either installs the convention in a repository that has no `./lore`
or normalizes one that does. Under `./lore`, it renames entries, adds missing `README.md` files, and rewrites the two
instruction files; it never edits the contents of an existing entry, whatever state that entry is in. It stops when the
files are changed; whether to commit is the surrounding session's call.

## Where

The convention lives at the repository root. Find it as the nearest ancestor of the current directory that contains a
`.git`, `.jj`, `.sl`, or `.hg` entry; failing that, the nearest ancestor holding a top-level `AGENTS.md` or `CLAUDE.md`;
failing that, ask. In a monorepo the default is still the VCS root; "set up lore in `<path>`" puts it under that path
instead, with the top-level statement in that path's own instructions file.

## Is this repo lore at all

If `./lore` exists but does not look like the convention (no dated entries, no instruction file about history, contents
that read as live material such as a game's world-building or a documentation section), stop and show the user what is
there before changing anything. The normalization below is for a directory that is already lore in intent and merely
deviates in form.

## Fresh install

When `./lore` does not exist: create it, write `lore/AGENTS.md` from the canonical text, create the `CLAUDE.md` symlink,
add the top-level statement, and add the tool exclusion. Report the paths and stop.

## Normalize an existing directory

Check each of these and fix what deviates. Every fix is reported; every deviation that is not fixed is reported with the
reason.

- `lore/AGENTS.md` matches the canonical text in wording. Replace it if not. `lore/CLAUDE.md` is a symlink to it; create
  or repoint it if not. If `CLAUDE.md` is a regular file identical to `AGENTS.md` in a repository that cannot carry
  symlinks, leave it and say so.
- Every entry name has a `YYYY-MM-DD-` prefix. Rename an entry whose name carries the date in another unambiguous form
  (`YYYYMMDD-`, `YYYY_MM_DD-`); rename an entry whose name carries no date, or a form where day and month could swap, to
  `0000-00-00-<name>`. Do not read a date out of file times, VCS history, or the entry's contents. If the target name
  already exists, leave both and report the pair. Rename with the VCS's own move command where it has one (`git mv`,
  `sl mv`, `hg mv`); where the VCS tracks the working copy itself, as jj does, a plain `mv` is enough.
- Anything under `lore/` that is not an entry and not one of the two instruction files (an index `README.md` at the top
  level, an `assets/` directory, a `.gitkeep`) is reported, not renamed or removed. An index in particular is against
  the convention but deleting it is the user's call.
- After a rename, references to the old name from files outside `./lore` are updated; references between lore entries
  are left as they are, since entries are frozen, and listed in the report.
- Every directory entry has a `README.md`. Where one is missing, add one that states only what is visible: the entry's
  name, the date from the directory name, the list of files, and the file that reads as the main report when one stands
  out from filenames and first lines (say none stands out when that is the case). Include a line saying the README was
  added on today's date during set-up, not when the entry was written. This is a new file, not an edit to frozen
  content.
- The top-level statement is present in wording under a `## Lore` heading. When the instructions file has a heading
  whose subject is lore, that section, from the heading to the next heading of the same or higher level, is replaced by
  the canonical one, and the removed text is quoted in the report. When the only mention of lore is a line inside some
  other section, add the canonical section and leave that line alone.
- The tool exclusion is present for every tool that rewrites or checks Markdown, wherever that tool is configured:
  `dprint.json` excludes, a Prettier ignore file, a markdownlint ignore, a pre-commit hook's `exclude`, a `pyproject`
  tool table. Add `lore/` to each. If a tool clearly runs over Markdown but its configuration cannot be found, report
  that rather than guessing.
- References to `lore/` from outside the directory (a docs site nav, a script, a build step) contradict "nothing depends
  on it". Search for them and list them in the report; whether the statement is true is then the user's judgment.

## Where the top-level statement goes

In `AGENTS.md` when that file exists, else in `CLAUDE.md`. When both exist as regular files rather than one linking to
the other, in both. When neither exists, create `AGENTS.md` with a `# Agent instructions` title line and the statement,
and a `CLAUDE.md` symlink pointing at it, matching the layout inside `./lore`.

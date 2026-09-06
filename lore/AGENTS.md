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

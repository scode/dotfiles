---
name: scode-todo
description: Use only when the user explicitly invokes `/scode-todo` or `$scode-todo`.
---

# scode-todo

Use this skill for two cases:

1. `/scode-todo init` in an empty repository.
2. Normal TODO work in a repository that already has `TODO.md`, `DONE.md`, and `T_max.md`, or repo-local `AGENTS.md` /
   `CLAUDE.md` files that say to always use `$scode-todo`.

The goal is simple: keep the repo-local agent instructions tiny, keep the workflow deterministic, and keep the VCS
operations behind `bin/pull`, `bin/push`, and `bin/commit`.

`/scode-todo init` is a one-time setup step. After that, the repository should already be configured to keep using this
skill without another explicit install step.

## Init Workflow

When the user asks for `/scode-todo init`:

1. Treat the repository as empty except for VCS metadata such as `.git`, `.sl`, or `.hg`.
2. Ask which VCS preset to install:
   - `git` (standard git)
   - `graphite`
   - `sapling`
   - `custom`
3. For `sapling`, ask for the remote bookmark to push to if it is not `main`. Default to `main`.
4. For `custom`, ask for:
   - the shell command for `bin/pull`
   - the shell command for `bin/push`
   - the shell command for `bin/commit`
5. For custom commit commands, require that the command already includes its own staging step and uses `$message` (or
   `${message}`) where the commit message belongs.
6. Run `python3 scripts/init_todo_project.py --path <repo> --vcs <preset> ...` from this skill.
7. Ensure the initialized repo contains both `AGENTS.md` and `CLAUDE.md`, each with the minimal instruction to always
   use `$scode-todo` in that repository.
8. After init, commit the scaffold if the repository is under version control and the user asked to initialize the repo.
   Do not auto-push init changes unless the user explicitly asks.

Read [references/vcs-presets.md](references/vcs-presets.md) when the user wants to understand what each preset does or
you need to justify the generated wrapper commands.

## Working Files

The source of truth files are:

- `TODO.md` for open tasks
- `DONE.md` for completed tasks
- `T_max.md` for the largest allocated numeric ID
- optional `T-N.md` notes files for long-form task notes

`TODO.md` must keep this shape:

```md
# TODO

## Timed TODOs

- T-52[home,urgent]+ 2026-03-03 Take out the garbage
- T-54 2026-03-04 Call the insurance company

## Untimed TODOs

- T-53[errands]+ Replace bike tire
```

Rules:

- Timed entries are `T-N`, optional `[tags]`, optional `+`, then `YYYY-MM-DD`, then the summary.
- Timed TODOs stay sorted by due date ascending, then ID ascending.
- Untimed entries are `T-N`, optional `[tags]`, optional `+`, then the summary.
- Untimed TODOs do not need a global sort beyond keeping the file stable.

`DONE.md` must keep this shape:

```md
# DONE

## Completed TODOs

- T-52[home,urgent]+ 2026-03-03 Take out the garbage (done 2026-02-21)
- T-53[errands] Replace bike tire (done 2026-02-21)
```

Rules:

- Keep each completed entry on one line.
- Preserve the original `TODO.md` line exactly, then append `(done YYYY-MM-DD)`.

`T_max.md` is a plain integer. If you allocate a new TODO ID, update `T_max.md` in the same change.

`T-N.md` is optional. If it exists, the matching TODO line must carry a `+` marker immediately after the ID or tag
block.

## ID, Tag, And Notes Rules

- TODO IDs are `T-<number>`.
- Numeric references like `52` mean `T-52` when that is clearly the user's intent.
- IDs are unique and never reused.
- Tags live immediately after the ID as `T-N[tag1,tag2]`.
- Tags are lowercase, deduplicated, sorted ascending, and contain no spaces.
- Notes live in `T-N.md`.
- The `+` marker and the presence of `T-N.md` must stay in sync.
- When listing TODOs back to the user, preserve the inline ID, tags, and `+` marker exactly as stored.

## Natural Language Commands

Interpret requests like this:

- `Remind me to <task> at <DATE>` creates a timed TODO.
- `Remind me to <task> on monday` means the first upcoming Monday relative to today.
- Relative dates like `tomorrow` and `in 2 days` are fine when they are unambiguous.
- Normalize parsed dates to `YYYY-MM-DD`.
- If the user includes a bracketed tag list that is clearly metadata, treat it as tags instead of summary text.
- If the user includes `NOTES:`, store everything after it in `T-N.md` and add the `+` marker.
- `NOTES:` may be single-line or multi-line. Preserve the text faithfully.
- `T-50 is done` marks that TODO complete.
- `tag T-50 +foo` adds a tag.
- `tag T-50 -foo` removes a tag.
- Natural-language tagging and notes edits should be honored when the target TODOs are unambiguous.

If a date is ambiguous, or an ID target is uncertain, ask before editing.

## Completion Workflow

When a TODO is marked done:

1. Find the matching entry in `TODO.md`.
2. Remove it from `TODO.md`.
3. Append the original line to `DONE.md` with `(done YYYY-MM-DD)` using today's date.
4. Keep any matching `T-N.md` file unless the user explicitly asks to remove it.

## Repo Workflow

For an initialized TODO repo:

1. Run `bin/pull` from the repo root before making any user-requested modification.
2. If `bin/pull` fails, stop and report the error.
3. Make only the edits required for the request.
4. Commit the result after the requested change is complete.

Use the repo-local wrapper scripts as the source of truth for VCS operations.

NOTE: Repositories initialized by this skill use a three-step flow where `bin/commit` stages changes itself. Older
repositories may not. If the wrappers look pre-existing or behave differently, inspect them once and follow what they
actually do instead of assuming the generated preset behavior.

Commit and push rules:

- The user's exact requested change belongs in its own commit.
- If you also do formatting-only cleanup or reorganization, put that in a separate commit.
- Auto-push only when the changed files are limited to `TODO.md`, `DONE.md`, and `T-N.md`.
- Do not auto-push changes that touch `AGENTS.md`, `T_max.md`, `bin/*`, skill files, docs, or other repo files unless
  the user explicitly asks.
- If commit or push fails, report the error and ask what to do next.

## Safety

- Be conservative. Change only what the user asked for.
- Do not delete data unless the user explicitly asks.
- Do not rewrite unrelated TODOs just because you touched the file.
- Never delete notes files unless the user explicitly asks.

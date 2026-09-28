# skillette-snippets

The user keeps small standalone snippets (text, Markdown, HTML, HTML with JavaScript) in the private GitHub repository
`scode/snippets`, each served at its own URL. The user names it as `scode-snippets` in requests such as "create a
scode-snippets snippet with this content" or "what's in scode-snippets". You have access to the repository; do not ask
the user to confirm that.

NOTE: This file is only a pointer. The process for adding, changing, and publishing snippets lives in `AGENTS.md` at the
root of that repository, and that file is the authority. Do not work from memory of an earlier read or from anything
summarized here; the process changes, and this file will not be updated when it does.

Read the current `AGENTS.md` from the default branch before doing anything else, for example with

```sh
gh api repos/scode/snippets/contents/AGENTS.md -H 'Accept: application/vnd.github.raw'
```

or from a fresh clone of `https://github.com/scode/snippets.git`. Then follow it, including its instructions about where
to clone, what to commit, and what to report back. If you cannot read it, stop and say so rather than guessing at the
process.

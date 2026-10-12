# skillette-snippets

The user keeps small standalone snippets (text, Markdown, HTML, HTML with JavaScript) in the private GitHub repository
`scode/snippets`, each served at its own URL. The user names it as `scode-snippets` in requests such as "create a
scode-snippets snippet with this content" or "what's in scode-snippets". You have access to the repository; do not ask
the user to confirm that.

NOTE: Apart from the preview flow below, this file is only a pointer. The process for adding, changing, and publishing
snippets lives in `AGENTS.md` at the root of that repository, and that file is the authority. Do not work from memory of
an earlier read or from anything summarized here; the process changes, and this file will not be updated when it does.

Read the current `AGENTS.md` from the default branch before doing anything else, for example with

```sh
gh api repos/scode/snippets/contents/AGENTS.md -H 'Accept: application/vnd.github.raw'
```

or from a fresh clone of `https://github.com/scode/snippets.git`. Then follow it, including its instructions about where
to clone, what to commit, and what to report back, except where the preview flow below says otherwise. If you cannot
read it, stop and say so rather than guessing at the process.

## The preview flow

When the user asks for the preview flow ("use the preview flow", "scode snippet preview flow", "preview it as a
snippet", and similar), build the snippet exactly as `AGENTS.md` says, with the same stylepack and content rules, but do
not publish it. Nothing is committed and nothing is pushed. Instead, serve the result locally and hand the user a
loopback URL. The point is to see what a snippet would look like without putting anything on the public internet.

Here's how it differs from the normal flow:

- Clone, pick a slug, and write `snippets/<slug>/` in the temporary clone as `AGENTS.md` describes. The clone is what
  makes `../stylepacks/<name>/style.css` resolve, so do not copy the files somewhere else.
- Skip the commit and the push entirely. Do not run `git commit` or `git push` in the clone, even if `AGENTS.md` or
  habit says to.
- Start `preview-server` (next to this file) with the clone's `snippets/` directory as its only argument, using your
  harness's ordinary way of running a command in the background. It binds `127.0.0.1` on the first free port in
  14100-14120 and prints `listening http://127.0.0.1:<port>/` once it is up; it exits non-zero if no port in the range
  is free. It is a Deno script; if Deno is missing, the `scripts` skillette says what to do.
- Do not detach it from the session: no `nohup`, `setsid`, `disown`, or double fork. The server is meant to die along
  with the session that started it rather than linger as an orphan holding a port. In some harnesses that means it dies
  sooner than the user would like; that is accepted, and the user restarts the preview by asking again.
- Give the user `http://127.0.0.1:<port>/<slug>/`, with the trailing slash and without the `/s/` prefix the deployed
  site uses. Also tell them where the clone is and how to stop the server.

NOTE: Never widen the bind address or put the preview anywhere other than localhost, including to make it reachable from
another machine. If the user needs that, suggest SSH port forwarding and leave the server as it is.

If the user later asks to publish what they previewed, that is the normal flow from `AGENTS.md`, including its privacy
check; the preview having existed changes nothing about it.

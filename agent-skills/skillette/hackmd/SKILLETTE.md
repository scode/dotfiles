# skillette-hackmd

Mechanics for reading and writing HackMD notes from an agent session through `@hackmd/hackmd-cli`. The whole message
carrying the trigger, or the mention of HackMD, is the request: publish this file as a note, update note such-and-such
from this file, pull down a note, share a note with someone. This file tells you how to do those things without tripping
over the CLI; it does not prescribe a workflow, a manifest format, or a sync design. If the user wants a repeatable
sync, design it with them using these mechanics.

NOTE: A note whose read permission is `guest` is readable by anyone holding the link. Create notes owner-only unless the
user asked to share, and say which permissions you set when you report back.

## Preflight, every time

Run these before the first network-touching command of the session and act on what they say:

- `hackmd-cli --version` must be 2.5.1 or newer. Install or upgrade with `npm install -g @hackmd/hackmd-cli`. 2.5.0
  silently dropped `--readPermission` on `notes create` whenever `--writePermission` was also given (upstream issue 106,
  fixed by PR 107 and released 2026-08-26), and had no permission flags on `notes update` at all. Do not work around an
  old version; upgrade it.
- `hackmd-cli whoami` confirms the saved login still works. If it fails, ask the user to run `hackmd-cli login`
  themselves; it prompts for an API token created at https://hackmd.io/settings#api and writes `~/.hackmd/config.json`.
  Do not ask the user to paste the token into the conversation, do not echo it, and do not copy it into a repo.

The full flag reference is `hackmd-cli <topic> --help` (`notes`, `team-notes`, `folders`, `team-folders`, `export`,
`teams`, `history`). Use it rather than guessing at flags; the cheat sheet below covers only what an agent needs most.

## Commands that matter

```bash
hackmd-cli notes                                              # list my notes (add --output=json for scripting)
hackmd-cli notes --noteId=<id> --output=json                  # id, title, tags for one note; no permissions
hackmd-cli export --noteId=<id>                               # note content to stdout
hackmd-cli notes create --title='Title' --output=json < doc.md
hackmd-cli notes update --noteId=<id> --content="$(cat doc.md)"   # replaces the whole body; see below
hackmd-cli notes delete --noteId=<id>
```

`notes create` reads the body from stdin when it is not a terminal, and stdin wins over `--content`. Redirect the file
in. `notes update` does not read stdin at all as of 2.5.1: with no `--content` it sends an empty payload and HackMD
answers 400 Bad Request, which is what `hackmd-cli notes update --noteId=<id> < doc.md` gets you (upstream's own README
shows that pipe form; it is wrong). Pass the body as `--content="$(cat doc.md)"` instead. Command substitution strips
every trailing newline, so a file that ends in a blank line comes back from `export` one blank line short; when the user
wants the file byte-for-byte, PATCH it through the API with a `jq`-built body instead (see the API section). Linux also
refuses a single argument over 128 KB, but that never matters because HackMD's own cap is lower; see the next paragraph.

NOTE: HackMD rejects any create or update whose JSON request body exceeds 100 KiB (102,400 bytes) with
`413 {"message":"Bad Request"}`. The content is counted after JSON escaping, so every newline and double quote costs two
bytes, and a real Markdown document tops out somewhere around 90 KB on disk. The CLI sends the same request as the API,
so both paths hit the same wall and neither has a workaround. Check `wc -c` before you start; a document over the cap
has to be split into several notes, and how to split it is the user's call, not yours.

`notes create --output=json` returns the created note; parse `.id` out of it, and handle both a bare object and a
one-element array, since the CLI has returned both. The rendered page takes its title from the first level-one heading
in the content, while `--title` sets the title shown in note lists, so set both to the same thing.

Confirm what landed with `hackmd-cli export --noteId=<id>` after any create or update; it is cheap and it catches the
wrong-note and empty-stdin mistakes. `export` appends one newline to the stored content, and `--content` drops trailing
ones, so a `diff` against the source file whose only differences are trailing blank lines is a match; any other
difference means the wrong content landed.

When the user names a note by title, list with `--output=json` and pick the entry whose title matches exactly;
`--filter` does substring matching, so "Delete me" also matches "Delete me not". Confirm the id with
`notes --noteId=<id>` before anything destructive, and after a delete re-list to confirm the id is gone.

## Permissions

Values: `--readPermission` and `--writePermission` take `owner`, `signed_in`, or `guest`; `--commentPermission` takes
`disabled`, `forbidden`, `owners`, `signed_in_users`, or `everyone`. `--commentPermission` exists on `notes create` but
not on `notes update` as of 2.5.1, so decide about comments at creation time.

Default to `--readPermission=owner --writePermission=owner`. "Anyone with the link can read, only I can edit" is
`--readPermission=guest --writePermission=owner`, and `--commentPermission=disabled` with it avoids opening a feedback
channel nobody monitors; use that recipe only when the user asked to share.

To change permissions on an existing note, run `notes update --noteId=<id>` with the permission flags and no
`--content`; the body is left alone, and only fields you pass are sent. This is the one `notes update` form that works
without content, and it is how to share or unshare a note after the fact.

After setting permissions, read them back and compare to what you asked for. No CLI command shows permissions in any
output mode as of 2.5.1 (`notes --noteId=<id>` prints only id, title, tags, and paths, even with `-x` or
`--output=json`), so the read-back goes through the REST API with the same token the CLI uses:

```bash
cfg="${HMD_CLI_CONFIG_DIR:-$HOME/.hackmd}/config.json"
tok="${HMD_API_ACCESS_TOKEN:-$(jq -r .accessToken "$cfg")}"
api="${HMD_API_ENDPOINT_URL:-$(jq -r '.hackmdAPIEndpointURL // "https://api.hackmd.io/v1"' "$cfg" 2>/dev/null || echo https://api.hackmd.io/v1)}"
curl -sS -H "Authorization: Bearer $tok" "$api/notes/<id>" | jq '{readPermission, writePermission}'
```

The 2.5.0 bug was invisible without this check, and someone may have changed permissions in the web UI since. Re-assert
permissions on every update of a note that is meant to be shared, so they self-heal. `commentPermission` is not in the
`GET` response, so it cannot be read back; set it at creation and leave it.

The end-to-end test for guest readability is an anonymous request to the view link:
`curl -sS -o /dev/null -w '%{http_code}\n' 'https://hackmd.io/<id>?type=view'` prints 200 for a guest-readable note and
403 for an owner-only one. Run it after sharing, and after tightening, since it proves what a stranger sees rather than
what the API claims.

## Links and rendering

Share links as `https://hackmd.io/<id>?type=view`. Without `?type=view`, a visitor with edit rights lands in the split
editor and a read-only visitor gets a less clean view.

HackMD does not render GitHub-style `[^name]` footnotes; the markers stay as literal text. Publish as-is and warn the
user in your report; rewrite footnotes into inline text or ordinary links only when asked, since that changes their
document. Reference-style link definitions (`[label]: url`) work, but only within the note that carries them, so a
document split across several notes needs each note to carry the definitions it uses.

## Talking to the REST API directly

Everything except the permission read-back can be done through the CLI on 2.5.1. Go to the API for anything else only
when you need something the CLI lacks, or when you are scripting dozens of calls and a subprocess per call is the
bottleneck. Header `Authorization: Bearer <token>`, JSON bodies, base URL and token resolved as in the snippet above
(`HMD_API_ACCESS_TOKEN` wins over `accessToken` in the config file; `HMD_API_ENDPOINT_URL` wins over
`hackmdAPIEndpointURL`, which is only present when set; default `https://api.hackmd.io/v1`).

- `GET /notes/:id` reads metadata, permissions, and content.
- `PATCH /notes/:id` with a body of `content`, `readPermission`, and `writePermission` replaces content and permissions
  in one request; fields you leave out are left unchanged. Success is 202. With `$tok` and `$api` from the snippet
  above, an update that keeps the file byte-for-byte is
  `jq -Rs '{content: .}' doc.md | curl -sS -X PATCH -H "Authorization: Bearer $tok" -H 'Content-Type: application/json' -d @- "$api/notes/<id>"`.
- `DELETE /notes/:id` removes a note.

On a failed request, report the HTTP status and the first few hundred bytes of the body; that is where HackMD puts the
useful message.

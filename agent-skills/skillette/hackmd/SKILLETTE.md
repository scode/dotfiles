# skillette-hackmd

Mechanics for reading and writing HackMD notes from an agent session through `@hackmd/hackmd-cli`. The whole message
carrying the trigger, or the mention of HackMD, is the request: publish this file as a note, update note such-and-such
from this file, pull down a note, share a note with someone. This file tells you how to do those things without tripping
over the CLI; it does not prescribe a workflow, a manifest format, or a sync design. If the user wants a repeatable
sync, design it with them using these mechanics.

NOTE: A note whose read permission is `guest` is readable by anyone holding the link. Create notes owner-only unless the
user asked to share, and say which permissions you set when you report back.

NOTE: Create notes with comments enabled (`--commentPermission=owners` by default) unless the user explicitly asks for
them off. Neither the CLI nor the API can change comment permission after creation, so get it right in the create call;
see "Permissions" for what to do with a note that already has them off, and "Reading feedback" for how comments come
back.

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
Comments, versions, trash, and webhooks have no CLI commands as of 2.5.1 and are API-only. The source of truth for the
API is the Swagger spec at https://api.hackmd.io/v1/docs/swagger.json; the older developer-portal notes on hackmd.io are
deprecated and their download links 404, so do not send anyone there.

## Commands that matter

```bash
hackmd-cli notes                                              # list my notes (add --output=json for scripting)
hackmd-cli notes --noteId=<id> --output=json                  # id, title, tags for one note; no permissions
hackmd-cli export --noteId=<id>                               # note content to stdout
hackmd-cli notes create --title='Title' --output=json \
  --readPermission=owner --writePermission=owner --commentPermission=owners < doc.md
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

NOTE: HackMD rejects any create or update whose JSON request body exceeds 100 KiB (102,400 bytes) with `413
{"message":"Bad Request"}`. The content is counted after JSON escaping, so every newline and double quote costs two
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
`--filter` does substring matching, so "Delete me" also matches "Delete me not". Confirm the id with `notes
--noteId=<id>` before anything destructive, and after a delete re-list to confirm the id is gone.

## Permissions

Values: `--readPermission` and `--writePermission` take `owner`, `signed_in`, or `guest`; `--commentPermission` takes
`disabled`, `forbidden`, `owners`, `signed_in_users`, or `everyone`.

Comment permission is create-time only as far as the API goes, and that is a property of HackMD, not of the CLI. `POST
/notes` accepts `commentPermission` and `suggestEditPermission`; `PATCH /notes/:id` answers 422 to either, and `GET
/notes/:id` does not report them. So `--commentPermission` exists on `notes create` but not on `notes update`. Whether
the owner can flip it in the note's share settings in the web UI has not been checked; when a note turns out to have
comments off, tell the user and let them choose between changing it in the UI and having you recreate the note. Do not
delete and recreate on your own, since that discards the note's id, link, comments, and version history.
`suggestEditPermission` (values `disabled`, `forbidden`, `owners`, `signed_in_users`) has no CLI flag at all as of
2.5.1; set it by creating the note through `POST /notes` with a JSON body when it matters. I have not found what
distinguishes `disabled` from `forbidden`; both mean nobody can comment.

Default to `--readPermission=owner --writePermission=owner --commentPermission=owners`, and honor an explicit request
for `disabled` or `forbidden`. "Anyone with the link can read, only I can edit" is `--readPermission=guest
--writePermission=owner`; pair it with `--commentPermission=signed_in_users` (logged-in reviewers) or `everyone` (anyone
with the link) so the people you shared with can leave feedback, and use that recipe only when the user asked to share.

To change read or write permission on an existing note, run `notes update --noteId=<id>` with those flags and no
`--content`; the body is left alone, and only fields you pass are sent (`--tags`, `--permalink`, and `--parentFolderId`
work the same way). That is how to share or unshare a note after the fact.

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
permissions on every update of a note that is meant to be shared, so they self-heal. `commentPermission` and
`suggestEditPermission` are not in the `GET` response and cannot be set through the API after creation, so there is
nothing to read back or re-assert for them; get them right in the create call.

The end-to-end test for guest readability is an anonymous request to the view link: `curl -sS -o /dev/null -w
'%{http_code}\n' 'https://hackmd.io/<id>?type=view'` prints 200 for a guest-readable note and 403 for an owner-only one.
Run it after sharing, and after tightening, since it proves what a stranger sees rather than what the API claims.

## Links and rendering

Share links as `https://hackmd.io/<id>?type=view`. Without `?type=view`, a visitor with edit rights lands in the split
editor and a read-only visitor gets a less clean view.

HackMD does not render GitHub-style `[^name]` footnotes; the markers stay as literal text. Publish as-is and warn the
user in your report; rewrite footnotes into inline text or ordinary links only when asked, since that changes their
document. Reference-style link definitions (`[label]: url`) work, but only within the note that carries them, so a
document split across several notes needs each note to carry the definitions it uses.

## Reading feedback

None of this has CLI commands as of 2.5.1; everything here is `curl` against the API with `$tok` and `$api` resolved as
in the permissions snippet above. "Address the feedback on the note" can mean any of three things, because a reviewer
may have used any of them: inline comments, suggested edits, and direct edits.

Comments come from `GET $api/notes/<id>/comments`. It is paged (`page` from 1, `limit` default 50, max 100, response
carries `pagination.hasNext`), sorted by `sort=asc|desc`, and filtered by `threadId`, `commentStatus` and `threadStatus`
(each `open` or `hidden`), and `isThreadHead=true` for thread roots only. The fields an agent acts on: `body` (the
text); `excerpt` (the words of the note the comment was left on), `anchor` (their `offset` and `length` into the note
text when the comment was made), and `currentAnchor` (the same after later edits moved the text), all three nullable for
a comment on the note as a whole; `threadId` and `isThreadHead` (which replies belong to which root); `actor` (who wrote
it: `kind` plus `displayName`, and `userId` or `teamId` when it is not a guest); and `threadState` (`status` is `open`
or `hidden`, and a resolved thread is `hidden` with `reason` `resolved`; the other reasons are `spam`, `abuse`, and
`off_topic`). There is no endpoint to create a comment or a reply, so the agent cannot answer inside HackMD: answer in
the session or by changing the note. Once a thread has been acted on, `PUT
$api/notes/<id>/comments/<commentId>/resolution` on the thread root resolves it, `DELETE` on the same path reopens it,
and either answers 409 when the thread is already in that state, so a re-run is not idempotent.

Direct edits by the reviewer show up as ordinary content, and the versions API tells you what changed. `GET
$api/notes/<id>/versions` lists saved versions under `data`, paged with `meta` (`limit` default 50); on 2026-09-22 every
CLI and API update produced one, though the spec does not promise that. `GET $api/notes/<id>/versions/<versionId>`
returns that version with `content` when `content_available` is true, and `GET
$api/notes/<id>/versions/compare?base=version:<versionId>&target=note_content` returns `{"unified_diff": "…"}` of the
live note against that version. Both `base` and `target` take either `version:<id>` or `note_content`; anything else
(such as `target=live`) is refused with `invalid_request`, and compare answers 422 when a version's content is
unavailable. An update returns no version id (`PATCH` is a 202 with an empty body, and `notes update` prints nothing),
so right after publishing, list the versions and record the newest id, or make a named checkpoint with `POST
$api/notes/<id>/versions` and a body of `{"name": "…"}`, which does return the version's `id`. Compare against that id
before you overwrite the note, or the reviewer's edits are gone.

NOTE: A HackMD suggested edit is invisible through the API in every form until someone accepts it: it does not appear in
the comments list under any filter, not in the versions list, and not in the live content, and comparing the newest
version against `note_content` gives an empty diff. This was established empirically on 2026-09-22 rather than from
documentation, so treat "no changes visible" as inconclusive when the reviewer may have used suggestions; only accepted
suggestions are visible, after which they are ordinary content and come back through `export` or the versions diff like
any other edit.

Here is the read-back in three calls. `--fail-with-body` makes a 403 or 404 fail the pipe with HackMD's message instead
of a `jq` parse error. The first call lists every comment; group by `threadId` yourself, since `isThreadHead=true` would
drop the replies, and a reviewer's "never mind" is usually a reply:

```bash
curl -sS --fail-with-body -H "Authorization: Bearer $tok" "$api/notes/<id>/comments?threadStatus=open&limit=100" \
  | jq -r '.comments[] | "[\(.threadId)] \(.actor.displayName // .actor.kind) on \"\(.excerpt // "whole note")\": \(.body)"'
curl -sS --fail-with-body -H "Authorization: Bearer $tok" \
  "$api/notes/<id>/versions/compare?base=version:<versionId>&target=note_content" | jq -r .unified_diff
curl -sS --fail-with-body -X PUT -H "Authorization: Bearer $tok" "$api/notes/<id>/comments/<commentId>/resolution"
```

If polling is the wrong shape for what the user wants, HackMD has webhooks (`/webhooks` and `/teams/:path/webhooks` in
the spec). The spec does not enumerate event types; a hook's `eventDeliveryMode` is the single value
`all_supported_events`. They are not exercised here and this file does not prescribe a design around them.

## Talking to the REST API directly

The permission read-back and everything under "Reading feedback" need the API. Go to it for anything else only when you
need something the CLI lacks, or when you are scripting dozens of calls and a subprocess per call is the bottleneck.
Header `Authorization: Bearer <token>`, JSON bodies, base URL and token resolved as in the snippet above
(`HMD_API_ACCESS_TOKEN` wins over `accessToken` in the config file; `HMD_API_ENDPOINT_URL` wins over
`hackmdAPIEndpointURL`, which is only present when set; default `https://api.hackmd.io/v1`). The endpoint listing is the
Swagger spec named under Preflight.

- `GET /notes/:id` reads metadata, read and write permissions, and content. It does not return `commentPermission` or
  `suggestEditPermission`.
- `POST /notes` with a JSON body of `title`, `content`, `readPermission`, `writePermission`, `commentPermission`, and
  `suggestEditPermission` creates a note; this is the only way to set `suggestEditPermission`, and the only time the API
  lets you set either of those two.
- `PATCH /notes/:id` with a body of `content`, `readPermission`, and `writePermission` replaces content and permissions
  in one request; fields you leave out are left unchanged. Success is 202. `commentPermission` or
  `suggestEditPermission` in the body gets a 422. With `$tok` and `$api` from the snippet above, an update that keeps
  the file byte-for-byte is `jq -Rs '{content: .}' doc.md | curl -sS -X PATCH -H "Authorization: Bearer $tok" -H
  'Content-Type: application/json' -d @- "$api/notes/<id>"`.
- `DELETE /notes/:id` removes a note.
- `GET /notes/:id/comments`, `GET /notes/:id/comments/:commentId`, `PUT` and `DELETE
  /notes/:id/comments/:commentId/resolution`, `GET /notes/:id/versions`, `GET /notes/:id/versions/:versionId`, and `GET
  /notes/:id/versions/compare` are covered under "Reading feedback". `POST /notes/:id/versions` creates a named version
  and `PATCH /notes/:id/versions` renames one.

On a failed request, report the HTTP status and the first few hundred bytes of the body; that is where HackMD puts the
useful message.

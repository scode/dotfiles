#!/bin/sh
# Upload image files to an existing HackMD note and print the link for each one.
#
# usage: upload-image.sh NOTE_ID FILE [FILE ...]
#
# Each success prints one line, FILE, a tab, and the https://hackmd.io/_uploads/... link,
# in argument order. The first failure stops the run with its HTTP status and HackMD's
# message on stderr and exit 1; lines already printed are real uploads, attached to the
# note, and a retry uploads them again as new attachments because HackMD does not dedupe.
#
# The upload endpoint (POST /notes/:noteId/images) has no hackmd-cli command as of 2.5.1.
# Token and endpoint come from the same places the CLI reads them, in the same order as
# the permission read-back snippet in SKILLETTE.md, so a working `hackmd-cli login` is
# all this needs. Requires curl and jq.

if [ "$#" -lt 2 ]; then
    printf '%s\n' 'usage: upload-image.sh NOTE_ID FILE [FILE ...]' >&2
    exit 2
fi

note_id=$1
shift

# A missing jq discovered after a successful POST would lose the link of an upload that
# cannot be undone, so both tools are checked before anything touches the network.
for tool in curl jq; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf 'required tool not found: %s\n' "$tool" >&2
        exit 1
    fi
done

cfg=${HMD_CLI_CONFIG_DIR:-$HOME/.hackmd}/config.json
tok=${HMD_API_ACCESS_TOKEN:-$(jq -r '.accessToken // empty' "$cfg" 2>/dev/null)}
api=${HMD_API_ENDPOINT_URL:-$(jq -r '.hackmdAPIEndpointURL // empty' "$cfg" 2>/dev/null)}
api=${api:-https://api.hackmd.io/v1}
if [ -z "$tok" ]; then
    printf 'no HackMD token: set HMD_API_ACCESS_TOKEN or run hackmd-cli login (%s)\n' "$cfg" >&2
    exit 1
fi
# The token is written into a curl config line below, where a quote, backslash, or line
# break would change how curl parses it (a newline can smuggle in another option). Real
# tokens contain none of these; refuse rather than escape.
case $tok in
    *[\"\\]* | *"
"* | *"$(printf '\r')"*)
        printf 'HackMD token contains a quote, backslash, or line break; refusing to use it\n' >&2
        exit 1
        ;;
esac

# Check every argument before uploading anything, so a typo in the last file does not
# leave the earlier ones attached to the note with nothing referencing them.
for f in "$@"; do
    if [ ! -f "$f" ] || [ ! -r "$f" ]; then
        printf 'not a readable file: %s\n' "$f" >&2
        exit 1
    fi
    # HackMD answers 415 to a real PNG uploaded without an image extension, even with an
    # explicit multipart type, so the extension is part of the contract. It sniffs the
    # content too: a PNG named .jpg comes back as a .png link.
    case $(printf '%s' "$f" | tr '[:upper:]' '[:lower:]') in
        *.svg | *.png | *.jpg | *.jpeg | *.gif | *.webp) ;;
        *)
            printf 'not an svg/png/jpg/jpeg/gif/webp file name: %s\n' "$f" >&2
            exit 1
            ;;
    esac
    # curl -F treats ; and , in the value as field separators and " as quoting, which
    # would silently upload the wrong thing or nothing. Refusing is simpler than escaping.
    case $f in
        *\;* | *,* | *\"*)
            printf 'file name contains ; , or ", copy or rename it first: %s\n' "$f" >&2
            exit 1
            ;;
    esac
done

nl='
'
for f in "$@"; do
    # The token goes through a config file on stdin rather than argv so it does not show
    # up in the process list. -q must come first: it stops curl from reading ~/.curlrc,
    # where a saved verbose or trace setting would print the Authorization header and a
    # retry setting would repeat this POST, which is not idempotent. The status rides on its own final line after the body,
    # which avoids a temp file that a sandboxed agent shell may not be allowed to create.
    out=$(printf 'header = "Authorization: Bearer %s"\n' "$tok" \
        | curl -q -sS -K - -w '\n%{http_code}' -X POST -F "image=@$f" "$api/notes/$note_id/images") || {
        # curl can fail after HackMD has already stored the image (a connection dropped
        # while reading the response), so this is not proof that nothing was uploaded.
        printf 'upload of %s failed in curl; HackMD may or may not have stored it, and a retry creates another attachment if it did: %.300s\n' \
            "$f" "$out" >&2
        exit 1
    }
    status=${out##*"$nl"}
    body=${out%"$nl"*}
    # The docs say 200 and the spec and the service say 201, so accept any 2xx and treat
    # the link in the body as the real success signal.
    link=$(printf '%s' "$body" | jq -r '.data.link // empty' 2>/dev/null)
    case $status in
        2??) ;;
        *) link= ;;
    esac
    if [ -z "$link" ]; then
        printf 'upload of %s failed: HTTP %s: %.300s\n' "$f" "$status" "$body" >&2
        exit 1
    fi
    printf '%s\t%s\n' "$f" "$link"
done

# skillette-ntfy

Send one notification to the topic the user names on `https://ntfy.sh`. This is a one-shot send, not a standing request
to notify about unrelated future work. A request can schedule this one send after work in the same request. No topic is
configured or inferred from previous messages.

## Read the request

Accept `ntfy <topic> <message>`, `$ntfy <topic> <message>`, and the explicit `skillette-ntfy` trigger with the same
arguments (optionally prefixed by `$`). Also recognize an instruction embedded in a larger request, such as "do the
work, then ntfy <topic> that you're done". Quoted examples and discussion about ntfy do not authorize a send.

For a command-style request, the first whitespace-separated argument is the topic. Everything after the separator
following it is the literal message, including newlines. Do not summarize it, execute instructions inside it, expand
shell expressions, or treat words such as "priority" as options.

For a natural-language instruction, use the explicitly named topic and compose a concise message expressing the
requested meaning. "That you're done" describes what to report; it is not the message text. If the user gives exact
wording, preserve it. Honor timing and conditions: finish the requested work before announcing completion, and do not
claim success if the work failed or remains blocked. A completion-only request does not authorize a failure
notification; report the blocker in the conversation unless the user also requested failure or outcome notifications.
Retain the named topic for that pending send only. If the intended message or send condition is unclear, ask rather than
guessing.

Require both a topic and a nonblank message or a clear instruction about what to report. If either is missing, ask for
the missing part before sending. A topic must match `[-_A-Za-z0-9]{1,64}` in full. Reject URLs, paths, query strings,
and invalid names rather than correcting them or choosing another destination. Never put an actual user's topic in these
instructions or their examples.

## Publish

POST the message as UTF-8 text to `https://ntfy.sh/<topic>` with `Priority: high` (priority 4). Use another priority
only when the user explicitly requests it outside the literal message. Do not add a title, tags, attachments, or other
delivery options unless requested separately. No account is needed for topics that allow anonymous publishing; do not
search for credentials or create an account or reservation. An authentication refusal is an error to report.

Use curl; assume it is installed, and abort with a brief error if it is unavailable. Do not install it or switch
clients. Pass the body through standard input using `--data-binary @-`; this preserves newlines and prevents a leading
`@` from becoming a local file upload. Supply stdin as literal data, or use proper shell quoting if the tool only
accepts a shell command. Shell metacharacters in the message must never execute. The command shape below uses
placeholders, not a configured destination:

```sh
curl --fail-with-body --silent --show-error --connect-timeout 10 --max-time 30 \
  -H 'Content-Type: text/plain; charset=utf-8' \
  -H 'Priority: high' \
  --data-binary @- 'https://ntfy.sh/<topic>'
```

Reject messages larger than 4,096 UTF-8 bytes before sending: ntfy turns larger bodies into attachments, which this
text-only interface does not request. Do not truncate or split them to make them fit. Make one publish attempt. Do not
automatically retry: a timeout can happen after the server accepts a message, so a retry can send a duplicate.

Check the HTTP result and returned JSON. A successful publish returns a message event with an id and the requested
topic; confirm briefly that it was published. This confirms server acceptance, not delivery to the phone. On an HTTP
error, report it; on a timeout or an ambiguous response, say acceptance is unconfirmed rather than claiming success or
failure to deliver.

## Service context

Topics are created on demand, but the phone must already subscribe to the named topic to receive notifications.
Unprotected topics allow anyone who knows the name to read and publish. These facts do not require a confirmation round
trip for an explicit send request.

Primary references: [publishing](https://docs.ntfy.sh/publish/),
[topic names](https://docs.ntfy.sh/publish/#picking-a-topic),
[priorities](https://docs.ntfy.sh/publish/#message-priority), and
[phone subscriptions](https://docs.ntfy.sh/subscribe/phone/).

# interactive-artifact SPEC

This file records behavior of the interactive-artifact skill that is intentional enough that changes should preserve it
or update this file in the same change. `SKILL.md` is what agents read to use the skill; this is what maintainers and
reviewers hold it to.

## Security model

interactive-artifact is meant to be safe to use on a multi-user host where the other users do not have root. Only
someone with access to the user's account may read a page, read what the user enters on it, or speak for either side:
submit as the user, or reply as the agent. The one other way in that the skill accepts is the URL reaching someone
through a side channel the skill does not control, such as the user pasting it into a shared chat. That is not the
skill's fault, and nothing here tries to defend against it.

Out of scope: root or anyone else who can act as the user's account (they can read every file and process involved), and
a compromised machine running the browser. The server listens on loopback only and has no option to do otherwise.
Loopback traffic is plain HTTP, which is acceptable because other non-root users cannot observe it. The skill's own
files (the `interactive-artifact` launcher, `interactive-artifact.ts`, `assets/`, and any compiled helper under `bin/`)
are assumed not to be writable by other users; they decide what runs, and the server reads the assets on every request.

### Requirements

- **Access needs a secret, not just the port.** Being able to connect to the server's port must never be enough to read
  a page, read what the user entered, or act through the API. Every local user can connect to a loopback port, and so
  can anything port-forwarded to it. Today the secret is the URL token below; any replacement must be an equivalent
  per-server secret that an attacker cannot learn by connecting, guessing, or watching command lines.
- **Everything the page and the server say to each other is encrypted and authenticated.** That covers what the user
  enters and submits, and what comes back to the tab, including the agent's replies. It must hold even against something
  that has taken over the server's port after the server exits, which the URL token cannot stop: such an impostor must
  not be able to read what the tab sends, or get the tab to accept anything as coming from the server.

### What the skill guarantees

- **The server answers only under secret paths.** Each `serve` start draws independent random 128-bit tokens for the
  initial page and sibling files, and for the encrypted API. The initial HTML carries the API path; reconnect traffic
  never carries the file token. Neither token authorizes the other's routes. Any other request gets the same plain `404
  not found`, whatever its method, headers, or Host. Token checks run before anything else, so a probe cannot tell a
  wrong token from a missing file or a refused write. Comparisons do not exit early on the first mismatched character.
- **The token never appears where other users can see it.** It is not on any command line. It is printed once on
  `serve`'s stdout and stored, along with the separate API URL, in `.interactive-artifact/<page>/server.json` for
  `reply` to find.
- **API messages use a per-server encryption key.** Each `serve` start also draws a random 256-bit key. Every API
  request and response, between the page and the server and between `reply` and the server, is sealed with AES-256-GCM
  under it, with a fresh random 96-bit IV per message. GCM both encrypts and authenticates, so there is no separate MAC.
  Requests carry an encrypted random client id and an increasing sequence number. The additional data names the
  direction and call, and responses also bind the client id and sequence number, so a recorded response cannot answer a
  fresh call or move to another endpoint. GET requests carry their envelope in the `x-interactive-artifact` header.
  Response statuses travel inside the sealed payload, so errors cannot be forged either. The server refuses unsealed or
  unopenable requests with a 400, even when they carry the token.
- **The key is never sent where an impostor could see it.** It reaches the browser inside the page HTML, and `reply`
  through `server.json`. Requests never carry it. **The server delivers the initial HTML exactly once**, claiming that
  delivery before any asynchronous work. Both `/` and `/index.html` spend the same allowance; HEAD and other methods
  cannot spend it. Later requests get 410, including after a failed initial delivery. Someone who learns the URL token
  from an already loaded tab's traffic cannot fetch its key from the real server. A broken initial load, a manual
  reload, or a second tab needs a new server and link.
- **A loaded tab never fetches its executable shell again.** The initial response includes the skill's JavaScript and
  CSS inline. Live edits arrive as encrypted, authenticated fragments through `GET page` and replace only the page
  content inside the original document. Cards, drafts, note text, and the connection key stay in memory. The source page
  is not available through the ordinary sibling-file route.
- **Editing waits for the first state load.** The page, comments, and Submit dock stay inactive while saved answers
  load, including through connection failures and authenticated server errors. A successful request alone does not
  enable editing: the tab must first adopt and render that initial state, so its replacement cannot erase an edit made
  while loading. A loading banner explains the pause.
- **Connection failures reconnect; failed verification locks.** A refused connection or a ten-second timeout (including
  while reading the body) shows a reconnecting banner and temporarily freezes edits. The tab retains unsaved work and
  retries the same call with delays of one, two, four, then five seconds, capped at five seconds. Only a verified
  response restores editing. It does not navigate, fetch a new key, or fall back to plaintext. A response that does not
  open under the original key and this call's identity locks the tab permanently, stops traffic, and tells the user to
  ask for a new link without reloading. Authenticated server errors show as errors. A connection failure after the
  agent's authenticated `done` status instead ends the page calmly. `reply` refuses unverified responses too.
- **Retries do not repeat operations.** Each browser serializes its API calls. The server keeps the last request and
  answer for each client, returning a snapshot of that answer on an identical retry without repeating the action. Older,
  changed, or out-of-order requests are refused. Entries last until server exit, with a limit of 1024 clients (each CLI
  invocation counts as a new client); new clients are refused at capacity rather than evicting replay protection. An
  outage never advances the browser's sequence until it receives an answer. A locked tab still needs a new server and
  link: its token may have reached another listener, and a reload could load that listener's code.
- **Pages work only in a secure context.** The browser offers Web Crypto only on pages it treats as secure.
  `http://127.0.0.1` and `http://localhost` qualify, including through an `ssh -L` tunnel; a LAN address would not,
  which is one reason the server has no option to listen on one. Without Web Crypto the page locks instead of falling
  back to plaintext.
- **The page's directory must be private.** `serve` refuses to start unless the directory holding the page is owned by
  the user and has no group or other permission bits. The page file and anything served next to it are readable from
  disk, so a shared directory would leak them without touching the server. A private parent also means nobody else can
  plant or swap files inside it between interactive-artifact's checks and its writes, which is what makes the checks
  below race-free. That last part also assumes no other user can rename the page's directory itself, which holds when
  every directory above it is either private or sticky (`/tmp`, where `mktemp -d` puts it, is sticky).
  interactive-artifact does not check the ancestors.
- **`localhost` cannot be intercepted on the server's port.** The server listens on `127.0.0.1` and on the same port on
  `[::1]` as well (when the host has IPv6 loopback), and picks another port if someone already holds it there.
  `localhost` often resolves to `::1` first, in browsers and in `ssh -L`, so a free `[::1]` port would let another user
  receive the tab's requests, token included, and pass them through to read the page and its key. The tunnel `SKILL.md`
  gives spells out `127.0.0.1` on both ends anyway, which also covers the user's own machine, where interactive-artifact
  cannot hold ports.
- **One server per page.** `serve` takes an exclusive lock file in the page's state directory, after binding its port,
  and refuses to start while another server holds it. Two servers would each keep their own copy of the state and hand
  out the same submission numbers. A lock left by a crashed server is taken over once its heartbeat is stale and, on
  Linux, no interactive-artifact process has its pid any more (the command line is checked, so a reused pid does not
  count). `wait` and `reply` use the same test to tell a live server from a dead one. Two servers taking over the same
  stale lock within milliseconds of each other can both win; that is not guarded against. On the way out a server
  removes the lock and its heartbeat only if they are still its own. Each step of shutting down (closing listeners, then
  finishing queued calls and writes) waits at most a few seconds, so a hung request or filesystem cannot keep a stopped
  server alive, and the listeners are closed before the lock goes, so nothing keeps serving once it is gone.
- **API calls run one at a time.** Several of them await file writes while working on the state, and interleaving would
  let one overwrite another's changes.
- **State on disk is owner-only.** The state directories (`.interactive-artifact/`, `.interactive-artifact/<page>/`, and
  its `submissions/`) are created 0700 and tightened to 0700 if they already exist. A state directory that is a symlink,
  or is owned by someone else, is refused. Every file interactive-artifact writes, including the temporary file behind
  each atomic write, is created 0600 at creation time rather than chmodded afterwards, so its contents are never
  readable in between. A stale temporary file from a crashed run is removed rather than reused, because a mode applies
  only when a file is created.
- **Content never goes on a command line.** Command lines are visible to every local user (`ps`, and `/proc` unless it
  is mounted with `hidepid`). The only things interactive-artifact's commands take as arguments are paths, annotation
  ids, and options. `reply --file PATH` reads the reply text from a file (as `anchor --file` does the quote, and `status
  --file` the agent's message), and `SKILL.md` tells agents to write that file, and the page, with their harness's file
  tool. A heredoc is not enough: harnesses run a shell command as `bash -c '<the whole command>'`, so the heredoc body
  is on that bash's command line. `reply --stdin` and `--text TEXT`, and `serve --title TEXT`, still exist, for text
  that is not sensitive, and `SKILL.md` says so. It also tells agents never to open the URL with a command (`xdg-open`
  and the like), since the token would then be on a command line.
- **`reply` ignores proxy settings and refuses redirects.** Deno's fetch would otherwise send the tokened URL through
  `HTTP_PROXY` and its relatives even for `127.0.0.1`, so `reply` drops them before connecting. Browsers never proxy
  loopback. Neither `reply` nor the page follows a redirect from the API.
- **`wait` and `reply` check the state directory before trusting it.** They read files that decide what the agent
  believes the user said (`submissions/`) and where its replies go (`server.json`), so they run the same ownership and
  mode checks as `serve` and refuse a directory another user could have planted files in. `reply` also refuses a
  `server.json` whose page and API URLs are not exactly the shape `serve` writes (http, `127.0.0.1`, no user info, a
  token path), or whose origins differ.
- **Served files are never sniffed into HTML, and never run on the page's origin.** Files from the page's directory go
  out with `Content-Security-Policy: sandbox`, so an HTML, SVG, XHTML, or XML file there runs in an opaque origin and
  cannot reach the key. They also carry `X-Content-Type-Options: nosniff`, and a file with no recognized type as
  `application/octet-stream`. A saved log containing `<script>` would otherwise run on the page's origin and could read
  the key.
- **A page that cannot render still comes back inside the shell.** A missing file or a full HTML document renders as an
  error message in the normal page, so an open tab keeps polling and recovers once the agent fixes the file.
- **Browsers cannot be used against the server.** Write requests need an `x-interactive-artifact` header, which no
  cross-site request can send without a CORS preflight that the server never approves. If they carry an `Origin`, it
  must match. With a loopback bind, the Host header must be a loopback name, which stops DNS rebinding. These matter
  less now that the token gates everything, but they cost nothing and still apply if the token leaks through a side
  channel.
- **The tokened URL does not leak through Referer.** Pages are served with `Referrer-Policy: no-referrer` and the
  matching meta tag, so links on the page and the font requests to jsDelivr carry no Referer at all.

### Accepted risk: reloading into an impostor's page

Plain HTTP has no way to authenticate the page itself, only the messages after it. If the server has exited and another
local user has bound its port, then reloading the tab by hand, or opening the old URL again, loads whatever that user
serves. An impostor can answer any path, even without knowing the original page token. Their page could look like the
original and collect whatever the user types into it.

The damage can outlast that page. `127.0.0.1` is a secure context, so the impostor's page can register a service worker
for that address and port, and the browser keeps it. A later interactive-artifact server that happens to get the same
port then runs under that worker, which can read the page (key included) and everything the user does. The tab locks
when it finds a service worker controlling it, since interactive-artifact never installs one, but a hostile worker can
rewrite the page and remove that check. Ports normally differ between servers, which is why `SKILL.md` gives a tunnel
that reuses the server's own random port rather than a fixed one, and makes ssh fail when that port is taken.

This is accepted. Closing it would take TLS with a certificate the browser trusts, which a short-lived local server
cannot provide without much more machinery than the risk justifies. Both the reconnecting and locked banners tell the
user not to reload. Automatic updates do not navigate at all: they authenticate the new fragment inside the already
loaded document.

### Known gaps

- **Where `serve`'s stdout goes is the caller's business.** The printed URL is a credential. Claude Code writes
  background-task output under a 0700 directory, which is fine. A harness that writes it somewhere world-readable breaks
  the model, and the skill cannot detect that.
- **The page loads fonts from jsDelivr.** Only font files come from the CDN, never CSS. A stylesheet could read the page
  (attribute selectors can probe the key in the script tag), and a font file cannot, so the `@font-face` rules are
  written out in `interactive-artifact.css`. The CDN learns that someone opened an interactive-artifact page, which of
  the two Inter subsets (latin, latin-ext) its text uses, and, when JetBrains Mono is not installed locally, whether the
  page has code and in which weights. It learns nothing else about the content, and not the URL.
- **The page fragment itself is trusted, deliberately.** Script in the fragment runs on the page's origin, so it can
  read the key and act as the user. interactive-artifact adds no scripts beyond its own, and `SKILL.md` tells agents not
  to either, and to escape untrusted text they put on a page, but nothing in the browser enforces that: there is
  intentionally no Content-Security-Policy on the page, so that pages stay free to do whatever their author needs.
  (Files next to the page are sandboxed; see above.) Two stricter options were considered and not taken:
  - A strict CSP (`default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:;
    font-src 'self' https://cdn.jsdelivr.net; connect-src 'self'`): only interactive-artifact's own script runs, and
    images and other resources load only from the page's directory, `data:` URLs, and the font CDN. It would stop
    unescaped untrusted text from reaching the key, at the cost of page scripts, inline event handlers, and external
    images.
  - The same with `img-src https:` added: external images work again, but an image request can carry data to a third
    party (CSS can choose which image to load), which reopens a path for leaking what is on the page.
- **Sibling files are ordinary HTTP resources.** Images and other files next to the page use the original page token,
  which API polling and reconnects never expose. If a resource request itself reaches a port holder, it reveals that
  file capability, and the port holder can replace the requested resource or read sibling files from the real server.
  Knowing the file capability alone does not authorize API calls or another delivery of the key. The skill's own
  executable shell is inline and never requested this way.

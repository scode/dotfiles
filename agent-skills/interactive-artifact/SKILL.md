---
name: interactive-artifact
description: Use only when the user invokes `interactive-artifact`, `$interactive-artifact`, or `/interactive-artifact`, or asks for an interactive artifact in those words. Serves a styled, commentable HTML page from a short-lived local web server and collects the user's picks, comments, suggested edits, and notes back to the agent. Nothing is published; the page lives on 127.0.0.1 until the server stops.
---

# interactive-artifact

`interactive-artifact` serves one HTML fragment as a styled page from a short-lived local web server, and carries what
the user does on it back to you. Use it when terminal markdown is not enough: options to pick from, questions to answer,
a draft to mark up, a comparison grid, a chart. It is the local alternative to publishing a gist or a hosted page, so
nothing leaves the machine.

On every page the user can select text to comment on it, suggest a replacement, or copy it as Markdown; answer any
pickers you put on the page; and write a note. Submit sends all of it to you at once. You reply to comments and
suggestions on the page and edit the page itself; the open tab follows along.

## Running a page

`<skill-dir>/interactive-artifact` is the script; call it by absolute path.

1. Make a fresh directory for the page with `mktemp -d`. The server refuses a directory that other users can access,
   serves every other file in it (that is how `<img src="shot.png">` works), and keeps its state in a
   `.interactive-artifact/` subdirectory, so never put the page directly in `/tmp` or a repository.
2. Write the page into it as `page.html` with your harness's file-writing tool, never with a shell heredoc or `echo`:
   harnesses run a shell command as `bash -c '<the whole command>'`, so anything in the command text, heredocs included,
   is on a command line other users can read. See "Writing the page" below.
3. Start `interactive-artifact serve <dir>/page.html` in the background with your harness's background-run facility (not
   `&`). The first line of stdout is the URL (`http://127.0.0.1:<port>/<token>/`). Give it to the user. The token is the
   only thing keeping other users of the machine out of the page, so do not put the URL anywhere else: not in files,
   commits, or messages to other people or agents. Never pass it as an argument to a command either (`xdg-open`, `open`,
   a browser binary): command lines are visible to every user on the machine. The user clicks the link. Anything without
   the token gets a 404. Only one server runs per page; `serve` refuses to start a second one while the first is alive,
   so keep using the first (or stop it before starting another).
4. Start `interactive-artifact wait <dir>/page.html` in the background the same way. It exits when the user submits,
   printing the submission as Markdown. Handle it (see below), then start `wait` again. Keep one `wait` running for as
   long as the page is in use, and only one per page.
5. To change the page, edit the file. The open tab reloads by itself within about a second and keeps everything the user
   has entered, so never restart the server for an edit.
6. Stop the server when the user is done with the page. It also exits on its own after `--ttl` (default `4h`). A running
   `wait` then exits with status 4.

If the user reports a red banner saying the page is locked, stop the server and start a new one, then give them the new
URL (and the new tunnel command, if they use one). Never tell them to reload the locked tab. A lock means the tab talked
to something it could not verify, and its requests carry the URL token, so treat that token as burned even if the server
is still running. The new server has a new token and key, and everything the tab saved before it locked carries over.

Submissions queue on disk, so the user can submit while you are busy; the next `wait` returns at once with everything
not yet seen. `wait --timeout 30m` exits with status 3 if nothing arrives in time, and `wait --json` prints the raw
submissions instead of Markdown.

`serve` options: `--port N` (default `0`, a free port the OS picks), `--ttl 90m` (`s`, `m`, `h`, up to `596h`, or `0`
for no limit), `--title TEXT` (default: the first `<h1>`, then the file name; it is on the command line, so never
anything sensitive). The server always listens on `127.0.0.1` only.

If you are on a different machine from the user's browser, the user needs a tunnel that uses the server's own port on
both ends: `ssh -o ExitOnForwardFailure=yes -L 127.0.0.1:<port>:127.0.0.1:<port> <host>`, then the URL as printed. Tell
them that along with the URL. Spell out `127.0.0.1` rather than `localhost` (`localhost` often resolves to `::1` first,
and another user on either machine could be listening there), and do not suggest a fixed local port such as 8000:
`ExitOnForwardFailure` makes ssh fail instead of carrying on when someone else already holds the port, and a fresh port
per server means a port someone tampered with before is not reused. There is no way around the tunnel: pages are
encrypted with browser crypto that browsers offer only on `127.0.0.1` and `localhost`.

## Handling a submission

A submission lists, in this order: the user's note, the current answer of every picker on the current page they have
touched (flagged `changed` or `unchanged` since their last submission), each new comment or suggestion with the text it
is anchored to and the section it is in, replies on existing threads, and the ids the user withdrew.

Do what the submission asks, in the context of the task: a pick may decide what you build next, a comment on a draft may
need an answer or a change, a suggestion usually means editing the text. Then answer on the page so the user sees what
happened to each item:

- `interactive-artifact reply <dir>/page.html c3 --file <dir>/reply.txt` adds your reply to a thread. Use it to answer a
  question, push back, or ask for clarification. Write the reply into `<dir>/reply.txt` with your file-writing tool
  first (the same as the page, and for the same reason: anything in a shell command, heredocs included, is visible to
  every user on the machine). Overwrite the file for each reply.
- Add `--resolve` when the item is dealt with. Resolving marks the card done; the user can reply to reopen it. A bare
  `--resolve` without text is fine.
- Several ids share one reply: `interactive-artifact reply <dir>/page.html s4 s5 s7 --resolve --file <dir>/reply.txt`.

To apply a suggestion, edit the page source so the quoted text becomes the replacement. The quote is the rendered text
with whitespace collapsed, so search the source loosely; it may span inline tags. Once the text is gone from the page,
the card shows as orphaned, which is expected for an applied suggestion; resolve it.

Whenever you edit text that an open (unresolved) comment or suggestion is on, either resolve it, or point it at the new
text so the user still sees what it refers to: write the new text, exactly as it reads on the rendered page, to
`<dir>/quote.txt` with your file tool (enough of it that it appears only once on the page; the comment attaches to the
first place it appears), then run `interactive-artifact anchor <dir>/page.html c3 --file <dir>/quote.txt`. Otherwise the
card shows "This text is no longer on the page". Edits that leave a comment's quoted text alone need nothing; the page
finds it again by itself.

A reply to an id the user has removed fails with "no such comment"; the withdrawal arrives with their next submission.
Stop acting on withdrawn ids. Picks have no threads; if a pick needs acknowledging, say so in the page or the
conversation.

## Writing the page

The file holds body content only: no `<!doctype>`, `<html>`, `<head>`, or `<body>`. The server refuses a full document.
It wraps the fragment in a shell that loads the default stylesheet (Inter for text, JetBrains Mono Nerd Font for code,
light, dark, or following the system, chosen with a button in the corner), so do not add your own `<style>` or font
links unless a page genuinely needs something the stylesheet lacks. Do not add scripts, and do not put HTML or SVG files
you did not write in the page's directory: anything that runs on the page can read the user's input and submit as them.
The shell's own script handles everything interactive. When the page shows text from somewhere you do not control (issue
bodies, logs, web content), HTML-escape it (`&`, `<`, `>`, `"`, `'`) so it renders as text rather than markup.

Plain HTML elements are styled with no classes needed: headings, paragraphs, `strong`, `em`, `del`, `mark`, `kbd`,
inline `code`, `pre`, nested `ul`/`ol`, `dl`, `blockquote` with `cite`, `table` (add `class="num"` to numeric cells),
`details`/`summary`, `figure`/`figcaption`, `hr`, footnotes via `sup`. A paragraph right after the `<h1>` renders as a
subtitle. `examples/sample.html` exercises all of these plus a chart; read its source for the markup.

Keep the page focused on what you want the user to look at. Start with the conclusion or the decision you need, not with
background. Headings matter more than usual: each comment is reported with the heading it falls under.

## Pickers

Any element with `data-pick="<id>"` is a picker, and its descendants with `data-value="<value>"` are its choices. The
user clicks a choice to select it and clicks it again to clear it. Each picker gets a text field for a free-form answer.

- `data-multiple` on the picker allows several choices at once; without it, picking one clears the others.
- `data-label` on the picker or a choice sets the name used in submissions. The defaults are the picker's first `.q` or
  heading, and the choice's first heading, its `strong`, or its text.
- `data-placeholder` sets the text field's placeholder; `data-no-text` leaves the field out.
- Ids must be unique on the page and stable across edits; answers are stored by picker id and choice value.

Option cards become selectable cards (`<div class="options" data-pick="backend">` with `<article class="option"
data-value="sqlite">` children); any other choice element renders as a pill. A question list item with `data-pick` and
`<span data-value>` choices is the usual way to ask a question with fixed answers. `examples/interactive.html` has
single, multiple, and text-free pickers, and a draft to try comments and suggestions on.

## Presentation patterns

Ten opt-in component classes cover the common reasons to show a page. `examples/patterns.html` shows each one with its
markup; read it before using a pattern for the first time.

| Pattern           | Markup                                     | Use it for                                                    |
| ----------------- | ------------------------------------------ | ------------------------------------------------------------- |
| TL;DR summary     | `div.tldr`                                 | The conclusion up front, once per page                        |
| Callouts          | `div.callout` + `tip`, `warn`, `danger`    | The one caveat a skimming reader must not miss                |
| Option cards      | `div.options > article.option`             | Two to four alternatives to pick from, with `ul.pros`/`.cons` |
| Comparison matrix | `table.matrix`, cells `yes`/`partial`/`no` | Many options against the same criteria                        |
| Question list     | `ol.questions`, `.q`, `.default`           | Questions for the user, each with the default you'll assume   |
| Diff              | `pre.diff`, one `span` per line            | A proposed code change, in place                              |
| Stat tiles        | `div.stats > div.stat`                     | A handful of headline numbers                                 |
| Timeline          | `ol.timeline`, items `done`/`active`       | A plan or a history                                           |
| File tree         | nested `ul.tree`, items `add`/`del`        | A proposed or existing directory layout                       |
| Before and after  | `div.split > section.before/.after`        | Two versions of wording, output, or config                    |

Small utilities: `span.badge` (with `blue`, `green`, `amber`, `red`), `.muted`, and `span.icon` for Nerd Font glyphs
(`&#xf07c;`).

## Charts

Draw charts as inline SVG inside `figure.chart`, with `span.chart-title`, `span.chart-subtitle`, and a `ul.legend` when
there are two or more series. Use the stylesheet's classes instead of `fill=` or `stroke=` attributes, because SVG
attributes cannot follow the light and dark themes: `gridline`, `baseline`, `tick`, `label`, `label-2`, `line`, `dot`,
`area`, `bar`, and `s1` to `s4` for the series colors (in that order, never skipping). Give each data point a `<title>`
and a larger transparent `circle.hit` before its `dot` (a dot may have `r="0"` and grows on hover), and put the numbers
in a table under a `details` so they are readable without the chart. `examples/sample.html` has a complete two-series
line chart.

Compute coordinates with a script rather than by hand.

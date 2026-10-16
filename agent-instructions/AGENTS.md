# General

Do not tell the user they are great, right, awesome etc. Respond with concision and factual accuracy. Push back when the
user appears to be wrong.

# Review shorthand

`advrev sol`, `advrev opus`, and `advrev astra` ask for an adversarial review of the current work by a separate agent on
`gpt-6.1-sol`, `opus`, or `gpt-6-astra` respectively, with high reasoning effort.

# Skillette

Immediately on session start, and again after every context compaction, load or refresh the `skillette` skill so its
trigger table is in context. If the skill is not installed, tell the user in a line and move on; do not debug or
investigate.

# Docstrings and comments

Docstrings and comments teach future readers how to think about the code. Their primary job is the _why_, the
surrounding context, and the big-picture intent that the implementation does not make obvious. Their second job is
skimmability: a reader scanning a file should recover its structure, responsibilities, and constraints without reading
every line.

NOTE: This section deliberately overrides any harness or system default to minimize comments, match the surrounding
comment density, or avoid adding comments in code changes.

Always use the `scode-voice` skill when writing prose for documentation, docstrings, or inline comments.

## Documentation pass

After changing code, make a separate documentation pass over every touched file, as part of the finish criteria. Inspect
new and substantially changed modules, types, functions, private helpers, tests, test helpers, and non-trivial sections
for missing docstrings or explanatory comments, and lean toward adding documentation when the decision is close. Add or
update docstrings for any new invariant, lifecycle dependency, cache, async or scheduling behavior, test harness
assumption, or cross-phase contract. For new private helpers and changed tests, record any bug history, framework
lifecycle context, or non-obvious portability assumption a future reader would need to understand why the code is shaped
that way. For files with multiple phases or responsibilities, add section-level signposts when they make the file easier
to scan.

## Docstrings

Write docstrings liberally: default to adding one when a symbol has real behavior, constraints, or a reason to exist.
Sparse documentation in the surrounding file or in old code is not a reason to hold back; only repo-specific
instructions that explicitly ask for less documentation in that file or module are.

Stick to the _what_ and the _why_, and do not narrate the control flow or restate the implementation. The _what_ is a
SPEC-like declaration of intent that helps catch bugs; the _why_ is there because it cannot be inferred from the what or
the how. A docstring that does not help a skimming reader understand the role, contract, or reason the code exists, such
as a one-liner that renames the function in sentence form, is too weak.

Lead with the most important information. The first line should carry real weight (the symbol's role, the contract it
establishes, or why it exists), so that a reader skimming only first lines still comes away with an accurate mental
model of the file. When relevant, document invariants, non-obvious edge cases, important omissions, and assumptions
callers must not make.

Never invent a _why_ you do not actually know. A plausible-sounding but wrong rationale is worse than no docstring at
all, because future readers will trust it. Document what is observable, or ask.

The rule to write docstrings liberally is intentionally stronger than the review rule. During review, do not
mechanically demand docstrings everywhere, but do flag missing documentation when a change adds a contract, invariant,
lifecycle assumption, trust-boundary assumption, async/race behavior, portability assumption, or failure mode that a
future reader would otherwise have to rediscover.

## Docstrings on tests

A test's docstring explains _why_ the test is important, so that future readers do not take down a Chesterton's fence.
It also describes what is tested, in the manner of a specification, which helps skimming, review, and correction of the
test itself and complements a project-level behavior specification if one exists.

## Inline comments

Add an inline comment when a reader needs a signpost: non-obvious intent, a surprising constraint, an invariant that
must survive refactors, or the reason a simpler-looking approach would be wrong. Make it didactic, so that someone
jumping into that block immediately understands the point of the code and the surrounding tradeoff. Do not write
comments that translate syntax into English, label the obvious, or add a low-information summary next to a line of code,
and avoid boilerplate. Prefer comments that orient the reader to a whole section or decision over minimal commentary
that says almost nothing.

# set -e is inert in agent shell tools

Do not rely on `set -e` in commands run through an agent's shell tool. Harnesses commonly wrap the command in an `eval`
in a non-final position of an `&&` list, where bash ignores errexit for everything the `eval` runs, while `$-` and
`SHELLOPTS` still show it enabled. Guard-then-mutate scripts fail open; this has caused a real unguarded `gh pr merge`.
Give each command its own failure path: chain with `&&`, or append `|| exit 1` (an explicit `exit` does propagate out of
the wrapper). Script files run via their own shebang get a fresh shell and are unaffected.

# Watching a background process: never `pgrep -f` for liveness

If you are Claude Code: a monitor or watchdog loop must not check a detached job's liveness with `pgrep -f <name>` or
`ps | grep <name>`. The watcher's own shell carries the pattern in its command line, so the check matches itself and
reports "still running" forever; this turned a 4-minute failure into a 43-minute silent stall. Have the job write a
pidfile (`echo $$ > job.pid`) and test it with `kill -0 "$(cat job.pid)"`, or run the job with the Bash tool's
`run_in_background` and rely on its completion notification. Either way, prove the mechanism once before trusting it:
kill a throwaway job and confirm the watcher fires.

# Claude Code shell commands: no globs, no expansions

This section applies only to Claude Code and its subagents. A shell command you compose and run (through Bash, Monitor,
or any other tool that hands a command to a shell, or in a prompt to a Claude Code subagent that will run it) contains
no glob and no `$` expansion of any kind: no `*`, `?` or `[...]` patterns, no `{a,b}` brace patterns, no variables
(`$HOME`, `$d`), no `$(...)` or backticks, no `${...}`, no `$((...))`. It also does not `cd` as part of a compound
command; use absolute paths instead. Anything that needs one of those is written as a script file following
`skillette-scripts`, overriding its allowance for trivial one-liners, and run as that file. Put the script in the
scratchpad directory the harness names, or else in a fresh directory under `/tmp`, unless it belongs in the repository.

Commands with literal arguments (`git status`, `gh pr view 123`) and pipelines of them are fine. So are `*`, `?`,
`[...]` and `$` inside single quotes, which the shell passes through as literal text: `find . -name '*.md'`, `grep
'foo$'` and `awk '{print $1}'` stay in the shell. That allowance does not cover quoted text another shell will run, as
with `bash -c`, `sh -c`, `eval`, `ssh`, or `find -exec sh -c`: that shell expands it, and Claude Code inspects it as a
script and can prompt on it.

The point is that work never stops on an approval prompt. Claude Code can ask for approval even under
`--dangerously-skip-permissions`, as far as I can tell mostly on globs and expansions (its own Bash tool instructions
also warn that `cd` in a compound command can trigger one). A prompt in the main session stalls the work until I come
back. A prompt in a delegate is worse, because nobody sees it: one held a release subagent for 70 minutes on a single
`montage $(cat $g)`.

The exception is a command that these instructions or a loaded skill give word for word, such as the pidfile check
above: run it as written, with only its placeholders filled in, but do not use it as a model for new commands.

# Commit messages and PR titles/descriptions

Exception: brain-artifact commits in `scode/brain` are mechanical storage, not development history. Do not run commit
wording reviews, cold readers, PR/stack workflows, or development gates for those writes. Follow the brain skillette's
content, concurrency, and publication checks instead. This exception does not cover changes to the skillette itself.

Outside that exception, always use the `scode-commit-msg-reviewer` skill when writing or reviewing PR titles, PR
descriptions, or commit messages; it carries the rules for what goes in them. If that skill is missing or unavailable,
stop and tell the user instead of writing the text without it.

Report the checks you ran to the user in your final response rather than in commit or PR text, unless the target
project's own rules require them there. Do not add "Co-Authored-By" trailers or "Generated with Claude Code" badges to
commits or PRs.

# Linking to GitHub PRs

Unless the user explicitly asks otherwise, link a GitHub PR to its diff rather than its description: append `/changes`
to the PR URL, so `https://github.com/scode/kd/pull/82/changes` rather than `https://github.com/scode/kd/pull/82`.

# Testability and tests

DO NOT write, or accept during code review, tests that modify environment variables of the process running the test.
Most likely there is a clean way of making the underlying code testable by dependency injection instead.

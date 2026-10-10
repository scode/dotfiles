# skillette-scripts

The user strongly prefers Deno with TypeScript for scripts, over bash, Python, Perl, or anything else. The point is to
avoid the usual shell problems (quoting, portability between bash versions, failures that silently fall through) and to
get type checking and tests. This covers script files and ad-hoc programs alike. A trivial pipeline such as `git log |
head` is still fine in the shell.

Use something else only for a very strong reason, and say in a line what it was. Examples of such reasons: the new
script belongs alongside the project's existing scripts in another language, the code is part of something whose format
dictates the language (a Dockerfile `RUN` step, a CI `run:` block), or the task is a trivial one-liner. Deno not being
installed on this machine is not by itself such a reason. Ask the user whether to install it. If you are running
unattended with no one to ask, you may run `brew install deno` if brew is available and your permissions allow it. Only
when Deno cannot be had that way either, fall back to whatever scripting language is available, and say in a line that
Deno was missing. This is a strong preference, not a hard requirement, so do not stall on it.

## Conventions

- One file. Start it with `#!/usr/bin/env -S deno run -A` and make it executable. The `.ts` extension is optional: `deno
  run`, `deno check`, `deno test`, and imports all treat an extensionless file as TypeScript.
- Pin an exact version on every `jsr:` and `npm:` import (`jsr:@std/assert@1.0.19`, not `jsr:@std/assert`). Look up the
  current version rather than recalling one (`https://jsr.io/<scope>/<name>/meta.json`, `npm view <pkg> version`). Do
  not create a `deno.json`; outside a Deno project, dependencies go to Deno's global cache and nothing is written next
  to the script.
- Run subprocesses with `jsr:@david/dax`. Pass every variable through `${}` interpolation, which escapes it, and an
  array interpolates as separate arguments. Never build a command string by concatenation. A non-zero exit throws; use
  `.noThrow()` when you want to inspect the exit code. Like the shell, a pipeline only reports its last command's exit
  unless you add `.pipefail()`.
- Put tests in the script itself with `Deno.test`, and put the main logic behind `if (import.meta.main)`. `deno run`
  ignores `Deno.test`, and `deno test` skips the main block, so one file is both the tool and its tests.

## Checking and running

After writing or changing a script, and before running it, run `deno fmt --ext=ts <file>`, `deno check <file>`, and
`deno test -A <file>`, and fix what they report. A script you only run without changing needs none of this. The
`--ext=ts` is there because `deno fmt`, unlike the other commands, skips an extensionless file. Set `NO_COLOR=1` when
running Deno so the output is not full of ANSI codes. The first run of new imports prints a `Download` line per fetched
file to stderr; that is not an error.

NOTE: `deno test` runs sandboxed regardless of the `-A` in the shebang, which is why it needs `-A` too. Without it,
tests fail with `NotCapable` as soon as they touch the filesystem or environment, and that includes npm packages that
read `process.env` internally. Do not wrap calls in a catch-all that would turn such an error into a plausible-looking
result; catch the errors you expect and rethrow the rest.

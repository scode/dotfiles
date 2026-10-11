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
- Run subprocesses with `jsr:@david/dax`. Write a command in the `$` template the way you would type it in a shell, and
  use `${}` only for the values that vary. dax parses the literal text as shell words and escapes every interpolation,
  and an array interpolates as separate arguments. Never build a command string by concatenation, and do not build an
  argument array for fixed flags either. A non-zero exit throws; use `.noThrow()` when you want to inspect the exit
  code. Like the shell, a pipeline only reports its last command's exit unless you add `.pipefail()`.
- A newline inside a `$` template separates commands, as it does in a shell. Continue a long command onto the next line
  with a trailing `\`. Without it, the first line runs as a command of its own and the rest of the flags run as another.
- dax does not forward signals to the processes it starts. Ctrl-C reaches them anyway because the terminal signals the
  whole process group, but a SIGTERM sent only to the script (`kill`, launchd) leaves every child running as an orphan.
  A script that runs children in the foreground should run them under one `KillController` and kill it from a signal
  listener:

  ```ts
  const kill = new KillController();
  for (const sig of ["SIGINT", "SIGTERM"] as const) Deno.addSignalListener(sig, () => kill.kill());
  const results = await Promise.all(commands.map((c) => c.noThrow().signal(kill.signal)));
  ```

  There is no `exec`. To exit with a single command's status, use `Deno.exit((await cmd.noThrow()).code)`.
- dax runs some commands itself, `sleep` among them, so `` $`sleep 5` `` never starts a child process. Use the full path
  (`/usr/bin/sleep`) when the real binary matters, such as in a test that needs a process to signal.
- Put tests in the script itself with `Deno.test`, and put the main logic behind `if (import.meta.main)`. `deno run`
  ignores `Deno.test`, and `deno test` skips the main block, so one file is both the tool and its tests.

## Keeping scripts short

A Deno script should not come out much longer than the shell script it replaces. One port of a 110-line bash script came
back at about 330 lines, and almost none of that was TypeScript's fault; a rewrite that kept the behavior that mattered
was about 50. The extra length came from these, so avoid them:

- Tests out of proportion to the risk. Test logic that can be wrong in non-obvious ways: parsing, data transforms,
  anything with edge cases. A script that mostly launches commands may need no tests at all. Do not add parameters,
  types, or indirection whose only job is to let a test reach the process handling.
- Porting bash's workarounds instead of its intent. Re-running `$0` once per item for concurrency, introspecting a
  `case` statement with `sed`, and signal traps that exist only to make bash run its EXIT trap all have direct
  equivalents here (`Promise.all`, a `Map`, the `KillController` above). Exit codes that were a side effect of those
  workarounds do not need preserving.
- Catching everything at the top level. Deno prints an uncaught error with its stack trace and exits 1, so a top-level
  catch that prints the message and sets the exit code adds lines and loses the stack.
- Lookup tables written as an interface plus an array of object literals. A `Map` of tuples, one row per line under `//
  deno-fmt-ignore`, is shorter, and unlike a plain object it has no inherited keys, so a name like `toString` is not
  accidentally valid.

## Checking and running

After writing or changing a script, and before running it, run `deno fmt --line-width=120 --ext=ts <file>`, `deno check
<file>`, and `deno test -A <file>`, and fix what they report. A script you only run without changing needs none of this.
The default width of 80 columns wraps command chains, argument lists, and tables until a script is noticeably longer,
hence `--line-width=120`. The `--ext=ts` is there because `deno fmt`, unlike the other commands, skips an extensionless
file. Set `NO_COLOR=1` when running Deno so the output is not full of ANSI codes. The first run of new imports prints a
`Download` line per fetched file to stderr; that is not an error.

NOTE: `deno test` runs sandboxed regardless of the `-A` in the shebang, which is why it needs `-A` too. Without it,
tests fail with `NotCapable` as soon as they touch the filesystem or environment, and that includes npm packages that
read `process.env` internally. Do not wrap calls in a catch-all that would turn such an error into a plausible-looking
result; catch the errors you expect and rethrow the rest.

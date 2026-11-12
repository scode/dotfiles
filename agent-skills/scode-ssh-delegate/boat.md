# Boat sandbox workers

Read this file when an invocation of scode-ssh-delegate grants a boat budget, and again before the first use of a boat
worker after compaction or resume. It holds everything boat-specific: how a budget is registered, what a fresh sandbox
looks like, how to create, bootstrap, and use one, and the lifecycle rules that keep boat cheap. The trust rules shared
with SSH hosts, sprites, and tensorlake sandboxes are in `using-workers.md`.

Boat sandboxes (https://docs.boat.dev) are full x86_64 Ubuntu VMs driven through the `boat` CLI, which is assumed to be
installed and signed in. Do not check its authentication up front; a command that fails with an authentication error is
reported to the user like any other failure. Two things make boat different from sprites and tensorlake, and most of the
rules below follow from them:

- Boat charges for every sandbox start, not just for running time. The plan includes a number of starts per rolling
  minute, hour, and day (12, 60, and 200 on the $20 plan), and each start beyond those costs 3 to 7 cents or is refused
  outright. Running time, by contrast, is cheap: about $0.036 an hour for the default size. So the rules here favor
  keeping a sandbox for several units of work over creating one per unit.
- A sandbox does not stop when idle. It runs, and bills, until it is deleted or reaches a fixed deadline (its TTL),
  which counts from creation, not from the last activity, and stops it even in the middle of a command.

## Register a budget

Only the budget form exists for boat: `$scode-ssh-delegate boat:N`, `$scode-ssh-delegate boat N` where N is a bare
positive integer, or natural language such as "it's okay to use up to N boat sandboxes". In the terse forms, `boat` must
be a whole argument of its own (or the `boat:` prefix), and in `boat N` the integer right after it belongs to the
budget; arguments after that are classified as usual, so `boat 10 hostA` is a budget of 10 plus the SSH host `hostA`. A
hostname that merely contains "boat", such as `boat-runner-1`, is an SSH target. It registers a number, not workers, and
nothing is created at invocation time. When another workflow later needs a worker, this session creates a sandbox, uses
it, and deletes it. N caps how many owned sandboxes are alive at the same time; it is not a total the session spends
down, so deleting a sandbox frees its slot. A budget never touches sandboxes this session did not create.

There is no borrowed form for boat. If `boat` is followed by something other than a bare positive integer (`boat foo1
foo2`), or the user asks to use existing boat sandboxes by name, register nothing boat-related and tell the user that
only a budget is supported.

Record N and the naming prefix for sandboxes this session will own. The prefix is the same `ssh-delegate-<cwd>-` rule as
for sprites: `<cwd>` is the basename of the current working directory lowercased, with every character outside
`[a-z0-9]` replaced by `-` and runs of `-` collapsed. Owned sandboxes are named `<prefix><n>` with `n` starting at 1.

At registration time, run `boat list --all --json` and look for sandboxes whose `name` starts with the prefix. Any such
sandbox is a possible orphan from an earlier session that did not get to clean up. Report those to the user (name, `id`,
and `state`) and move on: do not delete them (another session in the same directory may be using them), do not reuse
them, and do not stop to ask. When choosing names for new sandboxes, skip any `n` already taken.

## Sandbox facts

Observed on a fresh `small` sandbox created with `--no-env` on 2026-11-12, and confirmed against boat's documentation
where it covers the same ground. The platform is under boat's control and changes without notice, so verify rather than
assume.

- Ubuntu 24.04 LTS, x86_64 only (boat has no arm64). The Unix user is `user` in `/home/user`, with passwordless `sudo`.
- Claude Code (`claude`) and Codex (`codex`) are preinstalled and on the `PATH` that `boat exec` uses; boat says it
  updates both to their latest release when it prepares each machine. Rust, Node, Python, Docker, git, rsync, and `gh`
  are also there. `gh` is not signed in.
- The sizes are fixed, and the size is chosen at creation with `--type`:

  | `--type`  | vCPU | RAM   | Disk   | $/hour |
  | --------- | ---- | ----- | ------ | ------ |
  | `small`   | 2    | 4 GB  | 51 GB  | 0.018  |
  | `default` | 4    | 8 GB  | 51 GB  | 0.036  |
  | `large`   | 8    | 16 GB | 111 GB | 0.072  |
  | `xlarge`  | 16   | 32 GB | 251 GB | 0.200  |

  `xlarge` needs the $100 plan or higher; `boat limits --json` lists which types the account can create under
  `machineTypes`. `small` reported 3916 MiB of RAM, below the 4 GiB the agent CLIs need, so use it only for work that
  runs no agent CLI.
- `boat exec` propagates the remote exit code, but it does **not** forward stdin (verified: zero bytes arrive), and it
  kills the command after 30 seconds unless given `--timeout` (verified: SIGTERM, exit 143, `"timedOut": true`).
  `--timeout 0` means no limit; boat's help says that above 600 seconds, or with 0, the output streams live and the
  command survives network cuts.
- When stdout is not a terminal, the output format depends on the timeout. With a timeout of 600 seconds or less, `boat
  exec` prints one JSON object with the command's `exitCode`, `stdout`, and `stderr`, and the output fields can come
  back truncated (`stdoutTruncated`, `stderrTruncated`). With `--timeout 0` or above 600, it prints JSON lines instead:
  an `{"type":"attached"}` frame, one `{"type":"stdout","data":...}` (or `stderr`) frame per chunk, and a final
  `{"type":"exit","exitCode":...}` frame, with no truncation and no bound on the output. The local exit code matches the
  remote one either way.
- With `--timeout 0` or above 600, the command runs detached on the sandbox, and killing the local `boat exec` with
  SIGKILL left it running (a reviewer verified this on 2026-11-12; SIGINT and SIGTERM did stop it).
- With `--no-env`, none of the account's stored environment variables or secret files reach the sandbox, and the boat
  CLI inside it is signed out. The platform still sets its own `ASCII_TOKEN` variable for its agent server; `boat login`
  inside the sandbox rejected it, so it did not act as an account login. Whether it grants anything else was not tested.
- Unless a sandbox is created with `--no-monitoring`, boat keeps the text and output of every `boat exec` (boat's docs
  say for 90 days), the output of other processes, and process command lines, and the history can outlive a sandbox that
  stops instead of being deleted.

## Creating an owned sandbox

Starts are checked before creating, and they are a pool shared with everything else on the account, including any
`runs-on: boat` CI jobs, which wait when a start window is full. Before each create, run `boat limits --json` and
proceed only when `canStart` is `true`, `nextStartDollars` is `0` (the start is free), and the smallest `remaining`
across `starts.minute`, `starts.hour`, and `starts.day` is at least the number of sandboxes about to be created. When
several are needed, either check against that number once or create them one at a time with a fresh check each; never
launch parallel creates on a single "next start is free" answer, because they all read the same window and can overshoot
it. If the start would cost money (`nextStartDollars` above 0), would be refused (`null`), or a create fails with a rate
limit error (`429`, `rate_limited`), do not retry in a loop. Reuse an owned sandbox if one is free; otherwise, if
`startWindowFull.freesAt` is a few minutes away, wait locally until then, and if it is further out, report to the user
and let them decide. Paying for starts is the user's call, never this skill's.

Then create:

```sh
boat new --no-env --no-snapshots --no-monitoring --type default --ttl 7200 --name NAME --json
```

The command prints JSON lines and returns once the sandbox is ready. Record the `id` (`bx_...`) from the `created` line
alongside the name, the type, and the deadline (`archiveAfter` in the final `ready` line) for every owned sandbox. Every
other `boat` command addresses the sandbox by that ID, not by its name.

Each flag is there for a reason:

- `--no-env` keeps the account's stored secrets out of the worker, which the trust rules in `using-workers.md` require.
  Never create a boat worker without it.
- `--no-snapshots` turns off boat's background capture of the disk. Without it, a credential file pushed for an agent
  run would land in snapshots that outlive the work. The cost is that a stop erases the sandbox instead of keeping it,
  which is fine because owned sandboxes are never stopped and resumed (see the lifecycle section).
- `--no-monitoring` stops boat from keeping command text, output, and process command lines, which would otherwise hold
  agent transcripts, source excerpts, and anything a command happens to print. Live output of a running exec still
  works; only boat's stored history (`boat history`, `boat ps` samples) goes away.
- `--type default` (4 vCPU, 8 GB) is the default size. Choose another type when the workload calls for it: `large` or
  `xlarge` for heavy builds or tests that need more cores, RAM, or disk, `small` for light work that runs no agent CLI.
  Say which type was chosen and why when creating anything other than `default`.
- `--ttl 7200` gives two hours. The TTL is a hard deadline, so pick one that covers the work planned for this sandbox
  with margin. Before starting a unit of work that might run past the deadline, check `archiveAfter` in `boat info ID
  --json` and push it out with `boat extend ID --ttl SECONDS` (which sets the remaining lifetime). A stop at the
  deadline kills running commands and, with `--no-snapshots`, loses the sandbox's disk.

## Bootstrapping a sandbox

Bootstrap only once another workflow selects the sandbox, and only what that work needs. Run the base apt packages from
the "Base packages (all kinds)" section of `using-workers.md`. The agent CLIs are already installed: check `claude
--version` or `codex --version` for the one the work selects and use it as is. Run the vendor installer from "Worker
contract for Ubuntu SSH hosts" in `using-workers.md` only if the selected CLI is missing or fails its version check; the
same upstream trust boundary rules apply.

There are no snapshot templates for boat. Bootstrap is short because the image already carries the CLIs and toolchains,
and snapshots are off for owned sandboxes.

## Credentials on a sandbox

The credential policy in `using-workers.md` applies unchanged; the mechanics differ. Keep every secret out of command
text and out of local command lines, and keep it visible only to the agent run that needs it, not to builds or tests
that run on the same sandbox.

`boat exec` has no per-command environment option, and `boat new -e` would put the value in every shell and process on
the sandbox for its whole life, so do not pass a token that way. For a Claude token (`CLAUDE_CODE_OAUTH_TOKEN` or
`ANTHROPIC_API_KEY`), write a file outside the checkout holding the one `export` line, readable only by the local user
(create it with `umask 077`, from a script rather than a command that spells out the value), push it with `boat scp FILE
ID:/home/user/.agent-env`, and delete the local copy. Then source it inside the agent's own exec, for example `boat exec
--timeout 0 ID -- sh -c '. /home/user/.agent-env && claude -p ...'`, so only that process tree sees it. Remove
`/home/user/.agent-env` when the agent run ends.

For Codex, push the file with `boat scp ~/.codex/auth.json ID:/home/user/.codex/auth.json` and `chmod 600` it in a
follow-up exec. The `~/.codex` directory already exists on the image, holding a boat-managed `config.toml` that adds a
computer-use MCP server; leave that file alone. Remove `auth.json` when the agent run ends.

Because a sandbox is reused across units of work, removing the credential at the end of each agent run is what keeps the
one-credential rule: a sandbox carries at most one agent credential at a time, and a unit of work for a different
provider starts only after the previous one's credential is gone. Deleting the sandbox removes whatever is left.

## Running work and transfer

The transport for every command is `boat exec --timeout SECONDS [--cwd DIR] ID -- COMMAND...`. Always pass `--timeout`:
`0` for agent runs and anything long, or a bound that comfortably covers the command. The 30 second default kills
anything slower. Keep each unit of work in one foreground exec and stay attached from the controlling side until it
exits; if the controlling side's own tooling caps how long a command may run, run the exec in the controlling side's
background with output to a local file and wait on that locally.

With `--timeout 0` the output arrives as JSON lines (see "Sandbox facts"), and an exec only finished cleanly if its
output ends with the `exit` frame. Have long-running work, and anything whose output matters, write to a file on the
sandbox, and fetch that file afterwards rather than relying on the streamed output.

To cancel an exec, send the local `boat exec` SIGINT or SIGTERM, never SIGKILL, because a hard-killed local process
leaves the remote command running. If any exec on a sandbox ended without its `exit` frame (the local side was killed,
crashed, or lost its connection), assume its command may still be running there: delete that sandbox rather than reusing
it, since an agent left running with permission checks off would otherwise keep writing into the next unit's work.

Files cross with `boat scp`: `boat scp -r LOCAL_DIR ID:/home/user/PATH` sends a tree, and `boat scp ID:PATH LOCAL`
brings one file back, subject to the bounded-return rule in `using-workers.md`. `boat scp` uses its own key pair, which
it creates locally on first use (`~/.ssh/ascii_box_ed25519`, with host keys in `~/.ssh/ascii_sandbox_known_hosts/`); it
does not use the user's SSH keys or agent, and nothing here should add agent forwarding to it. `boat push ID DIR --to
PATH` also sends a directory, but it copies every file git does not ignore, including untracked ones, so prefer `boat
scp` of an explicitly selected tree or a tarball built outside the checkout.

Stay off the interactive surfaces: no `boat ssh` sessions, `boat forward`, `boat host`, desktop, or `boat prompt`. Those
are for people, and `boat prompt` would run an agent with boat's own credential plumbing rather than the one this skill
transfers.

## Sandbox lifecycle and cost

- **Reuse before creating.** Every create is a start, and starts are the scarce resource. When the orchestrating
  workflow has another unit of work coming and an owned sandbox of a suitable type is free, run it there after clearing
  the previous unit's work directory and credential, instead of deleting and creating. A default sandbox idling for an
  hour costs less than a single paid start. A sandbox whose last exec ended without its `exit` frame is never reused
  (see "Running work and transfer").
- **Never stop and resume.** A resume is also a start, and owned sandboxes have snapshots off, so a stop erases them
  anyway. Use delete, not stop.
- **Delete when the work is done.** Delete with `boat delete ID --yes` as soon as the orchestrating workflow has no
  further work for that sandbox, and delete every owned sandbox before this session ends or the workflow finishes,
  including after failures and aborts. The TTL is a backstop for a session that dies before cleanup, not the cleanup
  plan. Afterwards, check `boat list --all --json` for any remaining sandbox carrying the prefix that this session
  recorded as its own. If cleanup itself fails, report the exact names and IDs so the user can remove them.
- **Stay within the budget.** The count of owned sandboxes never exceeds the registered budget, and a create happens
  only when its start is free (see "Creating an owned sandbox").

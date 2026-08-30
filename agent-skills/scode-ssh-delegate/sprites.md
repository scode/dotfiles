# Sprite workers

Read this file when an invocation of scode-ssh-delegate names sprites or grants a sprite budget, and again before the
first use of a sprite worker after compaction or resume. It holds everything sprite-specific: how sprites are
registered, what the Fly.io image looks like, how a sprite is bootstrapped, and the lifecycle rules that keep them
cheap. The trust rules shared with SSH hosts are in `using-workers.md`.

Sprites are Fly.io sandboxes (https://fly.io/sprites/) driven through the `sprite` CLI. They bill compute only while
active and pause on their own shortly after, which is what makes them suitable for bursts of parallel work and what
makes most "leave it running" habits wrong here; see the lifecycle section at the end before running anything on one.

## Register sprites

Sprites are recognised by two invocation forms, and the two are deliberately different in what they let this session do
with existing sprites:

- **Named sprites** — `$scode-ssh-delegate sprites foo1 foo2`, or natural language such as "use the sprites foo1 foo2".
  These are _borrowed_: the user already owns them, and this session only runs work on them. Never destroy, checkpoint,
  or restore a borrowed sprite, and leave its filesystem as found apart from the disposable work directories that
  `using-workers.md` has every task use.
- **A sprite budget** — `$scode-ssh-delegate sprites:3`, or natural language such as "it's okay to use up to 3 sprites"
  or "use up to N sprites". This registers a number, not workers. Nothing is created at invocation time. When another
  workflow later needs a worker, this session creates a sprite, uses it, and destroys it; at most N such sprites exist
  at once. A budget never touches sprites this session did not create, so existing sprites are neither reused nor
  destroyed under a budget, even if their names look related.

Both forms can coexist with SSH hosts, and with each other, in one invocation.

Sprite names beginning with `-` are rejected. Sprite work needs the `sprite` CLI on `PATH` and a configured org
(`sprite org list` shows one); if either is missing, report that and register nothing sprite-related.

### Registering named sprites

Confirm each name appears in `sprite list`, then collect the same inventory as for SSH hosts with one command per
sprite:

```sh
sprite exec -s NAME -- sh -c 'hostname; grep -E "^(ID|VERSION_ID)=" /etc/os-release; nproc; free -m'
```

Apply the same acceptance rules (`ID=ubuntu`, `VERSION_ID=26.04`, at least 4 GiB). A sprite that is not listed, or whose
exec fails, is rejected and reported individually. Record the accepted sprite in the worker pool with the transport
prefix `sprite exec -s NAME --` and the kind "borrowed".

NOTE: the first exec against a paused sprite includes its wake-up (under a second for a warm sprite, a couple of seconds
for a cold one). That is normal and not a reason to reject the sprite.

### Registering a budget

Record the budget N and the naming prefix for sprites this session will own. The prefix is `ssh-delegate-<cwd>-` where
`<cwd>` is the basename of the current working directory lowercased, with every character outside `[a-z0-9]` replaced by
`-` and runs of `-` collapsed. Owned sprites are then named `<prefix><n>` with `n` starting at 1.

At registration time, run `sprite list --prefix <prefix>`. Any sprite already carrying the prefix is a possible orphan
from an earlier session that did not get to clean up. Report those names to the user as possible orphans and move on: do
not destroy them (they may be in use by another session in the same directory), do not reuse them, and do not stop to
ask. When choosing names for new sprites, skip any `n` that is already taken.

## Sprite inventory facts

The details below were observed on a fresh sprite in August 2026 and are the reason the sprite bootstrap differs from
the SSH one. Verify rather than assume; the base image is under Fly.io's control and changes without notice.

- The OS is Ubuntu 26.04 LTS, but it is a custom image, not a stock install. Each sprite has 8 vCPUs; memory autoscales
  (8 to 15 GiB total was observed) and is not a fixed number to design around; the writable overlay is about 100 GB.
- The Unix user is `sprite` (uid 1001) with passwordless `sudo`.
- `PATH` for a non-TTY exec is `~/.local/bin:/.sprite/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin`.
  `~/.local/bin` holds preinstalled `claude`, `codex`, `gemini`, and `cursor-agent`. `/.sprite/bin` holds shims for
  `cargo`, `rustup`, `node`, `npm`, `bun`, `deno`, `python3`, `pip`, `poetry`, `go`, `java`, `ruby`, `elixir`, `gh`, and
  others, pointing at root-owned toolchains under `/.sprite/languages`.
- The preinstalled agent CLIs and language toolchains lag. On the image observed, `claude` was 2.1.233 against a current
  2.1.251, `codex` 0.147.0 against 0.151.0, and `cargo` 1.90.0 (a July 2025 build) against a current 1.98.0. The Rust
  case is a pinned default rather than a missing toolchain: `rustup show` lists both `stable` and `1.90.0`, with
  `1.90.0` as the default, and `rustup update stable` alone leaves `cargo` at 1.90.0.
- There is no SSH server. `sprite exec` and `sprite file push`/`pull` are the only transport; `rsync` over SSH does not
  apply.

## Bootstrapping a sprite

An accepted sprite is either a fresh sprite from the Fly.io base image or a borrowed sprite prepared during an earlier
use of this skill. Do not assume the preinstalled tools are current, and do not skip bootstrap because a tool is on
`PATH`; the image ships stale versions (see the inventory facts above). Bootstrap only once another workflow selects the
sprite, and only what that work needs. Every command in the bootstrap runs through the sprite's exec prefix and must
finish inside that single exec (see the lifecycle rules below).

Bootstrap consists of the base apt packages from the "Base packages (both kinds)" section of `using-workers.md`,
followed by:

- `claude update`, then verify with `claude --version`. This upgrades the preinstalled copy in `~/.local/bin` in place;
  it was observed to take the image's 2.1.233 to the then-current 2.1.251.
- Homebrew, then `brew install codex`. Download `https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh`
  completely over HTTPS and run it with `NONINTERACTIVE=1 bash`; it installs to `/home/linuxbrew/.linuxbrew`. The
  preinstalled `~/.local/bin/codex` still shadows the brew one on the default `PATH`, so every later exec that runs
  Codex must put `/home/linuxbrew/.linuxbrew/bin` first: `PATH=/home/linuxbrew/.linuxbrew/bin:$PATH`. Verify with
  `codex --version` under that `PATH`.
- Whatever language toolchain the task needs, brought to a current version rather than trusted as shipped. For Rust,
  `rustup update stable && rustup default stable` through the preinstalled shim is enough: the shim's rustup home is
  writable by the `sprite` user despite being root-owned, and on a fresh sprite this moved `cargo` from 1.90.0 to
  1.98.0. If that does not produce the version the task needs, fall back to the standard `rustup` installer
  (`https://sh.rustup.rs`, run with `sh -s -- -y --no-modify-path`) into `~/.rustup`/`~/.cargo` with `~/.cargo/bin`
  first on `PATH`; that path was also verified to yield a current toolchain. Either way, check `cargo --version` before
  building. Treat other preinstalled toolchains the same way: check the version the task needs, try the shim's own
  version manager first, and install a user-owned copy if that is not enough.

The Homebrew installer and the vendor installers above are upstream trust boundaries, exactly as on SSH hosts.

### Credentials on a sprite

The credential policy in `using-workers.md` applies unchanged; the mechanics differ. Pass the Claude token per exec with
`--env CLAUDE_CODE_OAUTH_TOKEN=...` rather than persisting it, and push `auth.json` with `sprite file push -p` to
`~/.codex/auth.json` followed by `chmod 600`. An owned sprite is destroyed with its credential when the work ends; on a
borrowed sprite, remove the pushed `auth.json` when the work ends unless it was already there before this session. A
sprite ships `gh` on `PATH`; that does not change the rule that the worker has no GitHub credentials.

### Transfer on a sprite

The transport for every command is `sprite exec -s NAME -- COMMAND`, with `--dir` for the working directory and `--env`
for per-run variables. Source goes in with `sprite file push -r -p SRC NAME:DEST` for a directory tree, or as a tarball
pushed and unpacked in the same exec that uses it. Results come back with `sprite file pull NAME:PATH LOCAL`, subject to
the bounded-return rule in `using-workers.md`.

## Sprite lifecycle and cost

Sprites bill compute only while active. Activity means an attached `sprite exec`, an open TTY session, or an open
connection to the sprite's URL; about 30 seconds after the last of those ends, the sprite pauses. Pausing stops billing
but also stops every process and discards RAM; only the filesystem survives. The rules below follow from that.

- **An exec is the unit of work, and detaching ends it.** Nothing outlives the exec that started it, so `nohup`,
  `setsid`, `&`, `sprite-services`, and any other "start it and come back later" pattern is wrong here. Put the whole
  unit of work (bootstrap steps, a build, an agent run) in one foreground `sprite exec` and keep it attached from the
  controlling side until it exits. If the controlling side's own tooling caps how long a command may run, run the
  `sprite exec` in the controlling side's background with output to a local file and wait on that locally; the sprite
  side stays a single attached exec either way.
- **Never open a console.** `sprite console` and `sprite exec --tty` create TTY sessions that keep the sprite active
  (and billed) after detach. Use non-TTY `sprite exec` only.
- **No idle time on the sprite side.** Do not poll, sleep, or wait on the sprite for something that happens elsewhere;
  do that locally and issue a fresh exec when there is work. The ~30-second pause window is what makes sprites cheap
  between bursts, and an exec that sits waiting defeats it.
- **Concurrency is the point.** Several execs may run at once, both across sprites and on the same sprite (8 vCPUs go a
  long way for small jobs). Prefer many short parallel execs to one long serial one.
- **Check for strays after every burst.** Run `sprite sessions ls -s NAME` after a batch completes and kill anything
  still listed with `sprite sessions kill`. A suspended non-TTY session from an exec that was interrupted locally can
  keep the sprite from pausing.
- **Owned sprites are destroyed, not left to pause.** Create with `sprite create --skip-console NAME`. Destroy with
  `sprite destroy --force NAME` as soon as that sprite's unit of work is done, and destroy every owned sprite that still
  exists before this session ends or the orchestrating workflow finishes, including after failures and aborts. If
  cleanup itself fails, report the exact names so the user can destroy them. The count of owned sprites never exceeds
  the registered budget.
- **Borrowed sprites are left as found.** No destroy, no checkpoint, no restore. Remove disposable work directories and
  any pushed credential when done; leave everything else, including bootstrap upgrades, in place.

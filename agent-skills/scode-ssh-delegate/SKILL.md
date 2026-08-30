---
name: scode-ssh-delegate
description: >
  Make Ubuntu 26.04 LTS hosts and Fly.io sprites available as remote workers. Use when the user invokes
  scode-ssh-delegate or $scode-ssh-delegate followed by one or more hostnames, including terse forms such as
  "$scode-ssh-delegate foo" or "$scode-ssh-delegate foo, bar, baz"; when it is followed by named sprites, such as
  "$scode-ssh-delegate sprites foo1 foo2" or "use the sprites foo1 foo2"; or when it grants a sprite budget, such as
  "$scode-ssh-delegate sprites:3" or "it's okay to use up to 3 sprites". Immediately identify each named host's or
  sprite's OS, CPU count, and memory, then retain the supported workers (or the sprite budget) for possible use by
  another workflow. This invocation does not request delegation.
---

# scode SSH Delegate

This skill declares remote capacity; it does not decide whether or what to delegate. Another active skill or the user's
instructions own that decision and all task decomposition. This skill only establishes which workers are available, the
environment they provide, and the trust boundaries that apply if another workflow uses them.

Two kinds of worker exist. SSH hosts are machines the user already runs and reaches over SSH. Sprites are Fly.io
sandboxes (https://fly.io/sprites/) driven through the `sprite` CLI, which the user either names (borrowed sprites) or
lets this session create up to a budget (owned sprites). Both kinds end up in the same worker pool with the same trust
rules; they differ in transport, bootstrap, and lifecycle, and each of those differences is called out below.

## Register SSH hosts

Treat values following the invocation that are not a sprite form (see below) as SSH targets. Commas are optional
separators, so `$scode-ssh-delegate foo` and `$scode-ssh-delegate foo, bar, baz` are complete requests. SSH uses the
current Unix username unless the user supplies another normal SSH target form. Do not require the user to say that the
hosts are available or repeat them in a later task.

Immediately connect to every supplied target and collect only:

- The reported hostname.
- `ID` and `VERSION_ID` from `/etc/os-release`.
- The logical CPU count from `nproc`.
- Total and available memory from `free`.

Use `BatchMode=yes` and `ConnectTimeout=10` and never weaken host-key verification: plain `ssh` keeps its normal
`known_hosts` checking, and `tailscale ssh` checks against the key advertised by the tailnet. Reject unreachable
targets, targets whose SSH argument begins with `-`, and every OS other than `ID=ubuntu` with `VERSION_ID=26.04`. Report
each failure independently. Both supported CLIs currently require at least 4 GiB of RAM, so reject hosts below that
total as well. Retain each successful target, together with the transport selected for it below, in the current
conversation's available worker pool.

### Transport selection

The user supplies a plain `[user@]host` target and never says which transport to use. Prefer `tailscale ssh` when it
applies, because it verifies the server's host key against the key the Tailscale coordination server advertises for that
node, so a fresh controlling machine with an empty `known_hosts` connects without a trust-on-first-use prompt and
without weakening `StrictHostKeyChecking`. Plain `ssh` remains the path for everything else.

Decide per target, in this order:

- If `tailscale` is on `PATH` and `tailscale ip HOST` succeeds for the host part of the target, the host is a tailnet
  peer. A nonzero exit for any reason, including a stopped or logged-out daemon, means "not a peer" for this purpose.
  Attempt `tailscale ssh TARGET -o BatchMode=yes -o ConnectTimeout=10 'COMMAND'` first. The options must follow the
  target: everything after the target is passed to the system `ssh`, and options placed before it make the wrapper print
  its usage instead of connecting.
- If that attempt fails because no host key is known (stderr says `No ... host key is known for HOST` and then
  `Host key verification failed`, where HOST is the MagicDNS name rather than what the user typed), the peer is on the
  tailnet but is not running Tailscale SSH. Retry once with plain
  `ssh -o BatchMode=yes -o ConnectTimeout=10 TARGET 'COMMAND'` and its normal `known_hosts` checking. If that retry also
  fails host-key verification (the key is not in `known_hosts` and `BatchMode` forbids the prompt), reject the target
  and tell the user to connect once manually; do not add the key on their behalf.
- A changed-key warning (`REMOTE HOST IDENTIFICATION HAS CHANGED`) is a security signal, not a missing Tailscale SSH
  server. Reject the target, report the warning verbatim, and do not retry over plain `ssh`. Likewise do not retry plain
  `ssh` after a timeout or authentication failure from `tailscale ssh`; that failure is the result.
- If `tailscale` is absent or the host is not a peer, use plain `ssh` directly.

NOTE: being a tailnet peer does not mean the node runs Tailscale SSH, and there is no cheap way to tell beforehand.
`tailscale status --json` and `tailscale whois` do not expose advertised SSH host keys, and `tailscale debug netmap`
needs root or operator rights. The attempt itself is the check.

Record the transport that succeeded for each accepted worker as the exact command prefix to reuse: either
`tailscale ssh TARGET -o BatchMode=yes` or `ssh -o BatchMode=yes TARGET`. Every later command and transfer to that
worker uses that prefix; for rsync that means `-e 'tailscale ssh'` or `-e 'ssh -o BatchMode=yes'`, which both work
unchanged because the wrapper passes the remote command through to `ssh`. Transport selection happens only during
registration (and again on a later reprobe of the same target); do not re-run it on each use.

Do not inspect commands, packages, Codex or Claude authentication, project dependencies, or other machine state during
this initial inventory. Do not bootstrap a host merely because the user declared it. A later invocation adds successful
targets; reprobing a target refreshes its inventory, and a failed reprobe removes it.

## Register sprites

Sprites are recognised by two invocation forms, and the two are deliberately different in what they let this session do
with existing sprites:

- **Named sprites** — `$scode-ssh-delegate sprites foo1 foo2`, or natural language such as "use the sprites foo1 foo2".
  These are _borrowed_: the user already owns them, and this session only runs work on them. Never destroy, checkpoint,
  or restore a borrowed sprite, and leave its filesystem as found apart from the disposable work directories described
  later.
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

### Sprite inventory facts

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

## Worker contract

### Ubuntu SSH hosts

An accepted SSH worker is either a fresh base Ubuntu 26.04 LTS installation or a machine prepared during an earlier use
of this skill. No other Ubuntu release or distribution is supported.

The base packages needed for the supported worker environment are:

```sh
sudo -n apt-get update
sudo -n env DEBIAN_FRONTEND=noninteractive apt-get install -y \
    build-essential ca-certificates curl git rsync
```

Only after another workflow selects a host, install whichever missing CLIs that work requires. Confirm the worker's
architecture meets the selected CLI's current requirements. The Codex installer is
`https://chatgpt.com/codex/install.sh`; run the downloaded script with `CODEX_NON_INTERACTIVE=1 sh`. The Claude Code
installer is `https://claude.ai/install.sh`; run it with Bash, not `sh`. Require HTTPS for the initial URL and
redirects, download each installer completely before executing it, and verify each installed command. The
vendor-controlled installers are an explicit upstream trust boundary.

Both native installers place their command in `~/.local/bin`. Add that directory to `PATH` explicitly in non-login SSH
commands before deciding that a CLI is absent.

### Sprites

An accepted sprite is either a fresh sprite from the Fly.io base image or a borrowed sprite prepared during an earlier
use of this skill. Do not assume the preinstalled tools are current, and do not skip bootstrap because a tool is on
`PATH`; the image ships stale versions (see the inventory facts above). Bootstrap only once another workflow selects the
sprite, and only what that work needs. Every command in the bootstrap runs through the sprite's exec prefix and must
finish inside that single exec (see the lifecycle rules below).

Bootstrap consists of the same base apt packages as for SSH hosts, followed by:

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

### Credentials (both kinds)

Only when another workflow has selected a worker for actual use, inspect whether its required CLI and authentication are
ready. For Codex, copying `~/.codex/auth.json` to the same path on a headless worker is an officially supported fallback
when the controlling host actually uses file-backed credentials. Otherwise use Codex device authentication or another
supported login path. For Claude Code automation with a Pro, Max, Team, or Enterprise subscription, prefer a token
produced by `claude setup-token` and pass it as `CLAUDE_CODE_OAUTH_TOKEN`; do not assume its local credential cache is
portable. A Console user may instead pass `ANTHROPIC_API_KEY` to the selected Claude invocation. These are the only
agent credentials this skill permits transferring.

On a sprite, pass the Claude token per exec with `--env CLAUDE_CODE_OAUTH_TOKEN=...` rather than persisting it, and push
`auth.json` with `sprite file push -p` to `~/.codex/auth.json` followed by `chmod 600`. An owned sprite is destroyed
with its credential when the work ends; on a borrowed sprite, remove the pushed `auth.json` when the work ends unless it
was already there before this session.

Announce the transfer because these are reusable secrets. Copy only the credential needed by the selected provider, keep
any persisted secret readable only by the remote user, and verify authentication. Do not overwrite an existing
credential merely because an authentication check returned an unexpected error.

Apart from an agent credential intentionally installed as above, treat the worker as having no GitHub, cloud, or
package-registry credentials and no access to private repository remotes. A sprite ships `gh` on `PATH`; that does not
change this. Never forward the SSH agent or copy other home-directory state. Project-specific public prerequisites may
be installed when the selected task needs them; never add private registry configuration.

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

## Using an accepted worker

The orchestrating workflow chooses the host, provider, exact local source state, transfer mechanism, remote working
directory, prompt, commands, and result format. Whatever mechanism it chooses runs over the transport recorded for that
worker during registration; for a sprite that is `sprite exec -s NAME -- COMMAND`, with `--dir` for the working
directory and `--env` for per-run variables. A user-specified commit, revision, or working-copy snapshot remains
binding. Resolve private refs locally and treat the local checkout as the source of truth; the remote worker must not
contact a private repository remote to clone, fetch, pull, or push.

If the orchestrating workflow chooses unrestricted execution, the proven entrypoints are `codex exec --yolo` and
`claude -p --dangerously-skip-permissions`. These modes give the agent full access to the remote Unix account. They are
worker capabilities, not a request from this skill to run either agent. Use a disposable remote directory, and remember
that the remote agent has none of the controlling session's conversation.

Choose what to transfer for the task at hand. Over SSH that is normally `rsync`; to a sprite it is
`sprite file push -r -p SRC NAME:DEST` for a directory tree (or a tarball pushed and unpacked in the same exec that uses
it). Do not transfer credentials as source material; if that conflicts with a requested working-copy snapshot, stop for
direction. Do not blindly copy repository metadata, ignored files, or unrelated local state.

In the return direction, treat command output as returned data and keep it bounded. Do not mirror or reverse-rsync the
remote workspace. Retrieve only a small, explicitly selected set of patches, changed files, reports, or logs into local
staging outside the checkout (`sprite file pull NAME:PATH LOCAL` on a sprite), then inspect them before applying
anything. Remove disposable source and prompts from the worker when they are no longer needed.

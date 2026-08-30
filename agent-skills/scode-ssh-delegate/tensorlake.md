# Tensorlake sandbox workers

Read this file when an invocation of scode-ssh-delegate names tensorlake sandboxes or grants a sandbox budget, and again
before the first use of a tensorlake worker after compaction or resume. It holds everything tensorlake-specific: how
sandboxes are registered, what the base image looks like, how a sandbox is bootstrapped, and the lifecycle rules that
keep them cheap. The trust rules shared with SSH hosts and sprites are in `using-workers.md`.

Tensorlake sandboxes (https://docs.tensorlake.ai/sandboxes/quickstart) are microVMs driven through the `tl` CLI. They
bill compute per second while running and suspend on their own after an idle timeout chosen at creation, which is what
makes them suitable for bursts of parallel work. Unlike sprites, the CPU/memory shape is chosen at creation rather than
fixed by the platform, and a sandbox can be checkpointed and cloned. See the lifecycle section at the end before running
anything on one.

## Register sandboxes

Sandboxes are recognised by two invocation forms, deliberately different in what they let this session do:

- **Named sandboxes** — `$scode-ssh-delegate sbx foo1 foo2`, or natural language such as "use the tensorlake sandboxes
  foo1 foo2". These are _borrowed_: the user already owns them, and this session only runs work on them. Never
  terminate, explicitly suspend, or checkpoint a borrowed sandbox (idle suspend happens on its own and is fine), and
  leave its filesystem as found apart from the disposable work directories that `using-workers.md` has every task use
  and the bootstrap installs the selected work required.
- **A sandbox budget** — `$scode-ssh-delegate sbx:3`, or natural language such as "it's okay to use up to 3 tensorlake
  sandboxes". This registers a number, not workers. Nothing is created at invocation time. When another workflow later
  needs a worker, this session creates a sandbox, uses it, and terminates it; at most N such sandboxes exist at once. A
  budget never touches sandboxes this session did not create, so existing sandboxes are neither reused nor terminated
  under a budget, even if their names look related.

Both forms can coexist with SSH hosts and sprites in one invocation.

Sandbox names beginning with `-` are rejected. Tensorlake work needs the `tl` CLI on `PATH` and working authentication
(`tl whoami` prints an organization and project); if either is missing, report that and register nothing
tensorlake-related.

### Registering named sandboxes

Confirm each name appears in `tl sbx ls`, then collect the same inventory as for SSH hosts with one command per sandbox:

```sh
tl sbx exec NAME sh -c 'hostname; grep -E "^(ID|VERSION_ID)=" /etc/os-release; nproc; free -m'
```

The acceptance rules are the tensorlake variants: `ID=ubuntu` with `VERSION_ID=24.04` (the stock image, not 26.04), and
at least 4 GiB of RAM because that is what the agent CLIs need. A sandbox that is not listed, or whose exec fails, is
rejected and reported individually. Record the accepted sandbox in the worker pool with the transport
`tl sbx exec [OPTIONS] NAME COMMAND` and the kind "borrowed". The options position matters: `-w`, `-e`, and `-t` must
come between `exec` and the name, because everything after the name is the remote command — this is not a prefix that
options can be appended to.

NOTE: an exec against a suspended sandbox transparently resumes it (about a second was observed). That is normal and not
a reason to reject the sandbox.

### Registering a budget

Record the budget N and the naming prefix for sandboxes this session will own. The prefix is the same
`ssh-delegate-<cwd>-` rule as for sprites: `<cwd>` is the basename of the current working directory lowercased, with
every character outside `[a-z0-9]` replaced by `-` and runs of `-` collapsed. Owned sandboxes are then named
`<prefix><n>` with `n` starting at 1. This mangling produces tensorlake's own name alphabet (lowercase letters, digits,
hyphens, starting with a letter); truncate the `<cwd>` component if needed so the full name stays within tensorlake's
63-character limit.

At registration time, run `tl sbx ls` and look for the prefix. Any sandbox already carrying it is a possible orphan from
an earlier session that did not get to clean up. Report those names to the user as possible orphans and move on: do not
terminate them (they may be in use by another session in the same directory), do not reuse them, and do not stop to ask.
Also run `tl sbx checkpoint ls`: a snapshot whose Sandbox ID column matches the ID of a prefixed sandbox from the
listing is reported the same way, because snapshots bill storage monthly and survive their sandbox. A snapshot whose
source sandbox no longer exists cannot be attributed to this skill from the listing alone; leave those alone entirely —
never delete a snapshot you cannot attribute. When choosing names for new sandboxes, skip any `n` that is already taken.

## Sandbox inventory facts

The details below were observed on fresh sandboxes in August 2026 and are the reason the tensorlake bootstrap differs
from the sprite one. Verify rather than assume; the platform is under Tensorlake's control and changes without notice.

- The default image is `tensorlake/ubuntu-minimal`: Ubuntu 24.04 LTS, no systemd. The Unix user is `tl-user` (uid 1000)
  with passwordless `sudo`. Preinstalled: curl, git, python3 (3.12), node (24). No agent CLIs and no version-manager
  shims were found — unlike the sprite image there is nothing stale to distrust, just nothing there.
- The CPU/memory/disk shape is chosen at creation and gated by the account's plan, not discovered. The free plan caps at
  1 vCPU / 1 GiB / 1 concurrent sandbox, which is below what agent work needs; the paid credits plan allows up to 4 vCPU
  / 16 GiB / 100 GiB disk and 100 concurrent sandboxes, with RAM limited to 1x-8x the vCPU count. A plan-limit error
  from `tl sbx create` names the exceeded cap; report it rather than retrying smaller, because a shape below the skill's
  4 GiB minimum for agent work is not worth creating.
- The idle timeout is set at creation (`--timeout`, in seconds) and **cannot be changed afterwards** — not on resume,
  and `tl sbx update` only touches network configuration. It is an idle threshold, not a lifetime: an attached exec
  counts as activity for its entire duration, and the clock only runs between execs.
- `tl sbx exec` propagates exit codes and supports `-w` (workdir), `-e` (env), and `-t` (per-exec timeout). It does
  **not** forward stdin (verified: zero bytes arrive).
- `tl sbx cp` copies single files in either direction. Directories fail with a misleading "file not found" error, so
  trees travel as tarballs (see transfer below).
- Billing is per second while running (about $0.40/hour was observed for the default shape below on credits pricing)
  and drops to snapshot storage only ($0.07/GiB-month at the full disk allocation) while suspended.

## Creating an owned sandbox

When another workflow selects a worker under a budget, create it as:

```sh
tl sbx create --timeout 60 -c 4 -m 8192 NAME
```

The command prints the new sandbox's ID; record it alongside the name for every owned sandbox (and record every template
snapshot ID the same way). That ledger is what makes the snapshot sweep in the lifecycle section safe:
`tl sbx checkpoint ls` identifies a snapshot's source only by sandbox ID, and once a sandbox is terminated its
name-to-ID mapping is gone from every listing.

The shape is fixed at 4 vCPUs and 8 GiB for now; the orchestrating workflow does not choose per-task shapes. The 60
second idle timeout is deliberate: long enough that local think-time between exec bursts does not thrash suspend/resume
cycles, short enough that an abandoned sandbox stops billing compute within a minute. Because the sandbox is named,
hitting the timeout suspends it rather than destroying it, and the next exec resumes it transparently — bootstrap state
survives idle gaps. Do not create unnamed (ephemeral) sandboxes: those are destroyed at the idle timeout, taking any
bootstrap with them.

## Bootstrapping a sandbox

An accepted sandbox is either a fresh instance of the stock image or a borrowed sandbox prepared during an earlier use
of this skill. Bootstrap only once another workflow selects the sandbox, and only what that work needs. Bootstrap
consists of the base apt packages from the "Base packages (all kinds)" section of `using-workers.md`, followed by the
vendor installers exactly as on SSH hosts (see "Worker contract for Ubuntu SSH hosts" there): the image ships no agent
CLIs, so the Claude Code and Codex installers run from scratch and land in `~/.local/bin`. Both were verified to install
and run on the 8 GiB shape in August 2026. The same upstream trust boundary rules apply.

### Snapshot templates: bootstrap once, clone many

Tensorlake can checkpoint a sandbox's filesystem and create new sandboxes from the checkpoint, which sprites cannot.
When a burst needs several similar workers, this is worth the extra bookkeeping: bootstrap one owned sandbox, then

```sh
tl sbx checkpoint NAME          # prints a snapshot ID; completes in seconds
tl sbx create -s SNAP_ID --timeout 60 NAME-2
```

Clones inherit the template's shape (and can override it with `-c`/`-m`), come up in a couple of seconds with the
bootstrapped filesystem intact, and count against the budget N exactly like sandboxes created from scratch. The snapshot
itself is owned state: delete it with `tl sbx checkpoint rm SNAP_ID` as soon as the burst is done. This is a judgment
call for the orchestrating workflow — for one or two workers the plain bootstrap is simpler; for many, the snapshot
amortizes the 1-2 minute bootstrap into seconds per clone. Never checkpoint a borrowed sandbox.

### Credentials on a sandbox

The credential policy in `using-workers.md` applies unchanged; the mechanics differ. Pass the Claude token per exec with
`-e`, expanding it from a local variable (`-e CLAUDE_CODE_OAUTH_TOKEN="$CLAUDE_CODE_OAUTH_TOKEN"`) rather than pasting
the literal into the command, so the secret does not land in command text or transcripts. For Codex, first create the
directory with an exec running `install -d -m 700 /home/tl-user/.codex`, then push with
`tl sbx cp ~/.codex/auth.json NAME:/home/tl-user/.codex/auth.json` and `chmod 600` it in a follow-up exec. An owned
sandbox is terminated with its credential when the work ends; on a borrowed sandbox, remove the pushed `auth.json` when
the work ends unless it was already there before this session.

NOTE: a named sandbox that idles while a pushed credential file is on disk gets that file captured in the platform's
suspend snapshot, and on a borrowed sandbox this skill has no permitted way to remove that snapshot. Prefer the per-exec
token form on borrowed sandboxes, and push a credential file to one only when the selected provider offers no
alternative — announcing, as always, what is being transferred.

### Transfer on a sandbox

The transport for every command is `tl sbx exec [OPTIONS] NAME COMMAND`, with `-w` for the working directory and `-e`
for per-run variables, both placed before the name (anything after the name is the remote command). Because `tl sbx cp`
only moves single files and exec forwards no stdin, a source tree goes in as a tarball: `tar czf` locally, written
outside the checkout so the archive cannot include itself, then `tl sbx cp` the tarball and `tar xzf` in the exec that
uses it. Results come back with `tl sbx cp NAME:PATH LOCAL` one file at a time, subject to the bounded-return rule in
`using-workers.md`.

## Sandbox lifecycle and cost

A running sandbox bills compute whether or not anything executes on it; the idle-suspend timeout is what caps the waste,
and the rules below exist to keep that mechanism working.

- **An attached exec is the unit of work.** Suspension freezes every process, so a background job started with `nohup`,
  `&`, or `--detach` is frozen mid-flight at the first idle gap and only limps forward while later execs happen to keep
  the sandbox awake. Put the whole unit of work in one foreground `tl sbx exec` and stay attached from the controlling
  side until it exits. If the controlling side's own tooling caps how long a command may run, run the exec in the
  controlling side's background with output to a local file and wait on that locally.
- **No idle time on the sandbox side.** Do not poll, sleep, or wait on the sandbox for something that happens elsewhere;
  do that locally and issue a fresh exec when there is work. Suspend costs nearly nothing and resume takes about a
  second, so there is no reason to keep a sandbox artificially awake.
- **Stay off the interactive surfaces.** Tensorlake documents SSH sessions, PTY connections, and open ports as activity,
  so `tl sbx ssh`, PTY sessions, and tunnels hold the sandbox running (and billed) as long as they are open.
  Non-interactive `tl sbx exec` is the only transport this skill uses.
- **Concurrency is the point.** Several execs may run at once, both across sandboxes and on the same sandbox. Prefer
  many short parallel execs to one long serial one.
- **Owned sandboxes are terminated, and their snapshots swept.** Terminate with `tl sbx terminate NAME` as soon as that
  sandbox's unit of work is done, and terminate every owned sandbox that still exists before this session ends or the
  orchestrating workflow finishes, including after failures and aborts. Then run `tl sbx checkpoint ls` and
  `tl sbx checkpoint rm SNAPSHOT_ID` for every snapshot whose Sandbox ID matches the recorded ID of an owned sandbox,
  plus every recorded template snapshot — both kinds, because the platform also creates suspend snapshots on idle and
  **snapshots survive termination** and bill storage monthly (a leftover 10 GiB suspend snapshot was observed after
  terminating its sandbox). Match against the recorded IDs only; never delete a snapshot whose sandbox ID is not in this
  session's ledger. If cleanup itself fails, report the exact sandbox names and snapshot IDs so the user can remove
  them. The count of owned sandboxes never exceeds the registered budget.
- **Borrowed sandboxes are left as found.** No terminate, no explicit suspend, no checkpoint. Remove disposable work
  directories and any pushed credential when done; leave everything else, including bootstrap installs, in place.

# scode-ssh-delegate

An agent skill that registers remote machines as workers so that some other workflow in the same session (such as
scode-galaxy-brain) can run work on them instead of on the local machine. Invoking it only declares capacity; it does
not by itself delegate anything.

NOTE: This is not a general remote-execution tool. It supports exactly three kinds of worker, and all are treated as
disposable: the agent may install tools, copy source, and run agents on them with permission checks disabled. Do not
point it at machines where that is not acceptable.

## SSH hosts

Give it one or more SSH targets:

```text
$scode-ssh-delegate foo
$scode-ssh-delegate foo, bar, alice@baz
```

The agent connects to each one right away, records its OS, CPU count, and memory, and keeps the ones that pass: Ubuntu
26.04 LTS with at least 4 GiB of RAM. Anything else is rejected with the reason. Hosts on your tailnet are reached with
`tailscale ssh` when they run Tailscale SSH and plain `ssh` otherwise; host-key checking is never weakened, so a host
you have never connected to before over plain `ssh` needs one manual connection first.

Nothing is installed at registration. When a workflow later picks a host, the agent installs what that task needs (base
build tools, then Claude Code or Codex if an agent is going to run there) and, with an announcement, copies over the one
agent credential the task needs. It never forwards your SSH agent, never copies other home-directory state, and treats
the host as having no access to private repositories: source is pushed from the local checkout, results are pulled back
as a small explicit set of files.

## Sprites

[Fly.io sprites](https://fly.io/sprites/) are on-demand Ubuntu sandboxes driven by the `sprite` CLI. They cost compute
only while a command is running on them and pause on their own about 30 seconds after the last one finishes, so they
suit short bursts of parallel work. The `sprite` CLI has to be installed and logged in (`sprite org list` shows an org).

There are two ways to offer them, and they behave differently:

```text
$scode-ssh-delegate sprites foo1 foo2      # or: use the sprites foo1 foo2
$scode-ssh-delegate sprites:3              # or: it's okay to use up to 3 sprites
```

Named sprites are borrowed. They must already exist; the agent runs work on them and leaves them alone otherwise: no
destroy, no checkpoint or restore, and any pushed credential or work directory is removed afterwards.

A budget is a count, not a list. Nothing is created when you grant it. When a workflow needs a worker, the agent creates
a sprite named `ssh-delegate-<directory>-N`, uses it, and destroys it as soon as that work is done; at most N exist at
once, and all of them are gone by the time the session finishes. A budget never touches sprites the session did not
create. If sprites with that prefix already exist from an earlier session, the agent tells you about them as possible
orphans and neither reuses nor destroys them.

Sprites come with `claude`, `codex`, and common language toolchains preinstalled, but the versions lag. The agent
updates whichever ones the task needs before using them. Everything about credentials, source transfer, and results is
the same as for SSH hosts.

Both forms can be combined with each other and with SSH hosts in one invocation.

## Tensorlake sandboxes

[Tensorlake sandboxes](https://docs.tensorlake.ai/sandboxes/quickstart) are microVMs driven by the `tl` CLI. They bill
compute per second while running and suspend on their own after an idle timeout, so they suit the same bursty work as
sprites; what they add is a chosen CPU/memory shape per sandbox and filesystem snapshots that can be cloned. The `tl`
CLI has to be installed and authenticated (`tl whoami` shows an organization).

The two invocation forms mirror the sprite ones and behave the same way:

```text
$scode-ssh-delegate sbx foo1 foo2      # or: use the tensorlake sandboxes foo1 foo2
$scode-ssh-delegate sbx:3              # or: it's okay to use up to 3 tensorlake sandboxes
```

Named sandboxes are borrowed; a budget is a count of sandboxes the agent creates (4 vCPUs, 8 GiB, 60 second idle
timeout), uses, and terminates, with the same orphan reporting as sprites. Unlike sprites, the stock image is Ubuntu
24.04 with no agent CLIs preinstalled, so the agent installs them fresh; for a burst of several similar workers it may
bootstrap one sandbox, checkpoint it, and clone the rest from the snapshot, deleting the snapshot when the burst is
done. Snapshots outlive their sandbox and bill storage monthly, so cleanup includes sweeping them.

## What the agent will not do

- Delegate on its own. Some other workflow or instruction has to decide to use the workers.
- Keep anything running on a sprite or sandbox between commands. Both pause or suspend when nothing is attached, which
  stops or freezes every process, so long-lived servers and background jobs do not work there; the agent is told not to
  try.
- Open interactive sessions on sprites or sandboxes, since those keep the instance billed after the agent has moved on.
- Clone from, fetch from, or push to private repository remotes from a worker.

The full rules the agent follows are in [SKILL.md](SKILL.md), which is what an agent loads up front; the sprite rules in
[sprites.md](sprites.md), the tensorlake rules in [tensorlake.md](tensorlake.md), and the use-time rules in
[using-workers.md](using-workers.md) are read only when a session actually registers such a worker or hands work to one.

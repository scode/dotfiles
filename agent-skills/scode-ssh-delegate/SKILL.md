---
name: scode-ssh-delegate
description: >
  Make Ubuntu 26.04 LTS hosts, Fly.io sprites, and Tensorlake sandboxes available as remote workers. Use when the user
  invokes scode-ssh-delegate or $scode-ssh-delegate followed by one or more hostnames, including terse forms such as
  "$scode-ssh-delegate foo" or "$scode-ssh-delegate foo, bar, baz"; when it is followed by named sprites or sandboxes,
  such as "$scode-ssh-delegate sprites foo1 foo2", "$scode-ssh-delegate sbx foo1 foo2", or "use the sprites foo1
  foo2"; or when it grants a sprite or sandbox budget, such as "$scode-ssh-delegate sprites:3", "$scode-ssh-delegate
  sbx:3", or "it's okay to use up to 3 sprites". Immediately identify each named host's, sprite's, or sandbox's OS,
  CPU count, and memory, then retain the supported workers (or the budgets) for possible use by another workflow. This
  invocation does not request delegation. If retained context after compaction or resume says workers or a budget were
  registered, re-read this skill and its companion files before touching them.
---

# scode SSH Delegate

This skill declares remote capacity; it does not decide whether or what to delegate. Another active skill or the user's
instructions own that decision and all task decomposition. This skill only establishes which workers are available, the
environment they provide, and the trust boundaries that apply if another workflow uses them.

Three kinds of worker exist. SSH hosts are machines the user already runs and reaches over SSH; registering them is
described in this file. Sprites are Fly.io sandboxes driven through the `sprite` CLI, and tensorlake sandboxes are
Tensorlake microVMs driven through the `tl` CLI; for both, the user either names existing instances or lets this session
create them up to a budget. All kinds end up in the same worker pool with the same trust rules.

This file is deliberately limited to what every invocation needs. Companion files next to it are read on demand so that
a session which never touches sprites or sandboxes, or never actually delegates, does not pay for that text:

- `sprites.md` — read it before classifying any argument whenever the invocation mentions sprites in any form: named
  ones (`sprites foo1 foo2`, "use the sprites ..."), a budget (`sprites:3`, "up to N sprites"), the singular, or a
  trailing remark such as "they're sprites". It covers sprite registration, the Fly.io image, bootstrap, and the
  lifecycle rules that keep sprites cheap. A sprite mention with neither names nor a number is not a registration; ask
  the user which they meant. Do not register or touch a sprite without having read the file.
- `tensorlake.md` — read it before classifying any argument whenever the invocation mentions tensorlake sandboxes in any
  form: named ones (`sbx foo1 foo2`, "use the tensorlake sandboxes ..."), a budget (`sbx:3`, "up to N tensorlake
  sandboxes"), or a trailing remark such as "they're tensorlake sandboxes". It covers sandbox registration, the stock
  image, bootstrap, snapshot templates, and the lifecycle rules that keep sandboxes cheap. The same
  neither-names-nor-a-number rule applies: ask rather than guess. Do not register or touch a tensorlake sandbox without
  having read the file.
- `using-workers.md` — read it before the first command, transfer, install, or credential touches any registered worker,
  whoever asked for that: another skill, the user's instructions, or a follow-up request in the same conversation. It
  covers what may be installed on a worker, which credentials may be transferred and how, and how source and results
  cross the boundary. The rsync transport recorded at registration is not permission to skip it.

After compaction or resume, if the retained context says a sprite or sandbox budget or such workers were registered,
re-read `sprites.md` or `tensorlake.md` before the next operation on them; if workers were in use, re-read
`using-workers.md` before the next delegation.

## Trust rules that always apply

The full rules are in `using-workers.md`, but these hold from the moment a worker is registered and are repeated here so
that no session can act on a worker without having seen them:

- Never forward the SSH agent or copy home-directory state to a worker. The only credentials that may move are one agent
  credential for the selected provider (`~/.codex/auth.json`, `CLAUDE_CODE_OAUTH_TOKEN`, or `ANTHROPIC_API_KEY`), and
  the transfer is announced.
- A worker has no GitHub, cloud, or registry credentials and must not clone, fetch, pull, or push a private remote.
  Source is pushed from the local checkout.
- Results come back as a small, explicitly selected set of files into local staging, never as a mirror of the remote
  workspace.

## Register SSH hosts

Treat values following the invocation that are not a sprite or tensorlake form as SSH targets. Commas are optional
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

## Sprites and tensorlake sandboxes, in one paragraph

Registration and everything else about sprites is in `sprites.md`, and about tensorlake sandboxes in `tensorlake.md`;
this is only so that an SSH-only reader knows what the forms mean. Named sprites and sandboxes are borrowed: the session
runs work on them and never destroys them. A budget (`sprites:N` or `sbx:N`) is a count of instances this session may
create, use, and destroy, and it never touches instances the session did not create. Nothing is installed, transferred,
or run on any worker at registration.

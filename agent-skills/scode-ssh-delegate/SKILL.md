---
name: scode-ssh-delegate
description: >
  Make Ubuntu 26.04 LTS hosts available as SSH workers. Use when the user invokes scode-ssh-delegate or
  $scode-ssh-delegate followed by one or more hostnames, including terse forms such as "$scode-ssh-delegate foo" or
  "$scode-ssh-delegate foo, bar, baz". Immediately identify each host's Ubuntu version, CPU count, and memory, then
  retain the supported hosts for possible use by another workflow. This invocation does not request delegation.
---

# scode SSH Delegate

This skill declares remote capacity; it does not decide whether or what to delegate. Another active skill or the user's
instructions own that decision and all task decomposition. This skill only establishes which workers are available, the
environment they provide, and the trust boundaries that apply if another workflow uses them.

## Register hosts

Treat the values following the invocation as SSH targets. Commas are optional separators, so `$scode-ssh-delegate foo`
and `$scode-ssh-delegate foo, bar, baz` are complete requests. SSH uses the current Unix username unless the user
supplies another normal SSH target form. Do not require the user to say that the hosts are available or repeat them in a
later task.

Immediately connect to every supplied target and collect only:

- The reported hostname.
- `ID` and `VERSION_ID` from `/etc/os-release`.
- The logical CPU count from `nproc`.
- Total and available memory from `free`.

Use `BatchMode=yes` and `ConnectTimeout=10` while preserving normal SSH host-key verification. Reject unreachable
targets, targets whose SSH argument begins with `-`, and every OS other than `ID=ubuntu` with `VERSION_ID=26.04`. Report
each failure independently. Both supported CLIs currently require at least 4 GiB of RAM, so reject hosts below that
total as well. Retain each successful target in the current conversation's available worker pool.

Do not inspect commands, packages, Codex or Claude authentication, project dependencies, or other machine state during
this initial inventory. Do not bootstrap a host merely because the user declared it. A later invocation adds successful
targets; reprobing a target refreshes its inventory, and a failed reprobe removes it.

## Worker contract

An accepted worker is either a fresh base Ubuntu 26.04 LTS installation or a machine prepared during an earlier use of
this skill. No other Ubuntu release or distribution is supported.

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

Only when another workflow has selected a worker for actual use, inspect whether its required CLI and authentication are
ready. For Codex, copying `~/.codex/auth.json` to the same path on a headless worker is an officially supported fallback
when the controlling host actually uses file-backed credentials. Otherwise use Codex device authentication or another
supported login path. For Claude Code automation with a Pro, Max, Team, or Enterprise subscription, prefer a token
produced by `claude setup-token` and pass it as `CLAUDE_CODE_OAUTH_TOKEN`; do not assume its local credential cache is
portable. A Console user may instead pass `ANTHROPIC_API_KEY` to the selected Claude invocation. These are the only
agent credentials this skill permits transferring.

Announce the transfer because these are reusable secrets. Copy only the credential needed by the selected provider, keep
any persisted secret readable only by the remote user, and verify authentication. Do not overwrite an existing
credential merely because an authentication check returned an unexpected error.

Apart from an agent credential intentionally installed as above, treat the worker as having no GitHub, cloud, or
package-registry credentials and no access to private repository remotes. Never forward the SSH agent or copy other
home-directory state. Project-specific public prerequisites may be installed when the selected task needs them; never
add private registry configuration.

## Using an accepted worker

The orchestrating workflow chooses the host, provider, exact local source state, transfer mechanism, remote working
directory, prompt, commands, and result format. A user-specified commit, revision, or working-copy snapshot remains
binding. Resolve private refs locally and treat the local checkout as the source of truth; the remote worker must not
contact a private repository remote to clone, fetch, pull, or push.

If the orchestrating workflow chooses unrestricted execution, the proven entrypoints are `codex exec --yolo` and
`claude -p --dangerously-skip-permissions`. These modes give the agent full access to the remote Unix account. They are
worker capabilities, not a request from this skill to run either agent. Use a disposable remote directory, and remember
that the remote agent has none of the controlling session's conversation.

Choose what to transfer for the task at hand. Do not transfer credentials as source material; if that conflicts with a
requested working-copy snapshot, stop for direction. Do not blindly copy repository metadata, ignored files, or
unrelated local state.

In the return direction, treat command output as returned data and keep it bounded. Do not mirror or reverse-rsync the
remote workspace. Retrieve only a small, explicitly selected set of patches, changed files, reports, or logs into local
staging outside the checkout, then inspect them before applying anything. Remove disposable source and prompts from the
worker when they are no longer needed.

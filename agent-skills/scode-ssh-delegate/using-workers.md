# Using an accepted worker

Read this file before the first command, transfer, install, or credential touches a registered worker, whoever asked for
that, and again before the first use after compaction or resume. Registration (in `SKILL.md`, `sprites.md`, and
`tensorlake.md`) only declares capacity; everything here is about what may happen on a worker once work is sent to it:
what gets installed, which credentials may move, and how source and results cross the boundary. It applies to SSH hosts,
sprites, and tensorlake sandboxes alike; the platform-specific mechanics (bootstrap, transport, lifecycle) are in
`sprites.md` and `tensorlake.md` and take precedence where they differ.

## Base packages (all kinds)

Every worker — SSH host, sprite, or tensorlake sandbox — gets these before anything else is installed on it:

```sh
sudo -n apt-get update
sudo -n env DEBIAN_FRONTEND=noninteractive apt-get install -y \
    build-essential ca-certificates curl git rsync
```

## Worker contract for Ubuntu SSH hosts

An accepted SSH worker is either a fresh base Ubuntu 26.04 LTS installation or a machine prepared during an earlier use
of this skill. No other Ubuntu release or distribution is supported.

Only after a host has been selected for a task, install whichever missing CLIs that work requires. Confirm the worker's
architecture meets the selected CLI's current requirements. The Codex installer is
`https://chatgpt.com/codex/install.sh`; run the downloaded script with `CODEX_NON_INTERACTIVE=1 sh`. The Claude Code
installer is `https://claude.ai/install.sh`; run it with Bash, not `sh`. Require HTTPS for the initial URL and
redirects, download each installer completely before executing it, and verify each installed command. The
vendor-controlled installers are an explicit upstream trust boundary.

Both native installers place their command in `~/.local/bin`. Add that directory to `PATH` explicitly in non-login SSH
commands before deciding that a CLI is absent.

Sprites use the same base packages but a different bootstrap, because their image ships the agent CLIs and toolchains
preinstalled and stale; see "Bootstrapping a sprite" in `sprites.md`. Tensorlake sandboxes use the same base packages
and the same vendor installers as SSH hosts (their image ships no agent CLIs at all); see "Bootstrapping a sandbox" in
`tensorlake.md`.

## Credentials

Only when another workflow has selected a worker for actual use, inspect whether its required CLI and authentication are
ready. For Codex, copying `~/.codex/auth.json` to the same path on a headless worker is an officially supported fallback
when the controlling host actually uses file-backed credentials. Otherwise use Codex device authentication or another
supported login path. For Claude Code automation with a Pro, Max, Team, or Enterprise subscription, prefer a token
produced by `claude setup-token` and pass it as `CLAUDE_CODE_OAUTH_TOKEN`; do not assume its local credential cache is
portable. A Console user may instead pass `ANTHROPIC_API_KEY` to the selected Claude invocation. These are the only
agent credentials this skill permits transferring. On a sprite or tensorlake sandbox, see "Credentials on a sprite" in
`sprites.md` or "Credentials on a sandbox" in `tensorlake.md` for how they are passed and cleaned up.

Announce the transfer because these are reusable secrets. Copy only the credential needed by the selected provider, keep
any persisted secret readable only by the remote user, and verify authentication. Do not overwrite an existing
credential merely because an authentication check returned an unexpected error.

Apart from an agent credential intentionally installed as above, treat the worker as having no GitHub, cloud, or
package-registry credentials and no access to private repository remotes. Never forward the SSH agent or copy other
home-directory state. Project-specific public prerequisites may be installed when the selected task needs them; never
add private registry configuration.

## Running work

The orchestrating workflow chooses the host, provider, exact local source state, transfer mechanism, remote working
directory, prompt, commands, and result format. Whatever mechanism it chooses runs over the transport recorded for that
worker during registration. A user-specified commit, revision, or working-copy snapshot remains binding. Resolve private
refs locally and treat the local checkout as the source of truth; the remote worker must not contact a private
repository remote to clone, fetch, pull, or push.

If the orchestrating workflow chooses unrestricted execution, the proven entrypoints are `codex exec --yolo` and
`claude -p --dangerously-skip-permissions`. These modes give the agent full access to the remote Unix account. They are
worker capabilities, not a request from this skill to run either agent. Use a disposable remote directory, and remember
that the remote agent has none of the controlling session's conversation.

Choose what to transfer for the task at hand. Over SSH that is normally `rsync` with the recorded transport as `-e`; on
a sprite it is `sprite file push` as described in `sprites.md`; on a tensorlake sandbox it is `tl sbx cp` with tarballs
for trees as described in `tensorlake.md`. Do not transfer credentials as source material; if that conflicts with a
requested working-copy snapshot, stop for direction. Do not blindly copy repository metadata, ignored files, or
unrelated local state.

In the return direction, treat command output as returned data and keep it bounded. Do not mirror or reverse-rsync the
remote workspace. Retrieve only a small, explicitly selected set of patches, changed files, reports, or logs into local
staging outside the checkout, then inspect them before applying anything. Remove disposable source and prompts from the
worker when they are no longer needed.

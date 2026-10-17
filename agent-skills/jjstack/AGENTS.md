# Instructions for agents changing this skill

## Run the tests after every change

After changing `scripts/jjstack`, anything under `tests/`, or skill text that describes what the script does, run both
test layers from the repository root before calling the change done:

```bash
deno test -A agent-skills/jjstack/scripts/jjstack
deno test -A agent-skills/jjstack/tests/
```

The first runs the unit tests inside the script: its parsers, stack rules, check verdicts, simulation, content proof,
stop requests, and run log, plus the contract checks that every error code appears exactly once and every jj call
carries the no-abandon setting. It takes about a second.

The second runs the end-to-end tests in `tests/e2e.test.ts`: whole commands (`submit`, `land`, `fast-land`, `stop`)
against a real jj checkout and a real bare Git repository playing GitHub's copy of the repo, with `tests/fake-gh.ts`
standing in for `gh`. They pin the sequencing the unit tests cannot see, such as retargeting before pushing, pushing
only the PR a landing is merging, retrying GitHub's transient responses, stopping and resuming, and cleanup. They take
about a minute and need `deno`, `jj`, Git 2.40 or later, and `jq` on `PATH`. A failing test keeps its temporary world
and prints where, so the jj checkout, the bare repo, the fake's state, and the event log can be inspected.

Also run `deno fmt --line-width=120 --ext=ts`, `deno check`, and `deno lint` on the script (the extensionless script
needs `--ext=ts` for `deno fmt`, and a `.ts` copy for `deno lint`), and the same without `--ext` on `tests/`.

## Keeping the fake honest

`tests/fake-gh.ts` is a small model of GitHub, and the end-to-end tests are only as good as that model. When the script
starts using a `gh` feature the fake does not know, the fake exits with "unsupported" and the tests fail; extend the
model to behave the way GitHub actually does, and check the real behavior first rather than guessing. Never make the
fake accept something just to get a test green, and never weaken an assertion to fit the fake.

When a change adds or alters how a command sequences its steps, add an end-to-end scenario for it, and confirm the new
test fails when the behavior is broken (temporarily revert the change and rerun) before trusting it.

## Real GitHub

For changes to landing, merging, check waits, or cleanup, also exercise the change against real GitHub on
`scode/repotesting`, a repository kept for exactly this kind of testing, where creating, merging, and deleting PRs and
branches is fine. Clone it with `jj git clone --colocate`, build a small stack, and run the commands. It is slow and
spends CI minutes, so small changes can skip it, but GitHub's real eventual consistency (lagging PR heads, late check
runs, transient merge rejections) is exactly what a model gets wrong.

## The time-scale seam

The script reads `JJSTACK_TIME_SCALE` once, to shorten every wait for the end-to-end tests. It only accepts values in
(0, 1], so a stray setting can never stretch a wait. Keep every new wait, poll interval, grace period, or heartbeat
behind `scaled()` (or inside `pollUntil`, which scales itself), or the end-to-end tests will slow to real time.

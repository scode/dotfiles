---
name: scode-galaxy-brain
description: >
  Accomplish a goal by delegating suitable parts of the work to cost-effective models while the current session stays
  in charge of planning, quality gating, and all commit/PR management. Use when the user explicitly invokes
  scode-galaxy-brain, e.g. "Use scode-galaxy-brain to <goal>", optionally with a prefer-gpt, prefer-claude, or
  prefer-muse keyword and/or a request to work with concurrency.
  Also use when the user says "galaxy brain feedback: ..." to record feedback about how this skill performed. Once
  invoked, the skill stays active for the rest of the session — including across context compaction and resume — until
  the user expressly stops it, unless the invocation itself limited the scope up front; if retained context says it was
  active, re-read this skill before delegating.
---

# Scode Galaxy Brain

## Premise

You — the current session — are running on a state of the art, expensive model. The point of this skill is to spend that
capability where it matters (planning, judgment, design, quality control) and route suitable work to models likely to
finish it at lower total cost without significantly compromising quality. You stay in charge the whole time: you
decompose the goal, you decide what to delegate, you judge every result, and you own the overall change.

The goal is cost-effective quality; parallelize when it helps. The one hard limit is write concurrency: writers that
share a working tree run one at a time, and concurrent writers are allowed only when each one is genuinely isolated from
the others, with you integrating the results serially (see Concurrency below).

Delegation is for steps toward a change, never for managing the change itself. You always own version control: commits,
branches, pushes, PR creation and updates. Delegates must not commit, branch, push, or open PRs — your session carries
the user's VCS workflow preferences and skills, and those do not transfer to a sub agent.

## Composing with other skills

Galaxy-brain is a routing layer, not a workflow. When another skill or instruction defines its own process — roles,
steps, what counts as a valid run — that skill stays authoritative for the process. Galaxy-brain only decides which
model and effort executes each unit of work, how the delegation seam works, and how delegated output gets gated.

Keep the roles separate: do not merge this skill's orchestrator role with another skill's coordinator role, and do not
attribute one skill's constraints to the other. If another skill forbids or requires something, that rule comes from
that skill; reason about it (and explain it to the user) on that skill's terms.

Routing authority covers nested delegation too. When another active skill's process calls for spawning subagents —
reviewers, workers, whatever it names them — each spawn is still a galaxy-brain delegation: pick the model from the
appropriate work profile, set model and effort explicitly where the spawn mechanism supports it, and announce the choice
as usual. The trap is following the other skill's spawn instructions verbatim and letting its subagents silently inherit
this session's expensive model. Inheriting is fine only as a deliberate routing decision, stated as such. This claims
only the choices the other skill leaves open: if it explicitly demands a specific model, agent type, or effort for a
spawn, that demand is process, not a routing default — honor it like any other rule that skill owns, and attribute the
choice to that skill when you announce it.

## Staying active for the whole session

Activation is session-scoped, not turn- or task-scoped. Once the user invokes this skill, keep routing every delegation
through it — including spawns triggered by other skills, and including later tasks the user never mentions the skill on
— for the rest of the session. Only two things end it: the user expressly asking to stop, or an invocation that limited
the scope up front ("use scode-galaxy-brain for <this one thing>"). Finishing the task it was invoked for does not.
Context compaction, session resume, a tool restart, or a summary that fails to mention the skill does not end it either;
treat retained context that has gone quiet about galaxy-brain as a summarization artifact, not a decision anyone made.

After compaction or resume, if the retained context says or implies this skill was active, re-read this SKILL.md and
`~/.scode-galaxy-brainrc.md` (if it exists) before doing further substantive work — the routing rules do not survive
summarization reliably. If the retained context is ambiguous but mentions outstanding delegated work, model or effort
routing, or galaxy-brain at all, assume the skill is still active and say that you are assuming it.

When you write a handoff or pre-compaction note while this skill is active, include the routing-layer state: the current
goal, any provider preference, rc-file assumptions, delegations still in flight, and the next routing decision. Do this
even when no delegate is currently running — between delegations is exactly when a summary is most likely to drop the
skill. This is a backstop, not the mechanism: stickiness applies whether or not a handoff was ever written.

## Routing model

Do not rank models with universal cost or intelligence scores. Effective cost depends on the task: a nominally cheap
model can spend more tokens, take longer, require more review, and trigger an escalation when work exceeds its reliable
range. Route by work profile instead, then use the gate and escalation policy to control total cost through an accepted
result.

Each model name includes its configured reasoning effort. The family determines which delegation path from the mechanics
section applies. `sota` marks models trusted with critical review and the orchestrator role. Availability and user
overrides may remove or replace these defaults; see Local availability.

| model                 | family | sota |
| --------------------- | ------ | ---- |
| gpt-5.6-luna medium   | gpt    |      |
| gpt-5.6-terra medium  | gpt    |      |
| gpt-5.6-sol low       | gpt    |      |
| gpt-5.6-sol medium    | gpt    |      |
| gpt-5.6-sol high      | gpt    | yes  |
| haiku-4.5 high        | claude |      |
| sonnet-5 low          | claude |      |
| sonnet-5 medium       | claude |      |
| sonnet-5 high         | claude |      |
| opus-5 high           | claude |      |
| fable-5 high          | claude | yes  |
| muse-spark-1.2 low    | muse   |      |
| muse-spark-1.2 medium | muse   |      |
| muse-spark-1.2 high   | muse   |      |
| muse-spark-1.2 xhigh  | muse   |      |

The muse family is Meta's Muse Code harness and its Muse Spark model. It is an option, not a default: never route to it
on your own initiative. It enters a route only through an explicit `prefer-muse` preference, a user request naming it,
or a deliberate cross-family decision announced as such. Its profile placements below are provisional: they rest on
vendor-reported benchmarks rather than calibrated use, and until real use calibrates them neither its benchmark tier nor
its token price counts as a reason to cross families on your own. It carries no `sota` mark, so it is never the sole
critical-review gate, and no muse route ends at a trusted endpoint (see Delegation and escalation rules).

### Work profiles

Classify the task before choosing a model. Use the primary for the orchestrator's family when it is suitable and
available. Move to the escalation model after a substantive failure or when the task proves more demanding than its
initial classification.

| profile                   | use when                                                                 | GPT route                                  | Claude route                    | Muse route                                  |
| ------------------------- | ------------------------------------------------------------------------ | ------------------------------------------ | ------------------------------- | ------------------------------------------- |
| mechanical                | Deterministic tool use, searches, log scans, or tedious verified churn   | gpt-5.6-luna medium → gpt-5.6-terra medium | haiku-4.5 high → sonnet-5 low   | muse-spark-1.2 low → muse-spark-1.2 medium  |
| routine authored          | Producing or editing small prose/code where baseline taste matters       | gpt-5.6-sol low → gpt-5.6-sol medium       | sonnet-5 low → sonnet-5 medium  | muse-spark-1.2 medium → muse-spark-1.2 high |
| clear-spec implementation | Bounded implementation with strong acceptance checks                     | gpt-5.6-terra medium → gpt-5.6-sol medium  | sonnet-5 medium → sonnet-5 high | muse-spark-1.2 medium → muse-spark-1.2 high |
| complex implementation    | Cross-cutting behavior, difficult debugging, or meaningful ambiguity     | gpt-5.6-sol medium → gpt-5.6-sol high      | opus-5 high → fable-5 high      | muse-spark-1.2 high → muse-spark-1.2 xhigh  |
| design and synthesis      | API design, architecture, nuanced copy, or competing tradeoffs           | gpt-5.6-sol medium → gpt-5.6-sol high      | opus-5 high → fable-5 high      | none                                        |
| mechanical review         | Non-critical review: style, prose, idiomaticity, docs, slop, or patterns | gpt-5.6-sol medium → gpt-5.6-sol high      | sonnet-5 high → opus-5 high     | muse-spark-1.2 high → muse-spark-1.2 xhigh  |
| critical review           | Correctness, security, concurrency, data integrity, or test-quality gate | gpt-5.6-sol high                           | fable-5 high                    | none                                        |

A `none` route means the family has no suitable model for that profile. When a provider preference points at such a
route, that is the "no suitable model" reason to diverge: fall back to the orchestrator's own family's route for the
profile and announce the divergence as usual.

These assignments are defaults, not claims that every task in a profile is equivalent. Test quality is critical because
weak tests are how correctness defects survive review. Reviews route above similarly sized implementation work because
the reviewer is the gate: a missed finding may have no later backstop. Some profiles intentionally share routes today;
keeping their semantics separate lets later calibration change one without conflating different failure costs. A second
cross-family SOTA perspective may be worth its overhead for high-risk critical review. Orchestration is not a delegation
profile; planning, decomposition, quality gating, and VCS ownership remain with the current SOTA session.

Visual design is an exception to the GPT design-and-synthesis route. Real-world feedback on GPT-5.6 consistently rates
sol below the Claude models on visual design taste even while its coding reputation holds up. When a
design-and-synthesis task's output is primarily visual — UI, frontend styling, slides, anything judged by how it looks —
use the Claude route even from a GPT-family orchestrator, and treat this as a sufficient reason to diverge from a
prefer-gpt preference. Announce the divergence as usual. Non-visual design work such as API design, architecture, and
copy stays on the normal routes.

Long context is an exception to the GPT mechanical route. gpt-5.6-luna's long-context retrieval is far below the rest of
the family (reported around 41% on MRCR versus roughly 90% for terra and sol), so a mechanical task whose input is
genuinely large — whole-repo scans, big log files, long-document analysis — starts at gpt-5.6-terra medium instead of
luna, escalating to gpt-5.6-sol medium. This is about input size the model must actually reason across, not task
difficulty; small-input mechanical churn stays on luna. A luna delegate that loses track of earlier context mid-task is
this weakness surfacing, not a generic substantive failure — reroute to terra rather than counting it against the
profile.

### Native-path bias

When models are roughly equally suitable, prefer the model in the orchestrator's family. Same-family delegates normally
stay inside the current harness; crossing families adds process startup, context transfer, authentication, permission,
output-handling, and failure overhead. The native delegation path is the real reason for the preference. If a harness
can invoke another family natively, prefer the native path rather than following family names mechanically.

Apply the bias according to expected task size. These are judgment anchors, not hard thresholds:

| expected size | practical meaning                                                     | native-path bias                                                                   |
| ------------- | --------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| tiny          | Setup and review may cost as much as doing the task directly.         | Very strong. Usually do it directly or delegate natively.                          |
| short         | One bounded operation with little context or expected iteration.      | Strong. Cross harnesses only for a meaningful capability or total-cost advantage.  |
| medium        | A substantive task where model work dominates fixed startup overhead. | Moderate. Consider likely retries and review burden alongside startup overhead.    |
| large         | Extended work where cross-harness startup is a minor part of the run. | Weak. Choose the route most likely to finish cleanly at lower expected total cost. |

Do not select an unsuitable model merely to stay native. Cross families when the native family has no suitable model,
the other family is materially better for the task, a medium or large task has materially lower expected total cost on
the other route, a native attempt failed, the user requested that provider, or an independent cross-family perspective
is part of the goal. Expected total cost includes likely tokens, latency, review burden, and escalation risk — not
nominal token price alone. For critical work, reliability and useful independence take priority over the size-based
bias.

### Delegation and escalation rules

- First ask whether delegation is worthwhile. If specifying and reviewing the task costs more than doing it, do it
  yourself. This is especially common for tiny tasks.
- Set the profile by ambiguity, verification strength, required taste, and the cost of a wrong or missed result — not by
  apparent line count alone.
- Give a primary model one well-specified attempt. Fix trivial defects locally. After one substantive failure, escalate
  instead of repeatedly spending tokens on the same underpowered model.
- A GPT or Claude route with no listed escalation is already at the family's trusted endpoint. If it fails
  substantively, handle the work in the orchestrator or make a deliberate cross-family attempt; do not retry it
  mechanically. Muse routes are the exception: none of them ends at a trusted endpoint, so a substantive failure at the
  last listed muse model gets the same treatment — orchestrator or deliberate cross-family attempt — even though the
  model that failed is not trusted.
- When the primary's output is broadly wrong rather than fixable, preserve pre-existing user work, remove only the
  delegate's changes, and give the escalation model a fresh implementation task. Include concrete acceptance failures as
  evidence, but do not ask it to repair a structurally bad patch.
- Escalate immediately if the output shows that the task was misclassified. You have standing permission to reroute or
  do the work yourself without asking.
- Prefer cheaper, faster workhorses when validation is deterministic and inexpensive. Start stronger when incorrect
  output is difficult to detect or the delegate itself is the final review gate.
- Every time you delegate, tell the user which model and effort you picked, the work profile, and why any native or
  cross-family choice makes sense for the task's expected size. One announcement may cover a homogeneous fan-out batch
  that shares the same profile, model, effort, and rationale.

## Provider preference

The invocation may include the keyword `prefer-gpt`, `prefer-claude`, or `prefer-muse`. This expresses a preference
unrelated to model performance — typically the user has a large subscription with one provider and a small one with the
others, and wants spend steered accordingly. The default, absent a keyword, is no preference.

When a preference is given, route every delegation to the preferred family unless there is a very clear, strong reason
to diverge — for example repeated poor output, no suitable model for the selected profile, or a goal that explicitly
needs an independent cross-family perspective. An explicit provider preference overrides the default native-path bias.
When you diverge, say so and why.

## Concurrency preference

The invocation may ask for concurrency in plain words — "with concurrency", "parallelize where you can", or similar.
Absent such wording, the default stands: parallelize when it helps, but don't hunt for opportunities.

When the user asks for concurrency, treat parallelism as an active goal rather than an available tool. During planning,
decompose the work to surface independent units: fan out read-only work by default, and prefer isolated-writer fan-out
(see Concurrency) over a serial writer sequence whenever the tasks can be specified independently and the isolation
overhead — including the riskier merge — is worth the speed-up. When you keep a plausibly parallel step serial, briefly
say why.

The preference buys speed, never quality: every rule in Concurrency and The gate applies unchanged, and escalation or
review steps are never skipped to keep a fan-out moving.

## Local availability

The inventory and profiles describe models that exist; they do not know which ones the user can access in this
environment. Before your first delegation, check for `~/.scode-galaxy-brainrc.md`. If it exists, read it and honor it:
it contains natural language adjustments from the user, most commonly availability restrictions like "fable-5 is not
available, do not use" or "only claude models work here". Treat its contents as authoritative. Remove unavailable models
from every profile and use the next suitable option rather than preserving a preferred slot mechanically.

The rc file may replace the model inventory, override profile assignments, or add natural-language routing constraints.
An inventory it supplies replaces the default inventory wholesale: omitted models are unavailable for delegation. Model
names are the ids to invoke — pass GPT names to `codex -m` and muse names to `muse exec --model` without the trailing
effort word, and map Claude names to the nearest `--model` alias. When a replacement inventory introduces models absent
from the built-in profiles, the rc file must assign them to profiles or describe their roles well enough to do so. Ask
instead of inventing profile assignments when that information is missing. A new family also needs an invocation
mechanism; treat it as unavailable until the rc file provides one.

Legacy rc files may still contain the old `cost`, `intelligence`, and `taste` columns. Treat known rows as an
availability inventory and preserve family/SOTA metadata, but ignore the numeric scores. Apply the built-in work
profiles after filtering them to the listed models. Unknown models still require explicit profile roles under the rule
above. Tell the user that profile overrides are now the supported way to customize routing.

To spare the user manual copy-paste, they can ask you to seed the file. On that explicit request only, write the current
model inventory and work-profile table into `~/.scode-galaxy-brainrc.md`, preceded by a note that they replace the
defaults and are meant to be edited. Never discard existing content: append when safe, and stop to ask if the file
already contains an inventory or profile table.

If the file does not exist, all inventory models are assumed available, with one practical exception: a shelled-out
family whose CLI is not installed (`codex`, `claude`, or `muse` missing from `PATH`) is unavailable regardless of the rc
file. Treat that as a missing family when honoring a provider preference — say so and fall back — rather than as a
launch failure to retry. Apart from the seeding request above, do not create or edit this file yourself; it belongs to
the user.

## Delegation mechanics

Stay native within your own harness; shell out only when crossing vendors. First figure out which harness you are
running in, then:

- **Claude session → claude model**: use your native sub agent mechanism (e.g. the Agent/Task tool) with the model
  parameter set to the target model.
- **Claude session → gpt model**: shell out to `codex exec` (see below).
- **Codex session → gpt model**: use your native sub agent mechanism, specifying the target model.
- **Codex session → claude model**: shell out to `claude -p` (see below).
- **Any session → muse model**: shell out to `muse exec` (see below). Muse Code is only ever a delegate here, never the
  orchestrator, so there is no native muse path.

When delegating natively, also set the target reasoning effort if your sub agent mechanism has an effort parameter;
otherwise sub agents inherit the session's effort and that is acceptable.

Name every sub agent (label, description, or whatever your mechanism displays) so the name includes the task plus the
model and effort actually doing the work, e.g. `fix-foo-gpt-5.6-sol-medium`. Harness UIs otherwise show only the wrapper
or default model, which misleads anyone watching progress.

### Shelling out to codex

```sh
codex -c model_reasoning_effort=high exec --yolo -m gpt-5.6-sol -o <scratch-file> "$(cat <prompt-file>)" < /dev/null
```

- Reasoning effort is set with the global `-c model_reasoning_effort=<low|medium|high>` option before `exec`. Always
  pass it explicitly rather than relying on the user's config default; the startup header echoes the effective
  `reasoning effort:` if you need to confirm.
- `-o` writes the agent's final message to a file; read that file for the result instead of parsing stdout.
- Always keep the trailing `< /dev/null`. Given a prompt argument and a non-TTY stdin, `codex exec` reads stdin to EOF
  before starting work (its `Reading additional input from stdin...` startup line is that read), so a stdin that never
  delivers EOF blocks it at startup indefinitely. At least one agent harness omits its own stdin redirect from the
  wrapper exactly when the command text contains a heredoc, handing the child a pipe that never closes — the hang
  strikes only sometimes and is indistinguishable from a slow run except by its log. The explicit redirect holds even
  then.
- Keep heredocs out of the command that launches codex; a heredoc anywhere in the command text is the known trigger for
  that dropped redirect. Build the prompt in its own earlier command — write it to a scratch file — and pass it as
  `"$(cat <file>)"`.
- Treat a background run whose log stays at `Reading additional input from stdin...` and never reaches the version
  header as this startup hang, not a slow model. Kill it and relaunch with the redirect instead of waiting for a
  completion that will never come; a healthy run prints the header immediately after that line.
- A zero exit status is necessary but not sufficient. `codex exec` does return nonzero when the turn itself fails, but a
  turn that completes normally exits 0 even when its final message declines the task, reports a tool or sandbox failure
  the agent could not work around, or gives status instead of the work. Judge the `-o` file against the task's explicit
  acceptance criteria and reject anything that does not meet them. A result far shorter than the task warrants is the
  cheapest tell, though it is a reason to look rather than grounds to reject on its own — some correct answers really
  are one line. When a whole fan-out fails the same way, treat it as one broken execution path rather than N model
  failures: stop the batch and fix the path instead of escalating each delegate through it.
- Runs in the current working directory by default; pass `-C <dir>` to target elsewhere.
- Long tasks can exceed your shell tool's default timeout. Run them in the background and monitor them (see Monitoring
  below); use a foreground timeout only when it is shorter than the monitoring interval.

### Shelling out to claude

```sh
CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 \
  claude -p --model <alias> --effort <level> --dangerously-skip-permissions "$(cat <prompt-file>)" < /dev/null
```

- Model aliases: `sonnet`, `opus`, `haiku`, `fable`. Effort levels: `low`, `medium`, `high`, `xhigh`, `max`. The final
  response is printed to stdout.
- Print mode otherwise terminates background tasks after 600 seconds and exits successfully with a diagnostic instead of
  the requested result. Keep its inner wait unlimited; the outer orchestrator already owns monitoring and cancellation.
- A zero exit status is necessary but not sufficient. Reject empty or truncated output and results that do not satisfy
  the task's explicit acceptance criteria. Also reject the termination diagnostic, which starts with
  `Background tasks still running after`.
- The same outer shell timeout caveat applies.
- Use the same defensive launch pattern as for codex: keep the explicit `< /dev/null` and build the prompt in an earlier
  command instead of a heredoc. This is a precaution against the harness-side dropped redirect, worth taking for any
  shelled-out delegate — it does not claim that `claude -p` reads piped stdin after a prompt argument the way codex
  does.

### Shelling out to muse

```sh
muse exec --yolo --model muse-spark-1.2 --reasoning-effort medium --user-input-auto-resolve \
  --max-model-steps <N> --prompt-file <prompt-file> > <result-file> 2> <log-file> < /dev/null
```

The flag surface below comes from `muse exec --help`. The runtime behavior — output shape, exit codes, stdin handling,
policy enforcement, worktree lifecycle — was observed with Muse Code 0.2.1 rather than read from documentation; treat it
as an observation to re-check when the CLI changes, not as a stable contract.

- `--model` takes the id without the effort word; `--reasoning-effort` accepts
  `none|minimal|low|medium|high|xhigh|ultra` and defaults to `high`, so always pass it explicitly to match the inventory
  row you chose. Omitting `--model` uses the account's default, which was observed to be a variant id
  (`muse-spark-1.2-contributor`) rather than the public `muse-spark-1.2`; pass the public id explicitly.
- Without `--json`, stdout carries only the final message, so redirecting stdout to a result file captures exactly what
  you need to judge. Muse writes its own status lines (`muse: workspace root: ...`) to stderr, so keep the two streams
  separate as in the template. With `--json`, stdout is a JSONL event stream instead: the final message is the `text`
  field of the `run.terminal.completed` event, with `run.output.delta` events streaming before it.
- `--prompt-file` reads the prompt from a file, which removes the quoting problem that makes the other harnesses take
  the prompt as `"$(cat <file>)"`. Still build the prompt in its own earlier command and keep the explicit
  `< /dev/null`: the dropped-redirect precaution from the codex section applies to every shelled-out delegate.
  `muse exec` was observed to complete with a never-closing stdin, so the redirect here is defense in depth rather than
  a known hang fix.
- A headless run does not hang on a question either way: without `--user-input-auto-resolve` the delegate has no way to
  ask and simply ends its turn with the question as its final message; with it, the delegate is offered a
  `request_user_input` tool that cancels itself, so it learns explicitly that nobody is there and reports what it could
  not decide. Both exit 0, so a final message that is a question is a gate failure to catch by content, not by exit
  status. The flag stays in the template because the explicit cancel produced the more useful report.
  `--max-model-steps` is a runaway guard, not a budget: set it well above what the task should need (tens of steps for a
  small bounded edit, hundreds for implementation work that runs checks) so it only trips on a runaway.
- A zero exit status is necessary but not sufficient, exactly as for codex. A failed turn (unknown model, auth error,
  agent loop failure) was observed to exit 1 with the reason on stderr, while a turn that completes normally exits 0
  even when the final message declines or reports that it could not finish. Judge the captured message against the
  acceptance criteria.
- Always run with `--yolo`, for read-only delegates too. Muse has narrower policy switches (`--disable-write`,
  `--disable-shell`), but do not use them: a review or scan that looks read-only often still needs to write a scratch
  file to feed a tool, and a policy denial mid-task breaks the run instead of protecting anything. Read-only is a
  prompt-level instruction here, exactly as for the other harnesses.
- Runs in the current working directory by default; `--workspace <PATH>` is the analogue of codex's `-C`. For concurrent
  writers, `-w create` gives a native isolated tree. Pass `--session-id <uuid>` (a fresh `uuidgen`) so the tree is
  predictable: it is created at `.muse/worktrees/<repo-name>-<uuid>` on branch `muse/session-<uuid>`, muse adds
  `/.muse/worktrees/` to `.git/info/exclude`, and the worktree is retained after the run only when dirty — a run that
  changed nothing removes it. `-w` requires session logging, so do not combine it with `--no-session-log`. Integration
  and cleanup are yours, per Concurrency: extract the change set, apply and gate it in the main tree, then
  `git worktree remove --force` the tree and delete the branch.
- Killing a run is safe in both directions. `muse` is a wrapper around a `muse-bin-<version>` process; record that
  process's pid, since a backgrounded launch can hand you the wrapper's. The delegate's shell commands run in their own
  session, so a process-group kill would miss them, but both SIGTERM and SIGKILL to `muse-bin` were observed to take
  down every child (its helper process, the shell, and whatever the shell was running). SIGTERM additionally flushes the
  session log. A worktree run that is killed keeps its dirty worktree either way, so partial work is inspectable and the
  usual pre-relaunch cleanup applies.
- Foreign-harness caveats carry over: `--yolo` disables approval, the sandbox, and workspace trust checks, so use it
  only where you would accept the same for the orchestrating session, and the shell timeout / background monitoring
  rules below apply unchanged.

### Monitoring long-running delegates

The stdin hang above was found only after an orchestrator waited hours on a shelled-out code review that was never going
to finish. The lesson generalizes beyond that one bug: a background delegate has no guaranteed liveness or
forward-progress signal before it exits, and "no news yet" is not evidence of progress.

- Never wait open-endedly on a shelled-out delegate, and never make a foreground call whose timeout exceeds the
  monitoring interval — a blocked foreground wait bypasses monitoring entirely. Run long delegates in the background and
  record what you need to check on and kill them later: job handle or pid, log path, output path, start time, expected
  duration, and a hard deadline. Include still-running delegates in any handoff or pre-compaction note.
- Wake up and check every running delegate at least every 30 minutes — sooner when the expected duration is shorter —
  using whatever timer, scheduled wake-up, or bounded-wait mechanism your harness offers. Failing all else, cap each
  blocking wait at 30 minutes and re-check between waits. In a fan-out, check every member at each wake-up: the batch is
  only done when its slowest member is, and one hung member silently holds the whole batch.
- Check known hang signatures first; they are cheap and decisive. For codex, apply the startup-hang test from above —
  the stdin line is the last output and the version header never appeared — and kill and relaunch with the corrected
  invocation, since more waiting cannot help.
- Otherwise weigh the evidence by what the tool shows. `codex exec` streams a transcript while working: compare the log
  against the last check's baseline, and record the new observation for the next one. New output proves activity, not
  necessarily useful progress; a log that has not grown across a full interval is a reason to inspect the run, not by
  itself grounds to kill it — long reasoning stretches can be quiet. `claude -p` prints only the final response by
  default, so silence means nothing there; when a long run needs observability, launch it with a streaming output format
  (`--output-format stream-json --verbose`) instead. `muse exec` is the same on stdout: silent until the final message
  unless launched with `--json`, in which case its event log can be compared across checks like the codex transcript.
- Use the estimate and the deadline for different decisions. Crossing the expected duration triggers investigation, not
  a kill. Crossing the hard deadline means the run is over budget regardless of apparent liveness: kill it, capture the
  log, and treat the result as inconclusive.
- A hang or launch failure is an execution-path failure, not a substantive model failure — fix the path and relaunch
  once rather than escalating models over it. Make sure the kill takes down the delegate's children too, and before
  relaunching a writer in a shared tree, remove its attributable partial changes (or discard its isolated tree) so the
  retry starts from a known baseline.

### Writing the task spec

The delegate has none of your conversation context. Every delegation prompt must be self-contained:

- The goal and any constraints that bound it.
- Exact file paths or directories in scope.
- Acceptance criteria: what done looks like, concretely.
- Which checks to run (tests, linters, formatters) before reporting back.
- For read-only tasks: state explicitly that it must not edit any files.
- Always: no commits, no branches, no pushes, no PRs.
- Ask it to report what it did and call out any deviations from the spec.

Before delegating a task that writes to your working tree, note the current working-copy state — `git status`/`git diff`
or the equivalent in whatever VCS is in use — so you can attribute the delegate's changes cleanly afterwards. Writers in
isolated trees (see Concurrency) are attributable as long as the tree started clean from a recorded base — the normal
state of a fresh worktree or clone; note that base when you create it.

## Concurrency

- Read-only tasks (log scanning, code search, independent reviews) may run concurrently whenever they are independent of
  each other. Use this freely for fan-out work like scanning many logs or directories.
- Writers that share a working tree run one at a time, no exceptions. "They edit different files" is not a safe basis
  for parallelism: file-disjoint writers still see each other's half-finished edits when they run checks, still collide
  on lock files, generated code, and build state, and delegates drift out of their predicted scope. The failure mode is
  an interleaved diff nobody can attribute or cleanly revert.
- Writers may run concurrently when each one is isolated so that no delegate can observe or clobber another's
  in-progress work. Use whatever mechanism your harness offers at your discretion — native worktree isolation on a sub
  agent, a manually created `git worktree` that a shelled-out delegate is pointed at, a separate clone, or anything
  equivalent. The bar is that the tasks cannot conflict through any mutable state they touch, not a plan for writers to
  stay out of each other's way. A separate tree covers the files, but shared out-of-tree resources — build caches
  pointed outside the tree, test databases, ports, daemons — conflict straight through it; isolate those too or
  serialize.
- Isolation moves the merge to you instead of eliminating it. Integrate serially: extract each delegate's complete
  change set (plain `git diff` misses untracked files — new files, renames, and mode changes all count), gate it as
  usual, and apply it to the main tree one at a time. Keep each isolated tree until its result has been applied and
  validated. A broadly wrong result is discarded along with its tree, which is cheaper than untangling it from a shared
  one.
- Conflicts between accepted results are yours to resolve and an expected cost of this mode — disjoint task scopes make
  them rare, not impossible. Textual conflicts surface at apply time, but semantic conflicts apply cleanly, and a
  delegate's own checks only ever validated its isolated baseline. Re-run the relevant checks on the integrated main
  tree after each apply, and again after the last one.
- Two caveats before choosing isolation: isolated trees start from committed state, so delegates will not see
  uncommitted work in the main tree — commit it first if they need it (stashing does not help: it hides the work from
  the main tree without making it visible anywhere else), or serialize rather than committing user work merely to
  parallelize. And a fresh tree typically has no build cache, so writers that run checks may rebuild from scratch; weigh
  that against the parallelism gain.

## The gate

You are the quality gate for everything a delegate produces. Never accept a delegate's self-report as evidence the work
is good.

For code output:

1. Inspect the actual diff (`git diff`) yourself, not just the report.
2. Re-run the relevant checks yourself.
3. Then make a judgment call:
   - Small defects (naming, comments, minor logic): fix them yourself — a fixup round-trip costs more than doing it.
   - Substantive but well-specified defects: send one precise fixup round to the profile's escalation model.
   - If the escalation also fails, or the output shows that the profile itself was wrong, stop iterating. Do it yourself
     or move to a stronger profile without asking the user.

For read-only findings (reviews, scans, analysis): spot-verify against the cited code or data before relaying. When
reporting to the user, separate what you confirmed from delegate claims you did not verify.

In your final report to the user, briefly note which parts were delegated and to which models.

## Feedback capture

When the user says "galaxy brain feedback: ..." (or clearly signals feedback about how this skill performed), pause
whatever you are doing and record the feedback before resuming. The record exists so the skill's author can later hand
it to an agent working in the skill's source repository and ask for improvements — write it with that reader in mind.

Append (never overwrite) a markdown entry to `$XDG_STATE_HOME/scode-galaxy-brain/feedback.md`, defaulting to
`~/.local/state/scode-galaxy-brain/feedback.md` when `XDG_STATE_HOME` is unset. Create the directory if needed. After
writing, tell the user explicitly which file you appended to.

Each entry should be self-contained — the future reader has no access to this session:

- A `## <date> — <short title>` heading.
- The user's feedback, verbatim or near-verbatim.
- What you were doing when the problem occurred: the task, which model and delegation path was involved, the actual
  commands or prompts where relevant, and what went wrong (exact errors beat paraphrases).
- Your own analysis if you have one: root cause, and what change to the skill instructions would have prevented the
  problem. Mark speculation as such.

Avoid including private information (credentials, personal data), but do not sacrifice clarity of the problem
description to scrub aggressively — the user reviews the file before forwarding it anywhere.

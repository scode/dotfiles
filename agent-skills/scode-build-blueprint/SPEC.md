# scode-build-blueprint specification

Dependencies: agent-resumeable, scode-model-routing, scode-harness-shellout

This is the maintenance contract, not an execution entrypoint. Changes to behavior must keep the instructions and this
contract consistent.

- Planning uses the user's current session and produces a repository-grounded blueprint for a separate workhorse
  session. It never starts implementation or creates the execution log. The user chooses the executor model/effort.
- `/goal <absolute-blueprint-path>` suffices in a correctly selected session with the required skills installed.
  Execution needs no planning conversation; it resumes from recorded state and verifies repository drift.
- Initial top-level execution is limited to Codex and Claude Code because shared routing excludes Muse/OpenCode
  orchestrators. Those harnesses may host approved delegates. The operator selects the executor; self-model validation
  and identity-confirmation gates are forbidden. Recorded unknown or mismatched identity is not a preflight blocker.
  Worker permissions remain explicit blueprint pairs, never inferred from runtime identity.
- Blueprint design emphasizes requirements, invariants, interfaces, milestone evidence, autonomy boundaries, and
  consultation triggers. Routine implementation choices remain with the executor. New scope or authority is not implied
  by unattended execution; unresolved material choices are settled before handoff or block affected work.
- No dependency path activates Galaxy Brain or scode-build-goal. Routing supplies recommendations; shellout supplies
  verified launch mechanics; this skill owns the execution, delegation, and expert gate policy.
- Execution assumes an absent user. Routine in-scope decisions proceed autonomously; before declaring critical work
  blocked, consult the approved expert for an authorized path, or record why consultation is unavailable or unsafe.
  Existing expert blocker assessments need not be repeated. Immediate safety pauses, explicit permissions and caps, and
  required gates remain binding. Continue safe independent work; do not override explicit task-specific authority.
- All model delegates, including same-harness workers and process-defined reviewers, shell out with explicit approved
  model/effort settings. Native delegates and delegated fan-out are forbidden. Known background commands need no model.
- Ordinary workers are limited to the confirmed executor pair plus explicit approved pairs. State-of-the-art experts are
  limited to prescribed consultations, diagnostics, and reviews; they do not implement production changes. Routing,
  retries, fallback, or another skill's process cannot silently expand this authority.
- The executor owns authorized VCS operations. Delegates perform none. Shared-tree writers are serialized, concurrent
  writers require genuine isolation, and test evidence identifies a stable tested tree. Integration is verified again.
- Planning prepares the relevant test substrate before dependent handoffs. Workers own focused checks and repairs within
  their assignment; missing verification is an explicit partial handoff with an owner for the outstanding evidence.
  Related repairs normally resume that worker. Focused evidence never replaces required integrated checks.
- Known long-running commands use native background completion notifications when supported, otherwise bounded waits on
  known handles. No model or job wrapper is needed merely to wait; status files alone do not promise wakeups. Resource
  checks, stable-tree evidence, and process deadlines remain binding.
- An independent RAM/disk watchdog is mandatory before implementation or delegation and throughout owned work. It
  samples about every minute, alerts on low/falling resources and recovery, and is verified/restarted after failure or
  resume. A background process is preferred; model watchdogs use approved shellout worker routes and occupy concurrency
  slots. The executor pauses launches on resource or monitor failure, preserves ungated results, and cleans only owned
  resources. Without notifications the executor checks status at least once a minute. Completion stops the owned
  watchdog. Startup verification crosses the launch tool-call boundary; a same-call heartbeat is insufficient. A
  monitoring gap discovered at shutdown remains a disclosed gap, not something a replacement monitor can repair.
- Early expert use is encouraged to protect quality and avoid wasted work. Debugging time/failed-attempt thresholds are
  latest points for consultation, never minimum waiting periods. Expert consultation has direct evidence access and
  bounded diagnostic authority. Flaky-test acceptance requires repeated-run evidence, not one passing run or weakened
  tests.
- Expert calls and review/fix cycles have no default numerical caps. The producer may propose a cap only for a clear
  goal-specific reason and with user approval, recording both. Useful exchanges continue; stalled exchanges require
  explicit reassessment, not an arbitrary call-count stop. Model permissions, deadlines, and concurrency limits remain.
- When explicit caps exist, scheduled gates and unplanned consultations have separate identified allowances and declared
  counting units. Calls preserve the applicable allowance and consumption across follow-ups and replacements; allowances
  cannot be silently borrowed or reset. Without a cap budget fields are null, while all calls are still logged.
  Operational limits are distinct from measured tokens and costs.
- Independent expert reviews judge actual changes against original intent and acceptance evidence. Substantive fixes are
  re-reviewed; substantive disputes need expert adjudication. Budget exhaustion never lowers the gate. Default delivery
  is reviewed open draft PRs, not merged PRs.
- Related consultant, worker-repair, and reviewer follow-ups resume the same model conversation when available. New
  independent review gates start fresh; replacements retain unresolved evidence and budgets and record their reason.
- The existing working log retains findings, repair attempts and evidence, pending gates, session/process handles, and
  next actions. Failed or reverted repairs do not close findings. Completion and resume reconcile actual artifacts and
  the current tree; unresolved required findings or unverified repairs keep their gates open. Resumed workload after
  shutdown requires restored monitoring, without erasing earlier coverage gaps.
- Execution evidence lives in private, exclusively created UUID directories under XDG state, with a home-directory
  fallback for relative/unset XDG state. Identity and budgets survive resume; no newest-directory guessing or reuse of
  another execution's artifacts. The executor alone writes immutable events; the watchdog owns a separate mutable status
  artifact.
- Records cover local choices, worker delegation, expert calls, follow-ups, failures, repairs, outcomes, and available
  usage. Exact prompts and ephemeral task specifications are retained privately, along with deliverables and source
  usage records. Whole transcripts are optional, and temporary references are labeled as such. Decisions precede work;
  retrospective reconstruction is labeled and source-backed. Completion reconciles all attempts and local outcomes,
  disclosing gaps separately from implementation acceptance. Polling yields without shortening process deadlines, and
  failed continuation launches count toward the same corrected-path retry limit.
- Usage keeps provenance, requested/reported identities, delta/cumulative scope, and overlap semantics. Missing evidence
  is unknown, not zero. No automatic upload, pruning, pricing, or model evals. Recording failures are disclosed, never
  fabricated, and do not bypass authority or quality gates.

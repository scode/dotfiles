# Cursor support grew into configuration infrastructure

NOTE: Historical artifact, written 2026-09-21. Records the user's report, the available execution evidence, and the
proposed skill changes before implementation. This is one incident, not a measured evaluation of agent behavior.

## Sources and original complaint

The goal was built using `scode-build-goal` in this repository. The user brought the incident here after asking the
executing agent, approximately, “why is this so hard / taking so long are we over engineering something”. They wrote:

> Let's talk about what we can do to avoid this happening. This is completely off the rails. the notion of creating a handcrafted special JSON parser in this context is just wildly out of scope. As you can tell, simply me prompting very simply immediately surfaced insane behavior.
>
> One thing is, we could be more specific in the design, which I think is generally a good thing to make things be a little bit more upfront designed when building the goal, but with some trade-offs . another thing could be to take checkpoints where, for example, a reviewer is asked to assess whether specifically overengineering is going on, just to trigger this reasoning along these lines. What would you propose?

The executing agent's own assessment is in this **secret gist**, linked here at the user's explicit request:
<https://gist.github.com/scode/173e24fe6ca945f376f0dc3c1d803901>. Its title is “Goal-building failure: Cursor support
expanded into configuration infrastructure”. Secret means unlisted, not access-controlled; this entry retains the link
as source evidence.

The local sources inspected during this discussion were:

- `/home/scode/git/farhelm-fh-cursor-support-cursor-cli-goal.md` (already rewritten to basic support when inspected).
- `/home/scode/git/farhelm-fh-cursor-support-cursor-cli-goal-log.md`.
- `/home/scode/git/farhelm-fh-cursor-support-cursor-research/superseded-tracking-goal.md`.
- `goal-review.md` and `installer-design-audit.md` in that same research directory.

These paths identify the evidence available at the time; they are not portable dependencies of this repository.

## Screenshot text supplied by the user

The user supplied the following text as the screenshot context. Line gaps, truncated commands, and the terminal's
ellipsis are retained as supplied; they are not reconstructed from the later log. No screenshot image was available in
this conversation.

```text
• Edited ~/git/farhelm-fh-cursor-support-cursor-cli-goal-log.md (+18 -0)
    505    inspect final diff for excluded remnants, documentation pass, Astra review, cold VCS prose review and draft PR delivery.
    506 +
    507 +## 2026-09-21 20:59 UTC — basic support review corrections
    508 +
    511 +- Fixed review findings: literal protocol compatibility test now covers 25/26; launcher notice uses active structured
    512 +  Cursor or the two built-in Cursor profiles, never an inactive draft. Added focused browser coverage and extended the
    513 +  existing stored-launch round-trip test with Cursor Generic/no template. Requested same reviewer's focused follow-up
    514 +  in research/basic-review-followup.md. Corrected compiler test comment to avoid claiming profile coverage.
    515 +- Focused run 72998e15-985e-41b3-8721-b8d865410119 failed one test: search expected eight choices, now nine.
    516 +  Corrected that fixture; retained failure evidence. Rerun 54b58e4b-6a76-4102-8bd2-05cc6d26d053 is compiling,
    517 +  handle 79384. It also includes handshake and supervisor persistence tests omitted by the first selection.
    518 +- UI JS run be561b87-3f30-4194-9e9e-fc3b7af102b6 passed 151 tests. Desktop asset check passed all 19 assets,
    519 +  including web and desktop builds; final UI notice edits require refreshed web build for browser validation.
    520 +- NOTE: prior review launch event c5c0aaff-285d-4cb2-aa48-7bd35bbff481 has an incorrect 21:31 timestamp;
    521 +  actual launch was approximately 20:51 UTC. Native token accounting unavailable. Prior entries remain immutable.
    522 +- Next: collect focused tests, refresh/build and focused browser checks, final format/Clippy checks, review closure,
    523 +  cold VCS prose review and one draft PR. No tracking work, merge or live configuration changes.

• Ran test ! -e /home/scode/.scode-galaxy-brainrc.md && python3 /home/scode/.local/state/scode-galaxy-brain/sessions/
    /path/to/test-sleep-env/bin/python -B scripts/test-check-test-sleeps.py
    ```

• Waited for background terminal · nextest_dir=$(python3 scripts/install-pinned-nextest.py) && tmux_dir=$(scripts/build-pinned-tmux-
ci.sh) && PATH="$nextest_dir:$tmux_dir:$PATH" python3 scripts/record-test-run.py --runner nextest --kind development --tmux required
--selection 'Cursor launch, profiles, persistence, protocol and presentation' --concurrency '4 nextest slots; retries 0' --timeout

• The fixes are in place. I’m checking launch arguments, stored choices, fresh-only restart behavior, and the launcher notice in
  Chromium and WebKit. Cursor uses the existing Generic integration, so there is no new runtime subsystem.

• Ran git diff HEAD -- crates/farhelm-supervisor/src/service/handlers.rs crates/farhelm-helm/src/sessions.rs crates/farhelm-proto/
  │ src/lib.rs crates/farhelm-ui/src/list/create_form.rs
  └ diff --git a/crates/farhelm-helm/src/sessions.rs b/crates/farhelm-helm/src/sessions.rs
    index 52bc5e2..cf776ba 100644
    … +235 lines (ctrl + t to view transcript)
                 // The agent, offered from the helm catalog and defaulting
                 // to what a session was last created from on this helm (SPEC.md's

› btw can you summarize what went wrong here with over engineering in your vie
```

## What happened

According to the executing agent's retrospective, the original request was Cursor CLI support with roughly
Codex/Claude parity, and tracking was initially mandatory. Research found that a status callback could identify the
active conversation where the investigated plugin lifecycle events could not. The user rejected per-launch config
snapshots and accepted explicit local enable/check/disable commands operating on the real account configuration,
preserving settings and an existing status command. That authorized a product behavior; it did not settle the cost or
architecture of a configuration-management subsystem.

The archived goal made stronger promises mandatory. It required duplicate-key rejection, preservation of unknown
numeric values, and recommended replacing parsed field spans. It also required installation identity, private manifests,
pre-edit recovery bytes, ownership proof, concurrency checks, and restoration of the original command. These were
already in the plan handed to the executor. The incident was not merely a worker drifting away from a modest goal.

Review reinforced the plan. The installer audit called the installer and reporter “tractable”. The goal review said:

> The stated native-store, WAL, parser, callback-ordering and installer-race checks are appropriate implementation work with explicit refusal boundaries already selected.

The log records implementation starting at 18:33 UTC. At 18:36 the first delegated unit was pure configuration
byte-span editing and Cursor argv compatibility. The parser component was accepted at 19:03 after local fixes, 13
selected tests, and 55,987 generated differential cases against the vendor command parser. Those differential cases
tested argv compatibility; they should not be mistaken for 55,987 JSON tests. The configuration module included both
concerns. The installer then went through review, a failed gate, and a correction delegation. At 20:10 its filesystem
component was accepted with 39 tests passing. At the 20:19 checkpoint, most user-facing launch and tracking integration
still remained, and no PR existed.

Work also expanded into native SQLite/WAL verification, conversation locator state and ordering, old-command process
lifecycle, and platform-specific process identity. Some tracking constraints were real: explicit resume selected the
launch directory's storage bucket, so a conversation switched from another directory could not safely be resumed by
UUID alone. Refusing unsupported cross-directory resume was an accepted boundary. This does not establish that all the
surrounding machinery was necessary or proportionate.

The retrospective says the user expected configuration editing to mean locating the file, parsing it with an ordinary
parser, changing the structure, serializing, and writing through a temporary file and atomic rename. They later
clarified removal as removing an obvious Farhelm command without bookkeeping, then reduced the entire feature to basic
launch support without tracking. The revised goal explicitly excluded the custom parser, installer, status wrapper,
tracking state, native-store inspection, and new runtime kind. It required removing the discarded implementation.
The agent reported carrying out that deletion; delivery validation was still underway when the retrospective was
written. This entry does not claim an independently verified final Farhelm PR or deployment.

## Diagnosis and proposed changes

The discussion identified a failure to challenge invented requirements. Locally reasonable defensive decisions formed
a chain: tracking needed a callback, preserving a command suggested composition, restoration suggested metadata, and
metadata suggested ownership and recovery rules. Correctness reviews kept strengthening that chain. Asking many small
questions did not expose its aggregate cost. More detailed design by itself could have made the problem worse, because
the original design was already detailed enough to prescribe the overengineering.

The proposed changes were:

1. Before finalizing a goal, produce a short implementation outline: the existing path to extend, the smallest design
   that meets the request, substantial new state or subsystems, and expensive promises. Separate user requirements,
   repository constraints, and planner proposals. Preserve where requirements came from rather than turning “preserve
   settings” into an unexplained mandate for lossless editing. Design should expose consequential choices, not enumerate
   every defensive mechanism.
2. Give a fresh-context planning reviewer the original request, actual user decisions, proposed goal, and relevant
   repository context. Explicitly permit it to challenge planner-authored requirements. Ask which substantial mechanisms
   are necessary, what existing facilities or narrower supported behavior could replace them, and which simplifications
   would change an explicit user requirement. Require concrete alternatives; “no unnecessary complexity found” is a valid
   result. Reviewing only against the generated goal cannot reveal requirements that the goal itself invented.
3. During execution, reassess before implementing a substantial departure from the outline. A new parser, installation
   registry, recovery protocol, or compatibility layer is a signal, not a blacklist. Repeated corrective reviews of the
   same component should also prompt reconsideration of its necessity. Reuse existing delegation checkpoints. Prefer
   one independent planning review plus reviews triggered by substantial departures over periodic review swarms.
4. Bound unattended decision authority. The existing instruction to make the call, log it, and keep going for any
   unresolved fork needs a limit: routine choices can proceed, but a material scope expansion needs an agreed fallback
   or a user decision. Logging is not authorization. Removing unnecessary planner-invented machinery does not authorize
   silently dropping mandatory tracking or weakening a repository constraint.

The proposed review charter was:

> Identify where this plan does more than the user needs. For each substantial new mechanism, explain which requirement
> makes it necessary and whether an existing facility or a narrower supported behavior would suffice. Propose the
> smallest coherent design. You may challenge planner-authored requirements; identify any simplification that would
> require changing an explicit user requirement.

The existing delegation gate already asked whether the specification dropped something the user requested. The proposed
correction was to check the other direction too: whether it introduced unnecessary obligations. The intended outcome is
a reviewer able to say “this requirement should not have been introduced”, rather than only “this implementation needs
more safeguards”. The user requested this record in its own PR, then implementation in the next stacked PR, with an
adversarial review whose feedback must stay within that scope.
